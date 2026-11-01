-- Telemetría en producción, fase 1a (ver
-- docs/features-futuras/plan-observabilidad-en-produccion.md).
--
-- Hasta ahora la telemetría iba a `telemetria_diagnostico` del proyecto de
-- STAGING, escrita con la llave publicable (rol `anon`) y sin forma de
-- cruzarla con `dispositivos`. Esta migración crea su lugar en producción:
--
-- - Esquema `observabilidad`, NO expuesto por la API: los equipos no leen ni
--   escriben sus tablas. Sólo pueden ejecutar `registrar_telemetria`, que
--   valida todo y pone el equipo y la unidad desde el JWT (nunca del cuerpo).
--   `public.registrar_telemetria` es un envoltorio `security invoker` (el
--   linter no lo marca como `security definer` expuesto).
-- - El lote llega en formato COLUMNAR (ver "Formato del lote" en el plan):
--   cabecera común una vez, y por bloque los nombres de columna una vez y
--   después sólo filas de valores; la hora va en ms como diferencia con la
--   fila anterior del mismo bloque. Un campo ausente (no está en `cols`) y
--   uno `null` (está y vale `null`) se distinguen: van en bloques distintos.
-- - Todo o nada: un lote mal formado se rechaza entero (22023). Un `tipo`
--   que la nube todavía no conoce, un evento fuera de la ventana de tiempo o
--   demasiado grande NO tumban el lote (un equipo con una versión más nueva
--   no puede quedar trabado): se descartan, se cuentan y se informan.
-- - Sin duplicados: `lote_id` único. Un reintento responde `duplicado` sin
--   insertar otra vez.
-- - Retención: 30 días de detalle, resumen diario 1 año, y un tope de filas
--   de detalle (el proyecto está en el plan gratuito: 500 MB; si se pasa, la
--   base queda en sólo lectura). Lo hace `pg_cron` una vez al día. Los
--   números viven en `observabilidad.config`: cambiarlos es un `update`.
--
-- Errores (`errores`, `errores_grupo`), el traslado de `pg_net` y la Edge
-- Function receptora (brotli) van en la fase 1b.

-- pg_cron, como indica Supabase: en `pg_catalog`, no en `public` (el linter
-- marca `extension_in_public`). Sus tablas viven siempre en `cron`.
create extension if not exists pg_cron with schema pg_catalog;

create schema observabilidad;

comment on schema observabilidad is
  'Telemetría y errores de las apps. No expuesto por la API: se escribe sólo por registrar_telemetria. Ver docs/features-futuras/plan-observabilidad-en-produccion.md.';

-- `usage` sólo para poder ejecutar `registrar_telemetria` (desde el
-- envoltorio de `public`). Ninguna tabla de este esquema tiene permisos para
-- `authenticated`: con `usage` solo no se lee ni se escribe nada.
revoke all on schema observabilidad from public, anon;
grant usage on schema observabilidad to authenticated, service_role;

-- ---------------------------------------------------------------------------
-- Configuración: una sola fila.
-- ---------------------------------------------------------------------------
create table observabilidad.config (
  id boolean primary key default true check (id),
  dias_detalle int not null default 30 check (dias_detalle between 1 and 365),
  dias_resumen int not null default 365 check (dias_resumen between 30 and 3650),
  -- Filas de detalle (eventos + llamadas) como máximo. Por filas y no por
  -- bytes: después de un `delete` el tamaño en disco no baja enseguida (el
  -- espacio se reutiliza), y medir bytes haría que el tope siguiera borrando.
  -- ~600.000 filas son ~150 MB con índices.
  tope_filas_detalle bigint not null default 600000 check (tope_filas_detalle >= 10000),
  max_filas_lote int not null default 1000 check (max_filas_lote between 1 and 10000),
  max_bloques int not null default 64 check (max_bloques between 1 and 512),
  max_columnas int not null default 80 check (max_columnas between 1 and 512),
  max_bytes_evento int not null default 16384 check (max_bytes_evento between 256 and 65536),
  lotes_por_minuto int not null default 20 check (lotes_por_minuto between 1 and 600)
);

insert into observabilidad.config default values;

comment on table observabilidad.config is
  'Retención, tope y límites de la telemetría. Una sola fila; se cambia con un update en una migración.';

-- ---------------------------------------------------------------------------
-- Catálogo de eventos: el mismo que `src/telemetria/grupos.rs` (CATALOGO).
-- ---------------------------------------------------------------------------
create table observabilidad.catalogo_eventos (
  tipo text primary key check (tipo ~ '^[a-z][a-z0-9_]{0,63}$'),
  grupo text not null check (grupo in (
    'base', 'rendimiento', 'fluidez', 'sincronizacion', 'recursos', 'escaneo', 'depuracion'
  ))
);

insert into observabilidad.catalogo_eventos (tipo, grupo) values
  ('sesion_inicio', 'base'),
  ('arranque', 'base'),
  ('salida_anterior', 'base'),
  ('cierre', 'base'),
  ('panic', 'base'),
  ('error_js', 'base'),
  ('error_telemetria', 'base'),
  ('telemetria_descartada', 'base'),
  ('orden_telemetria', 'base'),
  ('cola_fallidos', 'base'),
  ('llamada_nucleo', 'rendimiento'),
  ('transicion_ingreso', 'rendimiento'),
  ('pantalla', 'rendimiento'),
  ('frames', 'fluidez'),
  ('ui_bloqueos', 'fluidez'),
  ('realtime', 'sincronizacion'),
  ('sincronizacion_auto', 'sincronizacion'),
  ('sesion_unidad', 'sincronizacion'),
  ('reloj', 'sincronizacion'),
  ('muestra_sistema', 'recursos'),
  ('muestra_webview', 'recursos'),
  ('memoria_baja', 'recursos'),
  ('ocr_sesion', 'escaneo'),
  ('camara_info', 'escaneo'),
  ('correccion_mrz', 'escaneo'),
  ('strictmode', 'depuracion'),
  ('retencion', 'depuracion');

-- ---------------------------------------------------------------------------
-- Detalle.
-- ---------------------------------------------------------------------------
create table observabilidad.sesiones (
  id uuid primary key,
  dispositivo_id uuid not null references public.dispositivos (id) on delete cascade,
  sitio_id uuid not null,
  plataforma text not null check (plataforma in ('movil', 'escritorio')),
  -- Sólo semver: el código de versión y el commit van aparte.
  version_app text not null check (version_app ~ '^[0-9]{1,4}\.[0-9]{1,4}\.[0-9]{1,4}$'),
  version_codigo int check (version_codigo > 0),
  "commit" text check ("commit" ~ '^[0-9a-f]{7,40}$'),
  ambiente text not null check (ambiente in ('produccion', 'staging')),
  iniciada_en timestamptz not null,
  sistema text check (pg_catalog.length(sistema) <= 80),
  modelo text check (pg_catalog.length(modelo) <= 80),
  ram_total_mb int check (ram_total_mb > 0),
  detalle jsonb not null default '{}' check (pg_catalog.jsonb_typeof(detalle) = 'object'),
  recibida_en timestamptz not null default now()
);

create index sesiones_dispositivo_idx on observabilidad.sesiones (dispositivo_id, iniciada_en desc);

create table observabilidad.eventos (
  id bigint generated always as identity primary key,
  sesion_id uuid not null references observabilidad.sesiones (id) on delete cascade,
  dispositivo_id uuid not null,
  ocurrido_en timestamptz not null,
  recibido_en timestamptz not null default now(),
  tipo text not null,
  grupo text not null,
  pantalla text check (pg_catalog.length(pantalla) <= 80),
  datos jsonb not null check (pg_catalog.jsonb_typeof(datos) = 'object')
);

create index eventos_dispositivo_idx on observabilidad.eventos (dispositivo_id, ocurrido_en desc);
create index eventos_ocurrido_idx on observabilidad.eventos (ocurrido_en);
create index eventos_sesion_idx on observabilidad.eventos (sesion_id);

-- `llamada_nucleo` (dos tercios de las filas) con columnas propias.
create table observabilidad.llamadas (
  id bigint generated always as identity primary key,
  sesion_id uuid not null references observabilidad.sesiones (id) on delete cascade,
  dispositivo_id uuid not null,
  ocurrido_en timestamptz not null,
  recibido_en timestamptz not null default now(),
  -- Nombre canónico (docs/telemetria-contrato.md).
  operacion text not null check (operacion ~ '^[a-z][a-z0-9_]{0,79}$'),
  llamadas int not null check (llamadas >= 0),
  errores int not null default 0 check (errores >= 0),
  p50_ms numeric check (p50_ms >= 0),
  p90_ms numeric check (p90_ms >= 0),
  max_ms numeric check (max_ms >= 0),
  total_ms numeric check (total_ms >= 0),
  red_tipo text check (red_tipo in (
    'sin_red', 'sin_internet', 'wifi', 'datos', 'ethernet', 'con_red', 'otra'
  )),
  errores_por_tipo jsonb check (errores_por_tipo is null or pg_catalog.jsonb_typeof(errores_por_tipo) = 'object')
);

create index llamadas_dispositivo_idx on observabilidad.llamadas (dispositivo_id, ocurrido_en desc);
create index llamadas_ocurrido_idx on observabilidad.llamadas (ocurrido_en);
create index llamadas_sesion_idx on observabilidad.llamadas (sesion_id);

-- Un lote recibido: evita duplicados y da el ritmo por equipo.
create table observabilidad.lotes (
  lote_id uuid primary key,
  dispositivo_id uuid not null,
  recibido_en timestamptz not null default now(),
  esquema int,
  filas int not null default 0,
  descartadas int not null default 0
);

create index lotes_dispositivo_idx on observabilidad.lotes (dispositivo_id, recibido_en desc);

-- ---------------------------------------------------------------------------
-- Resumen diario (día de Costa Rica): lo que se guarda un año y sirve para
-- comparar versiones. Los percentiles son aproximados: percentil de los p50
-- y p90 de cada ventana de medición, no de cada llamada.
-- ---------------------------------------------------------------------------
create table observabilidad.resumen_llamadas_diario (
  dia date not null,
  dispositivo_id uuid not null,
  version_app text not null,
  plataforma text not null,
  operacion text not null,
  llamadas bigint not null,
  errores bigint not null,
  p50_ms numeric,
  p90_ms numeric,
  max_ms numeric,
  primary key (dia, dispositivo_id, version_app, operacion)
);

create table observabilidad.resumen_eventos_diario (
  dia date not null,
  dispositivo_id uuid not null,
  version_app text not null,
  plataforma text not null,
  tipo text not null,
  eventos bigint not null,
  primary key (dia, dispositivo_id, version_app, tipo)
);

-- Qué hizo el mantenimiento diario. `accion = 'tope'` es la señal para la
-- alerta por correo de la fase 5.
create table observabilidad.mantenimiento_registro (
  id bigint generated always as identity primary key,
  ejecutado_en timestamptz not null default now(),
  accion text not null check (accion in ('diario', 'tope')),
  detalle jsonb not null default '{}'
);

-- Ninguna tabla tiene permisos para la API. RLS encendida igual, sin
-- políticas: si algún día se diera un GRANT por error, no se ve nada.
do $$
declare
  v_tabla text;
begin
  foreach v_tabla in array array[
    'config', 'catalogo_eventos', 'sesiones', 'eventos', 'llamadas', 'lotes',
    'resumen_llamadas_diario', 'resumen_eventos_diario', 'mantenimiento_registro'
  ] loop
    execute pg_catalog.format('alter table observabilidad.%I enable row level security', v_tabla);
    execute pg_catalog.format('revoke all on observabilidad.%I from public, anon, authenticated, service_role', v_tabla);
  end loop;
end $$;

-- ---------------------------------------------------------------------------
-- Recepción.
-- ---------------------------------------------------------------------------
--
-- p_lote:
-- {
--   "lote_id": uuid,
--   "esquema": 3,
--   "base_ms": <epoch ms>,
--   "sesion": { "id", "plataforma", "version_app", "version_codigo", "commit",
--               "ambiente", "iniciada_en", "sistema", "modelo",
--               "ram_total_mb", "detalle" },
--   "bloques": [ { "tipo": "llamada_nucleo",
--                  "cols": ["nombre", "llamadas", ...],
--                  "filas": [[<dt_ms>, <valor por col>...], ...] }, ... ]
-- }
--
-- Responde { "estado": "recibido" | "duplicado", "lote_id", "filas",
-- "descartadas_tipo", "descartadas_otras" }. Un lote mal formado es un
-- error 22023 (el equipo lo descarta, no lo reintenta); el ritmo excedido,
-- 53400 (el equipo reintenta más tarde).
create function observabilidad.registrar_telemetria(p_lote jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
  v_cfg observabilidad.config;
  v_lote_id uuid;
  v_base bigint;
  v_sesion jsonb;
  v_sesion_id uuid;
  v_dueno uuid;
  v_total int;
  v_llamadas int;
  v_eventos int;
  v_desconocidas int;
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede enviar telemetría' using errcode = '42501';
  end if;

  select * into strict v_cfg from observabilidad.config;

  -- Estructura de la cabecera.
  if pg_catalog.jsonb_typeof(p_lote) is distinct from 'object'
     or pg_catalog.jsonb_typeof(p_lote -> 'lote_id') is distinct from 'string'
     or pg_catalog.jsonb_typeof(p_lote -> 'base_ms') is distinct from 'number'
     or pg_catalog.jsonb_typeof(p_lote -> 'sesion') is distinct from 'object'
     or pg_catalog.jsonb_typeof(p_lote -> 'sesion' -> 'id') is distinct from 'string'
     or pg_catalog.jsonb_typeof(p_lote -> 'bloques') is distinct from 'array'
  then
    raise exception 'Lote mal formado: cabecera' using errcode = '22023';
  end if;

  v_lote_id := (p_lote ->> 'lote_id')::uuid;
  v_base := (p_lote ->> 'base_ms')::bigint;
  v_sesion := p_lote -> 'sesion';
  v_sesion_id := (v_sesion ->> 'id')::uuid;

  -- Ritmo por equipo.
  if (select pg_catalog.count(*) from observabilidad.lotes l
       where l.dispositivo_id = v_equipo.id
         and l.recibido_en > pg_catalog.now() - interval '1 minute') >= v_cfg.lotes_por_minuto then
    raise exception 'Demasiados lotes por minuto' using errcode = '53400';
  end if;

  -- Estructura de los bloques y de cada fila: todo o nada.
  if pg_catalog.jsonb_array_length(p_lote -> 'bloques') > v_cfg.max_bloques then
    raise exception 'Lote mal formado: demasiados bloques' using errcode = '22023';
  end if;

  if exists (
    select 1
      from pg_catalog.jsonb_array_elements(p_lote -> 'bloques') b(bl)
     where pg_catalog.jsonb_typeof(b.bl) is distinct from 'object'
        or pg_catalog.jsonb_typeof(b.bl -> 'tipo') is distinct from 'string'
        or pg_catalog.jsonb_typeof(b.bl -> 'cols') is distinct from 'array'
        or pg_catalog.jsonb_typeof(b.bl -> 'filas') is distinct from 'array'
        or pg_catalog.jsonb_array_length(b.bl -> 'cols') > v_cfg.max_columnas
        or exists (select 1 from pg_catalog.jsonb_array_elements(b.bl -> 'cols') c(col)
                    where pg_catalog.jsonb_typeof(c.col) is distinct from 'string'
                       or pg_catalog.length(c.col #>> '{}') not between 1 and 64)
        or (select pg_catalog.count(distinct c.col) <> pg_catalog.count(*)
              from pg_catalog.jsonb_array_elements_text(b.bl -> 'cols') c(col))
  ) then
    raise exception 'Lote mal formado: bloque' using errcode = '22023';
  end if;

  if exists (
    select 1
      from pg_catalog.jsonb_array_elements(p_lote -> 'bloques') b(bl),
           pg_catalog.jsonb_array_elements(b.bl -> 'filas') f(fila)
     where pg_catalog.jsonb_typeof(f.fila) is distinct from 'array'
        or pg_catalog.jsonb_array_length(f.fila) <> pg_catalog.jsonb_array_length(b.bl -> 'cols') + 1
        or pg_catalog.jsonb_typeof(f.fila -> 0) is distinct from 'number'
        or (f.fila ->> 0)::numeric < 0
        or (f.fila ->> 0)::numeric <> pg_catalog.trunc((f.fila ->> 0)::numeric)
  ) then
    raise exception 'Lote mal formado: fila' using errcode = '22023';
  end if;

  select pg_catalog.count(*)::int into v_total
    from pg_catalog.jsonb_array_elements(p_lote -> 'bloques') b(bl),
         pg_catalog.jsonb_array_elements(b.bl -> 'filas') f(fila);
  if v_total > v_cfg.max_filas_lote then
    raise exception 'Lote mal formado: demasiadas filas' using errcode = '22023';
  end if;

  -- Duplicado: después de validar, para no "gastar" un lote_id con un lote
  -- inválido. Si dos envíos iguales llegan a la vez, el segundo espera al
  -- índice único y sale por aquí.
  insert into observabilidad.lotes (lote_id, dispositivo_id, esquema)
  values (v_lote_id, v_equipo.id, (p_lote ->> 'esquema')::int)
  on conflict (lote_id) do nothing;
  if not found then
    return pg_catalog.jsonb_build_object('estado', 'duplicado', 'lote_id', v_lote_id);
  end if;

  -- La sesión: de este equipo, o nueva. El equipo y la unidad salen del JWT.
  select s.dispositivo_id into v_dueno from observabilidad.sesiones s where s.id = v_sesion_id;
  if found and v_dueno <> v_equipo.id then
    raise exception 'La sesión es de otro equipo' using errcode = '42501';
  end if;
  if not found then
    insert into observabilidad.sesiones (
      id, dispositivo_id, sitio_id, plataforma, version_app, version_codigo, "commit",
      ambiente, iniciada_en, sistema, modelo, ram_total_mb, detalle
    ) values (
      v_sesion_id, v_equipo.id, v_equipo.sitio_id,
      v_sesion ->> 'plataforma',
      v_sesion ->> 'version_app',
      (v_sesion ->> 'version_codigo')::int,
      v_sesion ->> 'commit',
      coalesce(v_sesion ->> 'ambiente', 'produccion'),
      (v_sesion ->> 'iniciada_en')::timestamptz,
      v_sesion ->> 'sistema',
      v_sesion ->> 'modelo',
      (v_sesion ->> 'ram_total_mb')::int,
      coalesce(v_sesion -> 'detalle', '{}'::jsonb)
    );
  end if;

  -- Decodificar el columnar e insertar. Un solo comando: o entra todo lo
  -- válido o, si una fila rompe un CHECK o un tipo de dato (23514, 22P02),
  -- nada: el lote entero se rechaza y el equipo lo trata como mal formado.
  with bloques as (
    select b.n as nb, b.bl ->> 'tipo' as tipo, b.bl -> 'cols' as cols, b.bl -> 'filas' as filas
      from pg_catalog.jsonb_array_elements(p_lote -> 'bloques') with ordinality b(bl, n)
  ), filas as (
    select bq.tipo, bq.cols, f.fila,
           v_base + pg_catalog.sum((f.fila ->> 0)::bigint) over (partition by bq.nb order by f.n) as ms
      from bloques bq,
           pg_catalog.jsonb_array_elements(bq.filas) with ordinality f(fila, n)
  ), decodificadas as (
    select fl.tipo,
           pg_catalog.to_timestamp(fl.ms / 1000.0) as ocurrido_en,
           coalesce((select pg_catalog.jsonb_object_agg(c.col, fl.fila -> (c.i::int))
                       from pg_catalog.jsonb_array_elements_text(fl.cols) with ordinality c(col, i)),
                    '{}'::jsonb) as datos,
           ce.grupo
      from filas fl
      left join observabilidad.catalogo_eventos ce on ce.tipo = fl.tipo
  ), validas as (
    select d.*
      from decodificadas d
     where d.grupo is not null
       and d.ocurrido_en >= pg_catalog.now() - pg_catalog.make_interval(days => v_cfg.dias_detalle)
       and d.ocurrido_en <= pg_catalog.now() + interval '1 day'
       and pg_catalog.pg_column_size(d.datos) <= v_cfg.max_bytes_evento
  ), ins_llamadas as (
    insert into observabilidad.llamadas (
      sesion_id, dispositivo_id, ocurrido_en, operacion, llamadas, errores,
      p50_ms, p90_ms, max_ms, total_ms, red_tipo, errores_por_tipo
    )
    select v_sesion_id, v_equipo.id, v.ocurrido_en,
           v.datos ->> 'nombre',
           (v.datos ->> 'llamadas')::int,
           coalesce((v.datos ->> 'errores')::int, 0),
           (v.datos ->> 'p50_ms')::numeric,
           (v.datos ->> 'p90_ms')::numeric,
           (v.datos ->> 'max_ms')::numeric,
           (v.datos ->> 'total_ms')::numeric,
           v.datos ->> 'red_tipo',
           nullif(v.datos -> 'errores_por_tipo', 'null'::jsonb)
      from validas v
     where v.tipo = 'llamada_nucleo'
    returning 1
  ), ins_eventos as (
    insert into observabilidad.eventos (sesion_id, dispositivo_id, ocurrido_en, tipo, grupo, pantalla, datos)
    select v_sesion_id, v_equipo.id, v.ocurrido_en, v.tipo, v.grupo,
           v.datos ->> 'pantalla',
           v.datos - 'pantalla'
      from validas v
     where v.tipo <> 'llamada_nucleo'
    returning 1
  )
  select (select pg_catalog.count(*) from ins_llamadas)::int,
         (select pg_catalog.count(*) from ins_eventos)::int,
         (select pg_catalog.count(*) from decodificadas where grupo is null)::int
    into v_llamadas, v_eventos, v_desconocidas;

  update observabilidad.lotes
     set filas = v_llamadas + v_eventos,
         descartadas = v_total - (v_llamadas + v_eventos)
   where lote_id = v_lote_id;

  return pg_catalog.jsonb_build_object(
    'estado', 'recibido',
    'lote_id', v_lote_id,
    'filas', v_llamadas + v_eventos,
    'descartadas_tipo', v_desconocidas,
    'descartadas_otras', v_total - (v_llamadas + v_eventos) - v_desconocidas
  );
end;
$$;

revoke all on function observabilidad.registrar_telemetria(jsonb) from public;
grant execute on function observabilidad.registrar_telemetria(jsonb) to authenticated, service_role;

-- Lo que expone la API: sin `security definer` (el linter no lo marca); toda
-- la validación está en la función de `observabilidad`.
create function public.registrar_telemetria(p_lote jsonb)
returns jsonb
language sql
security invoker
set search_path = ''
as $$
  select observabilidad.registrar_telemetria(p_lote);
$$;

revoke all on function public.registrar_telemetria(jsonb) from public;
grant execute on function public.registrar_telemetria(jsonb) to authenticated, service_role;

comment on function public.registrar_telemetria(jsonb) is
  'Recibe un lote columnar de telemetría del equipo que llama. Ver observabilidad.registrar_telemetria.';

-- ---------------------------------------------------------------------------
-- Mantenimiento diario.
-- ---------------------------------------------------------------------------

-- Recalcula el resumen de los días de Costa Rica desde `p_desde` (idempotente:
-- correrlo dos veces deja lo mismo).
create function observabilidad.resumir_desde(p_desde date)
returns void
language sql
security definer
set search_path = ''
as $$
  insert into observabilidad.resumen_llamadas_diario as r (
    dia, dispositivo_id, version_app, plataforma, operacion,
    llamadas, errores, p50_ms, p90_ms, max_ms
  )
  select (l.ocurrido_en at time zone 'America/Costa_Rica')::date,
         l.dispositivo_id, s.version_app, s.plataforma, l.operacion,
         pg_catalog.sum(l.llamadas), pg_catalog.sum(l.errores),
         (pg_catalog.percentile_cont(0.5) within group (order by l.p50_ms))::numeric,
         (pg_catalog.percentile_cont(0.9) within group (order by l.p90_ms))::numeric,
         pg_catalog.max(l.max_ms)
    from observabilidad.llamadas l
    join observabilidad.sesiones s on s.id = l.sesion_id
   where l.ocurrido_en >= (p_desde::timestamp at time zone 'America/Costa_Rica')
   group by 1, 2, 3, 4, 5
  on conflict (dia, dispositivo_id, version_app, operacion) do update
     set plataforma = excluded.plataforma,
         llamadas = excluded.llamadas,
         errores = excluded.errores,
         p50_ms = excluded.p50_ms,
         p90_ms = excluded.p90_ms,
         max_ms = excluded.max_ms;

  insert into observabilidad.resumen_eventos_diario as r (
    dia, dispositivo_id, version_app, plataforma, tipo, eventos
  )
  select (e.ocurrido_en at time zone 'America/Costa_Rica')::date,
         e.dispositivo_id, s.version_app, s.plataforma, e.tipo, pg_catalog.count(*)
    from observabilidad.eventos e
    join observabilidad.sesiones s on s.id = e.sesion_id
   where e.ocurrido_en >= (p_desde::timestamp at time zone 'America/Costa_Rica')
   group by 1, 2, 3, 4, 5
  on conflict (dia, dispositivo_id, version_app, tipo) do update
     set plataforma = excluded.plataforma,
         eventos = excluded.eventos;
$$;

revoke all on function observabilidad.resumir_desde(date) from public;

-- Resumen de los últimos 3 días, purga por retención y tope de filas. Lo
-- corre pg_cron una vez al día; también se puede correr a mano.
create function observabilidad.mantener()
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_cfg observabilidad.config;
  v_hoy date := (pg_catalog.now() at time zone 'America/Costa_Rica')::date;
  v_limite timestamptz;
  v_eventos bigint;
  v_llamadas bigint;
  v_lotes bigint;
  v_sesiones bigint;
  v_resumen bigint;
  v_filas bigint;
  v_por_tope bigint := 0;
  v_corte timestamptz;
  v_borradas bigint;
  v_vueltas int := 0;
  v_resultado jsonb;
begin
  select * into strict v_cfg from observabilidad.config;

  -- 1. Resumen antes de purgar: lo que se borra hoy ya quedó resumido.
  perform observabilidad.resumir_desde(v_hoy - 2);

  -- 2. Retención del detalle.
  v_limite := pg_catalog.now() - pg_catalog.make_interval(days => v_cfg.dias_detalle);
  delete from observabilidad.eventos where ocurrido_en < v_limite;
  get diagnostics v_eventos = row_count;
  delete from observabilidad.llamadas where ocurrido_en < v_limite;
  get diagnostics v_llamadas = row_count;
  delete from observabilidad.lotes where recibido_en < v_limite;
  get diagnostics v_lotes = row_count;
  delete from observabilidad.sesiones s
   where s.iniciada_en < v_limite
     and not exists (select 1 from observabilidad.eventos e where e.sesion_id = s.id)
     and not exists (select 1 from observabilidad.llamadas l where l.sesion_id = s.id);
  get diagnostics v_sesiones = row_count;

  -- 3. Retención del resumen.
  delete from observabilidad.resumen_llamadas_diario where dia < v_hoy - v_cfg.dias_resumen;
  get diagnostics v_resumen = row_count;
  delete from observabilidad.resumen_eventos_diario where dia < v_hoy - v_cfg.dias_resumen;
  get diagnostics v_borradas = row_count;
  v_resumen := v_resumen + v_borradas;

  -- 4. Tope: mientras haya más filas de detalle que el tope, se borra el día
  --    más viejo. Antes, se resume todo lo que queda (por si una corrida
  --    anterior no llegó a resumir algún día): nada sale del detalle sin
  --    quedar en el resumen.
  select (select pg_catalog.count(*) from observabilidad.eventos)
       + (select pg_catalog.count(*) from observabilidad.llamadas)
    into v_filas;
  if v_filas > v_cfg.tope_filas_detalle then
    perform observabilidad.resumir_desde((
      select (least(
                (select pg_catalog.min(ocurrido_en) from observabilidad.eventos),
                (select pg_catalog.min(ocurrido_en) from observabilidad.llamadas)
              ) at time zone 'America/Costa_Rica')::date));
  end if;

  loop
    select (select pg_catalog.count(*) from observabilidad.eventos)
         + (select pg_catalog.count(*) from observabilidad.llamadas)
      into v_filas;
    exit when v_filas <= v_cfg.tope_filas_detalle or v_vueltas >= 400;
    v_vueltas := v_vueltas + 1;

    select least(
             (select pg_catalog.min(ocurrido_en) from observabilidad.eventos),
             (select pg_catalog.min(ocurrido_en) from observabilidad.llamadas)
           ) + interval '1 day'
      into v_corte;
    exit when v_corte is null;

    delete from observabilidad.eventos where ocurrido_en < v_corte;
    get diagnostics v_borradas = row_count;
    v_por_tope := v_por_tope + v_borradas;
    delete from observabilidad.llamadas where ocurrido_en < v_corte;
    get diagnostics v_borradas = row_count;
    v_por_tope := v_por_tope + v_borradas;
  end loop;

  if v_por_tope > 0 then
    insert into observabilidad.mantenimiento_registro (accion, detalle)
    values ('tope', pg_catalog.jsonb_build_object(
      'filas_borradas', v_por_tope,
      'tope_filas_detalle', v_cfg.tope_filas_detalle,
      'filas_despues', v_filas
    ));
  end if;

  v_resultado := pg_catalog.jsonb_build_object(
    'eventos', v_eventos,
    'llamadas', v_llamadas,
    'lotes', v_lotes,
    'sesiones', v_sesiones,
    'resumen', v_resumen,
    'por_tope', v_por_tope,
    'filas_detalle', v_filas
  );
  insert into observabilidad.mantenimiento_registro (accion, detalle) values ('diario', v_resultado);
  return v_resultado;
end;
$$;

revoke all on function observabilidad.mantener() from public;

-- 03:07 de Costa Rica (09:07 UTC): fuera del horario de mayor uso. Con el
-- mismo nombre, volver a correr la migración reemplaza la tarea, no la
-- duplica.
select cron.schedule(
  'observabilidad-mantenimiento-diario',
  '7 9 * * *',
  'select observabilidad.mantener()'
);

-- Errores en producción, fase 1b (ver
-- docs/features-futuras/plan-observabilidad-en-produccion.md). Va después de
-- 20261101120000_observabilidad_telemetria y 20261101130000_observabilidad_panel.
--
-- Hasta ahora los errores iban a `errores_app` del proyecto de STAGING, con
-- una clave compilada en las apps (`x-clave-errores`). Ahora llegan a
-- producción como la telemetría:
--
-- - Los manda el equipo con SU token a la Edge Function `observabilidad`
--   (`/errores`), que limpia otra vez el texto de datos personales, calcula la
--   huella y llama a `registrar_errores` reenviando el mismo token. El
--   equipo, la unidad y la plataforma los pone la base desde el JWT, nunca
--   del cuerpo.
-- - `errores_grupo`: un error agrupado por huella, con su estado
--   (`nuevo`/`visto`/`resuelto`/`ignorado`). Si un error `resuelto` vuelve a
--   aparecer en una versión que no estaba entre las vistas al resolverlo, el
--   grupo pasa a `nuevo` (regresión). Un equipo viejo que sigue con la
--   versión del error no lo reabre.
-- - Sin duplicados: `event_id` único (el `id` del reporte). Un reintento no
--   inserta otra vez.
-- - Todo o nada: un reporte mal formado rechaza la llamada entera (22023).
-- - Retención: 180 días (`observabilidad.config.dias_errores`), con su propia
--   tarea diaria de pg_cron.
-- - Lectura y cambio de estado para el panel, sólo para el administrador.
--
-- Mismo patrón que la telemetría: la función real vive en `observabilidad`
-- (no expuesto por la API) y es `security definer`; en `public` queda un
-- envoltorio `security invoker`, así el linter no suma avisos.

-- ---------------------------------------------------------------------------
-- Configuración.
-- ---------------------------------------------------------------------------
alter table observabilidad.config
  add column dias_errores int not null default 180 check (dias_errores between 7 and 730),
  add column max_reportes_lote int not null default 50 check (max_reportes_lote between 1 and 500),
  add column errores_por_minuto int not null default 60 check (errores_por_minuto between 1 and 6000);

-- El mantenimiento de errores también deja su rastro.
alter table observabilidad.mantenimiento_registro
  drop constraint mantenimiento_registro_accion_check,
  add constraint mantenimiento_registro_accion_check check (accion in ('diario', 'tope', 'errores'));

-- ---------------------------------------------------------------------------
-- Tablas.
-- ---------------------------------------------------------------------------
create table observabilidad.errores_grupo (
  -- La calcula la Edge Function (ver `calcularHuella`): plataforma, origen,
  -- título sin números y los últimos marcos de código propio.
  huella text primary key check (huella ~ '^[0-9a-f]{16}$'),
  plataforma text not null check (plataforma in ('movil', 'escritorio')),
  origen text not null check (origen in ('panico', 'excepcion', 'anr', 'nativo', 'js', 'nucleo')),
  titulo text not null check (pg_catalog.length(titulo) between 1 and 300),
  primera_vez timestamptz not null default now(),
  ultima_vez timestamptz not null default now(),
  ocurrencias bigint not null default 0 check (ocurrencias >= 0),
  -- Versiones en las que apareció.
  versiones text[] not null default '{}',
  estado text not null default 'nuevo' check (estado in ('nuevo', 'visto', 'resuelto', 'ignorado')),
  estado_cambiado_en timestamptz not null default now(),
  -- Las versiones que había al marcarlo `resuelto`: una ocurrencia en otra
  -- versión es una regresión.
  versiones_al_resolver text[] not null default '{}'
);

create index errores_grupo_ultima_idx on observabilidad.errores_grupo (ultima_vez desc);

create table observabilidad.errores (
  id bigint generated always as identity primary key,
  huella text not null references observabilidad.errores_grupo (huella) on delete cascade,
  event_id text not null unique check (event_id ~ '^[A-Za-z0-9_-]{8,64}$'),
  dispositivo_id uuid not null references public.dispositivos (id) on delete cascade,
  sitio_id uuid not null,
  plataforma text not null check (plataforma in ('movil', 'escritorio')),
  ocurrido_en timestamptz not null,
  recibido_en timestamptz not null default now(),
  version_app text check (pg_catalog.length(version_app) <= 80),
  nivel text not null check (nivel in ('fatal', 'error')),
  origen text not null check (origen in ('panico', 'excepcion', 'anr', 'nativo', 'js', 'nucleo')),
  titulo text not null check (pg_catalog.length(titulo) between 1 and 300),
  -- Del marco más viejo al más nuevo (donde ocurrió). Ya limpios.
  traza jsonb not null default '[]' check (pg_catalog.jsonb_typeof(traza) = 'array'
                                           and pg_catalog.jsonb_array_length(traza) <= 30),
  pantalla text check (pg_catalog.length(pantalla) <= 60),
  -- Las últimas operaciones: nombre, si salió bien y cuándo. Nunca datos.
  operaciones jsonb not null default '[]' check (pg_catalog.jsonb_typeof(operaciones) = 'array'
                                                 and pg_catalog.jsonb_array_length(operaciones) <= 20),
  -- Datos del equipo (sistema, modelo, arquitectura…).
  contexto jsonb not null default '{}' check (pg_catalog.jsonb_typeof(contexto) = 'object')
);

create index errores_huella_idx on observabilidad.errores (huella, recibido_en desc);
create index errores_dispositivo_idx on observabilidad.errores (dispositivo_id, recibido_en desc);
create index errores_recibido_idx on observabilidad.errores (recibido_en);

-- Como el resto del esquema: sin permisos para la API y con RLS sin
-- políticas por si algún día se diera un GRANT por error.
alter table observabilidad.errores_grupo enable row level security;
alter table observabilidad.errores enable row level security;
revoke all on observabilidad.errores_grupo from public, anon, authenticated, service_role;
revoke all on observabilidad.errores from public, anon, authenticated, service_role;

-- ---------------------------------------------------------------------------
-- Recepción.
-- ---------------------------------------------------------------------------
--
-- p_reportes: arreglo de reportes ya limpios por la Edge Function:
--   { "event_id", "huella", "ocurrido_en", "version_app", "nivel", "origen",
--     "titulo", "traza", "pantalla", "operaciones", "contexto" }
--
-- Los reportes como filas. Un mismo `event_id` repetido dentro del lote
-- cuenta una vez; una hora imposible (más de un día en el futuro) o ausente
-- se cambia por la de recepción. Interna: sólo la usa registrar_errores.
create function observabilidad.leer_reportes(p_reportes jsonb)
returns table (
  event_id text, huella text, ocurrido_en timestamptz, version_app text, nivel text,
  origen text, titulo text, traza jsonb, pantalla text, operaciones jsonb, contexto jsonb
)
language sql
stable
set search_path = ''
as $$
  select distinct on (r.rep ->> 'event_id')
         r.rep ->> 'event_id',
         r.rep ->> 'huella',
         case when (r.rep ->> 'ocurrido_en')::timestamptz <= pg_catalog.now() + interval '1 day'
              then (r.rep ->> 'ocurrido_en')::timestamptz
              else pg_catalog.now() end,
         nullif(r.rep ->> 'version_app', ''),
         r.rep ->> 'nivel',
         r.rep ->> 'origen',
         r.rep ->> 'titulo',
         coalesce(nullif(r.rep -> 'traza', 'null'::jsonb), '[]'::jsonb),
         nullif(r.rep ->> 'pantalla', ''),
         coalesce(nullif(r.rep -> 'operaciones', 'null'::jsonb), '[]'::jsonb),
         coalesce(nullif(r.rep -> 'contexto', 'null'::jsonb), '{}'::jsonb)
    from pg_catalog.jsonb_array_elements(p_reportes) with ordinality r(rep, n)
   order by r.rep ->> 'event_id', r.n;
$$;

revoke all on function observabilidad.leer_reportes(jsonb) from public;

-- Responde { "estado": "recibido", "guardados", "repetidos",
-- "grupos_nuevos", "regresiones" }. Un lote mal formado es 22023 (el equipo
-- lo descarta); el ritmo excedido, 53400 (reintenta más tarde).
create function observabilidad.registrar_errores(p_reportes jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
  v_cfg observabilidad.config;
  v_plataforma text;
  v_total int;
  v_guardados int;
  v_grupos_nuevos int;
  v_regresiones int;
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede enviar errores' using errcode = '42501';
  end if;
  -- La plataforma sale del equipo, no del cuerpo. Un visor no tiene app.
  v_plataforma := case v_equipo.tipo when 'mobile' then 'movil' when 'pc' then 'escritorio' end;
  if v_plataforma is null then
    raise exception 'Este tipo de equipo no envía errores' using errcode = '42501';
  end if;

  select * into strict v_cfg from observabilidad.config;

  if pg_catalog.jsonb_typeof(p_reportes) is distinct from 'array' then
    raise exception 'Lote de errores mal formado: no es una lista' using errcode = '22023';
  end if;
  v_total := pg_catalog.jsonb_array_length(p_reportes);
  if v_total > v_cfg.max_reportes_lote then
    raise exception 'Lote de errores mal formado: demasiados reportes' using errcode = '22023';
  end if;
  if v_total = 0 then
    return pg_catalog.jsonb_build_object('estado', 'recibido', 'guardados', 0, 'repetidos', 0,
                                         'grupos_nuevos', 0, 'regresiones', 0);
  end if;

  -- Ritmo por equipo.
  if (select pg_catalog.count(*) from observabilidad.errores e
       where e.dispositivo_id = v_equipo.id
         and e.recibido_en > pg_catalog.now() - interval '1 minute') + v_total > v_cfg.errores_por_minuto then
    raise exception 'Demasiados errores por minuto' using errcode = '53400';
  end if;

  -- Estructura de cada reporte: todo o nada. Los CHECK de las tablas cubren
  -- largos y listas; aquí, lo que sin esto entraría como nulo o con el tipo
  -- equivocado.
  if exists (
    select 1
      from pg_catalog.jsonb_array_elements(p_reportes) r(rep)
     where pg_catalog.jsonb_typeof(r.rep) is distinct from 'object'
        or pg_catalog.jsonb_typeof(r.rep -> 'event_id') is distinct from 'string'
        or pg_catalog.jsonb_typeof(r.rep -> 'huella') is distinct from 'string'
        or pg_catalog.jsonb_typeof(r.rep -> 'titulo') is distinct from 'string'
        or pg_catalog.jsonb_typeof(r.rep -> 'origen') is distinct from 'string'
        or pg_catalog.jsonb_typeof(r.rep -> 'nivel') is distinct from 'string'
        or (r.rep ? 'traza' and pg_catalog.jsonb_typeof(r.rep -> 'traza') not in ('array', 'null'))
        or (r.rep ? 'operaciones' and pg_catalog.jsonb_typeof(r.rep -> 'operaciones') not in ('array', 'null'))
        or (r.rep ? 'contexto' and pg_catalog.jsonb_typeof(r.rep -> 'contexto') not in ('object', 'null'))
  ) then
    raise exception 'Lote de errores mal formado: reporte' using errcode = '22023';
  end if;

  -- Los grupos que todavía no existen (con 0 ocurrencias: se suman abajo).
  insert into observabilidad.errores_grupo (huella, plataforma, origen, titulo, primera_vez, ultima_vez)
  select distinct on (en.huella) en.huella, v_plataforma, en.origen, en.titulo, en.ocurrido_en, en.ocurrido_en
    from observabilidad.leer_reportes(p_reportes) en
   where not exists (select 1 from observabilidad.errores e where e.event_id = en.event_id)
   order by en.huella, en.ocurrido_en
  on conflict (huella) do nothing;
  get diagnostics v_grupos_nuevos = row_count;

  -- Guardar lo que no estaba y sumarlo a su grupo. Regresión: el grupo
  -- estaba `resuelto` y llega en una versión que no se había visto al
  -- resolverlo.
  with guardados as (
    insert into observabilidad.errores (
      huella, event_id, dispositivo_id, sitio_id, plataforma, ocurrido_en, version_app,
      nivel, origen, titulo, traza, pantalla, operaciones, contexto
    )
    select en.huella, en.event_id, v_equipo.id, v_equipo.sitio_id, v_plataforma, en.ocurrido_en,
           en.version_app, en.nivel, en.origen, en.titulo, en.traza, en.pantalla, en.operaciones,
           en.contexto
      from observabilidad.leer_reportes(p_reportes) en
    on conflict (event_id) do nothing
    returning huella, ocurrido_en, version_app
  ), por_grupo as (
    select g.huella,
           pg_catalog.count(*) as cuantos,
           pg_catalog.min(g.ocurrido_en) as primera,
           pg_catalog.max(g.ocurrido_en) as ultima,
           coalesce(pg_catalog.array_agg(distinct g.version_app) filter (where g.version_app is not null),
                    '{}') as versiones
      from guardados g
     group by g.huella
  ), con_regresion as (
    select pg.*,
           (eg.estado = 'resuelto' and not (pg.versiones <@ eg.versiones_al_resolver)) as regresion
      from por_grupo pg
      join observabilidad.errores_grupo eg on eg.huella = pg.huella
  ), actualizados as (
    update observabilidad.errores_grupo eg
       set ocurrencias = eg.ocurrencias + cr.cuantos,
           primera_vez = least(eg.primera_vez, cr.primera),
           ultima_vez = greatest(eg.ultima_vez, cr.ultima),
           versiones = (select coalesce(pg_catalog.array_agg(distinct u.v order by u.v), '{}')
                          from pg_catalog.unnest(eg.versiones || cr.versiones) u(v)),
           estado = case when cr.regresion then 'nuevo' else eg.estado end,
           estado_cambiado_en = case when cr.regresion then pg_catalog.now() else eg.estado_cambiado_en end
      from con_regresion cr
     where eg.huella = cr.huella
    returning 1
  )
  select (select coalesce(pg_catalog.sum(cr.cuantos), 0) from con_regresion cr)::int,
         (select pg_catalog.count(*) from con_regresion cr where cr.regresion)::int
    into v_guardados, v_regresiones;
  -- `actualizados` se ejecuta aunque no se lea (un CTE que modifica siempre
  -- corre completo).

  return pg_catalog.jsonb_build_object(
    'estado', 'recibido',
    'guardados', v_guardados,
    'repetidos', v_total - v_guardados,
    'grupos_nuevos', v_grupos_nuevos,
    'regresiones', v_regresiones
  );
end;
$$;

revoke all on function observabilidad.registrar_errores(jsonb) from public;
grant execute on function observabilidad.registrar_errores(jsonb) to authenticated, service_role;

create function public.registrar_errores(p_reportes jsonb)
returns jsonb
language sql
security invoker
set search_path = ''
as $$
  select observabilidad.registrar_errores(p_reportes);
$$;

revoke all on function public.registrar_errores(jsonb) from public;
grant execute on function public.registrar_errores(jsonb) to authenticated, service_role;

comment on function public.registrar_errores(jsonb) is
  'Recibe errores ya limpios del equipo que llama (vía la Edge Function observabilidad). Ver observabilidad.registrar_errores.';

-- ---------------------------------------------------------------------------
-- Mantenimiento diario: retención de 180 días.
-- ---------------------------------------------------------------------------
create function observabilidad.mantener_errores()
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_cfg observabilidad.config;
  v_limite timestamptz;
  v_errores bigint;
  v_grupos bigint;
  v_resultado jsonb;
begin
  select * into strict v_cfg from observabilidad.config;
  v_limite := pg_catalog.now() - pg_catalog.make_interval(days => v_cfg.dias_errores);

  delete from observabilidad.errores where recibido_en < v_limite;
  get diagnostics v_errores = row_count;
  -- Un grupo sin ocurrencias guardadas y sin novedades en el período se va;
  -- uno marcado `resuelto` o `ignorado` con ocurrencias, queda.
  delete from observabilidad.errores_grupo g
   where g.ultima_vez < v_limite
     and not exists (select 1 from observabilidad.errores e where e.huella = g.huella);
  get diagnostics v_grupos = row_count;

  v_resultado := pg_catalog.jsonb_build_object('errores', v_errores, 'grupos', v_grupos);
  insert into observabilidad.mantenimiento_registro (accion, detalle) values ('errores', v_resultado);
  return v_resultado;
end;
$$;

revoke all on function observabilidad.mantener_errores() from public;

-- 03:12 de Costa Rica (09:12 UTC), cinco minutos después del de telemetría.
select cron.schedule(
  'observabilidad-errores-diario',
  '12 9 * * *',
  'select observabilidad.mantener_errores()'
);

-- ---------------------------------------------------------------------------
-- Panel (sólo el administrador; ver observabilidad.exigir_administrador).
-- ---------------------------------------------------------------------------

-- Los grupos con actividad en los últimos `p_dias` (1 a 180), opcionalmente
-- de un estado. `en_periodo` y `equipos` cuentan sólo lo del período.
create function observabilidad.panel_errores_grupos(p_dias int, p_estado text)
returns table (
  huella text,
  plataforma text,
  origen text,
  titulo text,
  estado text,
  estado_cambiado_en timestamptz,
  primera_vez timestamptz,
  ultima_vez timestamptz,
  ocurrencias bigint,
  en_periodo bigint,
  equipos bigint,
  versiones text[]
)
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  v_desde timestamptz := pg_catalog.now()
                         - pg_catalog.make_interval(days => least(greatest(coalesce(p_dias, 7), 1), 180));
begin
  perform observabilidad.exigir_administrador();
  return query
    select g.huella, g.plataforma, g.origen, g.titulo, g.estado, g.estado_cambiado_en,
           g.primera_vez, g.ultima_vez, g.ocurrencias,
           pg_catalog.count(e.id), pg_catalog.count(distinct e.dispositivo_id), g.versiones
      from observabilidad.errores_grupo g
      left join observabilidad.errores e on e.huella = g.huella and e.recibido_en >= v_desde
     where g.ultima_vez >= v_desde
       and (p_estado is null or g.estado = p_estado)
     group by g.huella
     order by (g.estado = 'nuevo') desc, g.ultima_vez desc;
end;
$$;

revoke all on function observabilidad.panel_errores_grupos(int, text) from public;
grant execute on function observabilidad.panel_errores_grupos(int, text) to authenticated, service_role;

-- Las últimas ocurrencias de un grupo, con el equipo que las mandó.
create function observabilidad.panel_errores_detalle(p_huella text, p_limite int)
returns table (
  id bigint,
  ocurrido_en timestamptz,
  recibido_en timestamptz,
  dispositivo_id uuid,
  etiqueta text,
  version_app text,
  nivel text,
  origen text,
  titulo text,
  pantalla text,
  traza jsonb,
  operaciones jsonb,
  contexto jsonb
)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  perform observabilidad.exigir_administrador();
  return query
    select e.id, e.ocurrido_en, e.recibido_en, e.dispositivo_id, d.etiqueta, e.version_app,
           e.nivel, e.origen, e.titulo, e.pantalla, e.traza, e.operaciones, e.contexto
      from observabilidad.errores e
      left join public.dispositivos d on d.id = e.dispositivo_id
     where e.huella = p_huella
     order by e.recibido_en desc, e.id desc
     limit least(greatest(coalesce(p_limite, 20), 1), 100);
end;
$$;

revoke all on function observabilidad.panel_errores_detalle(text, int) from public;
grant execute on function observabilidad.panel_errores_detalle(text, int) to authenticated, service_role;

-- Cambia el estado de un grupo. Al marcarlo `resuelto` guarda las versiones
-- vistas hasta ahora: si vuelve en otra, es una regresión.
create function observabilidad.panel_errores_cambiar_estado(p_huella text, p_estado text)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_grupo observabilidad.errores_grupo;
begin
  perform observabilidad.exigir_administrador();
  if p_estado is null or p_estado not in ('nuevo', 'visto', 'resuelto', 'ignorado') then
    raise exception 'Estado de error desconocido' using errcode = '22023';
  end if;
  update observabilidad.errores_grupo g
     set estado = p_estado,
         estado_cambiado_en = pg_catalog.now(),
         versiones_al_resolver = case when p_estado = 'resuelto' then g.versiones else '{}' end
   where g.huella = p_huella
  returning * into v_grupo;
  if v_grupo.huella is null then
    raise exception 'No existe ese error' using errcode = 'P0002';
  end if;
  return pg_catalog.jsonb_build_object('huella', v_grupo.huella, 'estado', v_grupo.estado,
                                       'estado_cambiado_en', v_grupo.estado_cambiado_en);
end;
$$;

revoke all on function observabilidad.panel_errores_cambiar_estado(text, text) from public;
grant execute on function observabilidad.panel_errores_cambiar_estado(text, text) to authenticated, service_role;

-- Lo que expone la API: sin `security definer`.
create function public.panel_errores_grupos(p_dias int, p_estado text)
returns table (
  huella text, plataforma text, origen text, titulo text, estado text,
  estado_cambiado_en timestamptz, primera_vez timestamptz, ultima_vez timestamptz,
  ocurrencias bigint, en_periodo bigint, equipos bigint, versiones text[]
)
language sql
stable
security invoker
set search_path = ''
as $$ select * from observabilidad.panel_errores_grupos(p_dias, p_estado); $$;

create function public.panel_errores_detalle(p_huella text, p_limite int)
returns table (
  id bigint, ocurrido_en timestamptz, recibido_en timestamptz, dispositivo_id uuid,
  etiqueta text, version_app text, nivel text, origen text, titulo text, pantalla text,
  traza jsonb, operaciones jsonb, contexto jsonb
)
language sql
stable
security invoker
set search_path = ''
as $$ select * from observabilidad.panel_errores_detalle(p_huella, p_limite); $$;

create function public.panel_errores_cambiar_estado(p_huella text, p_estado text)
returns jsonb
language sql
security invoker
set search_path = ''
as $$ select observabilidad.panel_errores_cambiar_estado(p_huella, p_estado); $$;

revoke all on function public.panel_errores_grupos(int, text) from public;
revoke all on function public.panel_errores_detalle(text, int) from public;
revoke all on function public.panel_errores_cambiar_estado(text, text) from public;
grant execute on function public.panel_errores_grupos(int, text) to authenticated, service_role;
grant execute on function public.panel_errores_detalle(text, int) to authenticated, service_role;
grant execute on function public.panel_errores_cambiar_estado(text, text) to authenticated, service_role;

-- Consultas libres del asistente del lienzo (plan del asistente, Parte A,
-- etapa 3): el modelo escribe un SELECT sobre el esquema `ia` para lo que el
-- catálogo no cubre. Capas, de afuera hacia adentro:
--
-- 1. La Edge Function `lienzo-consulta` autoriza al administrador, valida el
--    SQL con un parser (una sola consulta de lectura, sólo `ia.*`, funciones
--    de una lista blanca) y manda a la base el SQL regenerado desde el árbol.
-- 2. `public.ia_consultar` (sólo service_role) pone la transacción en solo
--    lectura, baja al rol `ia_lector` (o `ia_lector_dueno`) y ejecuta con
--    límite de filas. Si la consulta cambió de rol, aborta sin devolver nada.
-- 3. `ia_lector` sólo puede leer las vistas de `ia`: ni tablas reales, ni
--    secretos. La telemetría sólo la ve `ia_lector_dueno`, según `ia_permisos`
--    (por cuenta, en la base: no depende de lo que haga el modelo).
--
-- Fuera de `ia` a propósito: secretos (códigos de vinculación, tokens push,
-- claves de dispositivos, hashes, activaciones de anfitriones, ids de
-- sesión), el registro del propio asistente y las IP de los equipos.

-- Roles sin login: sólo se llega a ellos con SET ROLE desde ia_consultar.
do $$
begin
  if not exists (select 1 from pg_roles where rolname = 'ia_lector') then
    create role ia_lector nologin noinherit;
  end if;
  if not exists (select 1 from pg_roles where rolname = 'ia_lector_dueno') then
    create role ia_lector_dueno nologin inherit;
  end if;
end $$;

-- El dueño ve todo lo general más la telemetría.
grant ia_lector to ia_lector_dueno;
-- service_role (la Edge Function, vía ia_consultar) puede bajar a estos roles,
-- sin heredar lo que ellos leen.
grant ia_lector, ia_lector_dueno to service_role with inherit false, set true;

create schema ia;
comment on schema ia is
  'Vistas de solo lectura para las consultas del asistente del lienzo. Sólo las leen ia_lector e ia_lector_dueno.';
revoke all on schema ia from public;
grant usage on schema ia to ia_lector, ia_lector_dueno;

-- ── Vistas generales (ia_lector) ────────────────────────────────────────────
-- Vistas comunes (no security_invoker): leen como su dueño, así ia_lector no
-- necesita permisos sobre las tablas reales.

create view ia.sitios as
  select id, nombre, created_at from public.sitios;
comment on view ia.sitios is 'Sitios (plantas) con puesto de control.';

create view ia.empresas as
  select id, nombre, activa, created_at, updated_at from public.empresas;
comment on view ia.empresas is 'Empresas contratistas. activa=false bloquea el acceso de su personal.';

-- Misma lógica que public.panel_contratistas_estado (si cambia una, cambiar la
-- otra). No se apoya en ella porque es security_invoker: revisaría permisos
-- sobre las tablas reales como ia_lector, que no los tiene.
create view ia.contratistas as
  select c.id, c.identificacion as cedula, c.nombre, c.empresa_id,
         coalesce(e.nombre, c.empresa_nombre) as empresa_nombre,
         coalesce(e.activa, true) as empresa_activa,
         c.tipo_ingreso, coalesce(c.es_personal_ruta, false) as es_personal_ruta, c.fecha_vencimiento_praind,
         c.activo, r.requiere_praind, c.fecha_vencimiento_praind - h.hoy as dias_para_vencer,
         case
           when not r.requiere_praind then 'NO_REQUIERE'
           when c.fecha_vencimiento_praind is null then 'SIN_REGISTRO'
           when c.fecha_vencimiento_praind < h.hoy then 'VENCIDA'
           when c.fecha_vencimiento_praind <= h.hoy + 30 then 'POR_VENCER'
           else 'VIGENTE'
         end as estado_praind,
         case
           when not coalesce(e.activa, true) then 'EMPRESA_INACTIVA'
           when not c.activo then 'SIN_ACCESO'
           when not r.requiere_praind then 'PERMITIDO'
           when c.fecha_vencimiento_praind is null then 'PRAIND_NO_REGISTRADO'
           when c.fecha_vencimiento_praind < h.hoy then 'PRAIND_VENCIDO'
           when c.fecha_vencimiento_praind <= h.hoy + 30 then 'PERMITIDO_CON_ADVERTENCIA'
           else 'PERMITIDO'
         end as estado_acceso,
         a.sitio_id as adentro_sitio_id, s.nombre as adentro_sitio_nombre, a.hora_entrada as adentro_desde
    from public.contratistas c
    cross join (select (now() at time zone 'America/Costa_Rica')::date as hoy) h
    left join public.empresas e on e.id = c.empresa_id
    cross join lateral (
      select coalesce(c.es_personal_ruta, false) or c.tipo_ingreso in ('PRAIND', 'IN_HOUSE') as requiere_praind
    ) r
    left join lateral (
      select i.sitio_id, i.hora_entrada from public.ingresos i
       where i.contratista_id = c.id and i.hora_salida is null
       order by i.hora_entrada desc limit 1
    ) a on true
    left join public.sitios s on s.id = a.sitio_id;
comment on view ia.contratistas is
  'Contratistas con su estado calculado. tipo_ingreso: PRAIND, IN_HOUSE, POR_CORREO, SWAT. '
  'estado_praind: VIGENTE, POR_VENCER (vence en 30 días o menos), VENCIDA, SIN_REGISTRO, NO_REQUIERE. '
  'estado_acceso: PERMITIDO, PERMITIDO_CON_ADVERTENCIA, PRAIND_VENCIDO, PRAIND_NO_REGISTRADO, SIN_ACCESO, '
  'EMPRESA_INACTIVA. dias_para_vencer negativo = ya vencido. adentro_* no nulo = está adentro de un sitio ahora.';

create view ia.ingresos as
  select id, sitio_id, contratista_id, contratista_cedula, contratista_nombre, empresa_nombre, tipo_ingreso,
         medio_ingreso, gafete_numero, placa, hora_entrada, hora_salida, usuario_entrada_nombre,
         usuario_salida_nombre, resultado_acceso, motivo_resultado, dispositivo_entrada_id, dispositivo_salida_id
    from public.ingresos;
comment on view ia.ingresos is
  'Ingresos de contratistas al sitio. hora_salida nula = sigue adentro. medio_ingreso: CAMINANDO, VEHICULO. '
  'resultado_acceso: PERMITIDO, PERMITIDO_CON_ADVERTENCIA. usuario_*_nombre = operador que registró.';

create view ia.empresas_proveedor as
  select id, sitio_id, nombre, activa, created_at, updated_at from public.empresas_proveedor;
comment on view ia.empresas_proveedor is
  'Empresas proveedoras, por sitio. Sólo para preguntas por EMPRESAS; "proveedores" a secas son personas (ia.ingresos_proveedor).';

create view ia.ingresos_proveedor as
  select id, sitio_id, cedula, nombre, empresa_id, empresa_nombre, placa, gafete_numero, hora_entrada, hora_salida,
         usuario_entrada_nombre, usuario_salida_nombre, dispositivo_entrada_id, dispositivo_salida_id
    from public.ingresos_proveedor;
comment on view ia.ingresos_proveedor is
  'Ingresos de proveedores. Cada fila es una PERSONA (nombre, cedula) que vino de una empresa proveedora '
  '(empresa_nombre). Cuando piden "proveedores" se refieren a esas personas: mostrá nombre (y su empresa al lado); '
  'agrupá por empresa sólo si piden empresas. hora_salida nula = sigue adentro.';

create view ia.ingresos_correo as
  select id, sitio_id, cedula, nombre, motivo, placa, gafete_numero, hora_entrada, hora_salida,
         usuario_entrada_nombre, usuario_salida_nombre, dispositivo_entrada_id, dispositivo_salida_id
    from public.ingresos_correo;
comment on view ia.ingresos_correo is 'Ingresos autorizados por correo (sin registro previo). hora_salida nula = sigue adentro.';

create view ia.movimientos_visita as
  select id, sitio_id, cita_visitante_id, visitante_cedula, visitante_nombre, empresa, anfitrion_nombre, motivo,
         gafete_numero, placa, hora_entrada, hora_salida, usuario_entrada_nombre, usuario_salida_nombre,
         dispositivo_entrada_id, dispositivo_salida_id
    from public.movimientos_visita;
comment on view ia.movimientos_visita is 'Entradas y salidas de visitas registradas en el puesto de control.';

-- Misma lógica que public.panel_adentro_ahora (security_invoker, ver arriba).
create view ia.adentro_ahora as
  select 'CONTRATISTA' as tipo, i.id, i.sitio_id, s.nombre as sitio_nombre, i.contratista_cedula as cedula,
         i.contratista_nombre as nombre, i.empresa_nombre, i.gafete_numero, nullif(btrim(i.placa), '') as placa,
         i.hora_entrada, i.usuario_entrada_nombre
    from public.ingresos i left join public.sitios s on s.id = i.sitio_id
   where i.hora_salida is null
  union all
  select 'PROVEEDOR', p.id, p.sitio_id, s.nombre, p.cedula, p.nombre, p.empresa_nombre, p.gafete_numero,
         nullif(btrim(p.placa), ''), p.hora_entrada, p.usuario_entrada_nombre
    from public.ingresos_proveedor p left join public.sitios s on s.id = p.sitio_id
   where p.hora_salida is null
  union all
  select 'PROVISIONAL_KOF', k.id, k.sitio_id, s.nombre, k.encargado_codigo_empleado, k.encargado_nombre, 'KOF',
         k.gafete_numero, null, k.hora_entrega, k.usuario_entrega_nombre
    from public.prestamos_gafete_provisional k left join public.sitios s on s.id = k.sitio_id
   where k.hora_devolucion is null
  union all
  select 'POR_CORREO', c.id, c.sitio_id, s.nombre, c.cedula, c.nombre, c.motivo, c.gafete_numero,
         nullif(btrim(c.placa), ''), c.hora_entrada, c.usuario_entrada_nombre
    from public.ingresos_correo c left join public.sitios s on s.id = c.sitio_id
   where c.hora_salida is null;
comment on view ia.adentro_ahora is
  'Quién está adentro de cada sitio en este momento. tipo: CONTRATISTA, PROVEEDOR, PROVISIONAL_KOF, POR_CORREO '
  '(en POR_CORREO, empresa_nombre trae el motivo; en PROVISIONAL_KOF, cedula trae el código de empleado).';

create view ia.gafetes as
  select id, sitio_id, numero, tipo, estado, contratista_portador_id, contratista_portador_nombre,
         visita_portador_id, visita_portador_nombre, created_at, updated_at
    from public.gafetes;
comment on view ia.gafetes is
  'Gafetes físicos. tipo: CONTRATISTA, VISITA, PROVEEDOR, PROVISIONAL_KOF. estado: DISPONIBLE, PERDIDO, DE_BAJA. '
  '*_portador_* no nulo = prestado ahora.';

create view ia.prestamos_gafete_provisional as
  select id, sitio_id, encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega,
         usuario_entrega_nombre, hora_devolucion, usuario_devolucion_nombre, dispositivo_entrega_id,
         dispositivo_devolucion_id
    from public.prestamos_gafete_provisional;
comment on view ia.prestamos_gafete_provisional is 'Préstamos de gafetes provisionales a personal KOF. hora_devolucion nula = sin devolver.';

create view ia.usuarios as
  select id, cedula, nombre, rol, activo, created_at, updated_at from public.usuarios;
comment on view ia.usuarios is 'Usuarios de las apps (operadores del puesto de control). rol: ROOT, ADMINISTRADOR, OPERADOR.';

create view ia.bitacora_sesiones as
  select id, usuario_id, cedula, nombre, dispositivo_id, dispositivo_etiqueta, sitio_id, sitio_nombre, iniciada_en,
         cerrada_en, motivo_cierre
    from public.bitacora_sesiones;
comment on view ia.bitacora_sesiones is
  'Historial de sesiones de los operadores. motivo_cierre: salida, otra_unidad, desplazada, sin_cierre, equipo_retirado.';

create view ia.sesiones_activas as
  select usuario_id, dispositivo_id, sitio_id, iniciada_en, actualizada_en from public.sesiones_usuario;
comment on view ia.sesiones_activas is 'Sesión abierta de cada operador ahora (una por usuario).';

create view ia.dispositivos as
  select id, sitio_id, tipo, etiqueta, nombre_dispositivo, plataforma, app_version, version_build, created_at,
         vinculado_en, revoked_at, last_seen_at, oculto_en_panel
    from public.dispositivos;
comment on view ia.dispositivos is
  'Equipos vinculados. tipo: pc, mobile, visor. revoked_at no nulo = revocado. last_seen_at = última conexión.';

create view ia.avisos_enviados as
  select id, titulo, cuerpo, tipo, destino, enviado_por, enviado_en, destinatarios, enviados, no_recibieron
    from public.avisos_enviados;
comment on view ia.avisos_enviados is 'Avisos (notificaciones) enviados desde el panel a los equipos. tipo: normal, emergente.';

create view ia.avisos_acciones as
  select id, aviso_id, accion, dispositivo_id, dispositivo_etiqueta, usuario, ocurrio_en from public.avisos_acciones;
comment on view ia.avisos_acciones is 'Qué hizo cada equipo con un aviso. accion: archivado, borrado.';

create view ia.rutas as
  select id, sitio_id, numero, activo, created_at, updated_at from public.rutas;
comment on view ia.rutas is 'Rutas de reparto (KOF).';

create view ia.vehiculos_ruta as
  select id, sitio_id, numero_unidad, placa, activo, created_at, updated_at from public.vehiculos_ruta;
comment on view ia.vehiculos_ruta is 'Vehículos de las rutas de reparto.';

create view ia.encargados_ruta as
  select id, sitio_id, codigo_empleado, nombre, cedula, activo, created_at, updated_at from public.encargados_ruta;
comment on view ia.encargados_ruta is 'Encargados (choferes) de las rutas de reparto, personal KOF.';

create view ia.salidas_ruta as
  select id, sitio_id, vehiculo_id, vehiculo_placa, vehiculo_numero_unidad, encargado_id, encargado_nombre,
         hora_salida, usuario_salida_nombre, hora_retorno, usuario_retorno_nombre
    from public.salidas_ruta;
comment on view ia.salidas_ruta is 'Salidas y retornos de los vehículos de reparto. hora_retorno nula = afuera.';

create view ia.anfitriones as
  select correo, nombre, activo, creado_en from public.anfitriones;
comment on view ia.anfitriones is 'Anfitriones: empleados que reciben visitas.';

create view ia.citas as
  select id, anfitrion_correo, motivo, fecha_desde, fecha_hasta, hora_estimada, estado, created_at, updated_at
    from public.citas;
comment on view ia.citas is 'Citas de visita agendadas por anfitriones. estado: VIGENTE, CANCELADA.';

create view ia.cita_visitantes as
  select id, cita_id, cedula, nombre, empresa, placa_vehiculo, created_at from public.cita_visitantes;
comment on view ia.cita_visitantes is 'Personas invitadas en cada cita.';

create view ia.cita_sitios as
  select cita_id, sitio_id from public.cita_sitios;
comment on view ia.cita_sitios is 'A qué sitios aplica cada cita.';

create view ia.administradores_panel as
  select correo, creado_en from public.administradores_panel;
comment on view ia.administradores_panel is 'Correos con acceso al panel de administración.';

-- ── Vistas sólo del dueño (ia_lector_dueno): telemetría y errores ────────────

create view ia.telemetria_ordenes as
  select dispositivo_id, grupos, vence_en, version, nota, actualizado_en, actualizado_por from public.telemetria_ordenes;
comment on view ia.telemetria_ordenes is 'Órdenes de telemetría vigentes por equipo.';

create view ia.telemetria_ordenes_historial as
  select id, dispositivo_id, etiqueta, grupos, vence_en, version, nota, cambiado_por, cambiado_en
    from public.telemetria_ordenes_historial;
comment on view ia.telemetria_ordenes_historial is 'Historial de cambios a las órdenes de telemetría.';

create view ia.eventos_seguridad_dispositivos as
  select id, ocurrido_en, dispositivo_id, tipo, ip, detalle from public.eventos_seguridad_dispositivos;
comment on view ia.eventos_seguridad_dispositivos is
  'Eventos de seguridad de los equipos (códigos inválidos, firmas inválidas, sesiones dudosas).';

-- Permisos: nadie fuera de los dos roles lee `ia`.
revoke all on all tables in schema ia from public, anon, authenticated, service_role;
grant select on all tables in schema ia to ia_lector;
revoke select on ia.telemetria_ordenes, ia.telemetria_ordenes_historial,
  ia.eventos_seguridad_dispositivos from ia_lector;
grant select on ia.telemetria_ordenes, ia.telemetria_ordenes_historial,
  ia.eventos_seguridad_dispositivos to ia_lector_dueno;

-- ── Quién ve la telemetría ───────────────────────────────────────────────────

create table public.ia_permisos (
  correo text not null,
  alcance text not null check (alcance in ('telemetria', 'alertas')),
  creado_en timestamptz not null default now(),
  primary key (correo, alcance)
);
comment on table public.ia_permisos is
  'Alcances extra del asistente por cuenta. telemetria = puede consultar telemetría y errores; '
  'alertas = ve las alertas de intentos de manipulación. Se edita por SQL.';
alter table public.ia_permisos enable row level security;
-- Cada cuenta ve sólo sus propios alcances (lo usa la RLS de ia_alertas).
create policy "cada cuenta ve sus alcances"
  on public.ia_permisos for select to authenticated
  using (correo = (select auth.email()));
revoke all on public.ia_permisos from public, anon, authenticated;
grant select on public.ia_permisos to authenticated, service_role;

insert into public.ia_permisos (correo, alcance) values
  ('daniel.bleach1@gmail.com', 'telemetria'),
  ('daniel.bleach1@gmail.com', 'alertas');

-- ── Alertas: intentos de que el asistente ejecute SQL o rompa sus reglas ─────

create table public.ia_alertas (
  id uuid primary key default gen_random_uuid(),
  creado_en timestamptz not null default now(),
  correo text not null,
  -- detector: SQL o manipulación en el mensaje, visto por la Edge Function;
  -- modelo: el modelo lo marcó; parser: el SQL del modelo no pasó el revisor.
  origen text not null check (origen in ('detector', 'modelo', 'parser')),
  motivo text not null check (char_length(motivo) between 1 and 500),
  mensaje text not null check (char_length(mensaje) between 1 and 4000),
  detalle text check (char_length(detalle) <= 8000),
  revisada_en timestamptz
);
comment on table public.ia_alertas is
  'Intentos de que el asistente del lienzo ejecute SQL escrito por la persona o se salga de sus reglas: quién, qué escribió y por qué se marcó.';
create index ia_alertas_creado_idx on public.ia_alertas (creado_en desc);
alter table public.ia_alertas enable row level security;
create policy "sólo quien tiene el alcance alertas las ve"
  on public.ia_alertas for select to authenticated
  using (exists (select 1 from public.ia_permisos p where p.correo = (select auth.email()) and p.alcance = 'alertas'));
create policy "quien tiene el alcance alertas las marca revisadas"
  on public.ia_alertas for update to authenticated
  using (exists (select 1 from public.ia_permisos p where p.correo = (select auth.email()) and p.alcance = 'alertas'));
revoke all on public.ia_alertas from public, anon, authenticated;
grant select, update (revisada_en) on public.ia_alertas to authenticated;
grant select, insert on public.ia_alertas to service_role;

-- ── Ejecutar una consulta ────────────────────────────────────────────────────

create function public.ia_consultar(p_sql text, p_correo text, p_limite integer default 2000)
returns json
language plpgsql
volatile
security invoker
set search_path = ''
as $$
declare
  v_original text := current_user;
  v_rol text;
  v_filas json;
  v_total bigint;
begin
  if p_limite is null or p_limite < 1 or p_limite > 5000 then
    raise exception 'Límite de filas fuera de rango (1 a 5000)' using errcode = '22023';
  end if;
  if p_sql is null or length(p_sql) > 8000 or position(';' in p_sql) > 0 then
    raise exception 'Consulta vacía, muy larga o con más de una sentencia' using errcode = '22023';
  end if;

  v_rol := case
    when exists (select 1 from public.ia_permisos where correo = p_correo and alcance = 'telemetria')
      then 'ia_lector_dueno'
    else 'ia_lector'
  end;

  -- De acá al final de la transacción, nada se escribe (pasar a solo lectura
  -- siempre se permite; volver a escritura, no).
  perform set_config('transaction_read_only', 'on', true);
  execute format('set local role %I', v_rol);

  execute format(
    'select array_to_json((array_agg(row_to_json(c)))[1:%s]), count(*) from (select * from (%s) as consulta limit %s) as c',
    p_limite, p_sql, p_limite + 1
  ) into v_filas, v_total;

  -- Si la consulta se las arregló para cambiar de rol o salir de solo lectura,
  -- no se devuelve nada.
  if current_user <> v_rol or current_setting('transaction_read_only') <> 'on' then
    raise exception 'La consulta intentó cambiar de rol o de modo' using errcode = '42501';
  end if;
  execute format('set local role %I', v_original);

  return json_build_object(
    'columnas', coalesce((select json_agg(k) from json_object_keys(v_filas -> 0) as k), '[]'::json),
    'filas', coalesce(v_filas, '[]'::json),
    'truncado', v_total > p_limite
  );
end;
$$;

comment on function public.ia_consultar(text, text, integer) is
  'Ejecuta un SELECT del asistente sobre el esquema ia, en solo lectura y con el rol de lectura que corresponde a la cuenta. Sólo service_role: la Edge Function lienzo-consulta valida el SQL antes.';

-- ── Describir lo que puede consultar cada cuenta ─────────────────────────────

create function public.ia_describir(p_correo text)
returns json
language sql
stable
security invoker
set search_path = ''
as $$
  with rol as (
    select case
      when exists (select 1 from public.ia_permisos where correo = p_correo and alcance = 'telemetria')
        then 'ia_lector_dueno'
      else 'ia_lector'
    end as nombre
  )
  select coalesce(json_agg(json_build_object(
           'vista', c.relname,
           'descripcion', obj_description(c.oid, 'pg_class'),
           'columnas', (select json_agg(json_build_object('nombre', a.attname, 'tipo', format_type(a.atttypid, a.atttypmod))
                                        order by a.attnum)
                          from pg_catalog.pg_attribute a
                         where a.attrelid = c.oid and a.attnum > 0 and not a.attisdropped)
         ) order by c.relname), '[]'::json)
    from pg_catalog.pg_class c, rol
   where c.relnamespace = 'ia'::regnamespace
     and c.relkind = 'v'
     and has_table_privilege(rol.nombre, c.oid, 'select');
$$;

comment on function public.ia_describir(text) is
  'Vistas de ia (con columnas y descripción) que puede consultar la cuenta. Sólo service_role.';

-- Permisos explícitos (AGENTS.md). A propósito, sólo service_role: ejecutar
-- SQL arbitrario pasa siempre por el parser de la Edge Function.
revoke all on function public.ia_consultar(text, text, integer) from public, anon, authenticated;
revoke all on function public.ia_describir(text) from public, anon, authenticated;
grant execute on function public.ia_consultar(text, text, integer) to service_role;
grant execute on function public.ia_describir(text) to service_role;

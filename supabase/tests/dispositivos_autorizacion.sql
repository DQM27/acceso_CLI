-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- `dispositivos` tiene una sola política de SELECT, "leer dispositivos
-- (propio sitio o admin_global)" (fusionada 2026-09-12 desde las dos
-- políticas separadas "leer dispositivos del propio sitio" y "admin_global
-- lee dispositivos" -- ver fusiona_politicas_permisivas_duplicadas_de_
-- visitas_y_panel, el advisor de performance las marcaba como "multiple
-- permissive policies" duplicadas). admin_global la necesita para el embed
-- `dispositivos!ingresos_dispositivo_entrada_id_fkey(tipo)` del historial
-- multi-sitio en el panel web (ver Historial.tsx/useAutoRefresh). Antes de
-- admin_global_lee_dispositivos (2026-09-06) NO existía ese acceso global --
-- si esta tercera comprobación empieza a fallar de nuevo, es que se sacó
-- esa mitad de la condición sin actualizar este test.
--
-- Alta/baja de dispositivos sigue siendo vía Edge Functions con
-- service_role (no pasa por RLS) -- esto sólo cubre la lectura directa.
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico A'),
  (gen_random_uuid(), 'Diagnóstico B');
select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico A'), true),
       set_config('diagnostico.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico B'), true),
       set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, secret_hash)
values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico PC A', 'diag-hash-a');

insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

-- Un dispositivo vigente por sitio como `sub` de los JWT simulados: la
-- política restrictiva "solo dispositivos vigentes" exige que el token sea
-- de un dispositivo real y activo (ver dispositivos_vigentes_y_vinculacion.sql).
-- Sin esto, los casos negativos pasarían por esa política y no por la de sitio.
insert into public.dispositivos (id, sitio_id, tipo, etiqueta, secret_hash) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico JWT A', 'diag-hash-jwt-a'),
  (gen_random_uuid(), current_setting('diagnostico.sitio_b')::uuid, 'pc', 'Diagnóstico JWT B', 'diag-hash-jwt-b');
select set_config('diagnostico.jwt_a', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico JWT A'), true),
       set_config('diagnostico.jwt_b', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico JWT B'), true);

set local role authenticated;

-- Un dispositivo lee los dispositivos de SU propio sitio.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sub', current_setting('diagnostico.jwt_a'), 'sitio_id', current_setting('diagnostico.sitio_a'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.dispositivos where etiqueta = 'Diagnóstico PC A') then
    raise exception 'Un dispositivo no puede leer los dispositivos de su propio sitio';
  end if;
end $$;

-- Pero NO los de otro sitio.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sub', current_setting('diagnostico.jwt_b'), 'sitio_id', current_setting('diagnostico.sitio_b'))::text,
  true);
do $$
begin
  if exists (select 1 from public.dispositivos where etiqueta = 'Diagnóstico PC A') then
    raise exception 'Un dispositivo puede leer dispositivos de otro sitio';
  end if;
end $$;

-- admin_global (panel web) SÍ puede leer dispositivos de cualquier sitio,
-- sin sitio_id en el JWT -- lo necesita el historial multi-sitio.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.dispositivos where etiqueta = 'Diagnóstico PC A') then
    raise exception 'admin_global no puede leer dispositivos';
  end if;
end $$;

select '3 comprobaciones de autorización correctas' as resultado;
rollback;

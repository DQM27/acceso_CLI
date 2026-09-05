-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- `dispositivos` solo tiene la política "leer dispositivos del propio
-- sitio" -- NO hay ninguna política de admin_global. El panel web
-- administra altas/bajas de dispositivos vía Edge Functions con
-- service_role (que no pasa por RLS), pero eso significa que si algún día
-- se agrega una lectura directa a esta tabla desde el panel (o se le pasa
-- "dispositivos" a useAutoRefresh para Realtime), NO va a traer nada: un
-- admin_global autenticado por Google no tiene sitio_id en su JWT. Este
-- test documenta ese hueco -- si se agrega una política admin_global acá,
-- hay que sumar el caso a este archivo, no solo confiar en que "ya
-- funciona" por las Edge Functions.
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

set local role authenticated;

-- Un dispositivo lee los dispositivos de SU propio sitio.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.dispositivos where etiqueta = 'Diagnóstico PC A') then
    raise exception 'Un dispositivo no puede leer los dispositivos de su propio sitio';
  end if;
end $$;

-- Pero NO los de otro sitio.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_b'))::text,
  true);
do $$
begin
  if exists (select 1 from public.dispositivos where etiqueta = 'Diagnóstico PC A') then
    raise exception 'Un dispositivo puede leer dispositivos de otro sitio';
  end if;
end $$;

-- admin_global (panel web) HOY NO puede leer dispositivos por esta vía --
-- gap conocido, documentado en docs/plan-panel-administrativo-web.md.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if exists (select 1 from public.dispositivos where etiqueta = 'Diagnóstico PC A') then
    raise exception 'admin_global ya puede leer dispositivos -- actualizar este comentario y useAutoRefresh en Dispositivos.tsx';
  end if;
end $$;

select '3 comprobaciones de autorización correctas' as resultado;
rollback;

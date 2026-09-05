-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- `sitios` solo tiene una política ("admin_global lee sitios") -- un
-- dispositivo normal (con sitio_id en el JWT pero sin fila en
-- administradores_panel) NO puede leer NI SU PROPIO sitio por esta tabla.
-- Esto es intencional según el histórico (antes ni siquiera admin_global
-- podía), pero vale dejarlo documentado: si algún dispositivo necesitara
-- el nombre de su sitio alguna vez, hoy tiene que resolverlo con datos que
-- ya trae localmente, no consultando esta tabla.
begin;

insert into public.sitios (id, nombre) values (gen_random_uuid(), 'Diagnóstico A');
select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico A'), true),
       set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true);
insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

set local role authenticated;

-- Un dispositivo con sitio_id propio no puede leer sitios en absoluto.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'))::text,
  true);
do $$
begin
  if exists (select 1 from public.sitios where nombre = 'Diagnóstico A') then
    raise exception 'Un dispositivo pudo leer sitios directamente (se esperaba que no)';
  end if;
end $$;

-- admin_global sí puede.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.sitios where nombre = 'Diagnóstico A') then
    raise exception 'admin_global no puede leer sitios';
  end if;
end $$;

select '2 comprobaciones de autorización correctas' as resultado;
rollback;

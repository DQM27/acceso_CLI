-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Dos sitios/dispositivos temporales (no dependen de datos reales) para
-- probar el aislamiento por sitio en INSERT y el acceso global de
-- admin_global en SELECT/UPDATE.
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico A'),
  (gen_random_uuid(), 'Diagnóstico B');

select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico A'), true),
       set_config('diagnostico.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico B'), true),
       set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, secret_hash) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico PC A', 'diag-hash-a'),
  (gen_random_uuid(), current_setting('diagnostico.sitio_b')::uuid, 'pc', 'Diagnóstico PC B', 'diag-hash-b');

select set_config('diagnostico.dispositivo_a', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico PC A'), true),
       set_config('diagnostico.dispositivo_b', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico PC B'), true);

insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

set local role authenticated;

-- Un dispositivo del sitio A puede crear un contratista EN su propio sitio.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'))::text,
  true);
do $$
begin
  insert into public.contratistas (id, sitio_id, dispositivo_origen_id, nombre)
  values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, 'Diagnóstico contratista A');
  if not found then
    raise exception 'Un dispositivo no pudo crear un contratista en su propio sitio';
  end if;
end $$;

-- Pero NO puede crear un contratista a nombre de otro sitio.
do $$
begin
  begin
    insert into public.contratistas (id, sitio_id, dispositivo_origen_id, nombre)
    values (gen_random_uuid(), current_setting('diagnostico.sitio_b')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, 'Diagnóstico contratista cruzado');
    raise exception 'Un dispositivo del sitio A pudo crear un contratista para el sitio B';
  exception
    when insufficient_privilege then null;
  end;
end $$;

-- Cualquier dispositivo autenticado puede LEER contratistas de cualquier
-- sitio (modelo global, ver migración crea_usuarios_globales) -- se
-- documenta acá a propósito: si algún día se decide acotar esto por sitio,
-- este test tiene que actualizarse junto con la política.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_b'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.contratistas where nombre = 'Diagnóstico contratista A') then
    raise exception 'Un dispositivo del sitio B no puede leer un contratista del sitio A (se esperaba lectura global)';
  end if;
end $$;

-- Un dispositivo de OTRO sitio también puede actualizar ese contratista
-- ajeno (misma política "actualizar contratistas (global)", qual: true) --
-- de nuevo, comportamiento intencional pero riesgoso: cualquier sesión
-- autenticada puede tocar cualquier contratista de cualquier sitio.
do $$
begin
  update public.contratistas set activo = false where nombre = 'Diagnóstico contratista A';
  if not found then
    raise exception 'Un dispositivo de otro sitio no pudo actualizar un contratista ajeno (se esperaba que sí, por diseño)';
  end if;
end $$;

-- admin_global también puede leer y actualizar sin necesitar sitio_id en el JWT.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.contratistas where nombre = 'Diagnóstico contratista A') then
    raise exception 'admin_global no puede leer contratistas';
  end if;
  update public.contratistas set activo = true where nombre = 'Diagnóstico contratista A';
  if not found then
    raise exception 'admin_global no puede actualizar contratistas';
  end if;
end $$;

select '5 comprobaciones de autorización correctas' as resultado;
rollback;

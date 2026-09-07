-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico A'),
  (gen_random_uuid(), 'Diagnóstico B');

select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico A'), true),
       set_config('diagnostico.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico B'), true),
       set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, secret_hash) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico PC A', 'diag-hash-a');

select set_config('diagnostico.dispositivo_a', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico PC A'), true);

insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

set local role authenticated;

-- Crear: acotado al propio sitio.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'))::text,
  true);
do $$
begin
  insert into public.empresas (id, sitio_id, dispositivo_origen_id, nombre)
  values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, 'Diagnóstico empresa A');
  if not found then
    raise exception 'Un dispositivo no pudo crear una empresa en su propio sitio';
  end if;
end $$;

do $$
begin
  begin
    insert into public.empresas (id, sitio_id, dispositivo_origen_id, nombre)
    values (gen_random_uuid(), current_setting('diagnostico.sitio_b')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, 'Diagnóstico empresa cruzada');
    raise exception 'Un dispositivo del sitio A pudo crear una empresa para el sitio B';
  exception
    when insufficient_privilege then null;
  end;
end $$;

-- Leer y actualizar: global, sin distinción de sitio, para cualquier
-- DISPOSITIVO autenticado (JWT con sitio_id) -- modelo global, igual que
-- contratistas/usuarios.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_b'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.empresas where nombre = 'Diagnóstico empresa A') then
    raise exception 'Un dispositivo de otro sitio no puede leer una empresa ajena (se esperaba lectura global)';
  end if;
  update public.empresas set activa = false where nombre = 'Diagnóstico empresa A';
  if not found then
    raise exception 'Un dispositivo de otro sitio no pudo actualizar una empresa ajena (se esperaba que sí, por diseño)';
  end if;
end $$;

-- admin_global también puede leer y actualizar sin necesitar sitio_id en el JWT.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.empresas where nombre = 'Diagnóstico empresa A') then
    raise exception 'admin_global no puede leer empresas';
  end if;
  update public.empresas set activa = true where nombre = 'Diagnóstico empresa A';
  if not found then
    raise exception 'admin_global no puede actualizar empresas';
  end if;
end $$;

-- El hueco que se cerró (cierra_acceso_global_a_cuentas_sin_dispositivo_
-- ni_admin): una sesión `authenticated` que no es dispositivo (sin
-- sitio_id) ni admin_global -- p. ej. cualquier cuenta de Google que
-- complete el login OAuth del panel sin estar en administradores_panel --
-- ya no puede leer ni escribir empresas.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'diagnostico-sin-permiso@example.com')::text,
  true);
do $$
begin
  if exists (select 1 from public.empresas where nombre = 'Diagnóstico empresa A') then
    raise exception 'Una sesión sin sitio_id ni admin_global pudo leer empresas (hueco de seguridad reabierto)';
  end if;
end $$;
do $$
begin
  update public.empresas set activa = false where nombre = 'Diagnóstico empresa A';
  if found then
    raise exception 'Una sesión sin sitio_id ni admin_global pudo actualizar una empresa (hueco de seguridad reabierto)';
  end if;
end $$;

select '7 comprobaciones de autorización correctas' as resultado;
rollback;

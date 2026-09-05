-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico A'),
  (gen_random_uuid(), 'Diagnóstico B');

select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico A'), true),
       set_config('diagnostico.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico B'), true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, secret_hash) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico PC A', 'diag-hash-a');

select set_config('diagnostico.dispositivo_a', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico PC A'), true);

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

-- Leer y actualizar: global, sin distinción de sitio (modelo global, igual
-- que contratistas/usuarios).
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

select '3 comprobaciones de autorización correctas' as resultado;
rollback;

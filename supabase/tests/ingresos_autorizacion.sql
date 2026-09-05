-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico A'),
  (gen_random_uuid(), 'Diagnóstico B');

select set_config('diagnostico.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico A'), true),
       set_config('diagnostico.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico B'), true),
       set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true);

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, secret_hash) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'pc', 'Diagnóstico PC A', 'diag-hash-a'),
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, 'visor', 'Diagnóstico visor A', 'diag-hash-visor');

select set_config('diagnostico.dispositivo_a', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico PC A'), true),
       set_config('diagnostico.dispositivo_visor', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico visor A'), true);

insert into public.contratistas (id, sitio_id, dispositivo_origen_id, nombre) values
  (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, 'Diagnóstico contratista');
select set_config('diagnostico.contratista_a', (select id::text from public.contratistas where nombre = 'Diagnóstico contratista'), true);

insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

set local role authenticated;

-- Un dispositivo 'pc' de su propio sitio puede registrar un ingreso.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'), 'tipo', 'pc')::text,
  true);
do $$
begin
  insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre, hora_entrada)
  values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, current_setting('diagnostico.contratista_a')::uuid, 'Diagnóstico contratista', now());
  if not found then
    raise exception 'Un dispositivo pc no pudo registrar un ingreso en su propio sitio';
  end if;
end $$;

-- Un dispositivo NO puede registrar un ingreso a nombre de otro sitio.
do $$
begin
  begin
    insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre, hora_entrada)
    values (gen_random_uuid(), current_setting('diagnostico.sitio_b')::uuid, current_setting('diagnostico.dispositivo_a')::uuid, current_setting('diagnostico.contratista_a')::uuid, 'Diagnóstico contratista', now());
    raise exception 'Un dispositivo pudo registrar un ingreso para otro sitio';
  exception
    when insufficient_privilege then null;
  end;
end $$;

-- Un dispositivo 'visor' (de solo lectura) NO puede registrar ingresos.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_a'), 'tipo', 'visor')::text,
  true);
do $$
begin
  begin
    insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre, hora_entrada)
    values (gen_random_uuid(), current_setting('diagnostico.sitio_a')::uuid, current_setting('diagnostico.dispositivo_visor')::uuid, current_setting('diagnostico.contratista_a')::uuid, 'Diagnóstico contratista', now());
    raise exception 'Un dispositivo visor pudo registrar un ingreso';
  exception
    when insufficient_privilege then null;
  end;
end $$;

-- Un dispositivo de OTRO sitio no puede leer el ingreso (a diferencia de
-- contratistas/usuarios, ingresos SÍ está acotado por sitio para
-- dispositivos normales).
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sitio_id', current_setting('diagnostico.sitio_b'), 'tipo', 'pc')::text,
  true);
do $$
begin
  if exists (select 1 from public.ingresos where contratista_nombre = 'Diagnóstico contratista') then
    raise exception 'Un dispositivo de otro sitio puede leer un ingreso ajeno';
  end if;
end $$;

-- admin_global SÍ puede leer el historial completo, de cualquier sitio.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.ingresos where contratista_nombre = 'Diagnóstico contratista') then
    raise exception 'admin_global no puede leer el historial completo de ingresos';
  end if;
end $$;

select '5 comprobaciones de autorización correctas' as resultado;
rollback;

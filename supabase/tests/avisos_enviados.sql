-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre la tabla `avisos_enviados`: el administrador del panel la lee, un
-- correo que no es administrador y un equipo no ven nada, nadie la escribe
-- desde un cliente (ni siquiera el administrador), anon no la toca, y los
-- CHECK rechazan un tipo desconocido o un título vacío.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin-avisos@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal-avisos@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a501', 'Unidad avisos A');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  ('cccccccc-0000-0000-0000-00000000c501', 'aaaaaaaa-0000-0000-0000-00000000a501', 'mobile', 'Celular avisos A1', '{"kty":"EC"}', 'huella-avisos-a1', null);

-- Lo que escribe la Edge Function (service_role: acá, sin RLS).
insert into public.avisos_enviados (id, titulo, cuerpo, tipo, destino, enviado_por, destinatarios, enviados, no_recibieron)
values ('dddddddd-0000-0000-0000-00000000d501', 'Corte de agua', 'Hasta las 14:00', 'emergente',
        '{"todos": true}', current_setting('diagnostico.correo_admin'), 3, 2, '{"Celular avisos A1"}');

-- Los CHECK: tipo desconocido y título vacío.
do $$
begin
  begin
    insert into public.avisos_enviados (titulo, cuerpo, tipo, destino, enviado_por)
    values ('Prueba', 'Texto', 'urgente', '{"todos": true}', 'x@example.com');
    raise exception 'check: se aceptó un tipo desconocido';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.avisos_enviados (titulo, cuerpo, tipo, destino, enviado_por)
    values ('', 'Texto', 'normal', '{"todos": true}', 'x@example.com');
    raise exception 'check: se aceptó un título vacío';
  exception
    when check_violation then null;
  end;
end $$;

set local role authenticated;

-- 1. El administrador del panel lee el historial.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
begin
  if not exists (select 1 from public.avisos_enviados where id = 'dddddddd-0000-0000-0000-00000000d501') then
    raise exception '1: el administrador no puede leer avisos_enviados';
  end if;
end $$;

-- 2. Ni el administrador escribe desde el cliente (sólo la Edge Function).
do $$
begin
  insert into public.avisos_enviados (titulo, cuerpo, tipo, destino, enviado_por)
  values ('Falso', 'Texto', 'normal', '{"todos": true}', 'x@example.com');
  raise exception '2: el administrador pudo insertar en avisos_enviados';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  update public.avisos_enviados set enviados = 99 where id = 'dddddddd-0000-0000-0000-00000000d501';
  raise exception '2: el administrador pudo modificar avisos_enviados';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  delete from public.avisos_enviados where id = 'dddddddd-0000-0000-0000-00000000d501';
  raise exception '2: el administrador pudo borrar avisos_enviados';
exception
  when insufficient_privilege then null;
end $$;

-- 3. Un correo que no es administrador no ve nada.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  if exists (select 1 from public.avisos_enviados) then
    raise exception '3: un correo no administrador pudo leer avisos_enviados';
  end if;
end $$;

-- 4. Un equipo (JWT de dispositivo) tampoco.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000c501',
  'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000a501', 'huella', 'huella-avisos-a1')::text, true);
do $$
begin
  if exists (select 1 from public.avisos_enviados) then
    raise exception '4: un equipo pudo leer avisos_enviados';
  end if;
end $$;

-- 5. anon no puede leer.
set local role anon;
do $$
begin
  perform 1 from public.avisos_enviados limit 1;
  raise exception '5: anon pudo leer avisos_enviados';
exception
  when insufficient_privilege then null;
end $$;

rollback;

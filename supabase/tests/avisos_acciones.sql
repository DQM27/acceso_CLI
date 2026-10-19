-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `registrar_accion_aviso` y la tabla `avisos_acciones`: un equipo
-- vigente anota que archivó o borró un aviso (con el nombre de la persona),
-- repetir la misma acción no duplica, un aviso desconocido no da error ni
-- fila, la hora no puede ser del futuro, un equipo retirado / anon /
-- parámetros inválidos se rechazan, y sólo el administrador del panel lee la
-- tabla (ni siquiera el equipo que la escribió).
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin-acciones@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal-acciones@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a601', 'Unidad acciones A');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  ('cccccccc-0000-0000-0000-00000000c601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'mobile', 'Celular acciones A1', '{"kty":"EC"}', 'huella-acciones-a1', null),
  ('cccccccc-0000-0000-0000-00000000c602', 'aaaaaaaa-0000-0000-0000-00000000a601', 'mobile', 'Celular acciones A2', '{"kty":"EC"}', 'huella-acciones-a2', null),
  ('cccccccc-0000-0000-0000-00000000c603', 'aaaaaaaa-0000-0000-0000-00000000a601', 'mobile', 'Celular acciones retirado', '{"kty":"EC"}', 'huella-acciones-r', now());

insert into public.avisos_enviados (id, titulo, cuerpo, tipo, destino, enviado_por)
values ('dddddddd-0000-0000-0000-00000000d601', 'Corte de agua', 'Hasta las 14:00', 'normal',
        '{"todos": true}', current_setting('diagnostico.correo_admin'));

create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text) returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella)::text, true)
$$;

set local role authenticated;

do $$
begin
  -- 1. Un equipo vigente anota que archivó y después que borró (con espacios
  --    en el nombre: se recortan).
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-acciones-a1');
  perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d601', 'archivado', '  ANA PÉREZ  ', now() - interval '5 minutes');
  perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d601', 'borrado', 'ANA PÉREZ', now() - interval '1 minute');

  -- 2. Repetir la misma acción (el teléfono reintenta) no duplica ni falla.
  perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d601', 'borrado', 'OTRA PERSONA', now());

  -- 3. Otro equipo anota la suya sobre el mismo aviso.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c602', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-acciones-a2');
  perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d601', 'borrado', 'LUIS SOTO', now() + interval '1 day');

  -- 4. Un aviso que el servidor no conoce: sin error y sin fila.
  perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d699', 'borrado', 'LUIS SOTO', now());

  -- 5. Parámetros inválidos.
  begin
    perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d601', 'ocultado', 'LUIS SOTO', now());
    raise exception '5: se aceptó una acción desconocida';
  exception when invalid_parameter_value then null;
  end;
  begin
    perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d601', 'borrado', '   ', now());
    raise exception '5: se aceptó un usuario vacío';
  exception when invalid_parameter_value then null;
  end;
  begin
    perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d601', null, 'LUIS SOTO', now());
    raise exception '5: se aceptó una acción nula';
  exception when invalid_parameter_value then null;
  end;

  -- 6. Un equipo retirado no anota nada.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c603', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-acciones-r');
  begin
    perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d601', 'borrado', 'LUIS SOTO', now());
    raise exception '6: un equipo retirado pudo anotar una acción';
  exception when insufficient_privilege then null;
  end;

  -- 7. Ni siquiera el equipo que escribe lee la tabla.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-acciones-a1');
  if exists (select 1 from public.avisos_acciones) then
    raise exception '7: un equipo pudo leer avisos_acciones';
  end if;
  begin
    insert into public.avisos_acciones (aviso_id, accion, dispositivo_id, dispositivo_etiqueta, usuario, ocurrio_en)
    values ('dddddddd-0000-0000-0000-00000000d601', 'borrado', 'cccccccc-0000-0000-0000-00000000c601', 'x', 'x', now());
    raise exception '7: un equipo pudo escribir directo en avisos_acciones';
  exception when insufficient_privilege then null;
  end;
end $$;

-- 8. El administrador del panel lee todo.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
declare
  n int;
begin
  select count(*) into n from public.avisos_acciones;
  -- A1 archivó, A1 borró, A2 borró; lo del aviso desconocido no cuenta.
  if n <> 3 then raise exception '8: se esperaban 3 acciones, hay %', n; end if;

  if not exists (
    select 1 from public.avisos_acciones
     where dispositivo_id = 'cccccccc-0000-0000-0000-00000000c601' and accion = 'borrado'
       and usuario = 'ANA PÉREZ' and dispositivo_etiqueta = 'Celular acciones A1'
  ) then
    raise exception '8: faltó la constancia de A1 (la repetida no debe pisar a la primera)';
  end if;
  if exists (select 1 from public.avisos_acciones where usuario = 'ANA PÉREZ' and usuario <> btrim(usuario)) then
    raise exception '8: el nombre no se recortó';
  end if;
  -- La hora del futuro se recorta a ahora.
  if exists (select 1 from public.avisos_acciones where ocurrio_en > now()) then
    raise exception '8: una acción quedó con hora del futuro';
  end if;
  -- Ni el administrador escribe directo.
  begin
    delete from public.avisos_acciones;
    raise exception '8: el administrador pudo borrar avisos_acciones';
  exception when insufficient_privilege then null;
  end;
end $$;

-- 9. Un correo que no es administrador no ve nada.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  if exists (select 1 from public.avisos_acciones) then
    raise exception '9: un correo no administrador pudo leer avisos_acciones';
  end if;
end $$;

-- 10. anon no puede ejecutar la función ni leer la tabla.
set local role anon;
do $$
begin
  perform public.registrar_accion_aviso('dddddddd-0000-0000-0000-00000000d601', 'borrado', 'X', now());
  raise exception '10: anon pudo ejecutar registrar_accion_aviso';
exception when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.avisos_acciones limit 1;
  raise exception '10: anon pudo leer avisos_acciones';
exception when insufficient_privilege then null;
end $$;

rollback;

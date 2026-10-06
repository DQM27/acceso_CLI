-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre 20261005140000_movimientos_visita_placa (medio de ingreso de las
-- visitas agendadas):
-- - El equipo de la unidad registra la placa con el movimiento (o NULL =
--   caminando) y la puede leer.
-- - La placa es dato de entrada: no se puede cambiar después.
-- - Una placa de más de 20 caracteres o vacía se rechaza.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a901', 'Unidad placa visita');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c901', 'aaaaaaaa-0000-0000-0000-00000000a901', 'pc', 'PC placa', '{"kty":"EC"}', 'huella-pv');

insert into public.anfitriones (correo, nombre) values ('anfitrion-pv@example.com', 'ANFITRION PRUEBA');
insert into public.citas (id, anfitrion_correo, fecha_desde, fecha_hasta) values
  ('bbbbbbbb-0000-0000-0000-00000000b901', 'anfitrion-pv@example.com', current_date, current_date);
insert into public.cita_sitios (cita_id, sitio_id) values
  ('bbbbbbbb-0000-0000-0000-00000000b901', 'aaaaaaaa-0000-0000-0000-00000000a901');
insert into public.cita_visitantes (id, cita_id, cedula, nombre) values
  ('bbbbbbbb-0000-0000-0000-00000000b911', 'bbbbbbbb-0000-0000-0000-00000000b901', '900000901', 'EN VEHICULO'),
  ('bbbbbbbb-0000-0000-0000-00000000b912', 'bbbbbbbb-0000-0000-0000-00000000b901', '900000902', 'CAMINANDO'),
  ('bbbbbbbb-0000-0000-0000-00000000b913', 'bbbbbbbb-0000-0000-0000-00000000b901', '900000903', 'PLACA LARGA');

create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text, p_tipo text default 'pc') returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella, 'tipo', p_tipo)::text, true)
$$;

set local role authenticated;

do $$
declare
  v_placa text;
  v_rechazado boolean;
begin
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c901', 'aaaaaaaa-0000-0000-0000-00000000a901', 'huella-pv');

  -- 1. En vehículo: la placa se guarda y se lee.
  insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, placa, hora_entrada, usuario_entrada_nombre)
  values ('eeeeeeee-0000-0000-0000-00000000e901', 'aaaaaaaa-0000-0000-0000-00000000a901', 'cccccccc-0000-0000-0000-00000000c901',
          'bbbbbbbb-0000-0000-0000-00000000b911', '900000901', 'EN VEHICULO', 'BCD123', now(), 'OPERADOR PRUEBA');
  select placa into v_placa from public.movimientos_visita where id = 'eeeeeeee-0000-0000-0000-00000000e901';
  if v_placa is distinct from 'BCD123' then
    raise exception 'FALLO 1: la placa no quedó guardada (%)', v_placa;
  end if;

  -- 2. Caminando: sin placa.
  insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, hora_entrada, usuario_entrada_nombre)
  values ('eeeeeeee-0000-0000-0000-00000000e902', 'aaaaaaaa-0000-0000-0000-00000000a901', 'cccccccc-0000-0000-0000-00000000c901',
          'bbbbbbbb-0000-0000-0000-00000000b912', '900000902', 'CAMINANDO', now(), 'OPERADOR PRUEBA');

  -- 3. La placa no se cambia después de la entrada.
  v_rechazado := false;
  begin
    update public.movimientos_visita set placa = 'OTRA1' where id = 'eeeeeeee-0000-0000-0000-00000000e901';
  exception when others then
    v_rechazado := sqlerrm like '%inmutables%';
  end;
  if not v_rechazado then
    raise exception 'FALLO 3: se pudo cambiar la placa de una entrada';
  end if;

  -- 4. Una placa imposible se rechaza.
  v_rechazado := false;
  begin
    insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, placa, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a901', 'cccccccc-0000-0000-0000-00000000c901',
            'bbbbbbbb-0000-0000-0000-00000000b913', '900000903', 'PLACA LARGA', repeat('X', 21), now(), 'OPERADOR PRUEBA');
  exception when check_violation then
    v_rechazado := true;
  end;
  if not v_rechazado then
    raise exception 'FALLO 4: se aceptó una placa de 21 caracteres';
  end if;

  -- 5. La salida sigue funcionando con la placa puesta.
  update public.movimientos_visita
     set hora_salida = now(), dispositivo_salida_id = 'cccccccc-0000-0000-0000-00000000c901', usuario_salida_nombre = 'OPERADOR PRUEBA'
   where id = 'eeeeeeee-0000-0000-0000-00000000e901';
end $$;

rollback;

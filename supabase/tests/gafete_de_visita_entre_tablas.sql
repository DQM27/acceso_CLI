-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre la parte (2) de 20261004120000_conflictos_misma_unidad_y_gafete_de_visita:
-- el gafete de VISITA se comparte entre `ingresos_correo` y
-- `movimientos_visita`, y la nube no admite el mismo gafete abierto en las
-- dos tablas a la vez dentro de una unidad.
-- - Con una visita abierta con el gafete 801 en la unidad A, un ingreso por
--   correo con el 801 en A se rechaza (23505, nombrando
--   `ingresos_correo_gafete_activo_sitio_idx`, que es lo que la cola del
--   núcleo reconoce como choque de gafete); en la unidad B se acepta.
-- - Al revés: con un ingreso por correo abierto con el 802, una visita con el
--   802 en la misma unidad se rechaza.
-- - Cerrada la visita, el gafete queda libre para un ingreso por correo.
-- - Una visita sin gafete no se frena.
-- - 20261004140000: dos visitas con el mismo gafete abierto en la unidad se
--   rechazan (`movimientos_visita_gafete_activo_sitio_idx`), y el rechazo de
--   una visita por un ingreso por correo nombra ese índice.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a801', 'Unidad gafete visita A'),
  ('aaaaaaaa-0000-0000-0000-00000000a802', 'Unidad gafete visita B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'pc',     'PC A',      '{"kty":"EC"}', 'huella-gv-a'),
  ('cccccccc-0000-0000-0000-00000000c802', 'aaaaaaaa-0000-0000-0000-00000000a801', 'mobile', 'Celular A', '{"kty":"EC"}', 'huella-gv-a2'),
  ('cccccccc-0000-0000-0000-00000000c803', 'aaaaaaaa-0000-0000-0000-00000000a802', 'mobile', 'Celular B', '{"kty":"EC"}', 'huella-gv-b');

insert into public.anfitriones (correo, nombre) values ('anfitrion-gv@example.com', 'ANFITRION PRUEBA');
insert into public.citas (id, anfitrion_correo, fecha_desde, fecha_hasta) values
  ('bbbbbbbb-0000-0000-0000-00000000b801', 'anfitrion-gv@example.com', current_date, current_date);
insert into public.cita_visitantes (id, cita_id, cedula, nombre) values
  ('bbbbbbbb-0000-0000-0000-00000000b811', 'bbbbbbbb-0000-0000-0000-00000000b801', '900000801', 'VISITANTE UNO'),
  ('bbbbbbbb-0000-0000-0000-00000000b812', 'bbbbbbbb-0000-0000-0000-00000000b801', '900000802', 'VISITANTE DOS');

create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text, p_tipo text default 'mobile') returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella, 'tipo', p_tipo)::text, true)
$$;

set local role authenticated;

do $$
declare
  v_rechazado boolean;
  v_mensaje text;
begin
  -- 1. La PC de A entrega el gafete de visita 801 a una visita.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'huella-gv-a');
  insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values ('eeeeeeee-0000-0000-0000-00000000e801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801',
          'bbbbbbbb-0000-0000-0000-00000000b811', '900000801', 'VISITANTE UNO', 801, now(), 'OPERADOR PRUEBA');

  -- 2. El celular de A (sin conexión cuando lo registró) intenta un ingreso
  --    por correo con el mismo 801: la nube lo rechaza como choque de gafete.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c802', 'aaaaaaaa-0000-0000-0000-00000000a801', 'huella-gv-a2');
  v_rechazado := false;
  begin
    insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c802',
            '900000811', 'CORREO PRUEBA', 'ENTREVISTA RH', 801, now(), 'OPERADOR PRUEBA');
  exception when unique_violation then
    v_rechazado := true;
    get stacked diagnostics v_mensaje = message_text;
  end;
  if not v_rechazado then raise exception 'G2a: el 801 está en una visita abierta, el ingreso por correo debía rechazarse'; end if;
  if v_mensaje not like '%ingresos_correo_gafete_activo_sitio_idx%' then
    raise exception 'G2b: el mensaje debe nombrar el índice que reconoce la cola, dio: %', v_mensaje;
  end if;

  -- 3. En la unidad B el 801 es otro gafete físico: se acepta.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c803', 'aaaaaaaa-0000-0000-0000-00000000a802', 'huella-gv-b');
  insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a802', 'cccccccc-0000-0000-0000-00000000c803',
          '900000812', 'CORREO PRUEBA B', 'ENTREVISTA RH', 801, now(), 'OPERADOR PRUEBA');

  -- 4. Al revés: ingreso por correo con el 802 en A, y después una visita
  --    con el 802 en A se rechaza.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c802', 'aaaaaaaa-0000-0000-0000-00000000a801', 'huella-gv-a2');
  insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c802',
          '900000813', 'CORREO PRUEBA DOS', 'ENTREVISTA RH', 802, now(), 'OPERADOR PRUEBA');
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'huella-gv-a');
  v_rechazado := false;
  begin
    insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801',
            'bbbbbbbb-0000-0000-0000-00000000b812', '900000802', 'VISITANTE DOS', 802, now(), 'OPERADOR PRUEBA');
  exception when unique_violation then
    v_rechazado := true;
    get stacked diagnostics v_mensaje = message_text;
  end;
  if not v_rechazado then raise exception 'G4a: el 802 está en un ingreso por correo abierto, la visita debía rechazarse'; end if;
  -- 20261004140000: el rechazo de una visita nombra su índice, para que la
  -- cola lo trate como choque de gafete.
  if v_mensaje not like '%movimientos_visita_gafete_activo_sitio_idx%' then
    raise exception 'G4b: el mensaje debe nombrar movimientos_visita_gafete_activo_sitio_idx, dio: %', v_mensaje;
  end if;

  -- 4c. Dos visitas con el mismo gafete en la misma unidad: la segunda se
  --     rechaza (20261004140000; antes la nube las aceptaba).
  v_rechazado := false;
  begin
    insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c802',
            'bbbbbbbb-0000-0000-0000-00000000b812', '900000802', 'VISITANTE DOS', 801, now(), 'OPERADOR PRUEBA');
  exception when unique_violation then
    v_rechazado := true;
    get stacked diagnostics v_mensaje = message_text;
  end;
  if not v_rechazado or v_mensaje not like '%movimientos_visita_gafete_activo_sitio_idx%' then
    raise exception 'G4c: el 801 ya está en una visita abierta de A, la segunda visita debía rechazarse por el índice (%)', v_mensaje;
  end if;

  -- 5. Una visita sin gafete no choca con nada.
  insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801',
          'bbbbbbbb-0000-0000-0000-00000000b812', '900000802', 'VISITANTE DOS', null, now(), 'OPERADOR PRUEBA');

  -- 6. Cerrada la visita del paso 1, el 801 queda libre para un ingreso por correo.
  update public.movimientos_visita
     set hora_salida = now(), dispositivo_salida_id = 'cccccccc-0000-0000-0000-00000000c801', usuario_salida_nombre = 'OPERADOR PRUEBA'
   where id = 'eeeeeeee-0000-0000-0000-00000000e801';
  insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801',
          '900000811', 'CORREO PRUEBA', 'ENTREVISTA RH', 801, now(), 'OPERADOR PRUEBA');
end $$;

rollback;

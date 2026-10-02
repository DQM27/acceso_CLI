-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Bloqueo cruzado de gafete (migración `gafete_en_uso_proveedor_y_kof`): en
-- una misma unidad, un gafete de proveedor o un gafete provisional KOF no
-- puede estar activo dos veces, aunque lo entreguen dos equipos distintos.
-- Al cerrarse el primero, el gafete se puede volver a entregar, y el mismo
-- número en OTRA unidad no choca. Se prueba como `service_role` porque lo que
-- se verifica es la restricción de la tabla, no la RLS.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a0a1', 'Unidad gafete A'),
  ('aaaaaaaa-0000-0000-0000-00000000a0a2', 'Unidad gafete B');
insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c0a1', 'aaaaaaaa-0000-0000-0000-00000000a0a1', 'pc', 'Gafete PC A1', '{"kty":"EC"}', 'diag-huella-gafete-a1'),
  ('cccccccc-0000-0000-0000-00000000c0a2', 'aaaaaaaa-0000-0000-0000-00000000a0a1', 'mobile', 'Gafete móvil A2', '{"kty":"EC"}', 'diag-huella-gafete-a2'),
  ('cccccccc-0000-0000-0000-00000000c0b1', 'aaaaaaaa-0000-0000-0000-00000000a0a2', 'pc', 'Gafete PC B1', '{"kty":"EC"}', 'diag-huella-gafete-b1');
insert into public.encargados_ruta (id, sitio_id, dispositivo_origen_id, codigo_empleado, nombre) values
  ('eeeeeeee-0000-0000-0000-00000000e0a1', 'aaaaaaaa-0000-0000-0000-00000000a0a1',
   'cccccccc-0000-0000-0000-00000000c0a1', 'DIAG-GAFETE-1', 'ENCARGADO DE PRUEBA');

set local role service_role;

-- 1. Proveedores.
do $$
begin
  insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre,
    empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values ('bbbbbbbb-0000-0000-0000-00000000b0a1', 'aaaaaaaa-0000-0000-0000-00000000a0a1',
    'cccccccc-0000-0000-0000-00000000c0a1', '111111111', 'PROVEEDOR UNO', 'EMPRESA', 9001, now(), 'OP');

  -- El mismo gafete desde otro equipo de la misma unidad: rechazado.
  begin
    insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre,
      empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a0a1',
      'cccccccc-0000-0000-0000-00000000c0a2', '222222222', 'PROVEEDOR DOS', 'EMPRESA', 9001, now(), 'OP');
    raise exception 'Un gafete de proveedor quedó activo dos veces en la misma unidad';
  exception when unique_violation then null;
  end;

  -- En otra unidad el mismo número no choca.
  insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre,
    empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a0a2',
    'cccccccc-0000-0000-0000-00000000c0b1', '333333333', 'PROVEEDOR TRES', 'EMPRESA', 9001, now(), 'OP');

  -- Cerrado el primero, el gafete se vuelve a entregar.
  update public.ingresos_proveedor
     set hora_salida = now(), usuario_salida_nombre = 'OP'
   where id = 'bbbbbbbb-0000-0000-0000-00000000b0a1';
  insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre,
    empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a0a1',
    'cccccccc-0000-0000-0000-00000000c0a2', '222222222', 'PROVEEDOR DOS', 'EMPRESA', 9001, now(), 'OP');
end $$;

-- 2. Gafetes provisionales KOF.
do $$
begin
  insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id,
    encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre)
  values ('bbbbbbbb-0000-0000-0000-00000000b0a2', 'aaaaaaaa-0000-0000-0000-00000000a0a1',
    'cccccccc-0000-0000-0000-00000000c0a1', 'eeeeeeee-0000-0000-0000-00000000e0a1',
    'ENCARGADO DE PRUEBA', 'DIAG-GAFETE-1', 9002, now(), 'OP');

  begin
    insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id,
      encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a0a1',
      'cccccccc-0000-0000-0000-00000000c0a2', 'eeeeeeee-0000-0000-0000-00000000e0a1',
      'ENCARGADO DE PRUEBA', 'DIAG-GAFETE-1', 9002, now(), 'OP');
    raise exception 'Un gafete provisional KOF quedó prestado dos veces en la misma unidad';
  exception when unique_violation then null;
  end;

  update public.prestamos_gafete_provisional
     set hora_devolucion = now(), usuario_devolucion_nombre = 'OP'
   where id = 'bbbbbbbb-0000-0000-0000-00000000b0a2';
  insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id,
    encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a0a1',
    'cccccccc-0000-0000-0000-00000000c0a2', 'eeeeeeee-0000-0000-0000-00000000e0a1',
    'ENCARGADO DE PRUEBA', 'DIAG-GAFETE-1', 9002, now(), 'OP');
end $$;

reset role;
select 'bloqueo cruzado de gafete correcto' as resultado;
rollback;

-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre 20261018130000_cambio_nube_lleva_la_fila_de_las_cuatro_clases: el
-- aviso `cambio_nube` (`private.emitir_cambio_nube_sitio`) lleva la fila
-- completa en `registro` para proveedores, ingresos por correo y préstamos
-- KOF, igual que para `ingresos`:
--   1. un ingreso de proveedor nuevo sale con su fila (y su gafete);
--   2. su salida sale con `hora_salida` en la fila;
--   3. un ingreso por correo y un préstamo KOF salen con su fila;
--   4. un DELETE sale sin fila (sólo el `id`), como siempre;
--   5. todo va por el canal privado del sitio (`sitio:<id>`).
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a701', 'Unidad aviso con fila');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'pc',
   'PC aviso con fila', '{"kty":"EC"}', 'huella-aviso-fila');

insert into public.encargados_ruta (id, sitio_id, dispositivo_origen_id, codigo_empleado, nombre) values
  ('dddddddd-0000-0000-0000-00000000d701', 'aaaaaaaa-0000-0000-0000-00000000a701',
   'cccccccc-0000-0000-0000-00000000c701', 'KOF-PRUEBA-701', 'ENCARGADO AVISO FILA');

insert into public.ingresos_proveedor (
  id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre, gafete_numero,
  hora_entrada, usuario_entrada_nombre
) values (
  'eeeeeeee-0000-0000-0000-00000000e701', 'aaaaaaaa-0000-0000-0000-00000000a701',
  'cccccccc-0000-0000-0000-00000000c701', '700000701', 'PROVEEDOR AVISO', 'Maika', 71,
  now() - interval '1 hour', 'Guarda'
);

update public.ingresos_proveedor
   set hora_salida = now(), dispositivo_salida_id = 'cccccccc-0000-0000-0000-00000000c701',
       usuario_salida_nombre = 'Guarda 2'
 where id = 'eeeeeeee-0000-0000-0000-00000000e701';

insert into public.ingresos_correo (
  id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero,
  hora_entrada, usuario_entrada_nombre
) values (
  'eeeeeeee-0000-0000-0000-00000000e702', 'aaaaaaaa-0000-0000-0000-00000000a701',
  'cccccccc-0000-0000-0000-00000000c701', '700000702', 'VISITA AVISO', 'Entrevista', 72,
  now(), 'Guarda'
);

insert into public.prestamos_gafete_provisional (
  id, sitio_id, dispositivo_entrega_id, encargado_id, encargado_nombre,
  encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre
) values (
  'eeeeeeee-0000-0000-0000-00000000e703', 'aaaaaaaa-0000-0000-0000-00000000a701',
  'cccccccc-0000-0000-0000-00000000c701', 'dddddddd-0000-0000-0000-00000000d701',
  'ENCARGADO AVISO FILA', 'KOF-PRUEBA-701', 73, now(), 'Guarda'
);

delete from public.ingresos_correo where id = 'eeeeeeee-0000-0000-0000-00000000e702';

create temporary table avisos_prueba on commit drop as
select payload, topic, private
  from realtime.messages
 where event = 'cambio_nube'
   and payload ->> 'sitio_id' = 'aaaaaaaa-0000-0000-0000-00000000a701';

do $$
begin
  -- 1. Proveedor nuevo, con su fila.
  if not exists (
    select 1 from avisos_prueba
     where payload ->> 'table' = 'ingresos_proveedor'
       and payload ->> 'operation' = 'INSERT'
       and payload -> 'registro' ->> 'id' = 'eeeeeeee-0000-0000-0000-00000000e701'
       and (payload -> 'registro' ->> 'gafete_numero')::bigint = 71
       and payload -> 'registro' ->> 'hora_salida' is null
  ) then
    raise exception 'El aviso de un ingreso de proveedor no lleva su fila';
  end if;

  -- 2. Su salida, con la hora de salida en la fila.
  if not exists (
    select 1 from avisos_prueba
     where payload ->> 'table' = 'ingresos_proveedor'
       and payload ->> 'operation' = 'UPDATE'
       and payload -> 'registro' ->> 'hora_salida' is not null
       and payload -> 'registro' ->> 'usuario_salida_nombre' = 'Guarda 2'
  ) then
    raise exception 'El aviso de la salida de un proveedor no lleva su fila';
  end if;

  -- 3. Correo y KOF, con su fila.
  if not exists (
    select 1 from avisos_prueba
     where payload ->> 'table' = 'ingresos_correo'
       and payload ->> 'operation' = 'INSERT'
       and payload -> 'registro' ->> 'motivo' = 'Entrevista'
  ) then
    raise exception 'El aviso de un ingreso por correo no lleva su fila';
  end if;
  if not exists (
    select 1 from avisos_prueba
     where payload ->> 'table' = 'prestamos_gafete_provisional'
       and payload ->> 'operation' = 'INSERT'
       and payload -> 'registro' ->> 'encargado_codigo_empleado' = 'KOF-PRUEBA-701'
       and payload -> 'registro' ->> 'hora_devolucion' is null
  ) then
    raise exception 'El aviso de un préstamo KOF no lleva su fila';
  end if;

  -- 4. Un DELETE sale sin fila, sólo con el id.
  if not exists (
    select 1 from avisos_prueba
     where payload ->> 'table' = 'ingresos_correo'
       and payload ->> 'operation' = 'DELETE'
       and payload ->> 'id' = 'eeeeeeee-0000-0000-0000-00000000e702'
       and payload -> 'registro' = 'null'::jsonb
  ) then
    raise exception 'El aviso de un DELETE lleva la fila o no lleva el id';
  end if;

  -- 5. Todo por el canal privado del sitio.
  if exists (
    select 1 from avisos_prueba
     where topic <> 'sitio:aaaaaaaa-0000-0000-0000-00000000a701' or not private
  ) then
    raise exception 'Un aviso salió por un canal que no es el privado del sitio';
  end if;
end $$;

rollback;

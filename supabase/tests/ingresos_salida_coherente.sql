-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- `ingresos_salida_coherente` (migración `usuarios_globales_y_coherencia`):
-- un ingreso abierto no lleva datos de salida y uno cerrado lleva quién lo
-- cerró. A propósito no se exige que la salida sea posterior a la entrada:
-- los relojes de dos equipos pueden estar desfasados.
begin;

insert into public.sitios (id, nombre) values ('aaaaaaaa-0000-0000-0000-0000000000c1', 'Unidad coherencia');
insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-0000000000c1', 'aaaaaaaa-0000-0000-0000-0000000000c1', 'pc', 'Coherencia PC', '{"kty":"EC"}', 'diag-huella-coherencia');
insert into public.contratistas (id, dispositivo_origen_id, nombre)
values ('dddddddd-0000-0000-0000-0000000000c1', 'cccccccc-0000-0000-0000-0000000000c1', 'Diagnóstico coherencia');

do $$
begin
  insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre, hora_entrada)
  values ('bbbbbbbb-0000-0000-0000-0000000000c1', 'aaaaaaaa-0000-0000-0000-0000000000c1',
          'cccccccc-0000-0000-0000-0000000000c1', 'dddddddd-0000-0000-0000-0000000000c1', 'Diagnóstico', now());

  -- Cerrado sin quién lo cerró: rechazado.
  begin
    update public.ingresos set hora_salida = now() where id = 'bbbbbbbb-0000-0000-0000-0000000000c1';
    raise exception 'Se aceptó una salida sin usuario';
  exception when check_violation then null;
  end;

  -- Abierto con datos de salida: rechazado.
  begin
    insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre,
                                 hora_entrada, usuario_salida_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-0000000000c1', 'cccccccc-0000-0000-0000-0000000000c1',
            'dddddddd-0000-0000-0000-0000000000c1', 'Diagnóstico', now(), 'OP');
    raise exception 'Se aceptó un ingreso abierto con datos de salida';
  exception when check_violation then null;
  end;

  -- Cierre normal, aunque la salida quede "antes" por reloj desfasado.
  update public.ingresos
     set hora_salida = hora_entrada - interval '30 seconds',
         usuario_salida_nombre = 'OP',
         dispositivo_salida_id = 'cccccccc-0000-0000-0000-0000000000c1'
   where id = 'bbbbbbbb-0000-0000-0000-0000000000c1';
  if not found then
    raise exception 'Un cierre válido fue rechazado';
  end if;
end $$;

select 'coherencia de salida correcta' as resultado;
rollback;

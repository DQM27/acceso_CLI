-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre 20261003190000_ingreso_unico_proveedores_y_kof:
-- - Proveedores: un equipo de la unidad B ve, con `ingreso_proveedor_activo`,
--   que el proveedor está adentro en la unidad A (la RLS se lo oculta); el
--   índice único rechaza un segundo ingreso abierto de la misma cédula en
--   cualquier unidad y deja registrarlo de nuevo tras la salida; y
--   `proveedores_activos_en_otras_unidades` reporta sólo lo de otras unidades.
-- - Gafete provisional KOF (el encargado se identifica por su código de
--   empleado): `prestamo_provisional_activo_de_encargado` ve el préstamo de
--   otra unidad; el índice único rechaza un segundo préstamo sin devolver al
--   mismo encargado (misma u otra unidad) y lo permite tras la devolución.
-- - Una sesión sin equipo y anon no consultan.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a601', 'Unidad proveedor A'),
  ('aaaaaaaa-0000-0000-0000-00000000a602', 'Unidad proveedor B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'pc',     'PC A',      '{"kty":"EC"}', 'huella-prov-a'),
  ('cccccccc-0000-0000-0000-00000000c602', 'aaaaaaaa-0000-0000-0000-00000000a602', 'mobile', 'Celular B', '{"kty":"EC"}', 'huella-prov-b'),
  ('cccccccc-0000-0000-0000-00000000c603', 'aaaaaaaa-0000-0000-0000-00000000a601', 'mobile', 'Celular A', '{"kty":"EC"}', 'huella-prov-a2');

insert into public.encargados_ruta (id, sitio_id, dispositivo_origen_id, codigo_empleado, nombre) values
  ('dddddddd-0000-0000-0000-00000000d601', 'aaaaaaaa-0000-0000-0000-00000000a601',
   'cccccccc-0000-0000-0000-00000000c601', 'KOF-PRUEBA-601', 'ENCARGADO PRUEBA UNICO');

-- Hace de equipo: JWT de dispositivo (sub, sitio_id, huella y tipo; crear
-- filas exige un tipo distinto de 'visor').
create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text) returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella, 'tipo', 'mobile')::text, true)
$$;

set local role authenticated;

-- ---- Proveedores ----
do $$
declare
  v_sitio uuid;
  v_nombre text;
  v_n int;
begin
  -- 1. La PC de la unidad A registra el ingreso del proveedor.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-prov-a');
  insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values ('eeeeeeee-0000-0000-0000-00000000e601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'cccccccc-0000-0000-0000-00000000c601',
          '900000601', 'PROVEEDOR PRUEBA UNICO', 'EMPRESA PRUEBA', 601, now(), 'OPERADOR PRUEBA');

  -- 2. Desde la unidad B la tabla sigue cerrada, pero la verificación lo ve.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c602', 'aaaaaaaa-0000-0000-0000-00000000a602', 'huella-prov-b');
  if exists (select 1 from public.ingresos_proveedor where cedula = '900000601') then
    raise exception 'P2a: un equipo de otra unidad no debería leer los ingresos de proveedor de A';
  end if;
  select sitio_id, sitio_nombre into v_sitio, v_nombre from public.ingreso_proveedor_activo(' 900000601 ');
  if v_sitio is distinct from 'aaaaaaaa-0000-0000-0000-00000000a601' or v_nombre <> 'Unidad proveedor A' then
    raise exception 'P2b: la verificación debería ver el ingreso en A, dio % / %', v_sitio, v_nombre;
  end if;
  select count(*) into v_n from public.ingreso_proveedor_activo('000000000');
  if v_n <> 0 then raise exception 'P2c: una cédula sin ingreso debería dar vacío'; end if;

  -- 3. El aviso posterior a sincronizar: B lo ve; A no ve el propio.
  select count(*) into v_n from public.proveedores_activos_en_otras_unidades(array['900000601', '000000000']);
  if v_n <> 1 then raise exception 'P3a: B debería ver 1 proveedor en otra unidad, vio %', v_n; end if;
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c603', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-prov-a2');
  select count(*) into v_n from public.proveedores_activos_en_otras_unidades(array['900000601']);
  if v_n <> 0 then raise exception 'P3b: A no debería ver su propio ingreso como de otra unidad'; end if;

  -- 4. Segundo ingreso abierto de la misma cédula en la misma unidad (otro equipo).
  begin
    insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a601', 'cccccccc-0000-0000-0000-00000000c603',
            '900000601', 'PROVEEDOR PRUEBA UNICO', 'EMPRESA PRUEBA', 602, now(), 'OPERADOR PRUEBA');
    raise exception 'P4a: se aceptó un segundo ingreso de proveedor abierto en la misma unidad';
  exception
    when unique_violation then null;
  end;

  -- 5. ...y en otra unidad.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c602', 'aaaaaaaa-0000-0000-0000-00000000a602', 'huella-prov-b');
  begin
    insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a602', 'cccccccc-0000-0000-0000-00000000c602',
            '900000601', 'PROVEEDOR PRUEBA UNICO', 'EMPRESA PRUEBA', 601, now(), 'OPERADOR PRUEBA');
    raise exception 'P5a: se aceptó un segundo ingreso de proveedor abierto en otra unidad';
  exception
    when unique_violation then null;
  end;

  -- 6. Tras la salida en A, B ya puede registrarlo.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-prov-a');
  update public.ingresos_proveedor set hora_salida = now(), usuario_salida_nombre = 'OPERADOR PRUEBA'
   where id = 'eeeeeeee-0000-0000-0000-00000000e601';
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c602', 'aaaaaaaa-0000-0000-0000-00000000a602', 'huella-prov-b');
  select count(*) into v_n from public.ingreso_proveedor_activo('900000601');
  if v_n <> 0 then raise exception 'P6a: después de la salida no debería quedar activo'; end if;
  insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a602', 'cccccccc-0000-0000-0000-00000000c602',
          '900000601', 'PROVEEDOR PRUEBA UNICO', 'EMPRESA PRUEBA', 601, now(), 'OPERADOR PRUEBA');
end $$;

-- ---- Gafete provisional KOF ----
do $$
declare
  v_sitio uuid;
  v_nombre text;
  v_n int;
begin
  -- 1. La PC de la unidad A le presta un gafete provisional al encargado.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-prov-a');
  insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre)
  values ('ffffffff-0000-0000-0000-00000000f601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'cccccccc-0000-0000-0000-00000000c601',
          'dddddddd-0000-0000-0000-00000000d601', 'ENCARGADO PRUEBA UNICO', 'KOF-PRUEBA-601', 9601, now(), 'OPERADOR PRUEBA');

  -- 2. Desde la unidad B, la verificación ve el préstamo y dónde.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c602', 'aaaaaaaa-0000-0000-0000-00000000a602', 'huella-prov-b');
  if exists (select 1 from public.prestamos_gafete_provisional where encargado_codigo_empleado = 'KOF-PRUEBA-601') then
    raise exception 'K2a: un equipo de otra unidad no debería leer los préstamos de A';
  end if;
  select sitio_id, sitio_nombre into v_sitio, v_nombre
    from public.prestamo_provisional_activo_de_encargado(' KOF-PRUEBA-601 ');
  if v_sitio is distinct from 'aaaaaaaa-0000-0000-0000-00000000a601' or v_nombre <> 'Unidad proveedor A' then
    raise exception 'K2b: la verificación debería ver el préstamo en A, dio % / %', v_sitio, v_nombre;
  end if;

  -- 3. Segundo préstamo sin devolver al mismo encargado, en otra unidad.
  begin
    insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a602', 'cccccccc-0000-0000-0000-00000000c602',
            'dddddddd-0000-0000-0000-00000000d601', 'ENCARGADO PRUEBA UNICO', 'KOF-PRUEBA-601', 9602, now(), 'OPERADOR PRUEBA');
    raise exception 'K3a: se aceptó un segundo préstamo al mismo encargado en otra unidad';
  exception
    when unique_violation then null;
  end;

  -- 4. ...y en la misma unidad, desde otro equipo.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c603', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-prov-a2');
  begin
    insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a601', 'cccccccc-0000-0000-0000-00000000c603',
            'dddddddd-0000-0000-0000-00000000d601', 'ENCARGADO PRUEBA UNICO', 'KOF-PRUEBA-601', 9603, now(), 'OPERADOR PRUEBA');
    raise exception 'K4a: se aceptó un segundo préstamo al mismo encargado en la misma unidad';
  exception
    when unique_violation then null;
  end;

  -- 5. Tras la devolución, ya se le puede prestar de nuevo (en otra unidad).
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c601', 'aaaaaaaa-0000-0000-0000-00000000a601', 'huella-prov-a');
  update public.prestamos_gafete_provisional
     set hora_devolucion = now(), usuario_devolucion_nombre = 'OPERADOR PRUEBA'
   where id = 'ffffffff-0000-0000-0000-00000000f601';
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c602', 'aaaaaaaa-0000-0000-0000-00000000a602', 'huella-prov-b');
  select count(*) into v_n from public.prestamo_provisional_activo_de_encargado(' KOF-PRUEBA-601 ');
  if v_n <> 0 then raise exception 'K5a: después de la devolución no debería quedar activo'; end if;
  insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a602', 'cccccccc-0000-0000-0000-00000000c602',
          'dddddddd-0000-0000-0000-00000000d601', 'ENCARGADO PRUEBA UNICO', 'KOF-PRUEBA-601', 9602, now(), 'OPERADOR PRUEBA');
end $$;

-- Una sesión sin equipo (ni administrador) no consulta.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', gen_random_uuid(), 'email', 'alguien@example.com')::text, true);
do $$
begin
  perform 1 from public.ingreso_proveedor_activo('900000601');
  raise exception 'Una sesión sin equipo pudo consultar ingresos de proveedor';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.proveedores_activos_en_otras_unidades(array['900000601']);
  raise exception 'Una sesión sin equipo pudo consultar proveedores de otras unidades';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.prestamo_provisional_activo_de_encargado(' KOF-PRUEBA-601 ');
  raise exception 'Una sesión sin equipo pudo consultar préstamos provisionales';
exception
  when insufficient_privilege then null;
end $$;

-- anon tampoco.
reset role;
set local role anon;
do $$
begin
  perform 1 from public.ingreso_proveedor_activo('900000601');
  raise exception 'anon pudo ejecutar ingreso_proveedor_activo';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.prestamo_provisional_activo_de_encargado(' KOF-PRUEBA-601 ');
  raise exception 'anon pudo ejecutar prestamo_provisional_activo_de_encargado';
exception
  when insufficient_privilege then null;
end $$;

rollback;

-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre 20261029120000_historial_panel_proveedores_correo_kof:
-- - El administrador del panel lee las tres vistas, de todas las unidades,
--   con la unidad, el tipo de dispositivo y las columnas sin tildes ni
--   mayúsculas (`*_p`) ya armadas.
-- - KOF publica la entrega y la devolución como entrada y salida.
-- - La RLS de cada tabla rige a través de la vista (`security_invoker`): un
--   equipo de la unidad B no ve lo de A, y una sesión que no es
--   administradora ni equipo no ve nada.
-- - Permisos: `authenticated` y `service_role` leen; `anon` no.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a871', 'Unidad Historial Á'),
  ('aaaaaaaa-0000-0000-0000-00000000a872', 'Unidad Historial B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c871', 'aaaaaaaa-0000-0000-0000-00000000a871', 'pc',     'PC A',      '{"kty":"EC"}', 'huella-hist-a'),
  ('cccccccc-0000-0000-0000-00000000c872', 'aaaaaaaa-0000-0000-0000-00000000a871', 'mobile', 'Celular A', '{"kty":"EC"}', 'huella-hist-a2'),
  ('cccccccc-0000-0000-0000-00000000c873', 'aaaaaaaa-0000-0000-0000-00000000a872', 'pc',     'PC B',      '{"kty":"EC"}', 'huella-hist-b');

insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre, placa,
    gafete_numero, hora_entrada, usuario_entrada_nombre, hora_salida, usuario_salida_nombre) values
  ('eeeeeeee-0000-0000-0000-00000000e871', 'aaaaaaaa-0000-0000-0000-00000000a871', 'cccccccc-0000-0000-0000-00000000c871',
   '900000871', 'JOSÉ PÉREZ', 'Distribuidora Ñandú', 'CL 371931', 7,
   now() - interval '3 hours', 'OPERADOR A', now() - interval '2 hours', 'OPERADOR A'),
  ('eeeeeeee-0000-0000-0000-00000000e872', 'aaaaaaaa-0000-0000-0000-00000000a872', 'cccccccc-0000-0000-0000-00000000c873',
   '900000872', 'ANA ROJAS', 'MAIKA', null, 8,
   now() - interval '5 hours', 'OPERADOR B', now() - interval '4 hours', 'OPERADOR B');

insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, placa,
    gafete_numero, hora_entrada, usuario_entrada_nombre, hora_salida, usuario_salida_nombre) values
  ('eeeeeeee-0000-0000-0000-00000000e873', 'aaaaaaaa-0000-0000-0000-00000000a871', 'cccccccc-0000-0000-0000-00000000c872',
   '900000873', 'LUIS MORA', 'Entrevista RH', null, 5,
   now() - interval '2 hours', 'OPERADOR A', now() - interval '1 hour', 'OPERADOR A');

insert into public.encargados_ruta (id, sitio_id, dispositivo_origen_id, codigo_empleado, nombre) values
  ('bbbbbbbb-0000-0000-0000-00000000b871', 'aaaaaaaa-0000-0000-0000-00000000a871', 'cccccccc-0000-0000-0000-00000000c871',
   'EMP-871', 'ENCARGADO HISTORIAL');

insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id, encargado_nombre,
    encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre, hora_devolucion, usuario_devolucion_nombre) values
  ('eeeeeeee-0000-0000-0000-00000000e874', 'aaaaaaaa-0000-0000-0000-00000000a871', 'cccccccc-0000-0000-0000-00000000c871',
   'bbbbbbbb-0000-0000-0000-00000000b871', 'ENCARGADO HISTORIAL', 'EMP-871', 3,
   now() - interval '6 hours', 'OPERADOR ENTREGA', now() - interval '5 hours', 'OPERADOR DEVOLUCION');

-- Un administrador del panel (correo en `administradores_panel`).
insert into public.administradores_panel (correo) values ('historial-admin@example.com');

-- Permisos de la API, antes de cambiar de rol.
do $$
declare
  v_vista text;
begin
  foreach v_vista in array array['panel_movimientos_proveedor', 'panel_movimientos_correo', 'panel_movimientos_kof'] loop
    if not has_table_privilege('authenticated', 'public.' || v_vista, 'select') then
      raise exception 'P1: authenticated debería leer %', v_vista;
    end if;
    if not has_table_privilege('service_role', 'public.' || v_vista, 'select') then
      raise exception 'P2: service_role debería leer %', v_vista;
    end if;
    if has_table_privilege('anon', 'public.' || v_vista, 'select') then
      raise exception 'P3: anon no debería leer %', v_vista;
    end if;
  end loop;
end $$;

create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text, p_tipo text default 'pc') returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella, 'tipo', p_tipo)::text, true)
$$;
-- Desde 20261020130000_permisos_api_explicitos ninguna función nueva es
-- ejecutable por `public`, tampoco esta auxiliar de la prueba.
grant execute on all functions in schema pg_temp to authenticated;

set local role authenticated;

-- 1. El administrador ve las dos unidades y las columnas armadas.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'historial-admin@example.com')::text, true);
do $$
declare
  v_n int;
  v_fila record;
begin
  select count(*) into v_n from public.panel_movimientos_proveedor where cedula in ('900000871', '900000872');
  if v_n <> 2 then
    raise exception 'H1a: el administrador debería ver los dos proveedores (de A y de B), vio %', v_n;
  end if;

  select * into v_fila from public.panel_movimientos_proveedor where cedula = '900000871';
  if v_fila.sitio_nombre <> 'Unidad Historial Á' or v_fila.dispositivo_entrada_tipo <> 'pc'
     or v_fila.nombre_p <> 'jose perez' or v_fila.empresa_p <> 'distribuidora nandu'
     or v_fila.unidad_p <> 'unidad historial a' or v_fila.placa_p <> 'cl 371931' then
    raise exception 'H1b: fila de proveedor mal armada: % / % / % / % / % / %',
      v_fila.sitio_nombre, v_fila.dispositivo_entrada_tipo, v_fila.nombre_p, v_fila.empresa_p, v_fila.unidad_p, v_fila.placa_p;
  end if;

  -- El buscador del panel filtra por las columnas plegadas.
  select count(*) into v_n from public.panel_movimientos_proveedor
   where nombre_p like '%perez%' and cedula like '90000087%';
  if v_n <> 1 then
    raise exception 'H1c: la búsqueda por nombre plegado debería dar 1, dio %', v_n;
  end if;

  select * into v_fila from public.panel_movimientos_correo where cedula = '900000873';
  if v_fila.motivo_p <> 'entrevista rh' or v_fila.dispositivo_entrada_tipo <> 'mobile' or v_fila.hora_salida is null then
    raise exception 'H1d: fila de correo mal armada: % / % / %', v_fila.motivo_p, v_fila.dispositivo_entrada_tipo, v_fila.hora_salida;
  end if;

  -- 2. KOF: entrega y devolución como entrada y salida.
  select * into v_fila from public.panel_movimientos_kof where codigo_empleado = 'EMP-871';
  if v_fila.nombre <> 'ENCARGADO HISTORIAL' or v_fila.codigo_p <> 'emp-871'
     or v_fila.usuario_entrada_nombre <> 'OPERADOR ENTREGA' or v_fila.usuario_salida_nombre <> 'OPERADOR DEVOLUCION'
     or v_fila.hora_entrada is null or v_fila.hora_salida <= v_fila.hora_entrada then
    raise exception 'H2: fila KOF mal armada: % / % / % / %',
      v_fila.nombre, v_fila.codigo_p, v_fila.usuario_entrada_nombre, v_fila.usuario_salida_nombre;
  end if;
end $$;

-- 3. Un equipo de B sólo ve lo de B.
select pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c873', 'aaaaaaaa-0000-0000-0000-00000000a872', 'huella-hist-b');
do $$
begin
  if exists (select 1 from public.panel_movimientos_proveedor where cedula = '900000871')
     or exists (select 1 from public.panel_movimientos_correo where cedula = '900000873')
     or exists (select 1 from public.panel_movimientos_kof where codigo_empleado = 'EMP-871') then
    raise exception 'H3a: un equipo de B no debería ver los movimientos de A';
  end if;
  if not exists (select 1 from public.panel_movimientos_proveedor where cedula = '900000872') then
    raise exception 'H3b: un equipo de B debería ver los movimientos de B';
  end if;
end $$;

-- 4. Una sesión que no es administradora ni equipo no ve nada.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'historial-nadie@example.com')::text, true);
do $$
begin
  if exists (select 1 from public.panel_movimientos_proveedor where cedula like '90000087%')
     or exists (select 1 from public.panel_movimientos_correo where cedula like '90000087%')
     or exists (select 1 from public.panel_movimientos_kof where codigo_empleado = 'EMP-871') then
    raise exception 'H4: una sesión sin permisos no debería ver ningún movimiento';
  end if;
end $$;

rollback;

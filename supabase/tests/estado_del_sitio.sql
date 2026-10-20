-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `estado_del_sitio`: por cada grupo pedido devuelve las salidas que la
-- nube ya tiene de los ids abiertos del equipo (y sólo esas) y lo abierto del
-- propio sitio (no de otro), con las columnas completas aunque vengan en
-- null; un grupo no pedido no aparece; sin ids no hay cierres; lo abierto es
-- lo mismo que un SELECT directo con la RLS del equipo; un equipo retirado no
-- ve nada y anon no la ejecuta.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a801', 'Unidad estado A'),
  ('aaaaaaaa-0000-0000-0000-00000000a802', 'Unidad estado B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  ('cccccccc-0000-0000-0000-00000000c801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'pc', 'Estado A1', '{"kty":"EC"}', 'huella-est-a1', null),
  ('cccccccc-0000-0000-0000-00000000c802', 'aaaaaaaa-0000-0000-0000-00000000a801', 'mobile', 'Estado A2', '{"kty":"EC"}', 'huella-est-a2', null),
  ('cccccccc-0000-0000-0000-00000000c803', 'aaaaaaaa-0000-0000-0000-00000000a801', 'mobile', 'Estado A3 retirado', '{"kty":"EC"}', 'huella-est-a3', now()),
  ('cccccccc-0000-0000-0000-00000000c804', 'aaaaaaaa-0000-0000-0000-00000000a802', 'mobile', 'Estado B1', '{"kty":"EC"}', 'huella-est-b1', null);

insert into public.contratistas (id, nombre, identificacion) values
  ('dddddddd-0000-0000-0000-00000000d801', 'Contratista estado 1', '9-0801-0001'),
  ('dddddddd-0000-0000-0000-00000000d802', 'Contratista estado 2', '9-0801-0002'),
  ('dddddddd-0000-0000-0000-00000000d803', 'Contratista estado 3', '9-0801-0003');

-- Contratistas: A1 abrió uno que A2 ya cerró (e801) y otro que sigue abierto
-- (e802, con empresa/placa/gafete en null); B tiene uno abierto (e803).
insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre,
    contratista_cedula, hora_entrada, hora_salida, dispositivo_salida_id, usuario_salida_nombre) values
  ('eeeeeeee-0000-0000-0000-00000000e801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801',
   'dddddddd-0000-0000-0000-00000000d801', 'Contratista estado 1', '9-0801-0001', now() - interval '2 hours',
   now() - interval '1 hour', 'cccccccc-0000-0000-0000-00000000c802', 'OPERADOR A2'),
  ('eeeeeeee-0000-0000-0000-00000000e802', 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801',
   'dddddddd-0000-0000-0000-00000000d802', 'Contratista estado 2', '9-0801-0002', now() - interval '1 hour',
   null, null, null),
  ('eeeeeeee-0000-0000-0000-00000000e803', 'aaaaaaaa-0000-0000-0000-00000000a802', 'cccccccc-0000-0000-0000-00000000c804',
   'dddddddd-0000-0000-0000-00000000d803', 'Contratista estado 3', '9-0801-0003', now() - interval '1 hour',
   null, null, null);

-- Proveedores: uno cerrado y uno abierto en A.
insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre,
    gafete_numero, hora_entrada, usuario_entrada_nombre, hora_salida, dispositivo_salida_id, usuario_salida_nombre) values
  ('ffffffff-0000-0000-0000-00000000f801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801',
   '9-0801-0011', 'Proveedor estado 1', 'Empresa P', 98001, now() - interval '2 hours', 'OPERADOR A1',
   now() - interval '1 hour', 'cccccccc-0000-0000-0000-00000000c802', 'OPERADOR A2'),
  ('ffffffff-0000-0000-0000-00000000f802', 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c802',
   '9-0801-0012', 'Proveedor estado 2', 'Empresa P', 98002, now() - interval '1 hour', 'OPERADOR A2',
   null, null, null);

-- Correo: uno abierto en A.
insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero,
    hora_entrada, usuario_entrada_nombre) values
  ('99999999-0000-0000-0000-000000009801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c802',
   '9-0801-0021', 'Correo estado 1', 'Entrega', 98011, now() - interval '1 hour', 'OPERADOR A2');

-- Préstamos KOF: uno devuelto y uno abierto en A.
insert into public.encargados_ruta (id, sitio_id, dispositivo_origen_id, codigo_empleado, nombre) values
  ('55555555-0000-0000-0000-000000005801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801', 'KOF-E801', 'ENCARGADO ESTADO');
insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id, encargado_nombre,
    encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre,
    hora_devolucion, dispositivo_devolucion_id, usuario_devolucion_nombre) values
  ('44444444-0000-0000-0000-000000004801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801',
   '55555555-0000-0000-0000-000000005801', 'ENCARGADO ESTADO', 'KOF-E801', 98021, now() - interval '3 hours', 'OPERADOR A1',
   now() - interval '2 hours', 'cccccccc-0000-0000-0000-00000000c802', 'OPERADOR A2'),
  ('44444444-0000-0000-0000-000000004802', 'aaaaaaaa-0000-0000-0000-00000000a801', 'cccccccc-0000-0000-0000-00000000c801',
   '55555555-0000-0000-0000-000000005801', 'ENCARGADO ESTADO', 'KOF-E801', 98022, now() - interval '1 hour', 'OPERADOR A1',
   null, null, null);

create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text) returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella)::text, true)
$$;

-- Los id de un grupo del resultado, sólo los de esta prueba
-- (`xxxxxxxx-0000-0000-0000-...`), para no depender de lo que ya haya abierto.
create or replace function pg_temp.ids(p_estado jsonb, p_grupo text) returns text[]
language sql as $$
  select coalesce(array_agg(x ->> 'id' order by x ->> 'id'), '{}')
    from jsonb_array_elements(coalesce(p_estado -> p_grupo, '[]')) x
   where x ->> 'id' like '________-0000-0000-0000-%'
$$;
-- Desde 20261020130000_permisos_api_explicitos ninguna función nueva es
-- ejecutable por `public`, tampoco estas auxiliares de la prueba.
grant execute on all functions in schema pg_temp to authenticated;

set local role authenticated;
select pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c801', 'aaaaaaaa-0000-0000-0000-00000000a801', 'huella-est-a1');

-- 1. Todos los grupos: cierres sólo de lo ya cerrado entre los ids pedidos;
--    abiertos sólo del propio sitio.
do $$
declare
  e jsonb := public.estado_del_sitio(
    array['ingresos', 'proveedor', 'correo', 'prestamos'],
    array['eeeeeeee-0000-0000-0000-00000000e801', 'eeeeeeee-0000-0000-0000-00000000e802'],
    array['ffffffff-0000-0000-0000-00000000f801'],
    array['99999999-0000-0000-0000-000000009801'],
    array['44444444-0000-0000-0000-000000004801', '44444444-0000-0000-0000-000000004802']);
begin
  if pg_temp.ids(e, 'cierres_ingresos') <> array['eeeeeeee-0000-0000-0000-00000000e801']
     or e -> 'cierres_ingresos' -> 0 ->> 'usuario_salida_nombre' <> 'OPERADOR A2' then
    raise exception '1: cierres_ingresos %', e -> 'cierres_ingresos';
  end if;
  if pg_temp.ids(e, 'ingresos_abiertos') <> array['eeeeeeee-0000-0000-0000-00000000e802'] then
    raise exception '1: ingresos_abiertos % (no debe traer el de la unidad B)', pg_temp.ids(e, 'ingresos_abiertos');
  end if;
  if pg_temp.ids(e, 'cierres_proveedor') <> array['ffffffff-0000-0000-0000-00000000f801']
     or pg_temp.ids(e, 'proveedor_abiertos') <> array['ffffffff-0000-0000-0000-00000000f802'] then
    raise exception '1: proveedor % / %', pg_temp.ids(e, 'cierres_proveedor'), pg_temp.ids(e, 'proveedor_abiertos');
  end if;
  if pg_temp.ids(e, 'cierres_correo') <> '{}'
     or pg_temp.ids(e, 'correo_abiertos') <> array['99999999-0000-0000-0000-000000009801'] then
    raise exception '1: correo % / %', pg_temp.ids(e, 'cierres_correo'), pg_temp.ids(e, 'correo_abiertos');
  end if;
  if pg_temp.ids(e, 'devoluciones_prestamos') <> array['44444444-0000-0000-0000-000000004801']
     or pg_temp.ids(e, 'prestamos_abiertos') <> array['44444444-0000-0000-0000-000000004802'] then
    raise exception '1: prestamos % / %', pg_temp.ids(e, 'devoluciones_prestamos'), pg_temp.ids(e, 'prestamos_abiertos');
  end if;
end $$;

-- 2. Sólo lo pedido, y sin ids no hay cierres.
do $$
declare
  e jsonb := public.estado_del_sitio(array['ingresos']);
  claves text[];
begin
  select array_agg(k order by k) into claves from jsonb_object_keys(e) k;
  if claves <> array['ingresos_abiertos'] then
    raise exception '2: claves %', claves;
  end if;
end $$;

-- 3. Mismas columnas que los GET de antes, también las que vienen en null.
do $$
declare
  e jsonb := public.estado_del_sitio(array['ingresos', 'prestamos'],
    array['eeeeeeee-0000-0000-0000-00000000e801'], null, null, array['44444444-0000-0000-0000-000000004801']);
  claves text[];
begin
  select array_agg(k order by k) into claves from jsonb_object_keys(
    (select x from jsonb_array_elements(e -> 'ingresos_abiertos') x where x ->> 'id' = 'eeeeeeee-0000-0000-0000-00000000e802')) k;
  if claves <> array['contratista_cedula', 'contratista_nombre', 'dispositivo_entrada_id', 'empresa_nombre',
                     'gafete_numero', 'hora_entrada', 'id', 'medio_ingreso', 'placa', 'tipo_ingreso',
                     'usuario_entrada_nombre'] then
    raise exception '3: columnas de ingresos_abiertos %', claves;
  end if;
  select array_agg(k order by k) into claves from jsonb_object_keys(e -> 'cierres_ingresos' -> 0) k;
  if claves <> array['hora_salida', 'id', 'usuario_salida_nombre'] then
    raise exception '3: columnas de cierres_ingresos %', claves;
  end if;
  select array_agg(k order by k) into claves from jsonb_object_keys(e -> 'devoluciones_prestamos' -> 0) k;
  if claves <> array['hora_devolucion', 'id', 'usuario_devolucion_nombre'] then
    raise exception '3: columnas de devoluciones_prestamos %', claves;
  end if;
end $$;

-- 4. Lo abierto es lo mismo que un SELECT directo con la RLS del equipo.
do $$
declare
  e jsonb := public.estado_del_sitio(array['ingresos']);
begin
  if (select coalesce(array_agg(x ->> 'id' order by x ->> 'id'), '{}') from jsonb_array_elements(e -> 'ingresos_abiertos') x)
     <> (select coalesce(array_agg(id::text order by id::text), '{}') from public.ingresos
          where sitio_id = 'aaaaaaaa-0000-0000-0000-00000000a801' and hora_salida is null) then
    raise exception '4: ingresos abiertos distintos al SELECT directo';
  end if;
end $$;

-- 4b. Conflictos: una cédula que este equipo tiene adentro (p. ej. registrada
--     sin red) y que la nube tiene abierta en la unidad B. Igual a la función
--     de la regla, que sigue siendo la fuente.
do $$
declare
  e jsonb := public.estado_del_sitio(array['ingresos'], null, null, null, null,
    array['9-0801-0003', '9-0801-0099']);
begin
  if e -> 'conflictos_ingresos' <> '[{"contratista_cedula": "9-0801-0003", "sitio_nombre": "Unidad estado B"}]'::jsonb then
    raise exception '4b: conflictos_ingresos %', e -> 'conflictos_ingresos';
  end if;
  if e -> 'conflictos_ingresos' <> (select jsonb_agg(f)
       from public.contratistas_activos_en_otras_unidades(array['9-0801-0003', '9-0801-0099']) f) then
    raise exception '4b: distinto a contratistas_activos_en_otras_unidades';
  end if;
  -- Sin cédulas no hay conflictos que buscar.
  if public.estado_del_sitio(array['ingresos']) ? 'conflictos_ingresos' then
    raise exception '4b: sin cédulas trajo conflictos';
  end if;
end $$;

-- 5. Un equipo retirado no ve nada.
select pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c803', 'aaaaaaaa-0000-0000-0000-00000000a801', 'huella-est-a3');
do $$
declare
  e jsonb := public.estado_del_sitio(array['ingresos', 'proveedor', 'correo', 'prestamos'],
    array['eeeeeeee-0000-0000-0000-00000000e801'], null, null, null);
  grupo text;
begin
  for grupo in select jsonb_object_keys(e) loop
    if jsonb_array_length(e -> grupo) <> 0 then
      raise exception '5: un equipo retirado vio %', grupo;
    end if;
  end loop;
end $$;

-- 6. anon no la ejecuta.
reset role;
set local role anon;
do $$
begin
  perform public.estado_del_sitio(array['ingresos']);
  raise exception '6: anon pudo ejecutar estado_del_sitio';
exception when insufficient_privilege then null;
end $$;
reset role;

rollback;

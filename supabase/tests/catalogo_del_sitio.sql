-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `catalogo_del_sitio`: trae sólo lo que cambió desde cada marca (los
-- gafetes con la suya), sólo lo del propio sitio en las tablas por sitio,
-- respeta el tamaño de página y el orden por id, devuelve las mismas
-- columnas que los GET de antes y lo mismo que un SELECT directo con la RLS
-- del equipo; un equipo retirado no ve nada y anon no la ejecuta.
--
-- Las filas de prueba tienen `updated_at` en 2099 y las consultas usan marcas
-- de 2099: así lo que ya existe en la base no entra en las cuentas.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a701', 'Unidad catalogo A'),
  ('aaaaaaaa-0000-0000-0000-00000000a702', 'Unidad catalogo B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  ('cccccccc-0000-0000-0000-00000000c701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'mobile', 'Catalogo A1', '{"kty":"EC"}', 'huella-cat-a1', null),
  ('cccccccc-0000-0000-0000-00000000c702', 'aaaaaaaa-0000-0000-0000-00000000a701', 'mobile', 'Catalogo A2 retirado', '{"kty":"EC"}', 'huella-cat-a2', now()),
  ('cccccccc-0000-0000-0000-00000000c703', 'aaaaaaaa-0000-0000-0000-00000000a702', 'mobile', 'Catalogo B1', '{"kty":"EC"}', 'huella-cat-b1', null);

-- Globales: una "vieja" (justo en la marca, no debe venir) y dos nuevas.
insert into public.empresas (id, nombre, activa, updated_at) values
  ('eeeeeeee-0000-0000-0000-00000000e701', 'Empresa catalogo vieja', true, '2099-01-01T00:00:00Z'),
  ('eeeeeeee-0000-0000-0000-00000000e702', 'Empresa catalogo nueva 1', true, '2099-01-02T00:00:00Z'),
  ('eeeeeeee-0000-0000-0000-00000000e703', 'Empresa catalogo nueva 2', false, '2099-01-03T00:00:00Z');
insert into public.contratistas (id, nombre, identificacion, empresa_id, empresa_nombre, activo,
    tipo_ingreso, fecha_vencimiento_praind, es_personal_ruta, updated_at) values
  ('dddddddd-0000-0000-0000-00000000d701', 'Persona catalogo', '9-0701-0701', 'eeeeeeee-0000-0000-0000-00000000e702',
   'Empresa catalogo nueva 1', true, 'SWAT', '2099-06-01', false, '2099-01-02T00:00:00Z');
insert into public.usuarios (id, cedula, nombre, rol, activo, updated_at) values
  ('bbbbbbbb-0000-0000-0000-00000000b701', '9-0701-0702', 'OPERADOR CATALOGO', 'OPERADOR', true, '2099-01-02T00:00:00Z');

-- Por sitio: A y B. El gafete viejo de A tiene fecha anterior a la marca de
-- gafetes pero posterior a la general, para probar que cada uno usa la suya.
insert into public.gafetes (id, sitio_id, dispositivo_origen_id, numero, tipo, estado, updated_at) values
  ('ffffffff-0000-0000-0000-00000000f701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'cccccccc-0000-0000-0000-00000000c701', 97001, 'CONTRATISTA', 'DISPONIBLE', '2099-01-02T00:00:00Z'),
  ('ffffffff-0000-0000-0000-00000000f702', 'aaaaaaaa-0000-0000-0000-00000000a701', 'cccccccc-0000-0000-0000-00000000c701', 97002, 'CONTRATISTA', 'PERDIDO', '2099-01-05T00:00:00Z'),
  ('ffffffff-0000-0000-0000-00000000f703', 'aaaaaaaa-0000-0000-0000-00000000a702', 'cccccccc-0000-0000-0000-00000000c703', 97003, 'CONTRATISTA', 'DISPONIBLE', '2099-01-05T00:00:00Z');
insert into public.rutas (id, sitio_id, dispositivo_origen_id, numero, activo, updated_at) values
  ('99999999-0000-0000-0000-000000009701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'cccccccc-0000-0000-0000-00000000c701', 97001, true, '2099-01-02T00:00:00Z'),
  ('99999999-0000-0000-0000-000000009702', 'aaaaaaaa-0000-0000-0000-00000000a702', 'cccccccc-0000-0000-0000-00000000c703', 97002, true, '2099-01-02T00:00:00Z');
insert into public.empresas_proveedor (id, sitio_id, dispositivo_origen_id, nombre, activa, updated_at) values
  ('88888888-0000-0000-0000-000000008701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'cccccccc-0000-0000-0000-00000000c701', 'Proveedor catalogo A', true, '2099-01-02T00:00:00Z'),
  ('88888888-0000-0000-0000-000000008702', 'aaaaaaaa-0000-0000-0000-00000000a702', 'cccccccc-0000-0000-0000-00000000c703', 'Proveedor catalogo B', true, '2099-01-02T00:00:00Z');

create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text) returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella)::text, true)
$$;

-- Los id de un grupo del resultado, para comparar sin depender del orden.
create or replace function pg_temp.ids(p_catalogo jsonb, p_grupo text) returns text[]
language sql as $$
  select coalesce(array_agg(x ->> 'id' order by x ->> 'id'), '{}')
    from jsonb_array_elements(p_catalogo -> p_grupo) x
$$;
-- Desde 20261020130000_permisos_api_explicitos ninguna función nueva es
-- ejecutable por `public`, tampoco estas auxiliares de la prueba.
grant execute on all functions in schema pg_temp to authenticated;

set local role authenticated;
select pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'huella-cat-a1');

-- 1. Sólo lo que cambió desde la marca (`>`, no `>=`), y sólo lo del propio sitio.
do $$
declare
  c jsonb := public.catalogo_del_sitio('2099-01-01T00:00:00Z', '2099-01-03T00:00:00Z');
begin
  if pg_temp.ids(c, 'empresas') <> array['eeeeeeee-0000-0000-0000-00000000e702', 'eeeeeeee-0000-0000-0000-00000000e703'] then
    raise exception '1: empresas %', pg_temp.ids(c, 'empresas');
  end if;
  if pg_temp.ids(c, 'contratistas') <> array['dddddddd-0000-0000-0000-00000000d701'] then
    raise exception '1: contratistas %', pg_temp.ids(c, 'contratistas');
  end if;
  if pg_temp.ids(c, 'usuarios') <> array['bbbbbbbb-0000-0000-0000-00000000b701'] then
    raise exception '1: usuarios %', pg_temp.ids(c, 'usuarios');
  end if;
  -- Gafetes con su propia marca (2099-01-03): sólo el de A del 05; ni el de
  -- A del 02 (anterior a su marca) ni el de B (otro sitio).
  if pg_temp.ids(c, 'gafetes') <> array['ffffffff-0000-0000-0000-00000000f702'] then
    raise exception '1: gafetes %', pg_temp.ids(c, 'gafetes');
  end if;
  if pg_temp.ids(c, 'rutas') <> array['99999999-0000-0000-0000-000000009701'] then
    raise exception '1: rutas %', pg_temp.ids(c, 'rutas');
  end if;
  if pg_temp.ids(c, 'empresas_proveedor') <> array['88888888-0000-0000-0000-000000008701'] then
    raise exception '1: empresas_proveedor %', pg_temp.ids(c, 'empresas_proveedor');
  end if;
end $$;

-- 2. Sin marca de gafetes trae todos los del sitio (incluido el del 02),
--    aunque la marca general esté puesta.
do $$
declare
  c jsonb := public.catalogo_del_sitio('2099-01-01T00:00:00Z', null);
begin
  if not pg_temp.ids(c, 'gafetes') @> array['ffffffff-0000-0000-0000-00000000f701', 'ffffffff-0000-0000-0000-00000000f702'] then
    raise exception '2: sin marca de gafetes faltan gafetes del sitio';
  end if;
  if 'ffffffff-0000-0000-0000-00000000f703' = any(pg_temp.ids(c, 'gafetes')) then
    raise exception '2: trajo un gafete de otro sitio';
  end if;
end $$;

-- 3. Tamaño de página y orden por id: con límite 1 viene sólo la primera.
do $$
declare
  c jsonb := public.catalogo_del_sitio('2099-01-01T00:00:00Z', '2099-01-01T00:00:00Z', 1);
begin
  if jsonb_array_length(c -> 'empresas') <> 1
     or c -> 'empresas' -> 0 ->> 'id' <> 'eeeeeeee-0000-0000-0000-00000000e702' then
    raise exception '3: la página no respeta el límite o el orden: %', c -> 'empresas';
  end if;
end $$;

-- 4. Mismas columnas que los `select=` de los GET (el núcleo las lee igual).
do $$
declare
  c jsonb := public.catalogo_del_sitio('2099-01-01T00:00:00Z', '2099-01-01T00:00:00Z');
  claves text[];
begin
  select array_agg(k order by k) into claves from jsonb_object_keys(c -> 'contratistas' -> 0) k;
  if claves <> array['activo', 'empresa_id', 'empresa_nombre', 'es_personal_ruta', 'fecha_vencimiento_praind',
                     'id', 'identificacion', 'nombre', 'tipo_ingreso', 'updated_at'] then
    raise exception '4: columnas de contratistas %', claves;
  end if;
  select array_agg(k order by k) into claves from jsonb_object_keys(c -> 'gafetes' -> 0) k;
  if claves <> array['contratista_portador_id', 'contratista_portador_nombre', 'estado', 'id', 'numero',
                     'tipo', 'updated_at', 'visita_portador_id'] then
    raise exception '4: columnas de gafetes %', claves;
  end if;
  select array_agg(k order by k) into claves from jsonb_object_keys(c -> 'usuarios' -> 0) k;
  if claves <> array['activo', 'cedula', 'id', 'nombre', 'rol', 'updated_at'] then
    raise exception '4: columnas de usuarios %', claves;
  end if;
end $$;

-- 5. Lo mismo que un SELECT directo con la RLS del equipo (lo que veía el GET).
do $$
declare
  c jsonb := public.catalogo_del_sitio('2099-01-01T00:00:00Z', '2099-01-01T00:00:00Z');
begin
  if pg_temp.ids(c, 'empresas') <> (select array_agg(id::text order by id::text) from public.empresas
                                     where updated_at > '2099-01-01T00:00:00Z') then
    raise exception '5: empresas distintas al SELECT directo';
  end if;
  if pg_temp.ids(c, 'gafetes') <> (select array_agg(id::text order by id::text) from public.gafetes
                                    where sitio_id = 'aaaaaaaa-0000-0000-0000-00000000a701'
                                      and updated_at > '2099-01-01T00:00:00Z') then
    raise exception '5: gafetes distintos al SELECT directo';
  end if;
end $$;

-- 6. Un equipo retirado no ve nada (política restrictiva de equipos vigentes).
select pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c702', 'aaaaaaaa-0000-0000-0000-00000000a701', 'huella-cat-a2');
do $$
declare
  c jsonb := public.catalogo_del_sitio(null, null);
  grupo text;
begin
  foreach grupo in array array['empresas', 'contratistas', 'usuarios', 'gafetes', 'rutas', 'empresas_proveedor'] loop
    if jsonb_array_length(c -> grupo) <> 0 then
      raise exception '6: un equipo retirado vio %', grupo;
    end if;
  end loop;
end $$;

-- 7. anon no la ejecuta.
reset role;
set local role anon;
do $$
begin
  perform public.catalogo_del_sitio(null, null);
  raise exception '7: anon pudo ejecutar catalogo_del_sitio';
exception when insufficient_privilege then null;
end $$;
reset role;

rollback;

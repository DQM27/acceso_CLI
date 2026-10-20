-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `verificar_entrada`: responde en un llamado lo mismo que las tres
-- consultas de antes (adentro por la misma vía en cualquier unidad, adentro
-- por otra vía, gafete en uso por el otro equipo del sitio), también para
-- KOF; una persona libre sale libre; el gafete propio no cuenta como
-- ocupado; anon no la ejecuta.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a901', 'Unidad verificar A'),
  ('aaaaaaaa-0000-0000-0000-00000000a902', 'Unidad verificar B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  ('cccccccc-0000-0000-0000-00000000c901', 'aaaaaaaa-0000-0000-0000-00000000a901', 'pc', 'Verificar A1', '{"kty":"EC"}', 'huella-ver-a1', null),
  ('cccccccc-0000-0000-0000-00000000c902', 'aaaaaaaa-0000-0000-0000-00000000a901', 'mobile', 'Verificar A2', '{"kty":"EC"}', 'huella-ver-a2', null),
  ('cccccccc-0000-0000-0000-00000000c903', 'aaaaaaaa-0000-0000-0000-00000000a902', 'mobile', 'Verificar B1', '{"kty":"EC"}', 'huella-ver-b1', null);

-- Un contratista adentro en la unidad B.
insert into public.contratistas (id, nombre, identificacion) values
  ('dddddddd-0000-0000-0000-00000000d901', 'Contratista verificar', '9-0901-0001');
insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre,
    contratista_cedula, hora_entrada) values
  ('eeeeeeee-0000-0000-0000-00000000e901', 'aaaaaaaa-0000-0000-0000-00000000a902', 'cccccccc-0000-0000-0000-00000000c903',
   'dddddddd-0000-0000-0000-00000000d901', 'Contratista verificar', '9-0901-0001', now() - interval '1 hour');

-- Un proveedor adentro en A, entrado por A2 con el gafete 99002.
insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre,
    gafete_numero, hora_entrada, usuario_entrada_nombre) values
  ('ffffffff-0000-0000-0000-00000000f901', 'aaaaaaaa-0000-0000-0000-00000000a901', 'cccccccc-0000-0000-0000-00000000c902',
   '9-0901-0002', 'Proveedor verificar', 'Empresa V', 99002, now() - interval '1 hour', 'OPERADOR A2');

-- Un encargado KOF con un préstamo abierto en A, entregado por A2 (gafete 99003).
insert into public.encargados_ruta (id, sitio_id, dispositivo_origen_id, codigo_empleado, nombre) values
  ('55555555-0000-0000-0000-000000005901', 'aaaaaaaa-0000-0000-0000-00000000a901', 'cccccccc-0000-0000-0000-00000000c902', 'KOF-E901', 'ENCARGADO VERIFICAR');
insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id, encargado_nombre,
    encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre) values
  ('44444444-0000-0000-0000-000000004901', 'aaaaaaaa-0000-0000-0000-00000000a901', 'cccccccc-0000-0000-0000-00000000c902',
   '55555555-0000-0000-0000-000000005901', 'ENCARGADO VERIFICAR', 'KOF-E901', 99003, now() - interval '1 hour', 'OPERADOR A2');

create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text) returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella)::text, true)
$$;
-- Desde 20261020130000_permisos_api_explicitos ninguna función nueva es
-- ejecutable por `public`, tampoco estas auxiliares de la prueba.
grant execute on all functions in schema pg_temp to authenticated;

set local role authenticated;
select pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c901', 'aaaaaaaa-0000-0000-0000-00000000a901', 'huella-ver-a1');

do $$
declare
  v jsonb;
begin
  -- 1. Adentro por la misma vía en otra unidad.
  v := public.verificar_entrada('CONTRATISTA', '9-0901-0001');
  if v -> 'misma_via' ->> 'sitio_id' <> 'aaaaaaaa-0000-0000-0000-00000000a902'
     or v -> 'misma_via' ->> 'sitio_nombre' <> 'Unidad verificar B' then
    raise exception '1: misma_via %', v;
  end if;
  -- Igual a la función de la regla, que sigue siendo la fuente.
  if (v -> 'misma_via' ->> 'sitio_id')::uuid is distinct from
     (select sitio_id from public.ingreso_activo_de_contratista('9-0901-0001') limit 1) then
    raise exception '1: distinto a ingreso_activo_de_contratista';
  end if;

  -- 2. Adentro por otra vía (es proveedor y quiere entrar como contratista).
  v := public.verificar_entrada('CONTRATISTA', '9-0901-0002');
  if v -> 'misma_via' <> 'null'::jsonb or v -> 'otra_via' ->> 'via' <> 'PROVEEDOR' then
    raise exception '2: %', v;
  end if;

  -- 3. Gafete en uso por el otro equipo del sitio.
  v := public.verificar_entrada('PROVEEDOR', '9-0901-0099', 99002);
  if (v ->> 'gafete_ocupado')::boolean is not true then
    raise exception '3: el gafete del otro equipo no figura ocupado: %', v;
  end if;

  -- 4. Una persona libre y un gafete libre.
  v := public.verificar_entrada('POR_CORREO', '9-0901-0098', 99999);
  if v <> '{"misma_via": null, "otra_via": null, "gafete_ocupado": false}'::jsonb then
    raise exception '4: %', v;
  end if;

  -- 5. KOF: encargado con préstamo abierto y gafete provisional del otro equipo.
  v := public.verificar_entrada('KOF', 'KOF-E901', 99003);
  if v -> 'misma_via' ->> 'sitio_id' <> 'aaaaaaaa-0000-0000-0000-00000000a901'
     or v -> 'otra_via' <> 'null'::jsonb
     or (v ->> 'gafete_ocupado')::boolean is not true then
    raise exception '5: %', v;
  end if;
end $$;

-- 6. El gafete propio no cuenta como "del otro equipo".
select pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c902', 'aaaaaaaa-0000-0000-0000-00000000a901', 'huella-ver-a2');
do $$
begin
  if (public.verificar_entrada('PROVEEDOR', '9-0901-0099', 99002) ->> 'gafete_ocupado')::boolean then
    raise exception '6: el gafete propio figura ocupado';
  end if;
end $$;

-- 7. anon no la ejecuta.
reset role;
set local role anon;
do $$
begin
  perform public.verificar_entrada('CONTRATISTA', '9-0901-0001');
  raise exception '7: anon pudo ejecutar verificar_entrada';
exception when insufficient_privilege then null;
end $$;
reset role;

rollback;

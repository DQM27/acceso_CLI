-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre las funciones del lienzo (`lienzo_contratistas`, `lienzo_ingresos`,
-- `lienzo_resumen_hoy`): cada filtro del catálogo, el resumen de hoy, que sólo un
-- administrador del panel las use y que `anon` no pueda ejecutarlas.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

-- Línea base del resumen, antes de insertar datos de prueba (staging tiene datos).
set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
select set_config('diagnostico.base', public.lienzo_resumen_hoy()::text, true);
reset role;

insert into public.sitios (id, nombre)
values ('aaaaaaaa-0000-0000-0000-00000000a301', 'Unidad lienzo A');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values ('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301',
        'pc', 'Diagnóstico lienzo', '{"kty":"EC"}', 'diag-huella-lienzo');

insert into public.empresas (id, nombre, activa) values
  ('ffffffff-0000-0000-0000-00000000f301', 'EMPRESA LIENZO A', true),
  ('ffffffff-0000-0000-0000-00000000f302', 'EMPRESA LIENZO B', true);

-- "Hoy" = día de Costa Rica, igual que las vistas.
insert into public.contratistas (id, nombre, identificacion, activo, empresa_id, tipo_ingreso,
                                 fecha_vencimiento_praind, es_personal_ruta)
select id::uuid, nombre, cedula, activo, empresa::uuid, tipo, hoy + dias, ruta
from (select (now() at time zone 'America/Costa_Rica')::date as hoy) h,
(values
  ('dddddddd-0000-0000-0000-00000000d301', 'VENCIDA',    '900000301', true,  'ffffffff-0000-0000-0000-00000000f301', 'PRAIND',   -1,   false),
  ('dddddddd-0000-0000-0000-00000000d302', 'POR VENCER', '900000302', true,  'ffffffff-0000-0000-0000-00000000f301', 'PRAIND',   30,   false),
  ('dddddddd-0000-0000-0000-00000000d303', 'VIGENTE',    '900000303', true,  'ffffffff-0000-0000-0000-00000000f301', 'IN_HOUSE', 31,   false),
  ('dddddddd-0000-0000-0000-00000000d304', 'SIN ACCESO', '900000304', false, 'ffffffff-0000-0000-0000-00000000f301', 'PRAIND',   -10,  false),
  ('dddddddd-0000-0000-0000-00000000d305', 'SWAT',       '900000305', true,  'ffffffff-0000-0000-0000-00000000f302', 'SWAT',     null, false),
  ('dddddddd-0000-0000-0000-00000000d308', 'RUTA',       '900000308', true,  'ffffffff-0000-0000-0000-00000000f302', 'SWAT',     -1,   true)
) as v(id, nombre, cedula, activo, empresa, tipo, dias, ruta);

-- Ingresos: VIGENTE adentro (hoy), POR VENCER ya salió (hoy), VENCIDA de ayer.
insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id,
                             contratista_cedula, contratista_nombre, empresa_nombre, tipo_ingreso,
                             hora_entrada, hora_salida, usuario_salida_nombre)
values
  ('bbbbbbbb-0000-0000-0000-00000000b301', 'aaaaaaaa-0000-0000-0000-00000000a301',
   'cccccccc-0000-0000-0000-00000000c301', 'dddddddd-0000-0000-0000-00000000d303',
   '900000303', 'VIGENTE', 'EMPRESA LIENZO A', 'IN_HOUSE',
   now() - interval '1 minute', null, null),
  ('bbbbbbbb-0000-0000-0000-00000000b302', 'aaaaaaaa-0000-0000-0000-00000000a301',
   'cccccccc-0000-0000-0000-00000000c301', 'dddddddd-0000-0000-0000-00000000d302',
   '900000302', 'POR VENCER', 'EMPRESA LIENZO A', 'PRAIND',
   now() - interval '2 minutes', now() - interval '1 minute', 'OP'),
  ('bbbbbbbb-0000-0000-0000-00000000b303', 'aaaaaaaa-0000-0000-0000-00000000a301',
   'cccccccc-0000-0000-0000-00000000c301', 'dddddddd-0000-0000-0000-00000000d301',
   '900000301', 'VENCIDA', 'EMPRESA LIENZO B', 'PRAIND',
   now() - interval '1 day', now() - interval '23 hours', 'OP');

insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre,
  empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
values ('bbbbbbbb-0000-0000-0000-00000000b304', 'aaaaaaaa-0000-0000-0000-00000000a301',
  'cccccccc-0000-0000-0000-00000000c301', '800000301', 'PROVEEDOR LIENZO', 'EMPRESA P', 9301, now(), 'OP');

-- Ayuda de las pruebas de contratistas: nombres de las filas de prueba (empresas
-- "EMPRESA LIENZO…", que además ejercita el filtro "contiene" de empresa). Corre
-- con el rol de quien la llama.
create or replace function pg_temp.nombres_contratistas(
  p_empresas text[] default null, p_excluir text[] default null, p_estados text[] default null,
  p_tipos text[] default null, p_sitio text default null, p_activo boolean default null,
  p_desde date default null, p_hasta date default null, p_nombre text default null,
  p_dias_desde integer default null, p_dias_hasta integer default null
) returns text language sql as $$
  select coalesce(string_agg(nombre, ',' order by nombre), '')
    from public.lienzo_contratistas(p_empresas, p_excluir, 'EMPRESA LIENZO', p_estados, p_tipos,
                                    p_sitio, p_activo, p_desde, p_hasta, p_nombre,
                                    p_dias_desde, p_dias_hasta);
$$;
-- Ninguna función nueva es ejecutable por `public` (permisos explícitos).
grant execute on function pg_temp.nombres_contratistas(text[], text[], text[], text[], text, boolean, date, date, text, integer, integer) to authenticated;

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);

-- Contratistas: cada filtro del catálogo, siempre acotado a las empresas de prueba.

do $$
declare
  v_hoy date := (now() at time zone 'America/Costa_Rica')::date;
begin
  if pg_temp.nombres_contratistas() is distinct from 'POR VENCER,RUTA,SIN ACCESO,SWAT,VENCIDA,VIGENTE' then
    raise exception 'Sin filtros: %', pg_temp.nombres_contratistas();
  end if;
  if pg_temp.nombres_contratistas(p_empresas => array['EMPRESA LIENZO A'])
     is distinct from 'POR VENCER,SIN ACCESO,VENCIDA,VIGENTE' then
    raise exception 'Filtro empresa (en): %', pg_temp.nombres_contratistas(p_empresas => array['EMPRESA LIENZO A']);
  end if;
  if pg_temp.nombres_contratistas(p_excluir => array['EMPRESA LIENZO B'])
     is distinct from 'POR VENCER,SIN ACCESO,VENCIDA,VIGENTE' then
    raise exception 'Filtro empresa (no en)';
  end if;
  if pg_temp.nombres_contratistas(p_estados => array['VENCIDA']) is distinct from 'RUTA,SIN ACCESO,VENCIDA' then
    raise exception 'Filtro estado PRAIND: %', pg_temp.nombres_contratistas(p_estados => array['VENCIDA']);
  end if;
  if pg_temp.nombres_contratistas(p_tipos => array['IN_HOUSE']) is distinct from 'VIGENTE' then
    raise exception 'Filtro tipo de ingreso';
  end if;
  if pg_temp.nombres_contratistas(p_sitio => 'Unidad lienzo A') is distinct from 'VIGENTE' then
    raise exception 'Filtro sitio (adentro)';
  end if;
  if pg_temp.nombres_contratistas(p_activo => false) is distinct from 'SIN ACCESO' then
    raise exception 'Filtro acceso activo';
  end if;
  if pg_temp.nombres_contratistas(p_desde => v_hoy - 5, p_hasta => v_hoy) is distinct from 'RUTA,VENCIDA' then
    raise exception 'Filtro vencimiento entre fechas: %',
      pg_temp.nombres_contratistas(p_desde => v_hoy - 5, p_hasta => v_hoy);
  end if;
  if pg_temp.nombres_contratistas(p_nombre => 'swa') is distinct from 'SWAT' then
    raise exception 'Filtro nombre (contiene)';
  end if;
  if pg_temp.nombres_contratistas(p_dias_desde => 0, p_dias_hasta => 30) is distinct from 'POR VENCER' then
    raise exception 'Filtro días para vencer: %',
      pg_temp.nombres_contratistas(p_dias_desde => 0, p_dias_hasta => 30);
  end if;
end $$;

-- Ingresos por rango de días y sus filtros.
do $$
declare
  v_hoy date := (now() at time zone 'America/Costa_Rica')::date;
  v_n integer;
begin
  select count(*) into v_n from public.lienzo_ingresos(v_hoy - 1, v_hoy, null, null, 'EMPRESA LIENZO');
  if v_n <> 3 then raise exception 'Ingresos de ayer y hoy: %', v_n; end if;

  select count(*) into v_n from public.lienzo_ingresos(v_hoy, v_hoy, null, null, 'EMPRESA LIENZO');
  if v_n <> 2 then raise exception 'Ingresos de hoy: %', v_n; end if;

  select count(*) into v_n from public.lienzo_ingresos(v_hoy - 1, v_hoy, null, array['EMPRESA LIENZO B'], 'EMPRESA LIENZO');
  if v_n <> 2 then raise exception 'Ingresos sin empresa B: %', v_n; end if;

  select count(*) into v_n from public.lienzo_ingresos(v_hoy - 1, v_hoy, array['EMPRESA LIENZO B']);
  if v_n <> 1 then raise exception 'Ingresos de empresa B: %', v_n; end if;

  select count(*) into v_n from public.lienzo_ingresos(v_hoy - 1, v_hoy, null, null, 'EMPRESA LIENZO', array['PRAIND']);
  if v_n <> 2 then raise exception 'Ingresos PRAIND: %', v_n; end if;

  select count(*) into v_n from public.lienzo_ingresos(v_hoy - 1, v_hoy, null, null, 'EMPRESA LIENZO', null, 'Unidad lienzo A', 'vencida');
  if v_n <> 1 then raise exception 'Ingresos por sitio y nombre: %', v_n; end if;

  -- El día se cuenta en hora de Costa Rica.
  select count(*) into v_n from public.lienzo_ingresos(v_hoy - 1, v_hoy - 1, null, null, 'EMPRESA LIENZO');
  if v_n <> 1 then raise exception 'Ingresos de ayer: %', v_n; end if;

  -- Rango inválido: al revés, más de 92 días o sin fechas.
  begin
    perform 1 from public.lienzo_ingresos(v_hoy, v_hoy - 1);
    raise exception 'Aceptó un rango al revés';
  exception when sqlstate '22023' then null; end;
  begin
    perform 1 from public.lienzo_ingresos(v_hoy - 93, v_hoy);
    raise exception 'Aceptó un rango de más de 92 días';
  exception when sqlstate '22023' then null; end;
  begin
    perform 1 from public.lienzo_ingresos(null, v_hoy);
    raise exception 'Aceptó un rango sin fecha';
  exception when sqlstate '22023' then null; end;
end $$;

-- Resumen de hoy: variación contra la línea base y detalle exacto de la unidad de prueba.
do $$
declare
  v_base jsonb := current_setting('diagnostico.base')::jsonb;
  v_ahora jsonb := public.lienzo_resumen_hoy();
  v_unidad jsonb := public.lienzo_resumen_hoy('aaaaaaaa-0000-0000-0000-00000000a301');
  v_dias jsonb;
begin
  if (v_ahora->>'adentro_contratistas')::int - (v_base->>'adentro_contratistas')::int <> 1 then
    raise exception 'Adentro (contratistas) inesperado: % vs %', v_ahora, v_base;
  end if;
  if (v_ahora->>'adentro_proveedores')::int - (v_base->>'adentro_proveedores')::int <> 1 then
    raise exception 'Adentro (proveedores) inesperado';
  end if;
  if (v_ahora->>'ingresos_hoy')::int - (v_base->>'ingresos_hoy')::int <> 2 then
    raise exception 'Ingresos de hoy inesperados';
  end if;
  if (v_ahora->>'praind_vencidos')::int - (v_base->>'praind_vencidos')::int <> 2 then
    raise exception 'PRAIND vencidos inesperados (VENCIDA y RUTA; SIN ACCESO no cuenta)';
  end if;
  if (v_ahora->>'praind_por_vencer')::int - (v_base->>'praind_por_vencer')::int <> 1 then
    raise exception 'PRAIND por vencer inesperados';
  end if;

  if (v_unidad->>'adentro_contratistas')::int <> 1 or (v_unidad->>'adentro_proveedores')::int <> 1
     or (v_unidad->>'ingresos_hoy')::int <> 2 then
    raise exception 'Resumen de la unidad inesperado: %', v_unidad;
  end if;

  v_dias := v_unidad->'ingresos_por_dia';
  if jsonb_array_length(v_dias) <> 14 then
    raise exception 'Deben ser 14 días: %', jsonb_array_length(v_dias);
  end if;
  if (v_dias->13->>'dia')::date <> (now() at time zone 'America/Costa_Rica')::date
     or (v_dias->13->>'cantidad')::int <> 2
     or (v_dias->12->>'cantidad')::int <> 1
     or (v_dias->0->>'cantidad')::int <> 0 then
    raise exception 'Ingresos por día inesperados: %', v_dias;
  end if;
end $$;

-- Quien no es administrador del panel no puede usar ninguna.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  begin
    perform 1 from public.lienzo_contratistas();
    raise exception 'Un usuario sin permiso consultó contratistas';
  exception when insufficient_privilege then null; end;
  begin
    perform 1 from public.lienzo_ingresos(current_date, current_date);
    raise exception 'Un usuario sin permiso consultó ingresos';
  exception when insufficient_privilege then null; end;
  begin
    perform public.lienzo_resumen_hoy();
    raise exception 'Un usuario sin permiso vio el resumen';
  exception when insufficient_privilege then null; end;
end $$;

-- anon no puede ejecutar ninguna; los demás grants son los explícitos.
reset role;
do $$
begin
  if has_function_privilege('anon', 'public.lienzo_resumen_hoy(uuid)', 'execute')
     or has_function_privilege('anon', 'public.lienzo_ingresos(date, date, text[], text[], text, text[], text, text)', 'execute')
     or has_function_privilege('anon', 'public.lienzo_contratistas(text[], text[], text, text[], text[], text, boolean, date, date, text, integer, integer)', 'execute') then
    raise exception 'anon puede ejecutar funciones del lienzo';
  end if;
  if not has_function_privilege('authenticated', 'public.lienzo_resumen_hoy(uuid)', 'execute')
     or not has_function_privilege('service_role', 'public.lienzo_resumen_hoy(uuid)', 'execute') then
    raise exception 'Faltan los grants a authenticated y service_role';
  end if;
end $$;
set local role anon;
do $$
begin
  perform public.lienzo_resumen_hoy();
  raise exception 'anon pudo ejecutar lienzo_resumen_hoy';
exception
  when insufficient_privilege then null;
end $$;

rollback;

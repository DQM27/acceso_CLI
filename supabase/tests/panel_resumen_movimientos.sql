-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `panel_resumen_movimientos`: los cuatro agregados (diario, por hora,
-- empresas, total) con contratistas y proveedores, el día y la hora de Costa
-- Rica (un ingreso a las 23:30 de Costa Rica ya es el día siguiente en UTC),
-- el filtro por unidad, la validación del rango, y que sólo un admin_global
-- vea movimientos.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a301', 'Unidad resumen A'),
  ('aaaaaaaa-0000-0000-0000-00000000a302', 'Unidad resumen B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301',
   'pc', 'Diagnóstico resumen A', '{"kty":"EC"}', 'diag-huella-resumen-a'),
  ('cccccccc-0000-0000-0000-00000000c302', 'aaaaaaaa-0000-0000-0000-00000000a302',
   'pc', 'Diagnóstico resumen B', '{"kty":"EC"}', 'diag-huella-resumen-b');

insert into public.contratistas (id, nombre, identificacion, activo) values
  ('dddddddd-0000-0000-0000-00000000d301', 'RESUMEN UNO', '900000301', true),
  ('dddddddd-0000-0000-0000-00000000d302', 'RESUMEN DOS', '900000302', true),
  ('dddddddd-0000-0000-0000-00000000d303', 'RESUMEN TRES', '900000303', true);

-- Período: 2 y 3 de marzo de 2026 en Costa Rica (UTC-6), o sea de
-- 2026-03-02 06:00Z a 2026-03-04 06:00Z. El 2 de marzo de 2026 es lunes.
insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id,
                             contratista_cedula, contratista_nombre, empresa_nombre,
                             tipo_ingreso, medio_ingreso, hora_entrada, hora_salida,
                             usuario_salida_nombre)
values
  -- Lunes 08:00, 120 min.
  ('bbbbbbbb-0000-0000-0000-00000000b301', 'aaaaaaaa-0000-0000-0000-00000000a301',
   'cccccccc-0000-0000-0000-00000000c301', 'dddddddd-0000-0000-0000-00000000d301',
   '900000301', 'RESUMEN UNO', 'EMP RESUMEN', 'IN_HOUSE', 'CAMINANDO',
   '2026-03-02 14:00Z', '2026-03-02 16:00Z', 'OP'),
  -- Lunes 09:30, 30 min, la misma persona: dos ingresos, una persona.
  ('bbbbbbbb-0000-0000-0000-00000000b302', 'aaaaaaaa-0000-0000-0000-00000000a301',
   'cccccccc-0000-0000-0000-00000000c301', 'dddddddd-0000-0000-0000-00000000d301',
   '900000301', 'RESUMEN UNO', 'EMP RESUMEN', 'IN_HOUSE', 'CAMINANDO',
   '2026-03-02 15:30Z', '2026-03-02 16:00Z', 'OP'),
  -- Lunes 23:30 en Costa Rica (martes en UTC), sin salida ni empresa.
  ('bbbbbbbb-0000-0000-0000-00000000b303', 'aaaaaaaa-0000-0000-0000-00000000a301',
   'cccccccc-0000-0000-0000-00000000c301', 'dddddddd-0000-0000-0000-00000000d302',
   '900000302', 'RESUMEN DOS', null, 'PRAIND', 'VEHICULO',
   '2026-03-03 05:30Z', null, null),
  -- Martes 08:00 en la unidad B, 60 min.
  ('bbbbbbbb-0000-0000-0000-00000000b304', 'aaaaaaaa-0000-0000-0000-00000000a302',
   'cccccccc-0000-0000-0000-00000000c302', 'dddddddd-0000-0000-0000-00000000d303',
   '900000303', 'RESUMEN TRES', 'EMP RESUMEN', 'SWAT', 'CAMINANDO',
   '2026-03-03 14:00Z', '2026-03-03 15:00Z', 'OP'),
  -- Fuera del período (sábado anterior): no cuenta.
  ('bbbbbbbb-0000-0000-0000-00000000b305', 'aaaaaaaa-0000-0000-0000-00000000a301',
   'cccccccc-0000-0000-0000-00000000c301', 'dddddddd-0000-0000-0000-00000000d303',
   '900000303', 'RESUMEN TRES', 'EMP RESUMEN', 'SWAT', 'CAMINANDO',
   '2026-02-28 14:00Z', '2026-02-28 15:00Z', 'OP');

-- Proveedor con placa: lunes 08:10, 30 min.
insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre,
  empresa_nombre, placa, gafete_numero, hora_entrada, usuario_entrada_nombre,
  hora_salida, usuario_salida_nombre)
values ('bbbbbbbb-0000-0000-0000-00000000b306', 'aaaaaaaa-0000-0000-0000-00000000a301',
  'cccccccc-0000-0000-0000-00000000c301', '800000301', 'PROVEEDOR RESUMEN', 'PROV RESUMEN',
  'ABC123', 9301, '2026-03-02 14:10Z', 'OP', '2026-03-02 14:40Z', 'OP');

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);

do $$
declare
  v_ambas constant uuid[] := array['aaaaaaaa-0000-0000-0000-00000000a301',
                                   'aaaaaaaa-0000-0000-0000-00000000a302']::uuid[];
  v_resumen jsonb;
  v_obtenido text;
begin
  v_resumen := public.panel_resumen_movimientos('2026-03-02 06:00Z', '2026-03-04 06:00Z', v_ambas);

  if v_resumen -> 'total' is distinct from
     '{"ingresos": 5, "con_salida": 4, "minutos_adentro": 240, "personas": 4}'::jsonb
  then
    raise exception 'Total inesperado: %', v_resumen -> 'total';
  end if;

  select string_agg(
           concat_ws('|', f ->> 'dia', f ->> 'unidad', f ->> 'tipo_persona', f ->> 'tipo_ingreso',
                     f ->> 'medio', f ->> 'ingresos', f ->> 'con_salida', f ->> 'minutos_adentro'),
           ', ' order by ordinal)
    into v_obtenido
    from jsonb_array_elements(v_resumen -> 'diario') with ordinality as t(f, ordinal);
  if v_obtenido is distinct from
     '2026-03-02|Unidad resumen A|CONTRATISTA|IN HOUSE|CAMINANDO|2|2|150, ' ||
     '2026-03-02|Unidad resumen A|CONTRATISTA|PRAIND|VEHÍCULO|1|0|0, ' ||
     '2026-03-02|Unidad resumen A|PROVEEDOR|—|VEHÍCULO|1|1|30, ' ||
     '2026-03-03|Unidad resumen B|CONTRATISTA|SWAT|CAMINANDO|1|1|60'
  then
    raise exception 'Resumen diario inesperado: %', v_obtenido;
  end if;

  select string_agg((f ->> 'dia_semana') || '@' || (f ->> 'hora') || '=' || (f ->> 'ingresos'),
                    ', ' order by ordinal)
    into v_obtenido
    from jsonb_array_elements(v_resumen -> 'por_hora') with ordinality as t(f, ordinal);
  if v_obtenido is distinct from '1@8=2, 1@9=1, 1@23=1, 2@8=1' then
    raise exception 'Resumen por hora inesperado: %', v_obtenido;
  end if;

  select string_agg(concat_ws('|', f ->> 'empresa', f ->> 'tipo_persona', f ->> 'ingresos', f ->> 'personas'),
                    ', ' order by ordinal)
    into v_obtenido
    from jsonb_array_elements(v_resumen -> 'empresas') with ordinality as t(f, ordinal);
  if v_obtenido is distinct from
     'EMP RESUMEN|CONTRATISTA|3|2, PROV RESUMEN|PROVEEDOR|1|1, SIN EMPRESA|CONTRATISTA|1|1'
  then
    raise exception 'Resumen de empresas inesperado: %', v_obtenido;
  end if;

  -- Sólo la unidad A.
  v_resumen := public.panel_resumen_movimientos('2026-03-02 06:00Z', '2026-03-04 06:00Z',
                                                array['aaaaaaaa-0000-0000-0000-00000000a301']::uuid[]);
  if (v_resumen #>> '{total,ingresos}')::int <> 4 then
    raise exception 'El filtro por unidad no se aplicó: %', v_resumen -> 'total';
  end if;

  -- Sin movimientos: listas vacías y totales en cero, nunca null.
  v_resumen := public.panel_resumen_movimientos('2001-01-01 06:00Z', '2001-01-02 06:00Z', v_ambas);
  if v_resumen is distinct from
     '{"diario": [], "por_hora": [], "empresas": [],
       "total": {"ingresos": 0, "con_salida": 0, "minutos_adentro": 0, "personas": 0}}'::jsonb
  then
    raise exception 'Resumen vacío inesperado: %', v_resumen;
  end if;
end $$;

-- Rangos inválidos: invertido y de más de un año.
do $$
begin
  perform public.panel_resumen_movimientos('2026-03-04 06:00Z', '2026-03-02 06:00Z');
  raise exception 'Aceptó un rango invertido';
exception
  when invalid_parameter_value then null;
end $$;
do $$
begin
  perform public.panel_resumen_movimientos('2025-01-01 06:00Z', '2026-01-03 06:00Z');
  raise exception 'Aceptó un rango de más de un año';
exception
  when invalid_parameter_value then null;
end $$;

-- Quien no es admin_global ni equipo no ve ningún movimiento.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
declare
  v_resumen jsonb;
begin
  v_resumen := public.panel_resumen_movimientos(
    '2026-03-02 06:00Z', '2026-03-04 06:00Z',
    array['aaaaaaaa-0000-0000-0000-00000000a301', 'aaaaaaaa-0000-0000-0000-00000000a302']::uuid[]);
  if (v_resumen #>> '{total,ingresos}')::int <> 0 then
    raise exception 'Un usuario sin permiso ve movimientos en el resumen: %', v_resumen -> 'total';
  end if;
end $$;

-- anon no puede ejecutarla.
reset role;
set local role anon;
do $$
begin
  perform public.panel_resumen_movimientos('2026-03-02 06:00Z', '2026-03-04 06:00Z');
  raise exception 'anon pudo ejecutar panel_resumen_movimientos';
exception
  when insufficient_privilege then null;
end $$;

rollback;

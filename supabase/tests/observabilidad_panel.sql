-- Ejecutar en una sola sesión, después de aplicar 20261101120000 y
-- 20261101130000. Todas las filas de prueba se revierten.
-- Cubre las lecturas del panel (`panel_telemetria_flota`,
-- `panel_telemetria_rendimiento`, `panel_telemetria_versiones`):
--  1. anon no tiene permiso; un equipo y una sesión humana que no es
--     administradora reciben 42501.
--  2. Flota: la sesión más reciente de cada equipo, con su etiqueta, el último
--     lote y las filas y descartes de las últimas 24 h (no los de antes).
--  3. Rendimiento: agrupa por equipo, versión y operación dentro de la
--     ventana pedida; el máximo de días es 30.
--  4. Versiones: por versión y plataforma desde el resumen diario, sólo de la
--     operación pedida y dentro de los días pedidos.
begin;

select set_config('prueba.admin', 'admin-observabilidad-panel@example.com', true);
insert into public.administradores_panel (correo) values (current_setting('prueba.admin'));

insert into public.sitios (id, nombre) values ('aaaaaaaa-0000-0000-0000-00000000b801', 'Unidad panel A');
insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000b801', 'aaaaaaaa-0000-0000-0000-00000000b801', 'mobile', 'Celular panel', '{"kty":"EC"}', 'huella-panel-1'),
  ('cccccccc-0000-0000-0000-00000000b802', 'aaaaaaaa-0000-0000-0000-00000000b801', 'pc', 'PC panel', '{"kty":"EC"}', 'huella-panel-2');

insert into observabilidad.sesiones (id, dispositivo_id, sitio_id, plataforma, version_app, version_codigo, ambiente, iniciada_en, sistema, modelo) values
  ('dddddddd-0000-0000-0000-00000000b801', 'cccccccc-0000-0000-0000-00000000b801', 'aaaaaaaa-0000-0000-0000-00000000b801', 'movil', '1.8.13', 58, 'produccion', now() - interval '3 days', 'Android 14', 'SM-A155M'),
  ('dddddddd-0000-0000-0000-00000000b802', 'cccccccc-0000-0000-0000-00000000b801', 'aaaaaaaa-0000-0000-0000-00000000b801', 'movil', '1.8.14', 59, 'produccion', now() - interval '2 hours', 'Android 14', 'SM-A155M'),
  ('dddddddd-0000-0000-0000-00000000b803', 'cccccccc-0000-0000-0000-00000000b802', 'aaaaaaaa-0000-0000-0000-00000000b801', 'escritorio', '1.10.8', null, 'produccion', now() - interval '1 hour', 'windows', null);

insert into observabilidad.lotes (lote_id, dispositivo_id, recibido_en, filas, descartadas) values
  ('eeeeeeee-0000-0000-0000-00000000b801', 'cccccccc-0000-0000-0000-00000000b801', now() - interval '2 days', 500, 9),
  ('eeeeeeee-0000-0000-0000-00000000b802', 'cccccccc-0000-0000-0000-00000000b801', now() - interval '10 minutes', 40, 1),
  ('eeeeeeee-0000-0000-0000-00000000b803', 'cccccccc-0000-0000-0000-00000000b801', now() - interval '5 minutes', 60, 0);

insert into observabilidad.llamadas (sesion_id, dispositivo_id, ocurrido_en, operacion, llamadas, errores, p50_ms, p90_ms, max_ms) values
  ('dddddddd-0000-0000-0000-00000000b801', 'cccccccc-0000-0000-0000-00000000b801', now() - interval '3 days', 'registrar_ingreso', 10, 1, 270, 300, 900),
  ('dddddddd-0000-0000-0000-00000000b802', 'cccccccc-0000-0000-0000-00000000b801', now() - interval '1 hour', 'registrar_ingreso', 5, 0, 20, 40, 60),
  ('dddddddd-0000-0000-0000-00000000b802', 'cccccccc-0000-0000-0000-00000000b801', now() - interval '30 minutes', 'registrar_ingreso', 7, 1, 30, 50, 80),
  ('dddddddd-0000-0000-0000-00000000b803', 'cccccccc-0000-0000-0000-00000000b802', now() - interval '20 minutes', 'listar_ingresos_activos', 3, 0, 13, 13, 36);

insert into observabilidad.resumen_llamadas_diario (dia, dispositivo_id, version_app, plataforma, operacion, llamadas, errores, p50_ms, p90_ms, max_ms) values
  ((now() at time zone 'America/Costa_Rica')::date - 40, 'cccccccc-0000-0000-0000-00000000b801', '1.8.12', 'movil', 'registrar_ingreso', 100, 2, 280, 320, 2000),
  ((now() at time zone 'America/Costa_Rica')::date - 3, 'cccccccc-0000-0000-0000-00000000b801', '1.8.13', 'movil', 'registrar_ingreso', 10, 1, 270, 300, 900),
  ((now() at time zone 'America/Costa_Rica')::date - 1, 'cccccccc-0000-0000-0000-00000000b801', '1.8.14', 'movil', 'registrar_ingreso', 12, 1, 25, 45, 80),
  ((now() at time zone 'America/Costa_Rica')::date - 1, 'cccccccc-0000-0000-0000-00000000b801', '1.8.14', 'movil', 'otra_operacion', 99, 0, 1, 1, 1);

-- 1. Permisos.
do $$
begin
  if has_function_privilege('anon', 'public.panel_telemetria_flota()', 'execute')
     or has_function_privilege('anon', 'public.panel_telemetria_rendimiento(int)', 'execute')
     or has_function_privilege('anon', 'public.panel_telemetria_versiones(text, int)', 'execute') then
    raise exception '1: anon puede leer la telemetría del panel';
  end if;
end $$;

set local role authenticated;

-- Un equipo vigente: no es administrador.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000b801',
  'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000b801', 'huella', 'huella-panel-1')::text, true);
do $$
begin
  perform * from public.panel_telemetria_flota();
  raise exception '1: un equipo leyó la flota';
exception when insufficient_privilege then null;
end $$;

-- Una sesión humana que no es administradora.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'otro@example.com')::text, true);
do $$
begin
  perform * from public.panel_telemetria_rendimiento(7);
  raise exception '1: alguien que no es administrador leyó el rendimiento';
exception when insufficient_privilege then null;
end $$;

-- El administrador.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('prueba.admin'))::text, true);

-- 2. Flota.
do $$
declare
  v record;
begin
  select * into strict v from public.panel_telemetria_flota() f
   where f.dispositivo_id = 'cccccccc-0000-0000-0000-00000000b801';
  if v.version_app <> '1.8.14' or v.version_codigo <> 59 or v.etiqueta <> 'Celular panel' then
    raise exception '2: no es la sesión más reciente del celular: %', row_to_json(v);
  end if;
  if v.filas_24h <> 100 or v.descartadas_24h <> 1 then
    raise exception '2: filas/descartes de 24 h mal contados (% / %)', v.filas_24h, v.descartadas_24h;
  end if;
  if v.ultimo_lote_en < now() - interval '6 minutes' then
    raise exception '2: último lote mal calculado';
  end if;
  select * into strict v from public.panel_telemetria_flota() f
   where f.dispositivo_id = 'cccccccc-0000-0000-0000-00000000b802';
  if v.ultimo_lote_en is not null or v.filas_24h <> 0 or v.plataforma <> 'escritorio' then
    raise exception '2: el equipo sin lotes quedó mal: %', row_to_json(v);
  end if;
end $$;

-- 3. Rendimiento.
do $$
declare
  v record;
begin
  -- Último día: sólo la versión nueva del celular (la vieja es de hace 3 días).
  select * into strict v from public.panel_telemetria_rendimiento(1) r
   where r.dispositivo_id = 'cccccccc-0000-0000-0000-00000000b801' and r.operacion = 'registrar_ingreso';
  if v.version_app <> '1.8.14' or v.llamadas <> 12 or v.errores <> 1 or v.max_ms <> 80 then
    raise exception '3: agregado de un día mal calculado: %', row_to_json(v);
  end if;
  -- Siete días: aparecen las dos versiones por separado.
  if (select count(*) from public.panel_telemetria_rendimiento(7) r
       where r.dispositivo_id = 'cccccccc-0000-0000-0000-00000000b801') <> 2 then
    raise exception '3: con 7 días no salen las dos versiones';
  end if;
  -- Más de 30 días se recorta a 30, y un valor inválido no rompe.
  perform * from public.panel_telemetria_rendimiento(1000);
  perform * from public.panel_telemetria_rendimiento(null);
end $$;

-- 4. Versiones.
do $$
declare
  v_versiones text[];
begin
  select array_agg(r.version_app order by r.primer_dia) into v_versiones
    from public.panel_telemetria_versiones('registrar_ingreso', 30) r;
  if v_versiones is distinct from array['1.8.13', '1.8.14'] then
    raise exception '4: versiones de 30 días: %', v_versiones;
  end if;
  select array_agg(r.version_app order by r.primer_dia) into v_versiones
    from public.panel_telemetria_versiones('registrar_ingreso', 90) r;
  if v_versiones is distinct from array['1.8.12', '1.8.13', '1.8.14'] then
    raise exception '4: versiones de 90 días: %', v_versiones;
  end if;
  if exists (select 1 from public.panel_telemetria_versiones('registrar_ingreso', 90) r where r.llamadas = 99) then
    raise exception '4: se mezcló otra operación';
  end if;
end $$;

rollback;

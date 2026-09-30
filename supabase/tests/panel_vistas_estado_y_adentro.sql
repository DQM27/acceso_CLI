-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `panel_contratistas_estado` (mismas reglas que `verificar_acceso`,
-- src/domain/acceso.rs) y `panel_adentro_ahora` (contratistas, proveedores y
-- KOF sin salida), y que sólo un admin_global las vea completas.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre)
values ('aaaaaaaa-0000-0000-0000-00000000a201', 'Unidad vistas A');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values ('cccccccc-0000-0000-0000-00000000c201', 'aaaaaaaa-0000-0000-0000-00000000a201',
        'pc', 'Diagnóstico vistas', '{"kty":"EC"}', 'diag-huella-vistas');

insert into public.empresas (id, nombre, activa) values
  ('ffffffff-0000-0000-0000-00000000f201', 'EMPRESA VISTAS ACTIVA', true),
  ('ffffffff-0000-0000-0000-00000000f202', 'EMPRESA VISTAS INACTIVA', false);

-- Un contratista por cada resultado de las reglas. "Hoy" = día de Costa Rica.
insert into public.contratistas (id, nombre, identificacion, activo, empresa_id, tipo_ingreso,
                                 fecha_vencimiento_praind, es_personal_ruta)
select id::uuid, nombre, cedula, activo, empresa::uuid, tipo, hoy + dias, ruta
from (select (now() at time zone 'America/Costa_Rica')::date as hoy) h,
(values
  ('dddddddd-0000-0000-0000-00000000d201', 'VENCIDA',    '900000201', true,  'ffffffff-0000-0000-0000-00000000f201', 'PRAIND',     -1,   false),
  ('dddddddd-0000-0000-0000-00000000d202', 'POR VENCER', '900000202', true,  'ffffffff-0000-0000-0000-00000000f201', 'PRAIND',     30,   false),
  ('dddddddd-0000-0000-0000-00000000d203', 'VIGENTE',    '900000203', true,  'ffffffff-0000-0000-0000-00000000f201', 'IN_HOUSE',   31,   false),
  ('dddddddd-0000-0000-0000-00000000d204', 'SIN ACCESO', '900000204', false, 'ffffffff-0000-0000-0000-00000000f201', 'PRAIND',     -10,  false),
  ('dddddddd-0000-0000-0000-00000000d205', 'SWAT',       '900000205', true,  'ffffffff-0000-0000-0000-00000000f201', 'SWAT',       null, false),
  ('dddddddd-0000-0000-0000-00000000d206', 'EMP INACT',  '900000206', false, 'ffffffff-0000-0000-0000-00000000f202', 'PRAIND',     90,   false),
  ('dddddddd-0000-0000-0000-00000000d207', 'SIN FECHA',  '900000207', true,  'ffffffff-0000-0000-0000-00000000f201', 'IN_HOUSE',   null, false),
  ('dddddddd-0000-0000-0000-00000000d208', 'RUTA',       '900000208', true,  'ffffffff-0000-0000-0000-00000000f201', 'SWAT',       -1,   true)
) as v(id, nombre, cedula, activo, empresa, tipo, dias, ruta);

insert into public.encargados_ruta (id, sitio_id, dispositivo_origen_id, codigo_empleado, nombre)
values ('eeeeeeee-0000-0000-0000-00000000e201', 'aaaaaaaa-0000-0000-0000-00000000a201',
        'cccccccc-0000-0000-0000-00000000c201', 'DIAG-VISTAS-1', 'ENCARGADO VISTAS');

-- Adentro: VIGENTE (abierto) y POR VENCER (ya salió: no cuenta).
insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id,
                             contratista_cedula, contratista_nombre, hora_entrada,
                             hora_salida, usuario_salida_nombre)
values
  ('bbbbbbbb-0000-0000-0000-00000000b201', 'aaaaaaaa-0000-0000-0000-00000000a201',
   'cccccccc-0000-0000-0000-00000000c201', 'dddddddd-0000-0000-0000-00000000d203',
   '900000203', 'VIGENTE', now() - interval '2 hours', null, null),
  ('bbbbbbbb-0000-0000-0000-00000000b202', 'aaaaaaaa-0000-0000-0000-00000000a201',
   'cccccccc-0000-0000-0000-00000000c201', 'dddddddd-0000-0000-0000-00000000d202',
   '900000202', 'POR VENCER', now() - interval '5 hours', now() - interval '1 hour', 'OP');

insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre,
  empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
values ('bbbbbbbb-0000-0000-0000-00000000b203', 'aaaaaaaa-0000-0000-0000-00000000a201',
  'cccccccc-0000-0000-0000-00000000c201', '800000203', 'PROVEEDOR VISTAS', 'EMPRESA P', 9201, now(), 'OP');

insert into public.prestamos_gafete_provisional (id, sitio_id, dispositivo_entrega_id, encargado_id,
  encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega, usuario_entrega_nombre)
values ('bbbbbbbb-0000-0000-0000-00000000b204', 'aaaaaaaa-0000-0000-0000-00000000a201',
  'cccccccc-0000-0000-0000-00000000c201', 'eeeeeeee-0000-0000-0000-00000000e201',
  'ENCARGADO VISTAS', 'DIAG-VISTAS-1', 9202, now(), 'OP');

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);

do $$
declare
  v_obtenido text;
begin
  select string_agg(nombre || '=' || estado_acceso || '/' || estado_praind, ', ' order by nombre)
    into v_obtenido
    from public.panel_contratistas_estado
   where id::text like 'dddddddd-0000-0000-0000-00000000d2%';

  if v_obtenido is distinct from
     'EMP INACT=EMPRESA_INACTIVA/VIGENTE, ' ||
     'POR VENCER=PERMITIDO_CON_ADVERTENCIA/POR_VENCER, ' ||
     'RUTA=PRAIND_VENCIDO/VENCIDA, ' ||
     'SIN ACCESO=SIN_ACCESO/VENCIDA, ' ||
     'SIN FECHA=PRAIND_NO_REGISTRADO/SIN_REGISTRO, ' ||
     'SWAT=PERMITIDO/NO_REQUIERE, ' ||
     'VENCIDA=PRAIND_VENCIDO/VENCIDA, ' ||
     'VIGENTE=PERMITIDO/VIGENTE'
  then
    raise exception 'Estados de contratistas inesperados: %', v_obtenido;
  end if;

  -- Adentro sólo quien tiene un ingreso abierto, con su unidad.
  select string_agg(nombre || '@' || coalesce(adentro_sitio_nombre, '-'), ', ' order by nombre)
    into v_obtenido
    from public.panel_contratistas_estado
   where id::text like 'dddddddd-0000-0000-0000-00000000d2%' and adentro_desde is not null;
  if v_obtenido is distinct from 'VIGENTE@Unidad vistas A' then
    raise exception 'Adentro en contratistas inesperado: %', v_obtenido;
  end if;

  select string_agg(tipo || ':' || nombre || '@' || sitio_nombre, ', ' order by tipo)
    into v_obtenido
    from public.panel_adentro_ahora
   where sitio_id = 'aaaaaaaa-0000-0000-0000-00000000a201';
  if v_obtenido is distinct from
     'CONTRATISTA:VIGENTE@Unidad vistas A, ' ||
     'PROVEEDOR:PROVEEDOR VISTAS@Unidad vistas A, ' ||
     'PROVISIONAL_KOF:ENCARGADO VISTAS@Unidad vistas A'
  then
    raise exception 'panel_adentro_ahora inesperado: %', v_obtenido;
  end if;
end $$;

-- Quien no es admin_global ni equipo no ve a nadie adentro.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  if exists (select 1 from public.panel_adentro_ahora
              where sitio_id = 'aaaaaaaa-0000-0000-0000-00000000a201') then
    raise exception 'Un usuario sin permiso ve quién está adentro';
  end if;
  if exists (select 1 from public.panel_contratistas_estado
              where id::text like 'dddddddd-0000-0000-0000-00000000d2%') then
    raise exception 'Un usuario sin permiso ve el estado de contratistas';
  end if;
end $$;

-- anon no puede consultar ninguna de las dos.
reset role;
set local role anon;
do $$
begin
  perform 1 from public.panel_adentro_ahora limit 1;
  raise exception 'anon pudo leer panel_adentro_ahora';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.panel_contratistas_estado limit 1;
  raise exception 'anon pudo leer panel_contratistas_estado';
exception
  when insufficient_privilege then null;
end $$;

rollback;

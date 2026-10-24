-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre las columnas nuevas de `lienzo_ingresos` (migración
-- `lienzo_ingresos_hora`): hora y día de la semana de la entrada en Costa Rica
-- (no en UTC), horas adentro con un decimal y vacío si sigue adentro; que los
-- filtros de siempre siguen andando; y los permisos (sin anon, sólo administradores).
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-hora-admin@example.com', true);
insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre) values ('aaaaaaaa-0000-0000-0000-00000000a501', 'Unidad horas pico');
insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values ('cccccccc-0000-0000-0000-00000000c501', 'aaaaaaaa-0000-0000-0000-00000000a501',
        'pc', 'Diagnóstico horas', '{"kty":"EC"}', 'diag-huella-horas');
insert into public.contratistas (id, nombre, identificacion, activo, tipo_ingreso)
values ('dddddddd-0000-0000-0000-00000000d501', 'CONTRATISTA HORAS', '900000501', true, 'IN_HOUSE');

-- Lunes 12/10/2026 a las 07:30 de Costa Rica (13:30 UTC), salió a las 10:15 (2,75 h → 2,8).
-- Domingo 18/10/2026 a las 23:10 de Costa Rica (lunes 05:10 UTC), sigue adentro.
insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_cedula, contratista_nombre,
                             empresa_nombre, tipo_ingreso, hora_entrada, hora_salida, usuario_salida_nombre)
values
  ('bbbbbbbb-0000-0000-0000-00000000b501', 'aaaaaaaa-0000-0000-0000-00000000a501',
   'cccccccc-0000-0000-0000-00000000c501', 'dddddddd-0000-0000-0000-00000000d501', '900000501', 'MAÑANA', 'EMPRESA HORAS', 'IN_HOUSE',
   '2026-10-12 13:30:00+00', '2026-10-12 16:15:00+00', 'OP'),
  ('bbbbbbbb-0000-0000-0000-00000000b502', 'aaaaaaaa-0000-0000-0000-00000000a501',
   'cccccccc-0000-0000-0000-00000000c501', 'dddddddd-0000-0000-0000-00000000d501', '900000502', 'NOCHE', 'EMPRESA HORAS', 'IN_HOUSE',
   '2026-10-19 05:10:00+00', null, null);

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text, true);
do $$
declare
  v_manana record;
  v_noche record;
begin
  select * into v_manana from public.lienzo_ingresos('2026-10-10', '2026-10-20', p_sitio => 'Unidad horas pico')
   where nombre = 'MAÑANA';
  select * into v_noche from public.lienzo_ingresos('2026-10-10', '2026-10-20', p_sitio => 'Unidad horas pico')
   where nombre = 'NOCHE';
  if v_manana is null or v_noche is null then
    raise exception 'Faltan los ingresos de prueba';
  end if;
  if v_manana.dia <> '2026-10-12' or v_manana.hora <> 7 or v_manana.dia_semana <> 'Lunes' then
    raise exception 'Mañana: esperaba 2026-10-12 a las 7 (Lunes), vino % a las % (%)', v_manana.dia, v_manana.hora, v_manana.dia_semana;
  end if;
  if v_manana.horas_adentro <> 2.8 then
    raise exception 'Horas adentro: esperaba 2.8, vino %', v_manana.horas_adentro;
  end if;
  -- En UTC ya es lunes 19 a las 5; en Costa Rica sigue siendo domingo 18 a las 23.
  if v_noche.dia <> '2026-10-18' or v_noche.hora <> 23 or v_noche.dia_semana <> 'Domingo' then
    raise exception 'Noche: esperaba 2026-10-18 a las 23 (Domingo), vino % a las % (%)', v_noche.dia, v_noche.hora, v_noche.dia_semana;
  end if;
  if v_noche.horas_adentro is not null then
    raise exception 'Si sigue adentro, horas_adentro va vacío; vino %', v_noche.horas_adentro;
  end if;
  -- Los filtros de siempre siguen andando.
  if exists (select 1 from public.lienzo_ingresos('2026-10-10', '2026-10-20', p_sitio => 'Unidad horas pico',
                                                  p_empresas_excluir => array['EMPRESA HORAS'])) then
    raise exception 'El filtro de empresa dejó de andar';
  end if;
end $$;
reset role;

-- Una cuenta que no es administradora del panel no puede leer.
set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'diagnostico-no-admin@example.com')::text, true);
do $$
begin
  perform public.lienzo_ingresos('2026-10-10', '2026-10-20');
  raise exception 'Una cuenta sin administración pudo leer lienzo_ingresos';
exception when insufficient_privilege then
  null;
end $$;
reset role;

do $$
begin
  if has_function_privilege('anon', 'public.lienzo_ingresos(date, date, text[], text[], text, text[], text, text)', 'execute') then
    raise exception 'anon puede ejecutar lienzo_ingresos';
  end if;
  if not has_function_privilege('authenticated', 'public.lienzo_ingresos(date, date, text[], text[], text, text[], text, text)', 'execute') then
    raise exception 'authenticated perdió el permiso sobre lienzo_ingresos';
  end if;
  if not has_function_privilege('service_role', 'public.lienzo_ingresos(date, date, text[], text[], text, text[], text, text)', 'execute') then
    raise exception 'service_role perdió el permiso sobre lienzo_ingresos';
  end if;
end $$;

rollback;

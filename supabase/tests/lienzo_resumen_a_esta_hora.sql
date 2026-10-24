-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `ingresos_semana_pasada_a_esta_hora` de `lienzo_resumen_hoy`: cuenta
-- los ingresos del mismo día de la semana pasada sólo hasta esta misma hora
-- (no los de más tarde ese día, ni los del día anterior). Y que los permisos
-- siguen iguales.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-resumen-admin@example.com', true);
insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre) values ('aaaaaaaa-0000-0000-0000-00000000a401', 'Unidad resumen');
insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values ('cccccccc-0000-0000-0000-00000000c401', 'aaaaaaaa-0000-0000-0000-00000000a401',
        'pc', 'Diagnóstico resumen', '{"kty":"EC"}', 'diag-huella-resumen');
insert into public.contratistas (id, nombre, identificacion, activo, tipo_ingreso)
values ('dddddddd-0000-0000-0000-00000000d401', 'CONTRATISTA RESUMEN', '900000401', true, 'IN_HOUSE');

-- Base antes de las filas de prueba (en el sitio de prueba: arranca en 0).
set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text, true);
select set_config('diagnostico.base',
  (public.lienzo_resumen_hoy('aaaaaaaa-0000-0000-0000-00000000a401') ->> 'ingresos_semana_pasada_a_esta_hora'), true);
reset role;

-- Hace 7 días y 1 hora (cuenta, si sigue siendo ese mismo día en Costa Rica),
-- hace 7 días menos 1 hora (más tarde ese día: no cuenta) y hace 8 días (otro día: no cuenta).
insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_cedula, contratista_nombre,
                             empresa_nombre, tipo_ingreso, hora_entrada, hora_salida, usuario_salida_nombre)
values
  ('bbbbbbbb-0000-0000-0000-00000000b401', 'aaaaaaaa-0000-0000-0000-00000000a401',
   'cccccccc-0000-0000-0000-00000000c401', 'dddddddd-0000-0000-0000-00000000d401', '900000401', 'ANTES', 'EMPRESA R', 'IN_HOUSE',
   now() - interval '7 days' - interval '1 minute', now() - interval '7 days', 'OP'),
  ('bbbbbbbb-0000-0000-0000-00000000b402', 'aaaaaaaa-0000-0000-0000-00000000a401',
   'cccccccc-0000-0000-0000-00000000c401', 'dddddddd-0000-0000-0000-00000000d401', '900000402', 'DESPUES', 'EMPRESA R', 'IN_HOUSE',
   now() - interval '7 days' + interval '1 minute', now() - interval '6 days', 'OP'),
  ('bbbbbbbb-0000-0000-0000-00000000b403', 'aaaaaaaa-0000-0000-0000-00000000a401',
   'cccccccc-0000-0000-0000-00000000c401', 'dddddddd-0000-0000-0000-00000000d401', '900000403', 'OTRO DIA', 'EMPRESA R', 'IN_HOUSE',
   now() - interval '8 days', now() - interval '8 days' + interval '1 hour', 'OP');

set local role authenticated;
do $$
declare
  v_resumen jsonb := public.lienzo_resumen_hoy('aaaaaaaa-0000-0000-0000-00000000a401');
  v_inicio_dia_pasado timestamptz :=
    (((now() at time zone 'America/Costa_Rica')::date - 7)::timestamp at time zone 'America/Costa_Rica');
  v_esperado integer := current_setting('diagnostico.base')::integer
    -- "ANTES" sólo cuenta si un minuto antes de esta hora seguía siendo ese día.
    + case when now() - interval '7 days' - interval '1 minute' >= v_inicio_dia_pasado then 1 else 0 end;
begin
  if not (v_resumen ? 'ingresos_semana_pasada_a_esta_hora') then
    raise exception 'El resumen no trae ingresos_semana_pasada_a_esta_hora';
  end if;
  if (v_resumen ->> 'ingresos_semana_pasada_a_esta_hora')::integer <> v_esperado then
    raise exception 'Semana pasada a esta hora: esperaba %, vino %', v_esperado,
      v_resumen ->> 'ingresos_semana_pasada_a_esta_hora';
  end if;
  -- El resto del resumen sigue ahí.
  if not (v_resumen ? 'ingresos_hoy' and v_resumen ? 'ingresos_por_dia' and v_resumen ? 'praind_vencidos') then
    raise exception 'El resumen perdió campos';
  end if;
end $$;
reset role;

do $$
begin
  if has_function_privilege('anon', 'public.lienzo_resumen_hoy(uuid)', 'execute') then
    raise exception 'anon puede ejecutar lienzo_resumen_hoy';
  end if;
  if not has_function_privilege('authenticated', 'public.lienzo_resumen_hoy(uuid)', 'execute') then
    raise exception 'authenticated perdió el permiso sobre lienzo_resumen_hoy';
  end if;
end $$;

rollback;

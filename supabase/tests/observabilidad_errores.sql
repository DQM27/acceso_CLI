-- Ejecutar en una sola sesión, después de aplicar 20261101120000,
-- 20261101130000 y 20261101140000. Todas las filas de prueba se revierten.
-- Cubre `registrar_errores`, el panel de errores y su mantenimiento
-- (20261101140000_observabilidad_errores):
--  1. anon no puede llamar; una sesión humana, un visor y un equipo retirado
--     tampoco.
--  2. Un equipo vigente manda errores: entran con su equipo, su unidad y su
--     plataforma puestos desde el JWT (no del cuerpo), se agrupan por huella
--     y una hora en el futuro se cambia por la de recepción.
--  3. Los mismos `event_id` otra vez (y uno repetido dentro del lote): no se
--     insertan ni se suman al grupo.
--  4. Lotes mal formados: error y nada guardado.
--  5. Las tablas no se leen ni se escriben desde la API.
--  6. Ritmo: pasado el máximo de errores por minuto, 53400.
--  7. Regresión: un grupo `resuelto` vuelve a `nuevo` sólo con una versión
--     que no se había visto al resolverlo; `ignorado` se queda ignorado.
--  8. Panel: sólo el administrador; grupos del período, detalle y cambio de
--     estado.
--  9. Mantenimiento: borra lo vencido y los grupos que quedan vacíos; la
--     tarea de pg_cron existe.
begin;

select set_config('prueba.admin', 'admin-observabilidad-errores@example.com', true);
insert into public.administradores_panel (correo) values (current_setting('prueba.admin'));

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000b901', 'Unidad errores A');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  ('cccccccc-0000-0000-0000-00000000b901', 'aaaaaaaa-0000-0000-0000-00000000b901', 'mobile', 'Celular errores 1', '{"kty":"EC"}', 'huella-err-1', null),
  ('cccccccc-0000-0000-0000-00000000b902', 'aaaaaaaa-0000-0000-0000-00000000b901', 'pc', 'PC errores 2', '{"kty":"EC"}', 'huella-err-2', null),
  ('cccccccc-0000-0000-0000-00000000b903', 'aaaaaaaa-0000-0000-0000-00000000b901', 'mobile', 'Celular errores retirado', '{"kty":"EC"}', 'huella-err-3', now()),
  ('cccccccc-0000-0000-0000-00000000b904', 'aaaaaaaa-0000-0000-0000-00000000b901', 'visor', 'Visor errores', '{"kty":"EC"}', 'huella-err-4', null);

-- Un reporte ya limpio, como lo arma la Edge Function.
create function pg_temp.reporte(p_event_id text, p_huella text, p_version text default '1.8.13',
                                p_titulo text default 'IllegalStateException: estado inválido')
returns jsonb language sql as $$
  select jsonb_build_object(
    'event_id', p_event_id,
    'huella', p_huella,
    'ocurrido_en', now() - interval '2 minutes',
    -- Intento de falsear la plataforma desde el cuerpo: se ignora.
    'plataforma', 'escritorio',
    'version_app', p_version,
    'nivel', 'fatal',
    'origen', 'excepcion',
    'titulo', p_titulo,
    'traza', jsonb_build_array(jsonb_build_object('funcion', 'registrar', 'archivo', 'Ingreso.kt', 'linea', 42, 'propio', true)),
    'pantalla', 'Ingreso',
    'operaciones', jsonb_build_array(jsonb_build_object('nombre', 'registrar_ingreso', 'ok', false, 'en', now())),
    'contexto', jsonb_build_object('os', 'Android', 'modelo', 'SM-A155M')
  );
$$;
grant execute on function pg_temp.reporte(text, text, text, text) to authenticated;

create function pg_temp.como_equipo(p_n int) returns void language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000b90' || p_n,
    'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000b901', 'huella', 'huella-err-' || p_n)::text, true);
$$;
grant execute on function pg_temp.como_equipo(int) to authenticated;

-- 1. Permisos.
do $$
begin
  if has_function_privilege('anon', 'public.registrar_errores(jsonb)', 'execute')
     or has_function_privilege('anon', 'observabilidad.registrar_errores(jsonb)', 'execute')
     or has_function_privilege('anon', 'public.panel_errores_grupos(int, text)', 'execute')
     or has_function_privilege('anon', 'public.panel_errores_detalle(text, int)', 'execute')
     or has_function_privilege('anon', 'public.panel_errores_cambiar_estado(text, text)', 'execute') then
    raise exception '1: anon puede ejecutar funciones de errores';
  end if;
end $$;

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'humano-errores@example.com')::text, true);
do $$
begin
  perform public.registrar_errores(jsonb_build_array(pg_temp.reporte('evento-humano-01', '00000000000000aa')));
  raise exception '1: una sesión humana pudo enviar errores';
exception when insufficient_privilege then null;
end $$;

select pg_temp.como_equipo(4);
do $$
begin
  perform public.registrar_errores(jsonb_build_array(pg_temp.reporte('evento-visor-001', '00000000000000aa')));
  raise exception '1: un visor pudo enviar errores';
exception when insufficient_privilege then null;
end $$;

select pg_temp.como_equipo(3);
do $$
begin
  perform public.registrar_errores(jsonb_build_array(pg_temp.reporte('evento-retirado1', '00000000000000aa')));
  raise exception '1: un equipo retirado pudo enviar errores';
exception when insufficient_privilege then null;
end $$;

-- 2. El celular manda tres errores: dos del mismo grupo y uno de otro (con
--    hora en el futuro).
select pg_temp.como_equipo(1);
do $$
declare
  v_respuesta jsonb;
begin
  v_respuesta := public.registrar_errores(jsonb_build_array(
    pg_temp.reporte('evento-a-000001', 'aaaaaaaaaaaaaaaa'),
    pg_temp.reporte('evento-a-000002', 'aaaaaaaaaaaaaaaa'),
    jsonb_set(pg_temp.reporte('evento-b-000001', 'bbbbbbbbbbbbbbbb', '1.8.13', 'NullPointerException'),
              '{ocurrido_en}', to_jsonb(now() + interval '3 days'))
  ));
  if v_respuesta ->> 'estado' <> 'recibido' or (v_respuesta ->> 'guardados')::int <> 3
     or (v_respuesta ->> 'repetidos')::int <> 0 or (v_respuesta ->> 'grupos_nuevos')::int <> 2
     or (v_respuesta ->> 'regresiones')::int <> 0 then
    raise exception '2: respuesta inesperada %', v_respuesta;
  end if;
end $$;

reset role;
do $$
declare
  v_grupo observabilidad.errores_grupo;
  v_error observabilidad.errores;
begin
  select * into strict v_grupo from observabilidad.errores_grupo where huella = 'aaaaaaaaaaaaaaaa';
  if v_grupo.ocurrencias <> 2 or v_grupo.plataforma <> 'movil' or v_grupo.estado <> 'nuevo'
     or v_grupo.versiones <> array['1.8.13'] or v_grupo.origen <> 'excepcion' then
    raise exception '2: grupo mal armado: %', row_to_json(v_grupo);
  end if;
  select * into strict v_error from observabilidad.errores where event_id = 'evento-a-000001';
  if v_error.dispositivo_id <> 'cccccccc-0000-0000-0000-00000000b901'
     or v_error.sitio_id <> 'aaaaaaaa-0000-0000-0000-00000000b901'
     or v_error.plataforma <> 'movil' then
    raise exception '2: el error no quedó a nombre del equipo del JWT: %', row_to_json(v_error);
  end if;
  if v_error.nivel <> 'fatal' or v_error.pantalla <> 'Ingreso' or v_error.version_app <> '1.8.13'
     or v_error.traza -> 0 ->> 'archivo' <> 'Ingreso.kt'
     or v_error.operaciones -> 0 ->> 'nombre' <> 'registrar_ingreso'
     or v_error.contexto ->> 'modelo' <> 'SM-A155M' then
    raise exception '2: campos mal guardados: %', row_to_json(v_error);
  end if;
  if (select ocurrido_en from observabilidad.errores where event_id = 'evento-b-000001') > now() + interval '1 minute' then
    raise exception '2: una hora en el futuro no se corrigió';
  end if;
end $$;

-- 3. Reintento: los mismos event_id, más uno repetido dentro del lote.
set local role authenticated;
do $$
declare
  v_respuesta jsonb;
begin
  v_respuesta := public.registrar_errores(jsonb_build_array(
    pg_temp.reporte('evento-a-000001', 'aaaaaaaaaaaaaaaa'),
    pg_temp.reporte('evento-a-000002', 'aaaaaaaaaaaaaaaa'),
    pg_temp.reporte('evento-a-000002', 'aaaaaaaaaaaaaaaa')
  ));
  if (v_respuesta ->> 'guardados')::int <> 0 or (v_respuesta ->> 'repetidos')::int <> 3
     or (v_respuesta ->> 'grupos_nuevos')::int <> 0 then
    raise exception '3: el reintento no se reconoció: %', v_respuesta;
  end if;
end $$;

reset role;
do $$
begin
  if (select ocurrencias from observabilidad.errores_grupo where huella = 'aaaaaaaaaaaaaaaa') <> 2
     or (select count(*) from observabilidad.errores where huella = 'aaaaaaaaaaaaaaaa') <> 2 then
    raise exception '3: el reintento sumó ocurrencias';
  end if;
end $$;

-- 4. Lotes mal formados: error y nada guardado.
set local role authenticated;
do $$
declare
  v_caso record;
begin
  for v_caso in
    select * from (values
      ('no es lista', jsonb_build_object('x', 1)),
      ('sin título', jsonb_build_array(pg_temp.reporte('evento-malo-0001', 'cccccccccccccccc') - 'titulo')),
      ('huella que no es hex', jsonb_build_array(pg_temp.reporte('evento-malo-0002', 'no-es-una-huella'))),
      ('origen desconocido', jsonb_build_array(jsonb_set(pg_temp.reporte('evento-malo-0003', 'cccccccccccccccc'),
                                                         '{origen}', '"otro"'))),
      ('traza que no es lista', jsonb_build_array(jsonb_set(pg_temp.reporte('evento-malo-0004', 'cccccccccccccccc'),
                                                            '{traza}', '{"a": 1}'))),
      ('traza demasiado larga', jsonb_build_array(jsonb_set(pg_temp.reporte('evento-malo-0005', 'cccccccccccccccc'),
        '{traza}', (select jsonb_agg(jsonb_build_object('funcion', 'f')) from generate_series(1, 31))))),
      ('event_id con caracteres raros', jsonb_build_array(pg_temp.reporte('evento malo; drop', 'cccccccccccccccc'))),
      ('hora que no es hora', jsonb_build_array(jsonb_set(pg_temp.reporte('evento-malo-0006', 'cccccccccccccccc'),
                                                          '{ocurrido_en}', '"ayer"'))),
      -- Un reporte bueno junto a uno malo: tampoco entra el bueno.
      ('uno bueno y uno malo', jsonb_build_array(pg_temp.reporte('evento-bueno-001', 'cccccccccccccccc'),
                                                  pg_temp.reporte('evento-malo-0007', 'cccccccccccccccc') - 'nivel')),
      ('demasiados reportes', (select jsonb_agg(pg_temp.reporte('evento-lote-' || lpad(i::text, 4, '0'), 'cccccccccccccccc'))
                                 from generate_series(1, 51) i))
    ) as c(nombre, reportes)
  loop
    begin
      perform public.registrar_errores(v_caso.reportes);
      raise exception '4: se aceptó un lote mal formado (%)', v_caso.nombre;
    exception
      when invalid_parameter_value or check_violation or invalid_text_representation
           or invalid_datetime_format or not_null_violation then null;
    end;
  end loop;
end $$;

reset role;
do $$
begin
  if exists (select 1 from observabilidad.errores_grupo where huella = 'cccccccccccccccc')
     or exists (select 1 from observabilidad.errores where event_id like 'evento-malo-%' or event_id like 'evento-bueno-%'
                                                       or event_id like 'evento-lote-%') then
    raise exception '4: un lote rechazado dejó filas';
  end if;
end $$;

-- 5. Nada de las tablas se lee ni se escribe desde la API.
do $$
begin
  if has_table_privilege('authenticated', 'observabilidad.errores', 'select')
     or has_table_privilege('authenticated', 'observabilidad.errores', 'insert')
     or has_table_privilege('authenticated', 'observabilidad.errores_grupo', 'select')
     or has_table_privilege('authenticated', 'observabilidad.errores_grupo', 'update')
     or has_table_privilege('anon', 'observabilidad.errores', 'select') then
    raise exception '5: la API tiene permisos sobre las tablas de errores';
  end if;
  if has_function_privilege('authenticated', 'observabilidad.mantener_errores()', 'execute')
     or has_function_privilege('authenticated', 'observabilidad.leer_reportes(jsonb)', 'execute') then
    raise exception '5: la API puede correr funciones internas de errores';
  end if;
end $$;

-- 6. Ritmo: con un máximo de 4 por minuto, el celular (que ya mandó 3) no
--    puede mandar 2 más.
update observabilidad.config set errores_por_minuto = 4;
set local role authenticated;
do $$
begin
  perform public.registrar_errores(jsonb_build_array(
    pg_temp.reporte('evento-ritmo-001', 'dddddddddddddddd'),
    pg_temp.reporte('evento-ritmo-002', 'dddddddddddddddd')));
  raise exception '6: no se aplicó el ritmo por minuto';
exception when sqlstate '53400' then null;
end $$;
reset role;
update observabilidad.config set errores_por_minuto = 60;

-- 7. Regresión. El administrador marca `aaaa…` resuelto (visto en 1.8.13) y
--    `bbbb…` ignorado.
set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('prueba.admin'))::text, true);
select public.panel_errores_cambiar_estado('aaaaaaaaaaaaaaaa', 'resuelto');
select public.panel_errores_cambiar_estado('bbbbbbbbbbbbbbbb', 'ignorado');

-- La PC (otra plataforma, mismo grupo para la prueba) vuelve a dar el error
-- en 1.8.13: un equipo sin actualizar no lo reabre.
select pg_temp.como_equipo(2);
do $$
declare
  v_respuesta jsonb;
begin
  v_respuesta := public.registrar_errores(jsonb_build_array(pg_temp.reporte('evento-a-000003', 'aaaaaaaaaaaaaaaa')));
  if (v_respuesta ->> 'regresiones')::int <> 0 then
    raise exception '7: la misma versión contó como regresión: %', v_respuesta;
  end if;
end $$;
reset role;
do $$
begin
  if (select estado from observabilidad.errores_grupo where huella = 'aaaaaaaaaaaaaaaa') <> 'resuelto' then
    raise exception '7: la misma versión reabrió el error';
  end if;
  if (select plataforma from observabilidad.errores where event_id = 'evento-a-000003') <> 'escritorio' then
    raise exception '7: la plataforma no salió del equipo (pc)';
  end if;
end $$;

-- En 1.8.14 sí: regresión. El ignorado se queda ignorado.
set local role authenticated;
do $$
declare
  v_respuesta jsonb;
begin
  v_respuesta := public.registrar_errores(jsonb_build_array(
    pg_temp.reporte('evento-a-000004', 'aaaaaaaaaaaaaaaa', '1.8.14'),
    pg_temp.reporte('evento-b-000002', 'bbbbbbbbbbbbbbbb', '1.8.14', 'NullPointerException')));
  if (v_respuesta ->> 'regresiones')::int <> 1 or (v_respuesta ->> 'guardados')::int <> 2 then
    raise exception '7: la versión nueva no contó como regresión: %', v_respuesta;
  end if;
end $$;
reset role;
do $$
declare
  v_grupo observabilidad.errores_grupo;
begin
  select * into strict v_grupo from observabilidad.errores_grupo where huella = 'aaaaaaaaaaaaaaaa';
  if v_grupo.estado <> 'nuevo' or v_grupo.ocurrencias <> 4 or v_grupo.versiones <> array['1.8.13', '1.8.14']
     or v_grupo.estado_cambiado_en < now() - interval '1 minute' then
    raise exception '7: el grupo no quedó como regresión: %', row_to_json(v_grupo);
  end if;
  if (select estado from observabilidad.errores_grupo where huella = 'bbbbbbbbbbbbbbbb') <> 'ignorado' then
    raise exception '7: un error ignorado se reabrió';
  end if;
end $$;

-- 8. Panel.
set local role authenticated;
select pg_temp.como_equipo(1);
do $$
begin
  perform * from public.panel_errores_grupos(7, null);
  raise exception '8: un equipo leyó los errores';
exception when insufficient_privilege then null;
end $$;
do $$
begin
  perform public.panel_errores_cambiar_estado('aaaaaaaaaaaaaaaa', 'visto');
  raise exception '8: un equipo cambió el estado de un error';
exception when insufficient_privilege then null;
end $$;

select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('prueba.admin'))::text, true);
do $$
declare
  v record;
  v_estado jsonb;
begin
  select * into strict v from public.panel_errores_grupos(7, null) g where g.huella = 'aaaaaaaaaaaaaaaa';
  if v.en_periodo <> 4 or v.equipos <> 2 or v.estado <> 'nuevo' then
    raise exception '8: grupo del panel mal calculado: %', row_to_json(v);
  end if;
  if exists (select 1 from public.panel_errores_grupos(7, 'nuevo') g where g.huella = 'bbbbbbbbbbbbbbbb') then
    raise exception '8: el filtro por estado no filtró';
  end if;
  if (select count(*) from public.panel_errores_detalle('aaaaaaaaaaaaaaaa', 2)) <> 2 then
    raise exception '8: el detalle no respetó el límite';
  end if;
  select * into strict v from public.panel_errores_detalle('aaaaaaaaaaaaaaaa', 1);
  if v.version_app <> '1.8.14' or v.etiqueta <> 'PC errores 2' then
    raise exception '8: el detalle no trae primero la última ocurrencia: %', row_to_json(v);
  end if;

  v_estado := public.panel_errores_cambiar_estado('aaaaaaaaaaaaaaaa', 'visto');
  if v_estado ->> 'estado' <> 'visto' then
    raise exception '8: no cambió el estado: %', v_estado;
  end if;
  begin
    perform public.panel_errores_cambiar_estado('aaaaaaaaaaaaaaaa', 'borrado');
    raise exception '8: aceptó un estado desconocido';
  exception when invalid_parameter_value then null;
  end;
  begin
    perform public.panel_errores_cambiar_estado('ffffffffffffffff', 'visto');
    raise exception '8: cambió un error que no existe';
  exception when no_data_found then null;
  end;
end $$;

-- 9. Mantenimiento: un error de hace 200 días (con su grupo, que queda
--    vacío) se borra; lo reciente queda.
reset role;
insert into observabilidad.errores_grupo (huella, plataforma, origen, titulo, primera_vez, ultima_vez, ocurrencias)
values ('eeeeeeeeeeeeeeee', 'movil', 'panico', 'viejo', now() - interval '200 days', now() - interval '200 days', 1);
insert into observabilidad.errores (huella, event_id, dispositivo_id, sitio_id, plataforma, ocurrido_en, recibido_en,
                                    nivel, origen, titulo)
values ('eeeeeeeeeeeeeeee', 'evento-viejo-001', 'cccccccc-0000-0000-0000-00000000b901',
        'aaaaaaaa-0000-0000-0000-00000000b901', 'movil', now() - interval '200 days', now() - interval '200 days',
        'fatal', 'panico', 'viejo');

do $$
declare
  v_resultado jsonb;
begin
  v_resultado := observabilidad.mantener_errores();
  if (v_resultado ->> 'errores')::int <> 1 or (v_resultado ->> 'grupos')::int <> 1 then
    raise exception '9: el mantenimiento no borró lo vencido: %', v_resultado;
  end if;
  if exists (select 1 from observabilidad.errores_grupo where huella = 'eeeeeeeeeeeeeeee')
     or not exists (select 1 from observabilidad.errores_grupo where huella = 'aaaaaaaaaaaaaaaa') then
    raise exception '9: el mantenimiento borró de más o de menos';
  end if;
  if not exists (select 1 from observabilidad.mantenimiento_registro where accion = 'errores') then
    raise exception '9: el mantenimiento no quedó registrado';
  end if;
  if not exists (select 1 from cron.job where jobname = 'observabilidad-errores-diario'
                    and schedule = '12 9 * * *' and command = 'select observabilidad.mantener_errores()') then
    raise exception '9: falta la tarea diaria de pg_cron';
  end if;
end $$;

rollback;

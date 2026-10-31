-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `registrar_telemetria` y el mantenimiento diario
-- (20261101120000_observabilidad_telemetria):
--  1. anon no puede llamar; una sesión humana (sin equipo) tampoco.
--  2. Un equipo vigente manda un lote columnar: entra, con su equipo y su
--     unidad puestos desde el JWT, la hora reconstruida al milisegundo, un
--     `null` explícito distinto de un campo ausente, `pantalla` en su columna
--     y `llamada_nucleo` en `llamadas`. Un tipo desconocido y un evento viejo
--     se descartan y se cuentan sin tumbar el lote.
--  3. El mismo `lote_id` otra vez: `duplicado`, sin filas nuevas.
--  4. Lotes mal formados (fila corta, diferencia negativa, columnas
--     repetidas, versión que no es semver, operación no canónica): error y
--     nada guardado, ni siquiera el `lote_id`.
--  5. Un equipo retirado no puede enviar.
--  6. Un equipo no puede escribir en la sesión de otro.
--  7. Las tablas no se leen ni se escriben desde la API.
--  8. Ritmo: pasado el máximo de lotes por minuto, 53400.
--  9. Mantenimiento: resume, purga lo vencido, respeta el tope de filas y lo
--     registra; la tarea de pg_cron existe.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000b701', 'Unidad observabilidad A');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at) values
  ('cccccccc-0000-0000-0000-00000000b701', 'aaaaaaaa-0000-0000-0000-00000000b701', 'mobile', 'Celular observabilidad 1', '{"kty":"EC"}', 'huella-obs-1', null),
  ('cccccccc-0000-0000-0000-00000000b702', 'aaaaaaaa-0000-0000-0000-00000000b701', 'pc', 'PC observabilidad 2', '{"kty":"EC"}', 'huella-obs-2', null),
  ('cccccccc-0000-0000-0000-00000000b703', 'aaaaaaaa-0000-0000-0000-00000000b701', 'mobile', 'Celular observabilidad retirado', '{"kty":"EC"}', 'huella-obs-3', now());

-- Hora base del lote de prueba, en ms, guardada para comparar después.
select set_config('prueba.base_ms', (floor(extract(epoch from now()) * 1000)::bigint - 60000)::text, true);

-- Arma un lote con los bloques dados (sesión fija del equipo 1).
create function pg_temp.lote(p_lote_id uuid, p_bloques jsonb, p_sesion jsonb default null)
returns jsonb language sql as $$
  select jsonb_build_object(
    'lote_id', p_lote_id,
    'esquema', 3,
    'base_ms', current_setting('prueba.base_ms')::bigint,
    'sesion', coalesce(p_sesion, jsonb_build_object(
      'id', 'dddddddd-0000-0000-0000-00000000b701',
      'plataforma', 'movil',
      'version_app', '1.8.14',
      'version_codigo', 59,
      'commit', 'abc1234',
      'ambiente', 'produccion',
      'iniciada_en', now() - interval '5 minutes',
      'sistema', 'Android 14',
      'modelo', 'SM-A155M',
      'ram_total_mb', 4096,
      -- Intento de falsear el equipo desde el cuerpo: se ignora.
      'dispositivo_id', 'cccccccc-0000-0000-0000-00000000b702'
    )),
    'bloques', p_bloques
  );
$$;
-- Las funciones nuevas no son ejecutables por `public` (permisos explícitos).
grant execute on function pg_temp.lote(uuid, jsonb, jsonb) to authenticated;

-- 1. anon no tiene permiso; una sesión humana no es un equipo.
do $$
begin
  if has_function_privilege('anon', 'public.registrar_telemetria(jsonb)', 'execute') then
    raise exception '1: anon puede ejecutar registrar_telemetria';
  end if;
  if has_function_privilege('anon', 'observabilidad.registrar_telemetria(jsonb)', 'execute') then
    raise exception '1: anon puede ejecutar observabilidad.registrar_telemetria';
  end if;
end $$;

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', 'humano-observabilidad@example.com')::text, true);

do $$
begin
  perform public.registrar_telemetria(pg_temp.lote(gen_random_uuid(), '[]'::jsonb));
  raise exception '1: una sesión humana pudo enviar telemetría';
exception when insufficient_privilege then null;
end $$;

-- 2. El equipo 1 manda un lote.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000b701',
  'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000b701', 'huella', 'huella-obs-1')::text, true);

do $$
declare
  v_respuesta jsonb;
begin
  v_respuesta := public.registrar_telemetria(pg_temp.lote(
    'eeeeeeee-0000-0000-0000-00000000b701',
    jsonb_build_array(
      jsonb_build_object(
        'tipo', 'llamada_nucleo',
        'cols', jsonb_build_array('nombre', 'llamadas', 'errores', 'p50_ms', 'p90_ms', 'max_ms', 'total_ms', 'red_tipo', 'errores_por_tipo'),
        'filas', jsonb_build_array(
          jsonb_build_array(0, 'registrar_ingreso', 12, 1, 38.5, 91, 140, 610, 'datos', jsonb_build_object('red', 1)),
          jsonb_build_array(1500, 'registrar_ingreso', 3, 0, 40, 70, 75, 130, 'wifi', null)
        )
      ),
      -- `pantalla` va a su columna; `carga` es un null explícito.
      jsonb_build_object(
        'tipo', 'frames',
        'cols', jsonb_build_array('pantalla', 'frames', 'carga'),
        'filas', jsonb_build_array(jsonb_build_array(250, 'activos', 300, null))
      ),
      -- Mismo tipo, sin la columna `carga`: ausente, no null.
      jsonb_build_object(
        'tipo', 'frames',
        'cols', jsonb_build_array('pantalla', 'frames'),
        'filas', jsonb_build_array(jsonb_build_array(500, 'ingreso', 120))
      ),
      -- Tipo que la nube no conoce: se descarta y se cuenta.
      jsonb_build_object(
        'tipo', 'evento_del_futuro',
        'cols', jsonb_build_array('x'),
        'filas', jsonb_build_array(jsonb_build_array(0, 1))
      ),
      -- Un evento normal de otro grupo.
      jsonb_build_object(
        'tipo', 'reloj',
        'cols', jsonb_build_array('confiable'),
        'filas', jsonb_build_array(jsonb_build_array(0, true))
      )
    )
  ));
  if v_respuesta ->> 'estado' <> 'recibido' or (v_respuesta ->> 'filas')::int <> 5
     or (v_respuesta ->> 'descartadas_tipo')::int <> 1 or (v_respuesta ->> 'descartadas_otras')::int <> 0 then
    raise exception '2: respuesta inesperada %', v_respuesta;
  end if;
end $$;

reset role;

do $$
declare
  v_base bigint := current_setting('prueba.base_ms')::bigint;
  v_sesion observabilidad.sesiones;
  v_llamada observabilidad.llamadas;
  v_evento observabilidad.eventos;
  v_lote observabilidad.lotes;
begin
  select * into strict v_sesion from observabilidad.sesiones where id = 'dddddddd-0000-0000-0000-00000000b701';
  if v_sesion.dispositivo_id <> 'cccccccc-0000-0000-0000-00000000b701'
     or v_sesion.sitio_id <> 'aaaaaaaa-0000-0000-0000-00000000b701' then
    raise exception '2: la sesión no quedó a nombre del equipo del JWT';
  end if;
  if v_sesion.version_app <> '1.8.14' or v_sesion.version_codigo <> 59 or v_sesion."commit" <> 'abc1234' then
    raise exception '2: versión de la sesión mal guardada';
  end if;

  if (select count(*) from observabilidad.llamadas where sesion_id = v_sesion.id) <> 2 then
    raise exception '2: no entraron las 2 llamadas';
  end if;
  select * into strict v_llamada from observabilidad.llamadas
   where sesion_id = v_sesion.id and llamadas = 12;
  if v_llamada.dispositivo_id <> 'cccccccc-0000-0000-0000-00000000b701'
     or v_llamada.operacion <> 'registrar_ingreso' or v_llamada.errores <> 1
     or v_llamada.p50_ms <> 38.5 or v_llamada.red_tipo <> 'datos'
     or v_llamada.errores_por_tipo <> '{"red": 1}'::jsonb then
    raise exception '2: llamada mal decodificada';
  end if;
  if v_llamada.ocurrido_en <> to_timestamp(v_base / 1000.0) then
    raise exception '2: hora de la primera llamada (% <> %)', v_llamada.ocurrido_en, to_timestamp(v_base / 1000.0);
  end if;
  -- La segunda: base + 1500 ms (diferencia acumulada dentro del bloque).
  if not exists (select 1 from observabilidad.llamadas
                  where sesion_id = v_sesion.id and llamadas = 3
                    and ocurrido_en = to_timestamp((v_base + 1500) / 1000.0)
                    and errores_por_tipo is null) then
    raise exception '2: hora o errores_por_tipo de la segunda llamada';
  end if;

  select * into strict v_evento from observabilidad.eventos
   where sesion_id = v_sesion.id and tipo = 'frames' and pantalla = 'activos';
  if v_evento.grupo <> 'fluidez' or v_evento.datos <> '{"frames": 300, "carga": null}'::jsonb
     or v_evento.ocurrido_en <> to_timestamp((v_base + 250) / 1000.0) then
    raise exception '2: evento con null explícito mal guardado: %', v_evento.datos;
  end if;
  if not exists (select 1 from observabilidad.eventos
                  where sesion_id = v_sesion.id and pantalla = 'ingreso'
                    and datos = '{"frames": 120}'::jsonb and not (datos ? 'carga')) then
    raise exception '2: el campo ausente no quedó ausente';
  end if;

  select * into strict v_lote from observabilidad.lotes where lote_id = 'eeeeeeee-0000-0000-0000-00000000b701';
  if v_lote.filas <> 5 or v_lote.descartadas <> 1 or v_lote.esquema <> 3 then
    raise exception '2: el lote no quedó registrado bien (% filas, % descartadas)', v_lote.filas, v_lote.descartadas;
  end if;
end $$;

-- 2b. Un evento fuera de la ventana de tiempo se descarta y se cuenta.
set local role authenticated;
do $$
declare
  v_respuesta jsonb;
begin
  v_respuesta := public.registrar_telemetria(jsonb_set(
    pg_temp.lote('eeeeeeee-0000-0000-0000-00000000b702',
      jsonb_build_array(jsonb_build_object('tipo', 'reloj', 'cols', jsonb_build_array('confiable'),
        'filas', jsonb_build_array(jsonb_build_array(0, true))))),
    '{base_ms}', to_jsonb(floor(extract(epoch from now() - interval '40 days') * 1000)::bigint)));
  if (v_respuesta ->> 'filas')::int <> 0 or (v_respuesta ->> 'descartadas_otras')::int <> 1 then
    raise exception '2b: el evento viejo no se descartó: %', v_respuesta;
  end if;
end $$;

-- 3. El mismo lote otra vez: duplicado, nada nuevo.
do $$
declare
  v_respuesta jsonb;
begin
  v_respuesta := public.registrar_telemetria(pg_temp.lote(
    'eeeeeeee-0000-0000-0000-00000000b701',
    jsonb_build_array(jsonb_build_object('tipo', 'frames', 'cols', jsonb_build_array('frames'),
      'filas', jsonb_build_array(jsonb_build_array(0, 1))))));
  if v_respuesta ->> 'estado' <> 'duplicado' then
    raise exception '3: un lote repetido no se reconoció: %', v_respuesta;
  end if;
end $$;

reset role;
do $$
begin
  -- 2 frames + 1 reloj del paso 2; el del paso 2b se descartó.
  if (select count(*) from observabilidad.eventos where sesion_id = 'dddddddd-0000-0000-0000-00000000b701') <> 3 then
    raise exception '3: el duplicado insertó filas';
  end if;
end $$;

-- 4. Lotes mal formados: error y nada guardado (ni el lote_id).
set local role authenticated;
do $$
declare
  v_caso record;
begin
  for v_caso in
    select * from (values
      ('fila corta', jsonb_build_array(jsonb_build_object('tipo', 'frames', 'cols', jsonb_build_array('a', 'b'),
        'filas', jsonb_build_array(jsonb_build_array(0, 1))))),
      ('diferencia negativa', jsonb_build_array(jsonb_build_object('tipo', 'frames', 'cols', jsonb_build_array('a'),
        'filas', jsonb_build_array(jsonb_build_array(-5, 1))))),
      ('diferencia con decimales', jsonb_build_array(jsonb_build_object('tipo', 'frames', 'cols', jsonb_build_array('a'),
        'filas', jsonb_build_array(jsonb_build_array(1.5, 1))))),
      ('columnas repetidas', jsonb_build_array(jsonb_build_object('tipo', 'frames', 'cols', jsonb_build_array('a', 'a'),
        'filas', jsonb_build_array(jsonb_build_array(0, 1, 2))))),
      ('bloques no es lista', '{"x": 1}'::jsonb),
      ('operación no canónica', jsonb_build_array(jsonb_build_object('tipo', 'llamada_nucleo',
        'cols', jsonb_build_array('nombre', 'llamadas'),
        'filas', jsonb_build_array(jsonb_build_array(0, 'registrarIngresoConSecreto', 1)))))
    ) as c(nombre, bloques)
  loop
    begin
      perform public.registrar_telemetria(pg_temp.lote('eeeeeeee-0000-0000-0000-00000000b7ff', v_caso.bloques));
      raise exception '4: se aceptó un lote mal formado (%)', v_caso.nombre;
    exception
      when invalid_parameter_value or check_violation or invalid_text_representation then null;
    end;
  end loop;

  -- Versión que no es semver (el formato viejo del escritorio).
  begin
    perform public.registrar_telemetria(pg_temp.lote('eeeeeeee-0000-0000-0000-00000000b7fe',
      '[]'::jsonb,
      jsonb_build_object('id', 'dddddddd-0000-0000-0000-00000000b7fe', 'plataforma', 'escritorio',
        'version_app', 'desktop-1.10.7+889cc4f', 'ambiente', 'produccion', 'iniciada_en', now())));
    raise exception '4: se aceptó una versión que no es semver';
  exception when check_violation then null;
  end;
end $$;

reset role;
do $$
begin
  if exists (select 1 from observabilidad.lotes where lote_id in (
      'eeeeeeee-0000-0000-0000-00000000b7ff', 'eeeeeeee-0000-0000-0000-00000000b7fe')) then
    raise exception '4: un lote rechazado dejó su lote_id';
  end if;
end $$;

-- 5. Un equipo retirado no envía.
set local role authenticated;
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000b703',
  'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000b701', 'huella', 'huella-obs-3')::text, true);
do $$
begin
  perform public.registrar_telemetria(pg_temp.lote(gen_random_uuid(), '[]'::jsonb));
  raise exception '5: un equipo retirado pudo enviar';
exception when insufficient_privilege then null;
end $$;

-- 6. El equipo 2 no puede escribir en la sesión del equipo 1.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000b702',
  'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000b701', 'huella', 'huella-obs-2')::text, true);
do $$
begin
  perform public.registrar_telemetria(pg_temp.lote(gen_random_uuid(),
    jsonb_build_array(jsonb_build_object('tipo', 'frames', 'cols', jsonb_build_array('frames'),
      'filas', jsonb_build_array(jsonb_build_array(0, 1))))));
  raise exception '6: un equipo escribió en la sesión de otro';
exception when insufficient_privilege then null;
end $$;

-- 7. Nada de las tablas se lee ni se escribe desde la API.
do $$
declare
  v_tabla text;
begin
  foreach v_tabla in array array['config', 'catalogo_eventos', 'sesiones', 'eventos', 'llamadas', 'lotes',
                                 'resumen_llamadas_diario', 'resumen_eventos_diario', 'mantenimiento_registro'] loop
    if has_table_privilege('authenticated', 'observabilidad.' || v_tabla, 'select')
       or has_table_privilege('authenticated', 'observabilidad.' || v_tabla, 'insert')
       or has_table_privilege('anon', 'observabilidad.' || v_tabla, 'select') then
      raise exception '7: la API tiene permisos sobre observabilidad.%', v_tabla;
    end if;
  end loop;
  if has_function_privilege('authenticated', 'observabilidad.mantener()', 'execute')
     or has_function_privilege('authenticated', 'observabilidad.resumir_desde(date)', 'execute') then
    raise exception '7: la API puede correr el mantenimiento';
  end if;
  begin
    perform 1 from observabilidad.eventos limit 1;
    raise exception '7: authenticated leyó observabilidad.eventos';
  exception when insufficient_privilege then null;
  end;
end $$;

-- 8. Ritmo: con un máximo de 2 lotes por minuto, el equipo 1 (que ya mandó
--    2) recibe 53400.
reset role;
update observabilidad.config set lotes_por_minuto = 2;
set local role authenticated;
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000b701',
  'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000b701', 'huella', 'huella-obs-1')::text, true);
do $$
begin
  perform public.registrar_telemetria(pg_temp.lote(gen_random_uuid(), '[]'::jsonb));
  raise exception '8: no se aplicó el ritmo por minuto';
exception when sqlstate '53400' then null;
end $$;

-- 9. Mantenimiento.
reset role;
update observabilidad.config set lotes_por_minuto = 20;

-- Una sesión vieja con detalle de hace 40 días (se purga) y otra con detalle
-- de hace 2 y 3 días (para el resumen y el tope).
insert into observabilidad.sesiones (id, dispositivo_id, sitio_id, plataforma, version_app, ambiente, iniciada_en) values
  ('dddddddd-0000-0000-0000-00000000b7aa', 'cccccccc-0000-0000-0000-00000000b701', 'aaaaaaaa-0000-0000-0000-00000000b701', 'movil', '1.8.12', 'produccion', now() - interval '41 days'),
  ('dddddddd-0000-0000-0000-00000000b7bb', 'cccccccc-0000-0000-0000-00000000b702', 'aaaaaaaa-0000-0000-0000-00000000b701', 'escritorio', '1.10.7', 'produccion', now() - interval '4 days');
insert into observabilidad.eventos (sesion_id, dispositivo_id, ocurrido_en, tipo, grupo, datos) values
  ('dddddddd-0000-0000-0000-00000000b7aa', 'cccccccc-0000-0000-0000-00000000b701', now() - interval '40 days', 'reloj', 'sincronizacion', '{}');
insert into observabilidad.llamadas (sesion_id, dispositivo_id, ocurrido_en, operacion, llamadas, errores, p50_ms, p90_ms, max_ms) values
  ('dddddddd-0000-0000-0000-00000000b7aa', 'cccccccc-0000-0000-0000-00000000b701', now() - interval '40 days', 'registrar_ingreso', 5, 0, 30, 40, 50),
  ('dddddddd-0000-0000-0000-00000000b7bb', 'cccccccc-0000-0000-0000-00000000b702', now() - interval '3 days', 'registrar_ingreso', 10, 1, 60, 90, 120),
  ('dddddddd-0000-0000-0000-00000000b7bb', 'cccccccc-0000-0000-0000-00000000b702', now() - interval '2 days', 'registrar_ingreso', 4, 0, 70, 95, 130);

do $$
declare
  v_resultado jsonb;
begin
  -- Sin tope que alcance: sólo retención.
  update observabilidad.config set tope_filas_detalle = 600000;
  v_resultado := observabilidad.mantener();
  if exists (select 1 from observabilidad.eventos where ocurrido_en < now() - interval '30 days')
     or exists (select 1 from observabilidad.llamadas where ocurrido_en < now() - interval '30 days') then
    raise exception '9: no se purgó el detalle vencido';
  end if;
  if exists (select 1 from observabilidad.sesiones where id = 'dddddddd-0000-0000-0000-00000000b7aa') then
    raise exception '9: no se purgó la sesión vieja sin detalle';
  end if;
  if not exists (select 1 from observabilidad.resumen_llamadas_diario
                  where dispositivo_id = 'cccccccc-0000-0000-0000-00000000b702'
                    and version_app = '1.10.7' and operacion = 'registrar_ingreso'
                    and dia = ((now() - interval '2 days') at time zone 'America/Costa_Rica')::date
                    and llamadas = 4) then
    raise exception '9: falta el resumen diario';
  end if;
  -- La corrida diaria resume desde hace 2 días: la llamada de hace 3 días
  -- todavía no está resumida (en producción la resumió una corrida anterior).
  -- Idempotente: correrlo de nuevo no duplica ni cambia el resumen.
  perform observabilidad.mantener();
  if (select count(*) from observabilidad.resumen_llamadas_diario
       where dispositivo_id = 'cccccccc-0000-0000-0000-00000000b702') <> 1 then
    raise exception '9: el resumen no es idempotente';
  end if;
  if (v_resultado ->> 'por_tope')::int <> 0 then
    raise exception '9: el tope borró sin necesidad';
  end if;
end $$;

-- Tope: con un tope menor que lo que hay, se borra el día más viejo y queda
-- registrado. (El mínimo permitido por el CHECK es 10.000: se baja el CHECK
-- sólo dentro de esta transacción para probar con pocas filas.)
alter table observabilidad.config drop constraint config_tope_filas_detalle_check;
update observabilidad.config set tope_filas_detalle = 3;
do $$
declare
  v_antes bigint;
  v_resultado jsonb;
begin
  select (select count(*) from observabilidad.eventos) + (select count(*) from observabilidad.llamadas) into v_antes;
  if v_antes <= 3 then
    raise exception '9: la prueba del tope necesita más de 3 filas (hay %)', v_antes;
  end if;
  v_resultado := observabilidad.mantener();
  if (v_resultado ->> 'filas_detalle')::bigint > 3 or (v_resultado ->> 'por_tope')::bigint = 0 then
    raise exception '9: el tope no se respetó: %', v_resultado;
  end if;
  if not exists (select 1 from observabilidad.mantenimiento_registro where accion = 'tope') then
    raise exception '9: el tope no quedó registrado';
  end if;
  -- Lo más viejo es lo que se fue: la llamada de hace 3 días ya no está…
  if exists (select 1 from observabilidad.llamadas where ocurrido_en < now() - interval '2 days 12 hours') then
    raise exception '9: el tope no borró lo más viejo primero';
  end if;
  -- …pero antes de borrarla quedó en el resumen.
  if not exists (select 1 from observabilidad.resumen_llamadas_diario
                  where dispositivo_id = 'cccccccc-0000-0000-0000-00000000b702'
                    and dia = ((now() - interval '3 days') at time zone 'America/Costa_Rica')::date
                    and llamadas = 10 and errores = 1) then
    raise exception '9: el tope borró detalle sin resumirlo';
  end if;
end $$;

do $$
begin
  if not exists (select 1 from cron.job where jobname = 'observabilidad-mantenimiento-diario'
                    and schedule = '7 9 * * *' and command = 'select observabilidad.mantener()') then
    raise exception '9: falta la tarea diaria de pg_cron';
  end if;
end $$;

rollback;

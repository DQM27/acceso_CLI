-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `sesion_usuario_en_unidad` (la hora del ingreso la calcula la nube:
-- `now()` menos lo que el equipo midió desde el ingreso),
-- `cerrar_sesion_usuario_en_unidad` y la bitácora (`bitacora_sesiones`,
-- `panel_bitacora_sesiones`): varios equipos de la misma unidad conviven,
-- otra unidad desplaza (gana el último ingreso), un ingreso sin conexión más
-- viejo queda desplazado, dos ingresos casi simultáneos se desempatan por
-- orden de llegada (gana el último), y cada inicio y cierre queda en la
-- bitácora con su motivo.
--
-- Dentro de la transacción `now()` no cambia, así que cada ingreso queda en
-- `now() - transcurrido` exacto.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true);
insert into public.administradores_panel (correo) values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a301', 'Unidad sesión A'),
  ('aaaaaaaa-0000-0000-0000-00000000a302', 'Unidad sesión B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301', 'pc',    'PC A',      '{"kty":"EC"}', 'huella-pc-a'),
  ('cccccccc-0000-0000-0000-00000000c302', 'aaaaaaaa-0000-0000-0000-00000000a301', 'mobile', 'Celular A', '{"kty":"EC"}', 'huella-cel-a'),
  ('cccccccc-0000-0000-0000-00000000c303', 'aaaaaaaa-0000-0000-0000-00000000a302', 'pc',    'PC B',      '{"kty":"EC"}', 'huella-pc-b');

insert into public.usuarios (id, cedula, nombre, rol, activo) values
  ('99999999-0000-0000-0000-000000000301', '900000301', 'OPERADOR SESION', 'OPERADOR', true);

-- Hace de equipo: JWT de dispositivo (sub, sitio_id, huella).
create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text) returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella)::text, true)
$$;

set local role authenticated;

do $$
declare
  v text;
  h bigint := 3600 * 1000;   -- una hora en ms
  m bigint := 60 * 1000;     -- un minuto en ms
begin
  -- 1. PC y celular de la unidad A conviven.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-pc-a');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000001', 3 * h);
  if v <> 'vigente' then raise exception '1a: PC A debería quedar vigente, dio %', v; end if;

  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c302', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-cel-a');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000002', 3 * h - 5 * m);
  if v <> 'vigente' then raise exception '1b: Celular A debería quedar vigente, dio %', v; end if;

  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-pc-a');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000001', 3 * h);
  if v <> 'vigente' then raise exception '1c: la PC A sigue vigente junto al celular, dio %', v; end if;

  -- 2. Entra en la unidad B (hace 2 h): gana y cierra la sesión de la unidad A.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c303', 'aaaaaaaa-0000-0000-0000-00000000a302', 'huella-pc-b');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000003', 2 * h);
  if v <> 'vigente' then raise exception '2a: PC B debería ganar, dio %', v; end if;

  -- 3. La PC A, al sincronizar, queda desplazada (su sesión ya se cerró).
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-pc-a');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000001', 3 * h);
  if v <> 'desplazada' then raise exception '3a: PC A debería quedar desplazada, dio %', v; end if;

  -- 4. Un ingreso sin conexión en A hace 2 h 30 min (antes que el de B) pierde.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c302', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-cel-a');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000004', 2 * h + 30 * m);
  if v <> 'desplazada' then raise exception '4a: el ingreso sin conexión más viejo debería perder, dio %', v; end if;

  -- 5. B sincroniza varias veces: sigue vigente y no duplica la bitácora.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c303', 'aaaaaaaa-0000-0000-0000-00000000a302', 'huella-pc-b');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000003', 2 * h);
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000003', 2 * h);
  if v <> 'vigente' then raise exception '5a: PC B debería seguir vigente, dio %', v; end if;

  -- 6. Nuevo ingreso en B sin haber cerrado el anterior, y luego salida.
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000005', 1 * h);
  if v <> 'vigente' then raise exception '6a: nuevo ingreso en PC B, dio %', v; end if;
  perform public.cerrar_sesion_usuario_en_unidad('900000301');

  -- 7. Cédula que no es usuario de la nube (ROOT local): no aplica.
  v := public.sesion_usuario_en_unidad('000000000', '11111111-0000-0000-0000-000000000099', 0);
  if v <> 'sin_usuario' then raise exception '7a: cédula desconocida, dio %', v; end if;

  -- 8. Empate: ingresos en A y en B con 500 ms de diferencia (menos que los
  --    márgenes de ~1 s). Ya no queda en duda: gana el último en llegar a
  --    la nube (PC B) y la sesión de A se cierra.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c302', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-cel-a');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000006', 10 * m);
  if v <> 'vigente' then raise exception '8a: Celular A debería quedar vigente, dio %', v; end if;
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c303', 'aaaaaaaa-0000-0000-0000-00000000a302', 'huella-pc-b');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000007', 10 * m - 500);
  if v <> 'vigente' then raise exception '8b: en el empate gana PC B (llegó última), dio %', v; end if;
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c302', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-cel-a');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000006', 10 * m);
  if v <> 'desplazada' then raise exception '8c: Celular A debería quedar fuera, dio %', v; end if;
  -- PC B sincroniza otra vez: sigue vigente (la decisión no se revisa).
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c303', 'aaaaaaaa-0000-0000-0000-00000000a302', 'huella-pc-b');
  v := public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000007', 10 * m - 500);
  if v <> 'vigente' then raise exception '8d: PC B debería seguir vigente, dio %', v; end if;
  perform public.cerrar_sesion_usuario_en_unidad('900000301');

  -- 9. Una duración negativa o absurda es un dato roto: error, no decisión.
  begin
    perform public.sesion_usuario_en_unidad('900000301', '11111111-0000-0000-0000-000000000008', -1);
    raise exception '9a: aceptó una duración negativa';
  exception
    when invalid_parameter_value then null;
  end;

  -- 10. Un equipo no ve la bitácora.
  if exists (select 1 from public.panel_bitacora_sesiones) then
    raise exception '10a: un equipo no debe ver la bitácora';
  end if;
end $$;

-- La unidad A recibió el aviso en vivo, y quedó el evento de seguridad.
reset role;
do $$
begin
  if not exists (
    select 1 from realtime.messages
     where topic = 'sitio:aaaaaaaa-0000-0000-0000-00000000a301' and event = 'sesion_cerrada'
       and payload ->> 'cedula' = '900000301'
  ) then
    raise exception 'No llegó el aviso sesion_cerrada a la unidad A';
  end if;
  if (select count(*) from public.eventos_seguridad_dispositivos
       where tipo = 'sesion_en_otra_unidad' and detalle ->> 'cedula' = '900000301'
         and detalle ->> 'sitio_anterior' = 'Unidad sesión A'
         and (detalle ->> 'equipos_cerrados')::int = 2) <> 1 then
    raise exception 'Falta el evento de seguridad del cambio de unidad';
  end if;
  if (select count(*) from public.eventos_seguridad_dispositivos
       where tipo = 'sesion_en_otra_unidad' and detalle ->> 'cedula' = '900000301'
         and detalle ->> 'sitio_anterior' = 'Unidad sesión A'
         and (detalle ->> 'equipos_cerrados')::int = 1) <> 1 then
    raise exception 'Falta el evento de seguridad del empate resuelto';
  end if;
  if exists (select 1 from public.eventos_seguridad_dispositivos
              where tipo = 'sesion_en_duda' and detalle ->> 'cedula' = '900000301') then
    raise exception 'Ya no debería quedar ninguna sesión en duda';
  end if;
  if exists (select 1 from public.sesiones_usuario where usuario_id = '99999999-0000-0000-0000-000000000301') then
    raise exception 'Tras la salida no debería quedar ninguna sesión abierta';
  end if;
end $$;

-- El administrador ve la bitácora completa.
set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text, true);
do $$
declare
  v_obtenido text;
begin
  select string_agg(dispositivo_etiqueta || '@' || sitio_nombre || '=' || coalesce(motivo_cierre, 'abierta'),
                    ', ' order by iniciada_en, dispositivo_etiqueta)
    into v_obtenido
    from public.panel_bitacora_sesiones
   where cedula = '900000301';

  if v_obtenido is distinct from
     'PC A@Unidad sesión A=otra_unidad, ' ||
     'Celular A@Unidad sesión A=otra_unidad, ' ||
     'Celular A@Unidad sesión A=desplazada, ' ||
     'PC B@Unidad sesión B=sin_cierre, ' ||
     'PC B@Unidad sesión B=salida, ' ||
     'Celular A@Unidad sesión A=otra_unidad, ' ||
     'PC B@Unidad sesión B=salida'
  then
    raise exception 'Bitácora inesperada: %', v_obtenido;
  end if;

  if exists (select 1 from public.panel_bitacora_sesiones
              where cedula = '900000301' and (cerrada_en is null or cerrada_en < iniciada_en)) then
    raise exception 'Toda sesión cerrada debe tener hora de cierre posterior al inicio';
  end if;
end $$;

-- Una sesión de panel (sin equipo) no puede registrar sesiones.
do $$
begin
  perform public.sesion_usuario_en_unidad('900000301', gen_random_uuid(), 0);
  raise exception 'Una sesión sin equipo pudo registrar una sesión';
exception
  when insufficient_privilege then null;
end $$;

-- anon no puede ejecutar las funciones ni leer la bitácora.
reset role;
set local role anon;
do $$
begin
  perform public.sesion_usuario_en_unidad('900000301', gen_random_uuid(), 0);
  raise exception 'anon pudo ejecutar sesion_usuario_en_unidad';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.panel_bitacora_sesiones limit 1;
  raise exception 'anon pudo leer la bitácora';
exception
  when insufficient_privilege then null;
end $$;

rollback;

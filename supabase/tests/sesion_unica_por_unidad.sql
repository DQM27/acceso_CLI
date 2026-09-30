-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `sesion_usuario_en_unidad`, `cerrar_sesion_usuario_en_unidad` y la
-- bitácora (`bitacora_sesiones`, `panel_bitacora_sesiones`): varios equipos
-- de la misma unidad conviven, otra unidad desplaza (gana el último
-- ingreso), un ingreso sin conexión más viejo queda desplazado, y cada
-- inicio y cierre queda en la bitácora con su motivo.
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
  t0 timestamptz := now() - interval '3 hours';
begin
  -- 1. PC y celular de la unidad A conviven.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-pc-a');
  v := public.sesion_usuario_en_unidad('900000301', t0);
  if v <> 'vigente' then raise exception '1a: PC A debería quedar vigente, dio %', v; end if;

  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c302', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-cel-a');
  v := public.sesion_usuario_en_unidad('900000301', t0 + interval '5 minutes');
  if v <> 'vigente' then raise exception '1b: Celular A debería quedar vigente, dio %', v; end if;

  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-pc-a');
  v := public.sesion_usuario_en_unidad('900000301', t0);
  if v <> 'vigente' then raise exception '1c: la PC A sigue vigente junto al celular, dio %', v; end if;

  -- 2. Entra en la unidad B: gana y cierra la sesión de la unidad A.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c303', 'aaaaaaaa-0000-0000-0000-00000000a302', 'huella-pc-b');
  v := public.sesion_usuario_en_unidad('900000301', t0 + interval '1 hour');
  if v <> 'vigente' then raise exception '2a: PC B debería ganar, dio %', v; end if;

  -- 3. Los equipos de A, al sincronizar, quedan desplazados.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-pc-a');
  v := public.sesion_usuario_en_unidad('900000301', t0);
  if v <> 'desplazada' then raise exception '3a: PC A debería quedar desplazada, dio %', v; end if;

  -- 4. Un ingreso sin conexión en A, anterior al de B, también pierde.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c302', 'aaaaaaaa-0000-0000-0000-00000000a301', 'huella-cel-a');
  v := public.sesion_usuario_en_unidad('900000301', t0 + interval '30 minutes');
  if v <> 'desplazada' then raise exception '4a: el ingreso sin conexión más viejo debería perder, dio %', v; end if;

  -- 5. B sincroniza varias veces: sigue vigente y no duplica la bitácora.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c303', 'aaaaaaaa-0000-0000-0000-00000000a302', 'huella-pc-b');
  v := public.sesion_usuario_en_unidad('900000301', t0 + interval '1 hour');
  v := public.sesion_usuario_en_unidad('900000301', t0 + interval '1 hour');
  if v <> 'vigente' then raise exception '5a: PC B debería seguir vigente, dio %', v; end if;

  -- 6. Nuevo ingreso en B sin haber cerrado el anterior, y luego salida.
  v := public.sesion_usuario_en_unidad('900000301', t0 + interval '2 hours');
  if v <> 'vigente' then raise exception '6a: nuevo ingreso en PC B, dio %', v; end if;
  perform public.cerrar_sesion_usuario_en_unidad('900000301');

  -- 7. Cédula que no es usuario de la nube (ROOT local): no aplica.
  v := public.sesion_usuario_en_unidad('000000000', now());
  if v <> 'sin_usuario' then raise exception '7a: cédula desconocida, dio %', v; end if;

  -- 8. Un equipo no ve la bitácora.
  if exists (select 1 from public.panel_bitacora_sesiones) then
    raise exception '8a: un equipo no debe ver la bitácora';
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
  perform public.sesion_usuario_en_unidad('900000301', now());
  raise exception 'Una sesión sin equipo pudo registrar una sesión';
exception
  when insufficient_privilege then null;
end $$;

-- anon no puede ejecutar las funciones ni leer la bitácora.
reset role;
set local role anon;
do $$
begin
  perform public.sesion_usuario_en_unidad('900000301', now());
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

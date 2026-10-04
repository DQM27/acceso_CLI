-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre 20261004130000_visitas_ven_otras_unidades:
-- - Un equipo de la unidad A registra una visita; uno de la unidad B no la
--   lee (RLS), pero `visita_activa_de_visitante` le dice que está en A.
-- - `visitantes_activos_en_otras_unidades`: B lo ve; el equipo que lo
--   registró no se ve a sí mismo; el otro equipo de A lo ve como
--   "(otro equipo de esta unidad)".
-- - El índice único rechaza la misma cédula abierta dos veces (en la misma
--   unidad o en otra) y deja registrar de nuevo tras la salida.
-- - Una sesión sin equipo no consulta.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a851', 'Unidad visitas A'),
  ('aaaaaaaa-0000-0000-0000-00000000a852', 'Unidad visitas B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c851', 'aaaaaaaa-0000-0000-0000-00000000a851', 'pc',     'PC A',      '{"kty":"EC"}', 'huella-vis-a'),
  ('cccccccc-0000-0000-0000-00000000c852', 'aaaaaaaa-0000-0000-0000-00000000a851', 'mobile', 'Celular A', '{"kty":"EC"}', 'huella-vis-a2'),
  ('cccccccc-0000-0000-0000-00000000c853', 'aaaaaaaa-0000-0000-0000-00000000a852', 'pc',     'PC B',      '{"kty":"EC"}', 'huella-vis-b');

insert into public.anfitriones (correo, nombre) values ('anfitrion-vis@example.com', 'ANFITRION PRUEBA');
insert into public.citas (id, anfitrion_correo, fecha_desde, fecha_hasta) values
  ('bbbbbbbb-0000-0000-0000-00000000b851', 'anfitrion-vis@example.com', current_date, current_date);
insert into public.cita_visitantes (id, cita_id, cedula, nombre) values
  ('bbbbbbbb-0000-0000-0000-00000000b861', 'bbbbbbbb-0000-0000-0000-00000000b851', '900000851', 'VISITANTE PRUEBA');

create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text, p_tipo text default 'mobile') returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella, 'tipo', p_tipo)::text, true)
$$;

set local role authenticated;

do $$
declare
  v_sitio uuid;
  v_nombre text;
  v_n int;
  v_rechazado boolean;
begin
  -- 1. La PC de A registra la visita.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c851', 'aaaaaaaa-0000-0000-0000-00000000a851', 'huella-vis-a');
  insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, hora_entrada, usuario_entrada_nombre)
  values ('eeeeeeee-0000-0000-0000-00000000e851', 'aaaaaaaa-0000-0000-0000-00000000a851', 'cccccccc-0000-0000-0000-00000000c851',
          'bbbbbbbb-0000-0000-0000-00000000b861', '900000851', 'VISITANTE PRUEBA', now(), 'OPERADOR PRUEBA');

  -- 2. Desde B la tabla está cerrada (esto era lo que dejaba ciego al
  --    núcleo), pero la función lo ve.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c853', 'aaaaaaaa-0000-0000-0000-00000000a852', 'huella-vis-b');
  if exists (select 1 from public.movimientos_visita where visitante_cedula = '900000851') then
    raise exception 'V2a: un equipo de otra unidad no debería leer las visitas de A';
  end if;
  select sitio_id, sitio_nombre into v_sitio, v_nombre from public.visita_activa_de_visitante(' 900000851 ');
  if v_sitio is distinct from 'aaaaaaaa-0000-0000-0000-00000000a851' or v_nombre <> 'Unidad visitas A' then
    raise exception 'V2b: la verificación debería ver la visita en A, dio % / %', v_sitio, v_nombre;
  end if;
  select count(*) into v_n from public.visita_activa_de_visitante('000000000');
  if v_n <> 0 then raise exception 'V2c: una cédula sin visita debería dar vacío'; end if;

  -- 3. Aviso posterior a sincronizar.
  select count(*) into v_n from public.visitantes_activos_en_otras_unidades(array['900000851', '000000000']);
  if v_n <> 1 then raise exception 'V3a: B debería ver 1 visita en otra unidad, vio %', v_n; end if;
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c851', 'aaaaaaaa-0000-0000-0000-00000000a851', 'huella-vis-a');
  select count(*) into v_n from public.visitantes_activos_en_otras_unidades(array['900000851']);
  if v_n <> 0 then raise exception 'V3b: el equipo que la registró no debería verse a sí mismo'; end if;
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c852', 'aaaaaaaa-0000-0000-0000-00000000a851', 'huella-vis-a2');
  select count(*), max(sitio_nombre) into v_n, v_nombre from public.visitantes_activos_en_otras_unidades(array['900000851']);
  if v_n <> 1 or v_nombre <> 'Unidad visitas A (otro equipo de esta unidad)' then
    raise exception 'V3c: el otro equipo de A debería ver el duplicado, vio % / %', v_n, v_nombre;
  end if;

  -- 4. El índice único: ni el otro equipo de A ni B pueden abrir otra.
  v_rechazado := false;
  begin
    insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a851', 'cccccccc-0000-0000-0000-00000000c852',
            'bbbbbbbb-0000-0000-0000-00000000b861', '900000851', 'VISITANTE PRUEBA', now(), 'OPERADOR PRUEBA');
  exception when unique_violation then v_rechazado := true;
  end;
  if not v_rechazado then raise exception 'V4a: la misma cédula no puede quedar abierta dos veces en A'; end if;

  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c853', 'aaaaaaaa-0000-0000-0000-00000000a852', 'huella-vis-b');
  v_rechazado := false;
  begin
    insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a852', 'cccccccc-0000-0000-0000-00000000c853',
            'bbbbbbbb-0000-0000-0000-00000000b861', '900000851', 'VISITANTE PRUEBA', now(), 'OPERADOR PRUEBA');
  exception when unique_violation then v_rechazado := true;
  end;
  if not v_rechazado then raise exception 'V4b: la misma cédula no puede quedar abierta en dos unidades'; end if;

  -- 5. Tras la salida en A, B ya puede registrarla.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c851', 'aaaaaaaa-0000-0000-0000-00000000a851', 'huella-vis-a');
  update public.movimientos_visita
     set hora_salida = now(), dispositivo_salida_id = 'cccccccc-0000-0000-0000-00000000c851', usuario_salida_nombre = 'OPERADOR PRUEBA'
   where id = 'eeeeeeee-0000-0000-0000-00000000e851';
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c853', 'aaaaaaaa-0000-0000-0000-00000000a852', 'huella-vis-b');
  select count(*) into v_n from public.visita_activa_de_visitante('900000851');
  if v_n <> 0 then raise exception 'V5a: después de la salida no debería quedar activa'; end if;
  insert into public.movimientos_visita (id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a852', 'cccccccc-0000-0000-0000-00000000c853',
          'bbbbbbbb-0000-0000-0000-00000000b861', '900000851', 'VISITANTE PRUEBA', now(), 'OPERADOR PRUEBA');
end $$;

-- Una sesión sin equipo (ni administrador) no consulta.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', gen_random_uuid(), 'email', 'alguien@example.com')::text, true);
do $$
begin
  perform public.visita_activa_de_visitante('900000851');
  raise exception 'V6: una sesión sin equipo no debería poder consultar';
exception when insufficient_privilege then null;
end $$;

rollback;

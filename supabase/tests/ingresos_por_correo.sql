-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre 20261003210000_ingresos_por_correo:
-- - Un equipo de la unidad A registra un ingreso por correo; uno de la
--   unidad B no lo lee (RLS), pero `ingreso_correo_activo` le dice que está
--   adentro en A, y `correos_activos_en_otras_unidades` lo reporta sólo a B.
-- - Los índices únicos rechazan la misma cédula adentro dos veces (misma u
--   otra unidad) y el mismo gafete dos veces en la unidad; tras la salida se
--   puede registrar de nuevo.
-- - La entrada es inmutable, la salida se registra una sola vez, el motivo
--   no puede quedar vacío y un visor no registra.
-- - El panel "adentro ahora" muestra la fila como POR_CORREO.
-- - Una sesión sin equipo y anon no consultan.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a701', 'Unidad correo A'),
  ('aaaaaaaa-0000-0000-0000-00000000a702', 'Unidad correo B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'pc',     'PC A',      '{"kty":"EC"}', 'huella-correo-a'),
  ('cccccccc-0000-0000-0000-00000000c702', 'aaaaaaaa-0000-0000-0000-00000000a702', 'mobile', 'Celular B', '{"kty":"EC"}', 'huella-correo-b'),
  ('cccccccc-0000-0000-0000-00000000c703', 'aaaaaaaa-0000-0000-0000-00000000a701', 'mobile', 'Celular A', '{"kty":"EC"}', 'huella-correo-a2');

-- Hace de equipo: JWT de dispositivo (sub, sitio_id, huella y tipo).
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
begin
  -- 1. La PC de la unidad A registra el ingreso por correo.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'huella-correo-a');
  insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values ('eeeeeeee-0000-0000-0000-00000000e701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'cccccccc-0000-0000-0000-00000000c701',
          '900000701', 'VISITA PRUEBA CORREO', 'ENTREVISTA RH', 701, now(), 'OPERADOR PRUEBA');
  select count(*) into v_n from public.panel_adentro_ahora where id = 'eeeeeeee-0000-0000-0000-00000000e701' and tipo = 'POR_CORREO' and empresa_nombre = 'ENTREVISTA RH';
  if v_n <> 1 then raise exception 'C1a: el panel debería mostrar el ingreso como POR_CORREO'; end if;

  -- 2. Desde la unidad B la tabla está cerrada, pero la verificación lo ve.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c702', 'aaaaaaaa-0000-0000-0000-00000000a702', 'huella-correo-b');
  if exists (select 1 from public.ingresos_correo where cedula = '900000701') then
    raise exception 'C2a: un equipo de otra unidad no debería leer los ingresos por correo de A';
  end if;
  select sitio_id, sitio_nombre into v_sitio, v_nombre from public.ingreso_correo_activo(' 900000701 ');
  if v_sitio is distinct from 'aaaaaaaa-0000-0000-0000-00000000a701' or v_nombre <> 'Unidad correo A' then
    raise exception 'C2b: la verificación debería ver el ingreso en A, dio % / %', v_sitio, v_nombre;
  end if;
  select count(*) into v_n from public.ingreso_correo_activo('000000000');
  if v_n <> 0 then raise exception 'C2c: una cédula sin ingreso debería dar vacío'; end if;

  -- 3. Aviso posterior a sincronizar: B lo ve; A no ve el propio.
  select count(*) into v_n from public.correos_activos_en_otras_unidades(array['900000701', '000000000']);
  if v_n <> 1 then raise exception 'C3a: B debería ver 1 ingreso en otra unidad, vio %', v_n; end if;
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c703', 'aaaaaaaa-0000-0000-0000-00000000a701', 'huella-correo-a2');
  select count(*) into v_n from public.correos_activos_en_otras_unidades(array['900000701']);
  if v_n <> 0 then raise exception 'C3b: A no debería ver su propio ingreso como de otra unidad'; end if;

  -- 4. Misma cédula adentro dos veces: en la misma unidad...
  begin
    insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a701', 'cccccccc-0000-0000-0000-00000000c703',
            '900000701', 'VISITA PRUEBA CORREO', 'ENTREVISTA RH', 702, now(), 'OPERADOR PRUEBA');
    raise exception 'C4a: se aceptó un segundo ingreso abierto de la misma cédula en la misma unidad';
  exception
    when unique_violation then null;
  end;

  -- 5. ...el mismo gafete dos veces en la unidad...
  begin
    insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a701', 'cccccccc-0000-0000-0000-00000000c703',
            '900000702', 'OTRA VISITA', 'ENTREVISTA RH', 701, now(), 'OPERADOR PRUEBA');
    raise exception 'C5a: se aceptó el mismo gafete en uso dos veces en la unidad';
  exception
    when unique_violation then null;
  end;

  -- 6. ...y la misma cédula en otra unidad.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c702', 'aaaaaaaa-0000-0000-0000-00000000a702', 'huella-correo-b');
  begin
    insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a702', 'cccccccc-0000-0000-0000-00000000c702',
            '900000701', 'VISITA PRUEBA CORREO', 'ENTREVISTA RH', 701, now(), 'OPERADOR PRUEBA');
    raise exception 'C6a: se aceptó un segundo ingreso abierto de la misma cédula en otra unidad';
  exception
    when unique_violation then null;
  end;

  -- 7. Motivo vacío: rechazado.
  begin
    insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a702', 'cccccccc-0000-0000-0000-00000000c702',
            '900000703', 'SIN MOTIVO', '   ', 703, now(), 'OPERADOR PRUEBA');
    raise exception 'C7a: se aceptó un ingreso por correo sin motivo';
  exception
    when check_violation then null;
  end;

  -- 8. Un visor no registra.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c702', 'aaaaaaaa-0000-0000-0000-00000000a702', 'huella-correo-b', 'visor');
  begin
    insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a702', 'cccccccc-0000-0000-0000-00000000c702',
            '900000704', 'VISITA DE VISOR', 'ENTREVISTA RH', 704, now(), 'OPERADOR PRUEBA');
    raise exception 'C8a: un visor pudo registrar un ingreso por correo';
  exception
    when insufficient_privilege then null;
  end;

  -- 9. La entrada es inmutable.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c701', 'aaaaaaaa-0000-0000-0000-00000000a701', 'huella-correo-a');
  begin
    update public.ingresos_correo set motivo = 'OTRO MOTIVO' where id = 'eeeeeeee-0000-0000-0000-00000000e701';
    raise exception 'C9a: se pudo cambiar el motivo de la entrada';
  exception
    when raise_exception then
      if sqlerrm not like '%inmutables%' then raise; end if;
  end;

  -- 10. Salida en A; una segunda salida se rechaza.
  update public.ingresos_correo set hora_salida = now(), dispositivo_salida_id = 'cccccccc-0000-0000-0000-00000000c701',
         usuario_salida_nombre = 'OPERADOR PRUEBA'
   where id = 'eeeeeeee-0000-0000-0000-00000000e701';
  begin
    update public.ingresos_correo set hora_salida = now() + interval '1 minute'
     where id = 'eeeeeeee-0000-0000-0000-00000000e701';
    raise exception 'C10a: se registró la salida dos veces';
  exception
    when raise_exception then
      if sqlerrm not like '%una vez%' then raise; end if;
  end;

  -- 11. Tras la salida, B ya puede registrarlo (con el mismo gafete en su unidad).
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c702', 'aaaaaaaa-0000-0000-0000-00000000a702', 'huella-correo-b');
  select count(*) into v_n from public.ingreso_correo_activo('900000701');
  if v_n <> 0 then raise exception 'C11a: después de la salida no debería quedar activo'; end if;
  insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a702', 'cccccccc-0000-0000-0000-00000000c702',
          '900000701', 'VISITA PRUEBA CORREO', 'ENTREVISTA RH', 701, now(), 'OPERADOR PRUEBA');
end $$;

-- Una sesión sin equipo (ni administrador) no consulta.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', gen_random_uuid(), 'email', 'alguien@example.com')::text, true);
do $$
begin
  perform 1 from public.ingreso_correo_activo('900000701');
  raise exception 'Una sesión sin equipo pudo consultar ingresos por correo';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.correos_activos_en_otras_unidades(array['900000701']);
  raise exception 'Una sesión sin equipo pudo consultar ingresos por correo de otras unidades';
exception
  when insufficient_privilege then null;
end $$;

-- anon tampoco: ni la función ni la tabla.
reset role;
set local role anon;
do $$
begin
  perform 1 from public.ingreso_correo_activo('900000701');
  raise exception 'anon pudo ejecutar ingreso_correo_activo';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.ingresos_correo;
  raise exception 'anon pudo leer ingresos_correo';
exception
  when insufficient_privilege then null;
end $$;

rollback;

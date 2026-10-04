-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre la regla "un contratista no puede tener dos ingresos abiertos, ni en
-- la misma unidad ni en otra" (20261003170000_ingreso_unico_entre_unidades):
-- un equipo de la unidad B ve, con `ingreso_activo_de_contratista`, que la
-- persona está adentro en la unidad A (antes la RLS se lo ocultaba); el
-- índice único rechaza el segundo ingreso abierto en cualquier unidad, pero
-- deja registrar de nuevo después de la salida; y
-- `contratistas_activos_en_otras_unidades` reporta lo de otras unidades y,
-- desde 20261004120000, lo abierto por el otro equipo de la misma unidad
-- (nunca lo abierto por el equipo que pregunta).
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a501', 'Unidad ingreso A'),
  ('aaaaaaaa-0000-0000-0000-00000000a502', 'Unidad ingreso B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c501', 'aaaaaaaa-0000-0000-0000-00000000a501', 'pc',     'PC A',      '{"kty":"EC"}', 'huella-ing-a'),
  ('cccccccc-0000-0000-0000-00000000c502', 'aaaaaaaa-0000-0000-0000-00000000a502', 'mobile', 'Celular B', '{"kty":"EC"}', 'huella-ing-b'),
  ('cccccccc-0000-0000-0000-00000000c503', 'aaaaaaaa-0000-0000-0000-00000000a501', 'mobile', 'Celular A', '{"kty":"EC"}', 'huella-ing-a2');

insert into public.contratistas (id, nombre) values
  ('dddddddd-0000-0000-0000-00000000d501', 'CONTRATISTA PRUEBA UNICO');

-- Hace de equipo: JWT de dispositivo (sub, sitio_id, huella y tipo; crear
-- ingresos exige un tipo distinto de 'visor').
create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text) returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella, 'tipo', 'mobile')::text, true)
$$;

set local role authenticated;

do $$
declare
  v_sitio uuid;
  v_nombre text;
  v_n int;
begin
  -- 1. La PC de la unidad A registra el ingreso.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c501', 'aaaaaaaa-0000-0000-0000-00000000a501', 'huella-ing-a');
  insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre, contratista_cedula, hora_entrada)
  values ('eeeeeeee-0000-0000-0000-00000000e501', 'aaaaaaaa-0000-0000-0000-00000000a501', 'cccccccc-0000-0000-0000-00000000c501',
          'dddddddd-0000-0000-0000-00000000d501', 'CONTRATISTA PRUEBA UNICO', '900000501', now());

  -- 2. Desde la unidad B, la tabla sigue cerrada (RLS)...
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c502', 'aaaaaaaa-0000-0000-0000-00000000a502', 'huella-ing-b');
  if exists (select 1 from public.ingresos where contratista_cedula = '900000501') then
    raise exception '2a: un equipo de otra unidad no debería leer los ingresos de A';
  end if;
  -- ...pero la verificación sí ve que está adentro, y dónde.
  select sitio_id, sitio_nombre into v_sitio, v_nombre from public.ingreso_activo_de_contratista(' 900000501 ');
  if v_sitio is distinct from 'aaaaaaaa-0000-0000-0000-00000000a501' or v_nombre <> 'Unidad ingreso A' then
    raise exception '2b: la verificación debería ver el ingreso en A, dio % / %', v_sitio, v_nombre;
  end if;
  select count(*) into v_n from public.ingreso_activo_de_contratista('000000000');
  if v_n <> 0 then raise exception '2c: una cédula sin ingreso debería dar vacío'; end if;

  -- 3. El aviso posterior a sincronizar, desde B, ve el de A.
  select count(*) into v_n from public.contratistas_activos_en_otras_unidades(array['900000501', '000000000']);
  if v_n <> 1 then raise exception '3a: B debería ver 1 activo en otra unidad, vio %', v_n; end if;
  --    Desde A no reporta su propio ingreso.
  -- El equipo que lo registró no se avisa a sí mismo.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c501', 'aaaaaaaa-0000-0000-0000-00000000a501', 'huella-ing-a');
  select count(*) into v_n from public.contratistas_activos_en_otras_unidades(array['900000501']);
  if v_n <> 0 then raise exception '3b: el equipo que lo registró no debería verse a sí mismo'; end if;
  -- El OTRO equipo de la misma unidad sí se avisa (20261004120000): si lo
  -- pregunta es porque también lo tiene abierto localmente, y el índice
  -- único le rechazó (o le va a rechazar) ese ingreso.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c503', 'aaaaaaaa-0000-0000-0000-00000000a501', 'huella-ing-a2');
  select count(*), max(sitio_nombre) into v_n, v_nombre from public.contratistas_activos_en_otras_unidades(array['900000501']);
  if v_n <> 1 or v_nombre <> 'Unidad ingreso A (otro equipo de esta unidad)' then
    raise exception '3c: el otro equipo de la unidad debería ver el duplicado, vio % / %', v_n, v_nombre;
  end if;

  -- 4. Segundo ingreso abierto de la misma cédula, en la misma unidad (otro
  --    equipo, p. ej. registrado sin conexión): rechazado por el índice.
  begin
    insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre, contratista_cedula, hora_entrada)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a501', 'cccccccc-0000-0000-0000-00000000c503',
            'dddddddd-0000-0000-0000-00000000d501', 'CONTRATISTA PRUEBA UNICO', '900000501', now());
    raise exception '4a: se aceptó un segundo ingreso abierto en la misma unidad';
  exception
    when unique_violation then null;
  end;

  -- 5. ...y en otra unidad.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c502', 'aaaaaaaa-0000-0000-0000-00000000a502', 'huella-ing-b');
  begin
    insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre, contratista_cedula, hora_entrada)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a502', 'cccccccc-0000-0000-0000-00000000c502',
            'dddddddd-0000-0000-0000-00000000d501', 'CONTRATISTA PRUEBA UNICO', '900000501', now());
    raise exception '5a: se aceptó un segundo ingreso abierto en otra unidad';
  exception
    when unique_violation then null;
  end;

  -- 6. Tras la salida en A, B ya puede registrarlo.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c501', 'aaaaaaaa-0000-0000-0000-00000000a501', 'huella-ing-a');
  update public.ingresos set hora_salida = now(), usuario_salida_nombre = 'OPERADOR PRUEBA' where id = 'eeeeeeee-0000-0000-0000-00000000e501';
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c502', 'aaaaaaaa-0000-0000-0000-00000000a502', 'huella-ing-b');
  select count(*) into v_n from public.ingreso_activo_de_contratista('900000501');
  if v_n <> 0 then raise exception '6a: después de la salida no debería quedar activo'; end if;
  insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre, contratista_cedula, hora_entrada)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a502', 'cccccccc-0000-0000-0000-00000000c502',
          'dddddddd-0000-0000-0000-00000000d501', 'CONTRATISTA PRUEBA UNICO', '900000501', now());
end $$;

-- Una sesión sin equipo (ni administrador) no consulta.
select set_config('request.jwt.claims', json_build_object(
  'role', 'authenticated', 'sub', gen_random_uuid(), 'email', 'alguien@example.com')::text, true);
do $$
begin
  perform 1 from public.ingreso_activo_de_contratista('900000501');
  raise exception 'Una sesión sin equipo pudo consultar ingresos activos';
exception
  when insufficient_privilege then null;
end $$;
do $$
begin
  perform 1 from public.contratistas_activos_en_otras_unidades(array['900000501']);
  raise exception 'Una sesión sin equipo pudo consultar activos en otras unidades';
exception
  when insufficient_privilege then null;
end $$;

-- anon tampoco.
reset role;
set local role anon;
do $$
begin
  perform 1 from public.ingreso_activo_de_contratista('900000501');
  raise exception 'anon pudo ejecutar ingreso_activo_de_contratista';
exception
  when insufficient_privilege then null;
end $$;

rollback;

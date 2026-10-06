-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre 20261004150000_persona_adentro_por_una_sola_via:
-- - Con la persona adentro como contratista en la unidad A, un ingreso de
--   proveedor (unidad B) o por correo (unidad A, cédula escrita con guiones)
--   con la misma cédula se rechaza: 23505 nombrando el índice de cédula
--   activa de la propia tabla, que es lo que reconoce la cola del núcleo.
-- - `persona_adentro_por_otra_via` responde por qué vía y dónde; para la
--   propia vía no responde nada; una vía desconocida es un error.
-- - Los avisos posteriores a sincronizar (`*_activos_en_otras_unidades`)
--   reportan a quien está adentro por otra vía, con "(como contratista)".
-- - Tras la salida, la persona puede entrar por otra vía.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-00000000a871', 'Unidad vía A'),
  ('aaaaaaaa-0000-0000-0000-00000000a872', 'Unidad vía B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella) values
  ('cccccccc-0000-0000-0000-00000000c871', 'aaaaaaaa-0000-0000-0000-00000000a871', 'pc',     'PC A',      '{"kty":"EC"}', 'huella-via-a'),
  ('cccccccc-0000-0000-0000-00000000c872', 'aaaaaaaa-0000-0000-0000-00000000a872', 'mobile', 'Celular B', '{"kty":"EC"}', 'huella-via-b');

insert into public.contratistas (id, dispositivo_origen_id, nombre) values
  ('dddddddd-0000-0000-0000-00000000d871', 'cccccccc-0000-0000-0000-00000000c871', 'PERSONA PRUEBA VIA');

create or replace function pg_temp.como_equipo(p_id uuid, p_sitio uuid, p_huella text, p_tipo text default 'mobile') returns void
language sql as $$
  select set_config('request.jwt.claims', json_build_object(
    'role', 'authenticated', 'sub', p_id, 'sitio_id', p_sitio, 'huella', p_huella, 'tipo', p_tipo)::text, true)
$$;

set local role authenticated;

do $$
declare
  v_via text;
  v_nombre text;
  v_n int;
  v_rechazado boolean;
  v_mensaje text;
begin
  -- 1. La PC de A registra a la persona como contratista.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c871', 'aaaaaaaa-0000-0000-0000-00000000a871', 'huella-via-a');
  insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_nombre, contratista_cedula, hora_entrada)
  values ('eeeeeeee-0000-0000-0000-00000000e871', 'aaaaaaaa-0000-0000-0000-00000000a871', 'cccccccc-0000-0000-0000-00000000c871',
          'dddddddd-0000-0000-0000-00000000d871', 'PERSONA PRUEBA VIA', '900000871', now());

  -- 2. B intenta registrarla como proveedor: se rechaza.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c872', 'aaaaaaaa-0000-0000-0000-00000000a872', 'huella-via-b');
  v_rechazado := false;
  begin
    insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a872', 'cccccccc-0000-0000-0000-00000000c872',
            '900000871', 'PERSONA PRUEBA VIA', 'EMPRESA PRUEBA', 871, now(), 'OPERADOR PRUEBA');
  exception when unique_violation then
    v_rechazado := true;
    get stacked diagnostics v_mensaje = message_text;
  end;
  if not v_rechazado then raise exception 'P2a: adentro como contratista, no puede entrar como proveedor'; end if;
  if v_mensaje not like '%como contratista%' or v_mensaje not like '%ingresos_proveedor_cedula_activa_idx%' then
    raise exception 'P2b: el mensaje debe decir la vía y nombrar el índice de proveedores, dio: %', v_mensaje;
  end if;

  -- 3. Por correo, en A y con la cédula escrita con guiones: también se rechaza.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c871', 'aaaaaaaa-0000-0000-0000-00000000a871', 'huella-via-a');
  v_rechazado := false;
  begin
    insert into public.ingresos_correo (id, sitio_id, dispositivo_entrada_id, cedula, nombre, motivo, gafete_numero, hora_entrada, usuario_entrada_nombre)
    values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a871', 'cccccccc-0000-0000-0000-00000000c871',
            '9-0000-0871', 'PERSONA PRUEBA VIA', 'ENTREVISTA RH', 871, now(), 'OPERADOR PRUEBA');
  exception when unique_violation then
    v_rechazado := true;
    get stacked diagnostics v_mensaje = message_text;
  end;
  if not v_rechazado or v_mensaje not like '%ingresos_correo_cedula_activa_idx%' then
    raise exception 'P3: adentro como contratista, no puede entrar por correo (aunque la cédula venga con guiones), dio: %', v_mensaje;
  end if;

  -- 4. Verificación en vivo.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c872', 'aaaaaaaa-0000-0000-0000-00000000a872', 'huella-via-b');
  select via, sitio_nombre into v_via, v_nombre from public.persona_adentro_por_otra_via('9-0000-0871', 'PROVEEDOR');
  if v_via is distinct from 'CONTRATISTA' or v_nombre <> 'Unidad vía A' then
    raise exception 'P4a: debería responder CONTRATISTA en Unidad vía A, dio % / %', v_via, v_nombre;
  end if;
  select count(*) into v_n from public.persona_adentro_por_otra_via('900000871', 'CONTRATISTA');
  if v_n <> 0 then raise exception 'P4b: la propia vía no es "otra vía"'; end if;
  begin
    perform public.persona_adentro_por_otra_via('900000871', 'VISITA');
    raise exception 'P4c: una vía desconocida debería dar error';
  exception when invalid_parameter_value then null;
  end;

  -- 5. Aviso posterior a sincronizar: B (que tendría a la persona abierta
  --    como proveedor localmente) ve el ingreso de contratista de A.
  select count(*), max(sitio_nombre) into v_n, v_nombre
    from public.proveedores_activos_en_otras_unidades(array['900000871']);
  if v_n <> 1 or v_nombre <> 'Unidad vía A (como contratista)' then
    raise exception 'P5a: el aviso debería nombrar la otra vía, vio % / %', v_n, v_nombre;
  end if;
  select count(*) into v_n from public.correos_activos_en_otras_unidades(array['9-0000-0871']);
  if v_n <> 1 then raise exception 'P5b: el aviso de correo también debería ver la otra vía (cédula con guiones)'; end if;
  -- El equipo que lo registró como contratista no se avisa a sí mismo.
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c871', 'aaaaaaaa-0000-0000-0000-00000000a871', 'huella-via-a');
  select count(*) into v_n from public.contratistas_activos_en_otras_unidades(array['900000871']);
  if v_n <> 0 then raise exception 'P5c: la misma vía y el mismo equipo no es un conflicto'; end if;

  -- 6. Tras la salida, puede entrar como proveedor.
  update public.ingresos set hora_salida = now(), usuario_salida_nombre = 'OPERADOR PRUEBA'
   where id = 'eeeeeeee-0000-0000-0000-00000000e871';
  perform pg_temp.como_equipo('cccccccc-0000-0000-0000-00000000c872', 'aaaaaaaa-0000-0000-0000-00000000a872', 'huella-via-b');
  insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a872', 'cccccccc-0000-0000-0000-00000000c872',
          '900000871', 'PERSONA PRUEBA VIA', 'EMPRESA PRUEBA', 871, now(), 'OPERADOR PRUEBA');
end $$;

rollback;

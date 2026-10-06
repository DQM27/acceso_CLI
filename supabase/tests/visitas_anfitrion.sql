-- Web de visitas: escrituras del anfitrión por funciones (migración
-- 20261005130000_visitas_web_anfitrion). Ejecutar en una sola sesión; todo
-- se revierte.
begin;

insert into public.sitios (id, nombre) values
  ('aaaaaaaa-0000-0000-0000-0000000000b1', 'Unidad visitas A'),
  ('aaaaaaaa-0000-0000-0000-0000000000b2', 'Unidad visitas B');
insert into public.dispositivos (id, sitio_id, tipo, etiqueta)
values ('dddddddd-0000-0000-0000-0000000000b1', 'aaaaaaaa-0000-0000-0000-0000000000b1', 'pc', 'PC visitas');
insert into public.anfitriones (correo, nombre, activo) values
  ('anfitriona@example.com', 'Ana', true),
  ('otro@example.com', 'Otro', true),
  ('inactiva@example.com', 'Inactiva', false);

create temp table ids (nombre text primary key, id uuid);
grant all on ids to authenticated;
insert into ids values
  ('cita', gen_random_uuid()), ('editada', gen_random_uuid()), ('otra', gen_random_uuid());

set local role authenticated;
select set_config('request.jwt.claims', json_build_object('role','authenticated','email','anfitriona@example.com')::text, true);

do $$
declare
  v_cita uuid := (select id from ids where nombre = 'cita');
  v_mensaje text;
begin
  -- Crear: la cédula queda en forma única.
  perform public.crear_cita_anfitrion(
    v_cita, (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date + 1,
    ' Reunión ', array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[],
    '[{"cedula":"01-0847-0293","nombre":"Carlos Mora","empresa":"ACME"}]'::jsonb, '09:00');
  if (select cedula from public.cita_visitantes where cita_id = v_cita) <> '108470293' then
    raise exception 'la cédula no quedó en forma única';
  end if;
  -- Reintento idéntico: devuelve el mismo id, no duplica.
  if public.crear_cita_anfitrion(
    v_cita, (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date + 1,
    ' Reunión ', array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[],
    '[{"cedula":"01-0847-0293","nombre":"Carlos Mora","empresa":"ACME"}]'::jsonb, '09:00') <> v_cita then
    raise exception 'el reintento no devolvió la misma cita';
  end if;

  -- La misma persona escrita de dos formas en el grupo: duplicada.
  begin
    perform public.crear_cita_anfitrion(
      gen_random_uuid(), (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date,
      null, array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[],
      '[{"cedula":"108470293","nombre":"Ana"},{"cedula":"1-0847-0293","nombre":"Bea"}]'::jsonb, null);
    raise exception 'aceptó la misma cédula dos veces';
  exception when raise_exception then
    get stacked diagnostics v_mensaje = message_text;
    if v_mensaje not like 'Cédula duplicada%' then raise; end if;
  end;

  -- Documento con las reglas del núcleo (migración
  -- 20261005133000_guardar_cita_reglas_del_nucleo): lo que la portería no
  -- reconocería no se agenda.
  declare
    v_doc text;
  begin
    foreach v_doc in array array[repeat('A', 21), 'AB', '12#45', '   '] loop
      begin
        perform public.crear_cita_anfitrion(
          gen_random_uuid(), (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date,
          null, array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[],
          jsonb_build_array(jsonb_build_object('cedula', v_doc, 'nombre', 'Ana Prueba')), null);
        raise exception 'aceptó el documento %', v_doc;
      exception when raise_exception then
        get stacked diagnostics v_mensaje = message_text;
        if v_mensaje not like 'Documento de visitante inválido%' then raise; end if;
      end;
    end loop;
  end;
  -- Un nombre de una letra: rechazado.
  begin
    perform public.crear_cita_anfitrion(
      gen_random_uuid(), (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date,
      null, array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[],
      '[{"cedula":"400000004","nombre":" A "}]'::jsonb, null);
    raise exception 'aceptó un nombre de una letra';
  exception when raise_exception then
    get stacked diagnostics v_mensaje = message_text;
    if v_mensaje <> 'Nombre de visitante inválido' then raise; end if;
  end;
  -- 20 caracteres sí; nombre con espacios colapsados y placa en mayúsculas.
  declare
    v_otra uuid := gen_random_uuid();
    v_fila record;
  begin
    perform public.crear_cita_anfitrion(
      v_otra, (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date,
      null, array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[],
      jsonb_build_array(jsonb_build_object('cedula', repeat('b', 20), 'nombre', '  Ana   María  Solís ', 'placa_vehiculo', ' abc123 ')), null);
    select cedula, nombre, placa_vehiculo into v_fila from public.cita_visitantes where cita_id = v_otra;
    if v_fila.cedula <> repeat('B', 20) or v_fila.nombre <> 'Ana María Solís' or v_fila.placa_vehiculo <> 'ABC123' then
      raise exception 'no normalizó como el núcleo: %', row_to_json(v_fila);
    end if;
  end;

  -- Fecha pasada: rechazada.
  begin
    perform public.crear_cita_anfitrion(
      gen_random_uuid(), (now() at time zone 'America/Costa_Rica')::date - 1, (now() at time zone 'America/Costa_Rica')::date,
      null, array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[], '[{"cedula":"100000001","nombre":"Ana Prueba"}]'::jsonb, null);
    raise exception 'aceptó una fecha pasada';
  exception when raise_exception then
    get stacked diagnostics v_mensaje = message_text;
    if v_mensaje <> 'La fecha de inicio no puede ser anterior a hoy' then raise; end if;
  end;

  -- Ya no se puede escribir directo en las tablas (antes la RLS lo dejaba).
  begin
    insert into public.citas (id, anfitrion_correo, fecha_desde, fecha_hasta, estado)
    values (gen_random_uuid(), 'anfitriona@example.com', '2020-01-01', '2020-01-01', 'VIGENTE');
    raise exception 'insertó una cita directo';
  exception when insufficient_privilege then null;
  end;
  update public.citas set motivo = 'cambiado a mano' where id = v_cita;
  if (select motivo from public.citas where id = v_cita) = 'cambiado a mano' then
    raise exception 'actualizó una cita directo';
  end if;
  begin
    insert into public.cita_visitantes (cita_id, cedula, nombre) values (v_cita, '2', 'Colado');
    raise exception 'agregó un visitante directo';
  exception when insufficient_privilege then null;
  end;

  -- Visitantes anteriores: aparece, buscando por nombre o por cédula con guiones.
  if not exists (select 1 from public.visitantes_anteriores('carlos') where cedula = '108470293') then
    raise exception 'visitantes_anteriores no encontró por nombre';
  end if;
  if not exists (select 1 from public.visitantes_anteriores('1-0847') where nombre = 'Carlos Mora') then
    raise exception 'visitantes_anteriores no encontró por cédula';
  end if;
end $$;

-- Editar: cancela la vieja y crea la nueva; la nueva no tiene a Carlos.
do $$
declare
  v_cita uuid := (select id from ids where nombre = 'cita');
  v_editada uuid := (select id from ids where nombre = 'editada');
begin
  perform public.editar_cita_anfitrion(
    v_cita, v_editada, (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date + 2,
    'Reunión movida', array['aaaaaaaa-0000-0000-0000-0000000000b2']::uuid[],
    '[{"cedula":"200000002","nombre":"Laura Vargas"}]'::jsonb, '10:30');
  if (select estado from public.citas where id = v_cita) <> 'CANCELADA' then
    raise exception 'la cita vieja no quedó cancelada';
  end if;
  if (select estado from public.citas where id = v_editada) <> 'VIGENTE'
     or (select count(*) from public.cita_visitantes where cita_id = v_editada) <> 1 then
    raise exception 'la cita nueva no quedó bien';
  end if;
  -- Reintento de la misma edición: devuelve la nueva sin error.
  if public.editar_cita_anfitrion(
    v_cita, v_editada, (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date + 2,
    'Reunión movida', array['aaaaaaaa-0000-0000-0000-0000000000b2']::uuid[],
    '[{"cedula":"200000002","nombre":"Laura Vargas"}]'::jsonb, '10:30') <> v_editada then
    raise exception 'el reintento de la edición falló';
  end if;
  -- Una cancelada ya no se edita.
  begin
    perform public.editar_cita_anfitrion(
      v_cita, gen_random_uuid(), (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date,
      null, array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[], '[{"cedula":"300000003","nombre":"Xavier Ruiz"}]'::jsonb, null);
    raise exception 'editó una cita cancelada';
  exception when raise_exception then null;
  end;
end $$;

-- Una entrada registrada en la portería (la escribe un equipo).
reset role;
insert into public.movimientos_visita (
  id, sitio_id, dispositivo_entrada_id, cita_visitante_id, visitante_cedula, visitante_nombre,
  gafete_numero, hora_entrada, usuario_entrada_nombre)
select gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-0000000000b1', 'dddddddd-0000-0000-0000-0000000000b1',
       v.id, v.cedula, v.nombre, 7, now(), 'Guarda'
  from public.cita_visitantes v where v.cita_id = (select id from ids where nombre = 'editada');
set local role authenticated;

do $$
declare
  v_editada uuid := (select id from ids where nombre = 'editada');
  v_fila record;
begin
  -- La anfitriona ve que llegó, con el gafete y la unidad.
  select * into v_fila from public.estado_visitantes_de_mis_citas(array[v_editada]);
  if v_fila.gafete_numero is distinct from 7 or v_fila.sitio_nombre <> 'Unidad visitas A' or v_fila.hora_salida is not null then
    raise exception 'estado de llegada incorrecto: %', row_to_json(v_fila);
  end if;
  -- Con alguien adentro, ya no se edita...
  begin
    perform public.editar_cita_anfitrion(
      v_editada, gen_random_uuid(), (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date,
      null, array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[], '[{"cedula":"300000003","nombre":"Xavier Ruiz"}]'::jsonb, null);
    raise exception 'editó una cita con alguien adentro';
  exception when raise_exception then null;
  end;
  -- ...pero se puede cancelar, y cancelar dos veces no falla.
  perform public.cancelar_cita_anfitrion(v_editada);
  perform public.cancelar_cita_anfitrion(v_editada);
  if (select estado from public.citas where id = v_editada) <> 'CANCELADA' then
    raise exception 'no se canceló';
  end if;
end $$;

-- Otro anfitrión no ve las llegadas ajenas ni puede cancelar ni editar.
select set_config('request.jwt.claims', json_build_object('role','authenticated','email','otro@example.com')::text, true);
do $$
declare
  v_editada uuid := (select id from ids where nombre = 'editada');
begin
  if exists (select 1 from public.estado_visitantes_de_mis_citas(array[v_editada])) then
    raise exception 'otro anfitrión vio llegadas ajenas';
  end if;
  begin
    perform public.cancelar_cita_anfitrion(v_editada);
    raise exception 'otro anfitrión canceló una cita ajena';
  exception when no_data_found then null;
  end;
  if exists (select 1 from public.visitantes_anteriores(null)) then
    raise exception 'otro anfitrión vio visitantes ajenos';
  end if;
end $$;

-- Un anfitrión inactivo no agenda.
select set_config('request.jwt.claims', json_build_object('role','authenticated','email','inactiva@example.com')::text, true);
do $$
begin
  perform public.crear_cita_anfitrion(
    gen_random_uuid(), (now() at time zone 'America/Costa_Rica')::date, (now() at time zone 'America/Costa_Rica')::date,
    null, array['aaaaaaaa-0000-0000-0000-0000000000b1']::uuid[], '[{"cedula":"100000001","nombre":"Ana Prueba"}]'::jsonb, null);
  raise exception 'un anfitrión inactivo agendó';
exception when insufficient_privilege then null;
end $$;

rollback;

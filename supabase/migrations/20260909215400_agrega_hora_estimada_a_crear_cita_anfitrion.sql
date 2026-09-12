-- Agrega p_hora_estimada (opcional, default null) a crear_cita_anfitrion --
-- parámetro nuevo al final con default, misma función (CREATE OR REPLACE
-- no rompe las llamadas existentes de 6 argumentos: siguen resolviendo a
-- esta misma función con hora_estimada implícita en null). Se valida
-- gratis por el tipo `time` del parámetro -- un valor mal formado falla en
-- el cast antes de que el cuerpo de la función corra siquiera. Participa
-- del hash de idempotencia igual que motivo: un reintento con hora
-- distinta es contenido distinto.
create or replace function public.crear_cita_anfitrion(
  p_id uuid,
  p_fecha_desde date,
  p_fecha_hasta date,
  p_motivo text,
  p_sitios uuid[],
  p_visitantes jsonb,
  p_hora_estimada time default null
)
returns uuid
language plpgsql
security invoker
set search_path = public
as $$
declare
  v_correo text;
  v_hoy date;
  v_existente record;
  v_hash text;
  v_n_sitios int;
  v_n_visitantes int;
  v_visitante jsonb;
  v_cedula text;
  v_nombre text;
  v_empresa text;
  v_placa text;
  v_vistas text[] := '{}';
  v_motivo text;
begin
  v_correo := auth.email();
  if v_correo is null then
    raise exception 'No autenticado' using errcode = '28000';
  end if;
  if not exists (select 1 from anfitriones where correo = v_correo and activo) then
    raise exception 'Cuenta no autorizada para agendar visitas' using errcode = '42501';
  end if;

  if (
    select count(*) from citas
    where anfitrion_correo = v_correo and created_at > now() - interval '1 hour'
  ) >= 20 then
    raise exception 'Demasiadas citas creadas en la última hora, intente más tarde' using errcode = '54000';
  end if;

  v_motivo := nullif(btrim(p_motivo), '');
  if v_motivo is not null and (length(v_motivo) > 1000 or v_motivo ~ '[[:cntrl:]]') then
    raise exception 'Motivo inválido';
  end if;

  v_n_sitios := coalesce(array_length(p_sitios, 1), 0);
  if v_n_sitios < 1 or v_n_sitios > 100 then
    raise exception 'Debe indicar entre 1 y 100 sitios';
  end if;
  if v_n_sitios <> (select count(distinct s) from unnest(p_sitios) as s) then
    raise exception 'Sitios duplicados en la solicitud';
  end if;
  if exists (
    select 1 from unnest(p_sitios) as s
    where not exists (select 1 from sitios where id = s)
  ) then
    raise exception 'Uno de los sitios indicados no existe';
  end if;

  v_n_visitantes := jsonb_array_length(p_visitantes);
  if v_n_visitantes is null or v_n_visitantes < 1 or v_n_visitantes > 50 then
    raise exception 'Debe indicar entre 1 y 50 visitantes';
  end if;

  v_hash := md5(
    v_correo || '|' || p_fecha_desde::text || '|' || p_fecha_hasta::text || '|' ||
    coalesce(v_motivo, '') || '|' ||
    coalesce((select string_agg(s::text, ',' order by s) from unnest(p_sitios) as s), '') || '|' ||
    p_visitantes::text || '|' || coalesce(p_hora_estimada::text, '')
  );

  select * into v_existente from citas where id = p_id;
  if found then
    if v_existente.anfitrion_correo <> v_correo then
      raise exception 'No autorizado' using errcode = '42501';
    end if;
    if v_existente.contenido_hash = v_hash then
      return v_existente.id;
    end if;
    raise exception 'Ya existe una solicitud con ese identificador y contenido distinto' using errcode = '23505';
  end if;

  v_hoy := (now() at time zone 'America/Costa_Rica')::date;
  if p_fecha_desde < v_hoy then
    raise exception 'La fecha de inicio no puede ser anterior a hoy';
  end if;
  if p_fecha_hasta < p_fecha_desde then
    raise exception 'La fecha de fin debe ser posterior o igual a la de inicio';
  end if;

  insert into citas (id, anfitrion_correo, motivo, fecha_desde, fecha_hasta, hora_estimada, estado, contenido_hash)
  values (p_id, v_correo, v_motivo, p_fecha_desde, p_fecha_hasta, p_hora_estimada, 'VIGENTE', v_hash);

  insert into cita_sitios (cita_id, sitio_id)
  select p_id, s from unnest(p_sitios) as s;

  for v_visitante in select * from jsonb_array_elements(p_visitantes) loop
    v_cedula := btrim(v_visitante ->> 'cedula');
    v_nombre := btrim(v_visitante ->> 'nombre');
    v_empresa := nullif(btrim(v_visitante ->> 'empresa'), '');
    v_placa := nullif(btrim(v_visitante ->> 'placa_vehiculo'), '');

    if v_cedula is null or v_cedula = '' or length(v_cedula) > 30 or v_cedula ~ '[[:cntrl:]]' then
      raise exception 'Cédula de visitante inválida';
    end if;
    if v_nombre is null or v_nombre = '' or length(v_nombre) > 150 or v_nombre ~ '[[:cntrl:]]' then
      raise exception 'Nombre de visitante inválido';
    end if;
    if v_empresa is not null and (length(v_empresa) > 150 or v_empresa ~ '[[:cntrl:]]') then
      raise exception 'Empresa de visitante inválida';
    end if;
    if v_placa is not null and (length(v_placa) > 20 or v_placa ~ '[[:cntrl:]]') then
      raise exception 'Placa de visitante inválida';
    end if;
    if v_cedula = any(v_vistas) then
      raise exception 'Cédula duplicada dentro del mismo grupo: %', v_cedula;
    end if;
    v_vistas := array_append(v_vistas, v_cedula);

    insert into cita_visitantes (cita_id, cedula, nombre, empresa, placa_vehiculo)
    values (p_id, v_cedula, v_nombre, v_empresa, v_placa);
  end loop;

  return p_id;
end;
$$;

revoke all on function public.crear_cita_anfitrion(uuid, date, date, text, uuid[], jsonb, time) from public;
grant execute on function public.crear_cita_anfitrion(uuid, date, date, text, uuid[], jsonb, time) to authenticated;

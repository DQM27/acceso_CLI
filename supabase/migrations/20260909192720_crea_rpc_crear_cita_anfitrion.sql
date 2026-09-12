-- RPC atómica pedida en docs/contrato-web-visitas.md: reemplaza las 3
-- peticiones REST separadas (citas + cita_sitios + cita_visitantes) que
-- podían dejar una cita a medias si la conexión se caía entre medio. Todo
-- corre en la transacción implícita de esta única llamada a función --
-- cualquier `raise exception` revierte TODO lo insertado hasta ese punto,
-- no hace falta un BEGIN/COMMIT manual.
--
-- SECURITY INVOKER (no DEFINER) a propósito: corre con los privilegios y el
-- RLS del propio anfitrión que llama -- los INSERT de acá abajo siguen
-- pasando por "anfitrion crea sus propias citas"/"anfitrion agrega
-- sitios/visitantes a sus propias citas", así que esta función no es una
-- puerta trasera que salte RLS; es sólo la forma de agrupar 3 escrituras ya
-- permitidas en una sola transacción atómica.
create or replace function public.crear_cita_anfitrion(
  p_id uuid,
  p_fecha_desde date,
  p_fecha_hasta date,
  p_motivo text,
  p_sitios uuid[],
  p_visitantes jsonb
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

  -- Cuota simple del lado servidor -- docs/contrato-web-visitas.md pide
  -- explícitamente no confiar sólo en los límites de la interfaz ("cualquiera
  -- con un token puede llamar directamente la API"). 20/hora es generoso
  -- para uso normal y bloquea un abuso automatizado básico; se puede ajustar
  -- sin tocar la web si hace falta.
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

  -- Hash de idempotencia sobre el contenido normalizado (ya validado antes
  -- de este punto en cuanto a forma, no todavía en cuanto a reglas de
  -- negocio) -- comparado ANTES de la validación de fecha pasada, ver más
  -- abajo: un reintento de una respuesta perdida puede llegar después de
  -- medianoche sobre una cita que sí se guardó a tiempo.
  v_hash := md5(
    v_correo || '|' || p_fecha_desde::text || '|' || p_fecha_hasta::text || '|' ||
    coalesce(v_motivo, '') || '|' ||
    coalesce((select string_agg(s::text, ',' order by s) from unnest(p_sitios) as s), '') || '|' ||
    p_visitantes::text
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

  insert into citas (id, anfitrion_correo, motivo, fecha_desde, fecha_hasta, estado, contenido_hash)
  values (p_id, v_correo, v_motivo, p_fecha_desde, p_fecha_hasta, 'VIGENTE', v_hash);

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

revoke all on function public.crear_cita_anfitrion(uuid, date, date, text, uuid[], jsonb) from public;
grant execute on function public.crear_cita_anfitrion(uuid, date, date, text, uuid[], jsonb) to authenticated;

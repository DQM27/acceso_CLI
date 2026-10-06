-- Web de visitas nueva (rama feat/web-visitas): lo que necesitan sus
-- pantallas, sobre el modelo de citas que ya leen escritorio y teléfono
-- (`citas`, `cita_sitios`, `cita_visitantes`, `movimientos_visita`).
--
-- 1. Todas las escrituras del anfitrión pasan por funciones con reglas.
--    Hasta ahora la RLS dejaba al anfitrión insertar y actualizar `citas`,
--    `cita_sitios` y `cita_visitantes` directo, saltándose la función
--    (sin límite por hora, sin validar fechas pasadas, sin revisar que su
--    cuenta siguiera activa, y pudiendo cambiar cualquier columna). Esas
--    políticas se van; las funciones pasan a SECURITY DEFINER con sus
--    propios controles. Escritorio y teléfono sólo LEEN estas tablas.
-- 2. `crear_cita_anfitrion`: las mismas reglas que antes, y además la cédula
--    se guarda en su forma única (`normalizar_cedula`, igual que el núcleo):
--    "1-0847-0293" y "108470293" son la misma persona también para detectar
--    duplicados dentro del grupo.
-- 3. `editar_cita_anfitrion`: cancela la cita y crea la nueva en un solo
--    paso, sólo si nadie de la cita entró todavía. Editar como "cancelar y
--    recrear" es a propósito: la cancelación les llega a todos los equipos
--    (ya lo hacía), así que un visitante quitado o un sitio quitado dejan de
--    valer en la portería sin cambiar escritorio ni teléfono. Editar en
--    sitio dejaría al equipo de un sitio quitado con la versión vieja (deja
--    de poder leer la cita) y al visitante quitado en la copia local.
-- 4. `cancelar_cita_anfitrion`: reemplaza el UPDATE directo; idempotente.
-- 5. `estado_visitantes_de_mis_citas`: quién llegó, a qué hora, con qué
--    gafete y si ya salió, para las citas del propio anfitrión (la RLS de
--    `movimientos_visita` sólo deja leer a los equipos del sitio).
-- 6. `visitantes_anteriores`: personas que el anfitrión ya agendó, para no
--    volver a escribirlas.

-- (1) Fuera las escrituras directas del anfitrión.
drop policy if exists "anfitrion crea sus propias citas" on public.citas;
drop policy if exists "anfitrion actualiza sus propias citas" on public.citas;
drop policy if exists "anfitrion agrega sitios a sus propias citas" on public.cita_sitios;
drop policy if exists "anfitrion quita sitios de sus propias citas" on public.cita_sitios;
drop policy if exists "anfitrion agrega visitantes a sus propias citas" on public.cita_visitantes;
drop policy if exists "anfitrion edita visitantes de sus propias citas" on public.cita_visitantes;
drop policy if exists "anfitrion quita visitantes de sus propias citas" on public.cita_visitantes;

-- Correo del anfitrión que llama, si está activo; si no, error.
create function private.anfitrion_activo_que_llama()
returns text
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  v_correo text := auth.email();
begin
  if v_correo is null then
    raise exception 'No autenticado' using errcode = '28000';
  end if;
  if not exists (select 1 from public.anfitriones where correo = v_correo and activo) then
    raise exception 'Cuenta no autorizada para agendar visitas' using errcode = '42501';
  end if;
  return v_correo;
end;
$$;

-- Mismo límite que tenía la función de crear: 20 citas por hora.
create function private.revisar_limite_de_citas(p_correo text)
returns void
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if (
    select count(*) from public.citas
    where anfitrion_correo = p_correo and created_at > now() - interval '1 hour'
  ) >= 20 then
    raise exception 'Demasiadas citas creadas en la última hora, intente más tarde' using errcode = '54000';
  end if;
end;
$$;

-- Hash del contenido, para la idempotencia (reintento con el mismo id).
create function private.hash_de_cita(
  p_correo text, p_fecha_desde date, p_fecha_hasta date, p_motivo text,
  p_sitios uuid[], p_visitantes jsonb, p_hora_estimada time
)
returns text
language sql
immutable
set search_path = ''
as $$
  select md5(
    p_correo || '|' || p_fecha_desde::text || '|' || p_fecha_hasta::text || '|' ||
    coalesce(nullif(btrim(p_motivo), ''), '') || '|' ||
    coalesce((select string_agg(s::text, ',' order by s) from unnest(p_sitios) as s), '') || '|' ||
    p_visitantes::text || '|' || coalesce(p_hora_estimada::text, '')
  )
$$;

-- Valida y guarda una cita nueva con sus sitios y visitantes. Sin controles
-- de quién llama: los hacen las funciones públicas antes de llamarla.
create function private.guardar_cita(
  p_correo text, p_id uuid, p_fecha_desde date, p_fecha_hasta date, p_motivo text,
  p_sitios uuid[], p_visitantes jsonb, p_hora_estimada time, p_hash text
)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_hoy date := (now() at time zone 'America/Costa_Rica')::date;
  v_motivo text := nullif(btrim(p_motivo), '');
  v_n_sitios int := coalesce(array_length(p_sitios, 1), 0);
  v_n_visitantes int := jsonb_array_length(p_visitantes);
  v_visitante jsonb;
  v_cedula text;
  v_nombre text;
  v_empresa text;
  v_placa text;
  v_vistas text[] := '{}';
begin
  if v_motivo is not null and (length(v_motivo) > 1000 or v_motivo ~ '[[:cntrl:]]') then
    raise exception 'Motivo inválido';
  end if;
  if v_n_sitios < 1 or v_n_sitios > 100 then
    raise exception 'Debe indicar entre 1 y 100 sitios';
  end if;
  if v_n_sitios <> (select count(distinct s) from unnest(p_sitios) as s) then
    raise exception 'Sitios duplicados en la solicitud';
  end if;
  if exists (select 1 from unnest(p_sitios) as s where not exists (select 1 from public.sitios where id = s)) then
    raise exception 'Uno de los sitios indicados no existe';
  end if;
  if v_n_visitantes is null or v_n_visitantes < 1 or v_n_visitantes > 50 then
    raise exception 'Debe indicar entre 1 y 50 visitantes';
  end if;
  if p_fecha_desde < v_hoy then
    raise exception 'La fecha de inicio no puede ser anterior a hoy';
  end if;
  if p_fecha_hasta < p_fecha_desde then
    raise exception 'La fecha de fin debe ser posterior o igual a la de inicio';
  end if;

  insert into public.citas (id, anfitrion_correo, motivo, fecha_desde, fecha_hasta, hora_estimada, estado, contenido_hash)
  values (p_id, p_correo, v_motivo, p_fecha_desde, p_fecha_hasta, p_hora_estimada, 'VIGENTE', p_hash);

  insert into public.cita_sitios (cita_id, sitio_id)
  select p_id, s from unnest(p_sitios) as s;

  for v_visitante in select * from jsonb_array_elements(p_visitantes) loop
    v_nombre := btrim(v_visitante ->> 'nombre');
    v_empresa := nullif(btrim(v_visitante ->> 'empresa'), '');
    v_placa := nullif(btrim(v_visitante ->> 'placa_vehiculo'), '');
    -- Forma única, igual que el núcleo; si no se puede normalizar, como vino.
    v_cedula := coalesce(public.normalizar_cedula(v_visitante ->> 'cedula'), btrim(v_visitante ->> 'cedula'));

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

    insert into public.cita_visitantes (cita_id, cedula, nombre, empresa, placa_vehiculo)
    values (p_id, v_cedula, v_nombre, v_empresa, v_placa);
  end loop;
end;
$$;

revoke all on function private.anfitrion_activo_que_llama() from public;
revoke all on function private.revisar_limite_de_citas(text) from public;
revoke all on function private.hash_de_cita(text, date, date, text, uuid[], jsonb, time) from public;
revoke all on function private.guardar_cita(text, uuid, date, date, text, uuid[], jsonb, time, text) from public;

-- (2) Crear: mismas reglas e idempotencia que antes.
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
security definer
set search_path = ''
as $$
declare
  v_correo text := private.anfitrion_activo_que_llama();
  v_hash text := private.hash_de_cita(v_correo, p_fecha_desde, p_fecha_hasta, p_motivo, p_sitios, p_visitantes, p_hora_estimada);
  v_existente record;
begin
  select * into v_existente from public.citas where id = p_id;
  if found then
    if v_existente.anfitrion_correo <> v_correo then
      raise exception 'No autorizado' using errcode = '42501';
    end if;
    if v_existente.contenido_hash = v_hash then
      return v_existente.id;
    end if;
    raise exception 'Ya existe una solicitud con ese identificador y contenido distinto' using errcode = '23505';
  end if;

  perform private.revisar_limite_de_citas(v_correo);
  perform private.guardar_cita(v_correo, p_id, p_fecha_desde, p_fecha_hasta, p_motivo, p_sitios, p_visitantes, p_hora_estimada, v_hash);
  return p_id;
end;
$$;

-- (3) Editar = cancelar la cita y crear la nueva (`p_nuevo_id`), atómico.
create function public.editar_cita_anfitrion(
  p_id uuid,
  p_nuevo_id uuid,
  p_fecha_desde date,
  p_fecha_hasta date,
  p_motivo text,
  p_sitios uuid[],
  p_visitantes jsonb,
  p_hora_estimada time default null
)
returns uuid
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_correo text := private.anfitrion_activo_que_llama();
  v_hash text := private.hash_de_cita(v_correo, p_fecha_desde, p_fecha_hasta, p_motivo, p_sitios, p_visitantes, p_hora_estimada);
  v_cita record;
  v_nueva record;
begin
  -- Reintento de una edición que ya se aplicó: devuelve la nueva.
  select * into v_nueva from public.citas where id = p_nuevo_id;
  if found then
    if v_nueva.anfitrion_correo = v_correo and v_nueva.contenido_hash = v_hash then
      return v_nueva.id;
    end if;
    raise exception 'Ya existe una solicitud con ese identificador y contenido distinto' using errcode = '23505';
  end if;

  select * into v_cita from public.citas where id = p_id for update;
  if not found or v_cita.anfitrion_correo <> v_correo then
    raise exception 'La visita no existe' using errcode = 'P0002';
  end if;
  if v_cita.estado <> 'VIGENTE' then
    raise exception 'La visita está cancelada: no se puede editar';
  end if;
  if exists (
    select 1 from public.movimientos_visita m
    join public.cita_visitantes v on v.id = m.cita_visitante_id
    where v.cita_id = p_id
  ) then
    raise exception 'Alguien de esta visita ya entró: no se puede editar. Agende una visita nueva si hace falta.';
  end if;

  perform private.revisar_limite_de_citas(v_correo);
  update public.citas set estado = 'CANCELADA' where id = p_id;
  perform private.guardar_cita(v_correo, p_nuevo_id, p_fecha_desde, p_fecha_hasta, p_motivo, p_sitios, p_visitantes, p_hora_estimada, v_hash);
  return p_nuevo_id;
end;
$$;

-- (4) Cancelar: idempotente; las personas que ya están adentro siguen
-- adentro, sólo deja de valer para entrar.
create function public.cancelar_cita_anfitrion(p_id uuid)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_correo text := private.anfitrion_activo_que_llama();
  v_cita record;
begin
  select * into v_cita from public.citas where id = p_id for update;
  if not found or v_cita.anfitrion_correo <> v_correo then
    raise exception 'La visita no existe' using errcode = 'P0002';
  end if;
  if v_cita.estado = 'VIGENTE' then
    update public.citas set estado = 'CANCELADA' where id = p_id;
  end if;
end;
$$;

-- (5) Llegadas: el último movimiento de cada visitante de las citas pedidas,
-- sólo si son del anfitrión que llama.
create function public.estado_visitantes_de_mis_citas(p_citas uuid[])
returns table (
  cita_visitante_id uuid,
  sitio_nombre text,
  hora_entrada timestamptz,
  hora_salida timestamptz,
  gafete_numero bigint
)
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  v_correo text := private.anfitrion_activo_que_llama();
begin
  if coalesce(array_length(p_citas, 1), 0) > 200 then
    raise exception 'Demasiadas visitas' using errcode = '22023';
  end if;
  return query
    select distinct on (m.cita_visitante_id)
           m.cita_visitante_id, s.nombre, m.hora_entrada, m.hora_salida, m.gafete_numero
      from public.movimientos_visita m
      join public.cita_visitantes v on v.id = m.cita_visitante_id
      join public.citas c on c.id = v.cita_id
      join public.sitios s on s.id = m.sitio_id
     where c.id = any (p_citas)
       and c.anfitrion_correo = v_correo
     order by m.cita_visitante_id, m.hora_entrada desc;
end;
$$;

-- (6) Visitantes que el anfitrión ya agendó (una fila por cédula, la más
-- reciente), filtrados por nombre o cédula. Lee con la RLS de quien llama.
create function public.visitantes_anteriores(p_busqueda text default null)
returns table (cedula text, nombre text, empresa text, placa_vehiculo text, ultima_vez date)
language sql
stable
security invoker
set search_path = ''
as $$
  select cedula, nombre, empresa, placa_vehiculo, ultima_vez
    from (
      select distinct on (v.cedula)
             v.cedula, v.nombre, v.empresa, v.placa_vehiculo, c.fecha_desde as ultima_vez
        from public.cita_visitantes v
        join public.citas c on c.id = v.cita_id
       where c.anfitrion_correo = auth.email()
         and (
           nullif(btrim(p_busqueda), '') is null
           or v.nombre ilike '%' || btrim(p_busqueda) || '%'
           or v.cedula ilike '%' || coalesce(public.normalizar_cedula(p_busqueda), btrim(p_busqueda)) || '%'
         )
       order by v.cedula, c.fecha_desde desc
    ) as ultimas
   order by ultima_vez desc
   limit 20
$$;

revoke all on function public.crear_cita_anfitrion(uuid, date, date, text, uuid[], jsonb, time) from public, anon;
revoke all on function public.editar_cita_anfitrion(uuid, uuid, date, date, text, uuid[], jsonb, time) from public, anon;
revoke all on function public.cancelar_cita_anfitrion(uuid) from public, anon;
revoke all on function public.estado_visitantes_de_mis_citas(uuid[]) from public, anon;
revoke all on function public.visitantes_anteriores(text) from public, anon;
grant execute on function public.crear_cita_anfitrion(uuid, date, date, text, uuid[], jsonb, time) to authenticated;
grant execute on function public.editar_cita_anfitrion(uuid, uuid, date, date, text, uuid[], jsonb, time) to authenticated;
grant execute on function public.cancelar_cita_anfitrion(uuid) to authenticated;
grant execute on function public.estado_visitantes_de_mis_citas(uuid[]) to authenticated;
grant execute on function public.visitantes_anteriores(text) to authenticated;

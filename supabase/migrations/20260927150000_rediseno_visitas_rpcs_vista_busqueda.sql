-- Rediseño de visitas (docs/auditorias/rediseno-visitas-2026-09-27.md,
-- 3.4/3.7) -- segundo corte de V2, sobre el esquema de
-- 20260927120000_rediseno_visitas_tablas_nuevas.sql (ya confirmado en
-- staging): las RPCs atómicas que el anfitrión usa desde la web, la vista
-- `mis_visitas` y la búsqueda de "visitantes anteriores".
--
-- Todas las RPCs son SECURITY INVOKER, mismo criterio que
-- `crear_cita_anfitrion`: corren con los privilegios y el RLS del propio
-- anfitrión que llama, no son una puerta trasera. La única excepción es
-- `private.resolver_visitante` (ver más abajo, por qué necesita
-- SECURITY DEFINER).

-- `visitas.contenido_hash` -- idempotencia de crear_visitas, mismo patrón
-- que `citas.contenido_hash` del esquema viejo (una visita por SITIO, así
-- que el hash se repite en todas las filas de un mismo grupo_id).
alter table public.visitas add column contenido_hash text;

-- =====================================================================
-- private.resolver_visitante -- inserta un visitante nuevo o devuelve el
-- id del que ya existe con ese documento (UNIQUE tipo_documento+
-- numero_documento). SECURITY DEFINER a propósito: la política de
-- SELECT/UPDATE de `visitantes` sólo deja ver/tocar a quien YA lo invitó
-- antes -- un anfitrión agendando por primera vez a alguien que otro
-- anfitrión ya registró antes no tiene, todavía, ninguna fila en
-- `visita_invitados` que lo autorice, así que el `INSERT ... ON CONFLICT
-- DO UPDATE`/el `SELECT` posterior chocarían con RLS. Esta función NO
-- expone datos: sólo puede crear una fila nueva o devolver el id de una
-- que ya existía con ESE documento exacto (que el propio llamador ya
-- conoce, porque lo tecleó) -- no permite buscar ni listar a nadie más.
create or replace function private.resolver_visitante(
  p_tipo_documento text,
  p_numero_documento text,
  p_nombre text,
  p_empresa text,
  p_telefono text,
  p_correo text
)
returns uuid
language plpgsql
security definer
set search_path = public
as $$
declare
  v_id uuid;
begin
  insert into visitantes (tipo_documento, numero_documento, nombre, empresa, telefono, correo)
  values (p_tipo_documento, p_numero_documento, p_nombre, p_empresa, p_telefono, p_correo)
  on conflict (tipo_documento, numero_documento) do nothing
  returning id into v_id;

  if v_id is null then
    select id into v_id from visitantes
    where tipo_documento = p_tipo_documento and numero_documento = p_numero_documento;
  end if;

  return v_id;
end;
$$;

revoke all on function private.resolver_visitante(text, text, text, text, text, text) from public;
grant execute on function private.resolver_visitante(text, text, text, text, text, text) to authenticated;

-- Falta una política de DELETE en visita_invitados -- la migración
-- anterior no la incluyó a propósito ("una persona no se borra, se
-- corrige"), pero acá sí hace falta: `editar_visita` reemplaza la lista
-- de invitados, y ya se garantiza en la función que nadie entró todavía
-- (por eso el `not exists` contra visita_movimientos, doble candado
-- además del bloqueo que ya hace editar_visita antes de llegar acá).
create policy "anfitrion quita invitados sin movimientos de su visita"
  on public.visita_invitados for delete to authenticated
  using (
    exists (
      select 1 from public.visitas v
      where v.id = visita_invitados.visita_id
        and v.anfitrion_id = (select a.id from public.anfitriones a where a.correo = (select auth.email()))
    )
    and not exists (
      select 1 from public.visita_movimientos vm where vm.invitado_id = visita_invitados.id
    )
  );

-- =====================================================================
-- crear_visitas -- "Nueva visita" (3.4/3.8): un pre-registro para uno o
-- varios sitios en un solo paso crea UNA visita POR SITIO (decidido: "una
-- visita = un sitio"), todas unidas por `p_grupo_id` (generado por el
-- cliente, como `p_id` en crear_cita_anfitrion) con los MISMOS invitados.
-- `p_invitados`: [{tipo_documento, numero_documento, nombre, empresa?,
-- telefono?, correo?, placa_vehiculo?}, ...].
-- =====================================================================
create or replace function public.crear_visitas(
  p_grupo_id uuid,
  p_sitios uuid[],
  p_fecha_desde date,
  p_fecha_hasta date,
  p_hora_desde time,
  p_hora_hasta time,
  p_tipo_visita text,
  p_motivo text,
  p_requiere_escolta boolean,
  p_invitados jsonb
)
returns uuid[]
language plpgsql
security invoker
set search_path = public
as $$
declare
  v_correo text;
  v_anfitrion_id uuid;
  v_hoy date;
  v_hash text;
  v_n_sitios int;
  v_n_invitados int;
  v_existente record;
  v_visitante jsonb;
  v_tipo_doc text;
  v_num_doc text;
  v_nombre text;
  v_empresa text;
  v_telefono text;
  v_correo_visitante text;
  v_placa text;
  v_visitante_id uuid;
  v_visitante_ids uuid[] := '{}';
  v_placas text[] := '{}';
  v_vistos text[] := '{}';
  v_sitio uuid;
  v_visita_id uuid;
  v_ids uuid[] := '{}';
  i int;
begin
  v_correo := auth.email();
  if v_correo is null then
    raise exception 'No autenticado' using errcode = '28000';
  end if;

  select id into v_anfitrion_id from anfitriones where correo = v_correo and activo;
  if v_anfitrion_id is null then
    raise exception 'Cuenta no autorizada para agendar visitas' using errcode = '42501';
  end if;

  -- Cuota simple del lado servidor, mismo criterio que crear_cita_anfitrion
  -- -- por grupo, no por fila, para no penalizar un "tour" de varios sitios
  -- más que un pre-registro de un solo sitio.
  if (
    select count(distinct grupo_id) from visitas
    where anfitrion_id = v_anfitrion_id and creado_en > now() - interval '1 hour'
  ) >= 20 then
    raise exception 'Demasiadas visitas creadas en la última hora, intente más tarde' using errcode = '54000';
  end if;

  p_motivo := nullif(btrim(p_motivo), '');
  if p_motivo is not null and (length(p_motivo) > 1000 or p_motivo ~ '[[:cntrl:]]') then
    raise exception 'Motivo inválido';
  end if;
  p_tipo_visita := nullif(btrim(p_tipo_visita), '');

  v_n_sitios := coalesce(array_length(p_sitios, 1), 0);
  if v_n_sitios < 1 or v_n_sitios > 20 then
    raise exception 'Debe indicar entre 1 y 20 sitios';
  end if;
  if v_n_sitios <> (select count(distinct s) from unnest(p_sitios) as s) then
    raise exception 'Sitios duplicados en la solicitud';
  end if;
  if exists (
    select 1 from unnest(p_sitios) as s
    where not exists (
      select 1 from anfitrion_sitios where anfitrion_id = v_anfitrion_id and sitio_id = s
    )
  ) then
    raise exception 'No tiene acceso a uno de los sitios indicados' using errcode = '42501';
  end if;

  v_n_invitados := jsonb_array_length(p_invitados);
  if v_n_invitados is null or v_n_invitados < 1 or v_n_invitados > 50 then
    raise exception 'Debe indicar entre 1 y 50 personas';
  end if;

  v_hash := md5(
    v_correo || '|' || p_fecha_desde::text || '|' || p_fecha_hasta::text || '|' ||
    p_hora_desde::text || '|' || p_hora_hasta::text || '|' ||
    coalesce(p_tipo_visita, '') || '|' || coalesce(p_motivo, '') || '|' ||
    p_requiere_escolta::text || '|' ||
    coalesce((select string_agg(s::text, ',' order by s) from unnest(p_sitios) as s), '') || '|' ||
    p_invitados::text
  );

  select * into v_existente from visitas where grupo_id = p_grupo_id limit 1;
  if found then
    if v_existente.anfitrion_id <> v_anfitrion_id then
      raise exception 'No autorizado' using errcode = '42501';
    end if;
    if v_existente.contenido_hash = v_hash then
      return array(select id from visitas where grupo_id = p_grupo_id order by sitio_id);
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
  if p_hora_hasta <= p_hora_desde then
    raise exception 'La hora de fin debe ser posterior a la de inicio';
  end if;

  -- Resolver/crear cada visitante UNA sola vez (fuera del loop de sitios)
  -- para no duplicar personas cuando hay más de un sitio.
  for v_visitante in select * from jsonb_array_elements(p_invitados) loop
    v_tipo_doc := upper(btrim(v_visitante ->> 'tipo_documento'));
    v_num_doc := btrim(v_visitante ->> 'numero_documento');
    v_nombre := btrim(v_visitante ->> 'nombre');
    v_empresa := nullif(btrim(v_visitante ->> 'empresa'), '');
    v_telefono := nullif(btrim(v_visitante ->> 'telefono'), '');
    v_correo_visitante := nullif(btrim(v_visitante ->> 'correo'), '');
    v_placa := nullif(btrim(v_visitante ->> 'placa_vehiculo'), '');

    if v_tipo_doc not in ('CEDULA', 'DIMEX', 'PASAPORTE', 'OTRO') then
      raise exception 'Tipo de documento inválido';
    end if;
    if v_num_doc is null or v_num_doc = '' or length(v_num_doc) > 30 or v_num_doc ~ '[[:cntrl:]]' then
      raise exception 'Número de documento inválido';
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

    if (v_tipo_doc || '|' || v_num_doc) = any(v_vistos) then
      raise exception 'Documento duplicado dentro del mismo grupo: %', v_num_doc;
    end if;
    v_vistos := array_append(v_vistos, v_tipo_doc || '|' || v_num_doc);

    v_visitante_id := private.resolver_visitante(v_tipo_doc, v_num_doc, v_nombre, v_empresa, v_telefono, v_correo_visitante);
    v_visitante_ids := array_append(v_visitante_ids, v_visitante_id);
    v_placas := array_append(v_placas, v_placa);
  end loop;

  -- Una visita por sitio, con los mismos invitados.
  foreach v_sitio in array p_sitios loop
    insert into visitas (
      sitio_id, anfitrion_id, tipo_visita, motivo, fecha_desde, fecha_hasta,
      hora_desde, hora_hasta, requiere_escolta, grupo_id, origen, creado_por, contenido_hash
    )
    values (
      v_sitio, v_anfitrion_id, p_tipo_visita, p_motivo, p_fecha_desde, p_fecha_hasta,
      p_hora_desde, p_hora_hasta, p_requiere_escolta, p_grupo_id, 'PRE_REGISTRO', v_correo, v_hash
    )
    returning id into v_visita_id;

    v_ids := array_append(v_ids, v_visita_id);

    for i in 1..array_length(v_visitante_ids, 1) loop
      insert into visita_invitados (visita_id, visitante_id, placa_vehiculo)
      values (v_visita_id, v_visitante_ids[i], v_placas[i]);
    end loop;
  end loop;

  return v_ids;
end;
$$;

revoke all on function public.crear_visitas(uuid, uuid[], date, date, time, time, text, text, boolean, jsonb) from public;
grant execute on function public.crear_visitas(uuid, uuid[], date, date, time, time, text, text, boolean, jsonb) to authenticated;

-- =====================================================================
-- editar_visita -- sólo mientras ningún invitado haya llegado (3.4/3.8).
-- Reemplaza la ventana/motivo/escolta y la lista COMPLETA de invitados de
-- ESA visita (un sitio). Para editar varios sitios de un mismo grupo_id,
-- la web llama esto una vez por visita.
-- =====================================================================
create or replace function public.editar_visita(
  p_visita_id uuid,
  p_fecha_desde date,
  p_fecha_hasta date,
  p_hora_desde time,
  p_hora_hasta time,
  p_motivo text,
  p_requiere_escolta boolean,
  p_invitados jsonb
)
returns void
language plpgsql
security invoker
set search_path = public
as $$
declare
  v_correo text;
  v_anfitrion_id uuid;
  v_visita record;
  v_hoy date;
  v_visitante jsonb;
  v_tipo_doc text;
  v_num_doc text;
  v_nombre text;
  v_empresa text;
  v_telefono text;
  v_correo_visitante text;
  v_placa text;
  v_visitante_id uuid;
  v_vistos text[] := '{}';
  v_ids_deseados uuid[] := '{}';
begin
  v_correo := auth.email();
  select id into v_anfitrion_id from anfitriones where correo = v_correo and activo;
  if v_anfitrion_id is null then
    raise exception 'Cuenta no autorizada' using errcode = '42501';
  end if;

  select * into v_visita from visitas where id = p_visita_id for update;
  if not found then
    raise exception 'Visita no encontrada';
  end if;
  if v_visita.anfitrion_id <> v_anfitrion_id then
    raise exception 'No autorizado' using errcode = '42501';
  end if;
  if v_visita.estado <> 'VIGENTE' then
    raise exception 'La visita ya está cancelada';
  end if;
  if exists (
    select 1 from visita_invitados
    where visita_id = p_visita_id and estado in ('EN_SITIO', 'FUERA', 'FINALIZADO')
  ) then
    raise exception 'No se puede editar una visita con invitados que ya ingresaron';
  end if;

  p_motivo := nullif(btrim(p_motivo), '');
  if p_motivo is not null and (length(p_motivo) > 1000 or p_motivo ~ '[[:cntrl:]]') then
    raise exception 'Motivo inválido';
  end if;

  v_hoy := (now() at time zone 'America/Costa_Rica')::date;
  if p_fecha_desde < v_hoy then
    raise exception 'La fecha de inicio no puede ser anterior a hoy';
  end if;
  if p_fecha_hasta < p_fecha_desde then
    raise exception 'La fecha de fin debe ser posterior o igual a la de inicio';
  end if;
  if p_hora_hasta <= p_hora_desde then
    raise exception 'La hora de fin debe ser posterior a la de inicio';
  end if;

  if jsonb_array_length(p_invitados) is null
     or jsonb_array_length(p_invitados) < 1
     or jsonb_array_length(p_invitados) > 50 then
    raise exception 'Debe indicar entre 1 y 50 personas';
  end if;

  update visitas set
    fecha_desde = p_fecha_desde,
    fecha_hasta = p_fecha_hasta,
    hora_desde = p_hora_desde,
    hora_hasta = p_hora_hasta,
    motivo = p_motivo,
    requiere_escolta = p_requiere_escolta
  where id = p_visita_id;

  for v_visitante in select * from jsonb_array_elements(p_invitados) loop
    v_tipo_doc := upper(btrim(v_visitante ->> 'tipo_documento'));
    v_num_doc := btrim(v_visitante ->> 'numero_documento');
    v_nombre := btrim(v_visitante ->> 'nombre');
    v_empresa := nullif(btrim(v_visitante ->> 'empresa'), '');
    v_telefono := nullif(btrim(v_visitante ->> 'telefono'), '');
    v_correo_visitante := nullif(btrim(v_visitante ->> 'correo'), '');
    v_placa := nullif(btrim(v_visitante ->> 'placa_vehiculo'), '');

    if v_tipo_doc not in ('CEDULA', 'DIMEX', 'PASAPORTE', 'OTRO') then
      raise exception 'Tipo de documento inválido';
    end if;
    if v_num_doc is null or v_num_doc = '' or length(v_num_doc) > 30 or v_num_doc ~ '[[:cntrl:]]' then
      raise exception 'Número de documento inválido';
    end if;
    if v_nombre is null or v_nombre = '' or length(v_nombre) > 150 or v_nombre ~ '[[:cntrl:]]' then
      raise exception 'Nombre de visitante inválido';
    end if;

    if (v_tipo_doc || '|' || v_num_doc) = any(v_vistos) then
      raise exception 'Documento duplicado dentro del mismo grupo: %', v_num_doc;
    end if;
    v_vistos := array_append(v_vistos, v_tipo_doc || '|' || v_num_doc);

    v_visitante_id := private.resolver_visitante(v_tipo_doc, v_num_doc, v_nombre, v_empresa, v_telefono, v_correo_visitante);
    v_ids_deseados := array_append(v_ids_deseados, v_visitante_id);

    insert into visita_invitados (visita_id, visitante_id, placa_vehiculo)
    values (p_visita_id, v_visitante_id, v_placa)
    on conflict (visita_id, visitante_id) do update
      set placa_vehiculo = excluded.placa_vehiculo;
  end loop;

  -- Quitar invitados que ya no están en la lista deseada -- sólo posible
  -- si nadie entró (ya lo garantizamos arriba), así que borrar es seguro.
  delete from visita_invitados
  where visita_id = p_visita_id
    and visitante_id <> all(v_ids_deseados);
end;
$$;

revoke all on function public.editar_visita(uuid, date, date, time, time, text, boolean, jsonb) from public;
grant execute on function public.editar_visita(uuid, date, date, time, time, text, boolean, jsonb) to authenticated;

-- =====================================================================
-- cancelar_visita -- idempotente (cancelar dos veces no es error). Sólo
-- cancela invitados que todavía no llegaron; a quien ya está adentro o ya
-- terminó no se le puede "descancelar" el pasado.
-- =====================================================================
create or replace function public.cancelar_visita(p_visita_id uuid)
returns void
language plpgsql
security invoker
set search_path = public
as $$
declare
  v_correo text;
  v_anfitrion_id uuid;
  v_visita record;
begin
  v_correo := auth.email();
  select id into v_anfitrion_id from anfitriones where correo = v_correo and activo;
  if v_anfitrion_id is null then
    raise exception 'Cuenta no autorizada' using errcode = '42501';
  end if;

  select * into v_visita from visitas where id = p_visita_id for update;
  if not found then
    raise exception 'Visita no encontrada';
  end if;
  if v_visita.anfitrion_id <> v_anfitrion_id then
    raise exception 'No autorizado' using errcode = '42501';
  end if;
  if v_visita.estado = 'CANCELADA' then
    return;
  end if;

  update visitas set estado = 'CANCELADA' where id = p_visita_id;

  update visita_invitados
  set estado = 'CANCELADA'
  where visita_id = p_visita_id
    and estado in ('PROGRAMADO', 'SOLICITADO', 'APROBADO');
end;
$$;

revoke all on function public.cancelar_visita(uuid) from public;
grant execute on function public.cancelar_visita(uuid) to authenticated;

-- =====================================================================
-- duplicar_visita -- "reagendar lo mismo otro día" (3.8): copia sitio/
-- tipo/motivo/ventana horaria/escolta e invitados de la visita original a
-- una nueva, con fechas nuevas. `p_nuevo_id` generado por el cliente,
-- idempotente (mismo patrón que crear_visitas/crear_cita_anfitrion).
-- =====================================================================
create or replace function public.duplicar_visita(
  p_visita_id uuid,
  p_nuevo_id uuid,
  p_fecha_desde date,
  p_fecha_hasta date
)
returns uuid
language plpgsql
security invoker
set search_path = public
as $$
declare
  v_correo text;
  v_anfitrion_id uuid;
  v_origen record;
  v_existente record;
  v_hoy date;
begin
  v_correo := auth.email();
  select id into v_anfitrion_id from anfitriones where correo = v_correo and activo;
  if v_anfitrion_id is null then
    raise exception 'Cuenta no autorizada' using errcode = '42501';
  end if;

  select * into v_existente from visitas where id = p_nuevo_id;
  if found then
    if v_existente.anfitrion_id <> v_anfitrion_id then
      raise exception 'No autorizado' using errcode = '42501';
    end if;
    return v_existente.id;
  end if;

  select * into v_origen from visitas where id = p_visita_id;
  if not found then
    raise exception 'Visita no encontrada';
  end if;
  if v_origen.anfitrion_id <> v_anfitrion_id then
    raise exception 'No autorizado' using errcode = '42501';
  end if;

  v_hoy := (now() at time zone 'America/Costa_Rica')::date;
  if p_fecha_desde < v_hoy then
    raise exception 'La fecha de inicio no puede ser anterior a hoy';
  end if;
  if p_fecha_hasta < p_fecha_desde then
    raise exception 'La fecha de fin debe ser posterior o igual a la de inicio';
  end if;

  insert into visitas (
    id, sitio_id, anfitrion_id, tipo_visita, motivo, fecha_desde, fecha_hasta,
    hora_desde, hora_hasta, requiere_escolta, grupo_id, origen, creado_por
  )
  values (
    p_nuevo_id, v_origen.sitio_id, v_anfitrion_id, v_origen.tipo_visita, v_origen.motivo,
    p_fecha_desde, p_fecha_hasta, v_origen.hora_desde, v_origen.hora_hasta,
    v_origen.requiere_escolta, p_nuevo_id, 'PRE_REGISTRO', v_correo
  );

  insert into visita_invitados (visita_id, visitante_id, placa_vehiculo)
  select distinct p_nuevo_id, vi.visitante_id, vi.placa_vehiculo
  from visita_invitados vi
  where vi.visita_id = p_visita_id;

  return p_nuevo_id;
end;
$$;

revoke all on function public.duplicar_visita(uuid, uuid, date, date) from public;
grant execute on function public.duplicar_visita(uuid, uuid, date, date) to authenticated;

-- =====================================================================
-- responder_solicitud -- aprobar/rechazar un walk-in (3.4/3.8). El enlace
-- firmado de un solo uso que llega por correo NO pasa por acá con la
-- sesión del anfitrión -- eso lo resuelve la Edge Function de correo
-- (todavía no escrita) con su propio token, no esta RPC.
-- =====================================================================
create or replace function public.responder_solicitud(
  p_invitado_id uuid,
  p_aprobar boolean,
  p_motivo_rechazo text
)
returns void
language plpgsql
security invoker
set search_path = public
as $$
declare
  v_correo text;
  v_anfitrion_id uuid;
  v_invitado record;
  v_visita record;
begin
  v_correo := auth.email();
  select id into v_anfitrion_id from anfitriones where correo = v_correo and activo;
  if v_anfitrion_id is null then
    raise exception 'Cuenta no autorizada' using errcode = '42501';
  end if;

  select * into v_invitado from visita_invitados where id = p_invitado_id for update;
  if not found then
    raise exception 'Solicitud no encontrada';
  end if;

  select * into v_visita from visitas where id = v_invitado.visita_id;
  if v_visita.anfitrion_id <> v_anfitrion_id then
    raise exception 'No autorizado' using errcode = '42501';
  end if;
  if v_invitado.estado <> 'SOLICITADO' then
    raise exception 'Esta solicitud ya fue resuelta';
  end if;

  if p_aprobar then
    update visita_invitados
    set estado = 'APROBADO', aprobado_por = v_correo, aprobado_en = now()
    where id = p_invitado_id;
  else
    p_motivo_rechazo := nullif(btrim(p_motivo_rechazo), '');
    if p_motivo_rechazo is null then
      raise exception 'El motivo de rechazo es obligatorio';
    end if;
    update visita_invitados
    set estado = 'RECHAZADO', aprobado_por = v_correo, aprobado_en = now(), motivo_rechazo = p_motivo_rechazo
    where id = p_invitado_id;
  end if;
end;
$$;

revoke all on function public.responder_solicitud(uuid, boolean, text) from public;
grant execute on function public.responder_solicitud(uuid, boolean, text) to authenticated;

-- =====================================================================
-- mis_visitas -- estado por persona en vivo (3.8: "Juan Pérez llegó
-- 9:12 · gafete 7"). security_invoker=true: NO agrega privilegios, sólo
-- junta lo que las políticas de cada tabla ya le dejan ver a quien
-- consulta (anfitrión, dispositivo del sitio, o admin).
-- =====================================================================
create view public.mis_visitas
with (security_invoker = true) as
select
  v.id as visita_id,
  v.sitio_id,
  s.nombre as sitio_nombre,
  v.anfitrion_id,
  v.tipo_visita,
  v.motivo,
  v.fecha_desde,
  v.fecha_hasta,
  v.hora_desde,
  v.hora_hasta,
  v.requiere_escolta,
  v.grupo_id,
  v.origen,
  v.estado as visita_estado,
  vi.id as invitado_id,
  vi.visitante_id,
  vt.tipo_documento,
  vt.numero_documento,
  vt.nombre as visitante_nombre,
  vt.empresa as visitante_empresa,
  vi.placa_vehiculo,
  vi.estado as invitado_estado,
  vi.aprobado_por,
  vi.aprobado_en,
  vi.motivo_rechazo,
  ultimo.entrada_en as ultima_entrada,
  ultimo.salida_en as ultima_salida,
  ultimo.gafete_numero as ultimo_gafete_numero
from public.visitas v
join public.sitios s on s.id = v.sitio_id
join public.visita_invitados vi on vi.visita_id = v.id
join public.visitantes vt on vt.id = vi.visitante_id
left join lateral (
  select m.entrada_en, m.salida_en, m.gafete_numero
  from public.visita_movimientos m
  where m.invitado_id = vi.id
  order by m.entrada_en desc
  limit 1
) ultimo on true;

comment on view public.mis_visitas is
  'Vista de lectura para la web de anfitriones (3.8) -- una fila por invitado, con su ultimo movimiento (entrada/salida/gafete) si lo tiene. security_invoker=true: la visibilidad la deciden las politicas de visitas/visita_invitados/visitantes/visita_movimientos, no esta vista.';

grant select on public.mis_visitas to authenticated;

-- =====================================================================
-- visitantes_anteriores_del_anfitrion -- "mis visitantes frecuentes"
-- (3.4/3.8): sólo entre quienes ESE anfitrión ya invitó, filtrado por
-- nombre o documento. SECURITY INVOKER: no necesita saltarse RLS, las
-- políticas ya restringen a "visitantes que invité antes" -- este filtro
-- de búsqueda sólo acota más ese mismo conjunto.
-- =====================================================================
create or replace function public.visitantes_anteriores_del_anfitrion(p_busqueda text default '')
returns table (
  id uuid,
  tipo_documento text,
  numero_documento text,
  nombre text,
  empresa text
)
language sql
security invoker
set search_path = public
stable
as $$
  select distinct vt.id, vt.tipo_documento, vt.numero_documento, vt.nombre, vt.empresa
  from visitantes vt
  join visita_invitados vi on vi.visitante_id = vt.id
  join visitas v on v.id = vi.visita_id
  where v.anfitrion_id = (select a.id from anfitriones a where a.correo = (select auth.email()))
    and (
      p_busqueda = ''
      or plegar_texto(vt.nombre) like '%' || plegar_texto(p_busqueda) || '%'
      or vt.numero_documento like '%' || p_busqueda || '%'
    )
  order by vt.nombre
  limit 20;
$$;

grant execute on function public.visitantes_anteriores_del_anfitrion(text) to authenticated;

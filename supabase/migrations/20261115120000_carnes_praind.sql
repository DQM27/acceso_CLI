-- Carnés PRAIND de contratistas (fase 1 de
-- docs/features-futuras/plan-fotos-praind.md).
--
-- Las imágenes viven en R2 (bucket privado `lattis-praind`) y las sirve el
-- Worker `praind-brisas`. Acá queda sólo lo que decide quién puede qué y
-- dónde está cada archivo:
--
-- 1. `contratistas.praind_foto`: ruta del carné vigente en R2, o nulo. Viaja
--    a los equipos con el catálogo, así saben sin consultar si hay carné.
--    Los equipos pueden actualizar `contratistas` (catálogo global), así que
--    un trigger impide que una sesión de cliente cambie esta columna: sólo
--    la escriben `praind_registrar` y `praind_quitar` (security definer).
-- 2. `fotos_praind`: bitácora de cada carné cargado (quién, cuándo, de qué
--    enlace). La lee sólo el panel. Al reemplazar o quitar un carné la fila
--    queda con `reemplazada_en`; si se borra el contratista queda con
--    `contratista_id` nulo, para poder limpiar su objeto en R2.
-- 3. `ajustes_praind`: interruptor `carga_desde_equipos`. Apagado (por
--    defecto) sólo el panel carga carnés; encendido, también los equipos.
-- 4. RPC que usa el Worker con el JWT de quien llama:
--    `praind_permisos()`, `praind_registrar(...)`, `praind_quitar(...)`.
--    El panel cambia el interruptor con `praind_cambiar_carga_desde_equipos`.
--
-- Ruta en R2: `contratistas/<contratista_id>/<sha256>.<png|jpg>`. El hash en
-- la ruta hace que cada objeto sea inmutable (el Worker lo sirve con caché
-- larga) y que volver a cargar el mismo archivo no cambie nada.

-- ---- 1. Columna en contratistas ----

alter table public.contratistas
  add column praind_foto text;

comment on column public.contratistas.praind_foto is
  'Ruta del carné PRAIND vigente en R2 (contratistas/<id>/<sha256>.<ext>). Nulo: sin carné. La escriben sólo praind_registrar y praind_quitar.';

-- Una sesión de cliente (equipo o panel con PostgREST) nunca cambia la
-- columna: se conserva la anterior en silencio, para que la sincronización
-- de un equipo que manda la fila completa no la pise ni falle. Las RPC
-- security definer corren como su dueño, así que sí pueden escribirla.
create function private.contratistas_proteger_praind_foto()
returns trigger
language plpgsql
set search_path to ''
as $$
begin
  if current_user in ('authenticated', 'anon') then
    if tg_op = 'INSERT' then
      new.praind_foto := null;
    else
      new.praind_foto := old.praind_foto;
    end if;
  end if;
  return new;
end;
$$;

revoke execute on function private.contratistas_proteger_praind_foto() from public, anon, authenticated;

create trigger contratistas_proteger_praind_foto
  before insert or update on public.contratistas
  for each row execute function private.contratistas_proteger_praind_foto();

-- ---- 2. Bitácora ----

create table public.fotos_praind (
  id uuid primary key default gen_random_uuid(),
  contratista_id uuid references public.contratistas (id) on delete set null,
  clave text not null unique,
  sha256 text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  bytes bigint not null check (bytes > 0),
  tipo_mime text not null check (tipo_mime in ('image/png', 'image/jpeg')),
  enlace_origen text,
  nombre_en_origen text,
  cargado_por text not null,
  cargado_en timestamptz not null default now(),
  reemplazada_en timestamptz
);

comment on table public.fotos_praind is
  'Bitácora de carnés PRAIND cargados. La fila vigente de cada contratista es la de reemplazada_en nulo.';

create index fotos_praind_contratista_idx on public.fotos_praind (contratista_id);
create unique index fotos_praind_una_vigente on public.fotos_praind (contratista_id)
  where reemplazada_en is null and contratista_id is not null;

alter table public.fotos_praind enable row level security;

create policy "leer fotos_praind (panel)"
  on public.fotos_praind for select to authenticated
  using ((select private.es_admin_global()));

create policy "solo dispositivos vigentes"
  on public.fotos_praind as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

grant select on public.fotos_praind to authenticated;
grant select, insert, update, delete on public.fotos_praind to service_role;

-- ---- 3. Interruptor ----

create table public.ajustes_praind (
  id boolean primary key default true check (id),
  carga_desde_equipos boolean not null default false,
  actualizado_por text,
  actualizado_en timestamptz not null default now()
);

comment on table public.ajustes_praind is
  'Una sola fila. carga_desde_equipos: si los equipos (escritorio y móvil) pueden cargar carnés PRAIND además del panel.';

insert into public.ajustes_praind (id) values (true);

alter table public.ajustes_praind enable row level security;

create policy "leer ajustes_praind (panel)"
  on public.ajustes_praind for select to authenticated
  using ((select private.es_admin_global()));

create policy "solo dispositivos vigentes"
  on public.ajustes_praind as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

grant select on public.ajustes_praind to authenticated;
grant select, update on public.ajustes_praind to service_role;

-- ---- 4. Funciones ----

-- Equipo vinculado y no retirado (JWT de dispositivo con huella válida).
create function private.praind_es_equipo()
returns boolean
language sql
stable
security definer
set search_path to ''
as $$
  select ((select auth.jwt()) ->> 'sitio_id') is not null
     and (select private.dispositivo_vigente())
$$;

create function private.praind_puede_cargar()
returns boolean
language sql
stable
security definer
set search_path to ''
as $$
  select (select private.es_admin_global())
      or ((select private.praind_es_equipo())
          and coalesce((select a.carga_desde_equipos from public.ajustes_praind a where a.id), false))
$$;

revoke execute on function private.praind_es_equipo() from public, anon;
revoke execute on function private.praind_puede_cargar() from public, anon;
grant execute on function private.praind_es_equipo() to authenticated;
grant execute on function private.praind_puede_cargar() to authenticated;

-- Lo que el Worker pregunta antes de servir o recibir un carné.
create function public.praind_permisos()
returns jsonb
language sql
stable
security definer
set search_path to ''
as $$
  select jsonb_build_object(
    'puede_ver', (select private.es_admin_global()) or (select private.praind_es_equipo()),
    'puede_cargar', (select private.praind_puede_cargar()),
    'es_admin', (select private.es_admin_global())
  )
$$;

-- Registra un carné ya escrito en R2 y lo deja como vigente. Devuelve la
-- clave del carné anterior para que el Worker borre ese objeto, o nulo si
-- no había o si es el mismo archivo.
create function public.praind_registrar(
  p_contratista_id uuid,
  p_clave text,
  p_sha256 text,
  p_bytes bigint,
  p_tipo_mime text,
  p_enlace_origen text default null,
  p_nombre_en_origen text default null
)
returns text
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_anterior text;
  v_extension text := case p_tipo_mime when 'image/png' then 'png' when 'image/jpeg' then 'jpg' end;
begin
  if not (select private.praind_puede_cargar()) then
    raise exception 'No tiene permiso para cargar carnés PRAIND.' using errcode = '42501';
  end if;
  if v_extension is null then
    raise exception 'El carné debe ser PNG o JPEG.' using errcode = '22023';
  end if;
  if p_sha256 is null or p_sha256 !~ '^[0-9a-f]{64}$'
     or p_clave is distinct from 'contratistas/' || p_contratista_id || '/' || p_sha256 || '.' || v_extension then
    raise exception 'La ruta del carné no corresponde al contratista ni al archivo.' using errcode = '22023';
  end if;

  select c.praind_foto into v_anterior
    from public.contratistas c
   where c.id = p_contratista_id
   for update;
  if not found then
    raise exception 'El contratista no existe.' using errcode = 'P0002';
  end if;

  if v_anterior = p_clave then
    return null;
  end if;

  update public.fotos_praind
     set reemplazada_en = now()
   where contratista_id = p_contratista_id and reemplazada_en is null;

  insert into public.fotos_praind
    (contratista_id, clave, sha256, bytes, tipo_mime, enlace_origen, nombre_en_origen, cargado_por)
  values
    (p_contratista_id, p_clave, p_sha256, p_bytes, p_tipo_mime,
     nullif(btrim(p_enlace_origen), ''), nullif(btrim(p_nombre_en_origen), ''),
     coalesce((select auth.email()), 'dispositivo:' || ((select auth.jwt()) ->> 'sub')))
  on conflict (clave) do update
     set contratista_id = excluded.contratista_id,
         reemplazada_en = null,
         cargado_por = excluded.cargado_por,
         cargado_en = now();

  update public.contratistas set praind_foto = p_clave where id = p_contratista_id;

  return v_anterior;
end;
$$;

-- Quita el carné vigente (sólo el panel). Devuelve la clave quitada para que
-- el Worker borre el objeto, o nulo si no había.
create function public.praind_quitar(p_contratista_id uuid)
returns text
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_anterior text;
begin
  if not (select private.es_admin_global()) then
    raise exception 'Sólo el panel puede quitar carnés PRAIND.' using errcode = '42501';
  end if;

  select c.praind_foto into v_anterior
    from public.contratistas c
   where c.id = p_contratista_id
   for update;
  if not found then
    raise exception 'El contratista no existe.' using errcode = 'P0002';
  end if;
  if v_anterior is null then
    return null;
  end if;

  update public.fotos_praind
     set reemplazada_en = now()
   where contratista_id = p_contratista_id and reemplazada_en is null;
  update public.contratistas set praind_foto = null where id = p_contratista_id;

  return v_anterior;
end;
$$;

create function public.praind_cambiar_carga_desde_equipos(p_permitir boolean)
returns void
language plpgsql
security definer
set search_path to ''
as $$
begin
  if not (select private.es_admin_global()) then
    raise exception 'Sólo el panel puede cambiar este ajuste.' using errcode = '42501';
  end if;
  update public.ajustes_praind
     set carga_desde_equipos = coalesce(p_permitir, false),
         actualizado_por = (select auth.email()),
         actualizado_en = now()
   where id;
end;
$$;

revoke execute on function public.praind_permisos() from public, anon;
revoke execute on function public.praind_registrar(uuid, text, text, bigint, text, text, text) from public, anon;
revoke execute on function public.praind_quitar(uuid) from public, anon;
revoke execute on function public.praind_cambiar_carga_desde_equipos(boolean) from public, anon;

grant execute on function public.praind_permisos() to authenticated, service_role;
grant execute on function public.praind_registrar(uuid, text, text, bigint, text, text, text) to authenticated, service_role;
grant execute on function public.praind_quitar(uuid) to authenticated, service_role;
grant execute on function public.praind_cambiar_carga_desde_equipos(boolean) to authenticated, service_role;

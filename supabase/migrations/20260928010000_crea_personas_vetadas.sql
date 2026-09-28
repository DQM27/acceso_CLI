-- Veto de acceso por persona (cédula), válido en todas las puertas y en
-- todos los sitios. Ver docs/features-futuras/plan-veto-por-persona.md.
--
-- Antes, el único veto era `contratistas.activo` (en los equipos,
-- `tiene_acceso`): estaba atado al ROL, así que un contratista vetado
-- entraba como proveedor o como visita con la misma cédula.
--
-- Diseño:
-- - `normalizar_cedula`: gemela de `domain::cedula::Cedula::normalizar` del
--   núcleo Rust. Las dos se prueban con los mismos casos
--   (tests/vectores_cedula.tsv).
-- - `personas_vetadas`: una fila por veto. Levantar no borra; queda la
--   historia completa. Un solo veto vigente por cédula.
-- - Sólo el administrador del panel web veta o levanta (decisión del dueño,
--   2026-09-27), y sólo por las RPC `vetar_persona`/`levantar_veto`: nadie
--   tiene INSERT/UPDATE/DELETE directo.
-- - Los equipos leen sólo lo mínimo para bloquear (id, cédula, vigencia,
--   updated_at). El motivo es un dato sensible (Ley 8968): sólo lo ve el
--   administrador, por `listar_personas_vetadas`.
-- - Aviso en vivo a TODOS los sitios (el veto es global), con la fila
--   mínima, para que las porterías lo apliquen al instante.

create or replace function public.normalizar_cedula(p_texto text)
returns text
language plpgsql
immutable
set search_path to ''
as $$
declare
  v text;
begin
  if p_texto is null then
    return null;
  end if;
  v := pg_catalog.upper(pg_catalog.regexp_replace(p_texto, '[[:space:].-]', '', 'g'));
  if v = '' or v !~ '^[A-Z0-9]+$' or pg_catalog.length(v) > 20 then
    return null;
  end if;
  -- Formato del TSE con cero inicial (0X-XXXX-XXXX): es la misma cédula de
  -- 9 dígitos.
  if v ~ '^0[0-9]{9}$' then
    v := pg_catalog.substr(v, 2);
  end if;
  return v;
end;
$$;

revoke all on function public.normalizar_cedula(text) from public;
grant execute on function public.normalizar_cedula(text) to authenticated;

create table public.personas_vetadas (
  id uuid primary key default gen_random_uuid(),
  cedula text not null
    check (cedula = public.normalizar_cedula(cedula)),
  nombre_referencia text,
  motivo text not null
    check (pg_catalog.length(pg_catalog.btrim(motivo)) between 3 and 500),
  vetado_por uuid not null,
  vetado_por_correo text not null,
  vetado_en timestamptz not null default now(),
  levantado_por uuid,
  levantado_por_correo text,
  levantado_en timestamptz,
  motivo_levantamiento text
    check (motivo_levantamiento is null
           or pg_catalog.length(pg_catalog.btrim(motivo_levantamiento)) between 3 and 500),
  updated_at timestamptz not null default now(),
  check ((levantado_en is null) = (levantado_por is null)),
  check ((levantado_en is null) = (motivo_levantamiento is null))
);

comment on table public.personas_vetadas is
  'Veto de acceso por persona (cédula canónica), global a todos los sitios y todas las puertas. Sólo se escribe por vetar_persona/levantar_veto.';

-- Un solo veto vigente por persona.
create unique index personas_vetadas_una_vigente_por_cedula
  on public.personas_vetadas (cedula)
  where levantado_en is null;

-- Descarga incremental de los equipos (marca de agua por updated_at).
create index personas_vetadas_updated_at
  on public.personas_vetadas (updated_at, id);

create or replace function public.personas_vetadas_actualizar_updated_at()
returns trigger
language plpgsql
set search_path to 'public'
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create trigger personas_vetadas_actualizar_updated_at
  before update on public.personas_vetadas
  for each row execute function public.personas_vetadas_actualizar_updated_at();

-- Lectura: RLS abierta a cualquier sesión autenticada (equipos y
-- administradores), pero con permiso sólo sobre las columnas mínimas. Sin
-- permisos de escritura directa.
alter table public.personas_vetadas enable row level security;

create policy "leer personas vetadas (global)"
  on public.personas_vetadas
  for select
  to authenticated
  using (true);

revoke all on public.personas_vetadas from anon, authenticated;
grant select (id, cedula, levantado_en, updated_at)
  on public.personas_vetadas to authenticated;

-- ---- RPC del administrador ----

create or replace function public.vetar_persona(
  p_cedula text,
  p_nombre text,
  p_motivo text
)
returns uuid
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_cedula text;
  v_id uuid;
begin
  if not private.es_admin_global() then
    raise exception 'Sólo un administrador puede vetar a una persona'
      using errcode = '42501';
  end if;
  v_cedula := public.normalizar_cedula(p_cedula);
  if v_cedula is null then
    raise exception 'La cédula no es válida' using errcode = '22023';
  end if;
  if p_motivo is null or pg_catalog.length(pg_catalog.btrim(p_motivo)) < 3 then
    raise exception 'El motivo es obligatorio' using errcode = '22023';
  end if;
  if exists (
    select 1 from public.personas_vetadas
    where cedula = v_cedula and levantado_en is null
  ) then
    raise exception 'Esta persona ya tiene un veto vigente' using errcode = '23505';
  end if;

  insert into public.personas_vetadas (
    cedula, nombre_referencia, motivo, vetado_por, vetado_por_correo
  ) values (
    v_cedula,
    nullif(pg_catalog.btrim(p_nombre), ''),
    pg_catalog.btrim(p_motivo),
    auth.uid(),
    auth.email()
  )
  returning id into v_id;
  return v_id;
end;
$$;

create or replace function public.levantar_veto(
  p_cedula text,
  p_motivo text
)
returns uuid
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_cedula text;
  v_id uuid;
begin
  if not private.es_admin_global() then
    raise exception 'Sólo un administrador puede levantar un veto'
      using errcode = '42501';
  end if;
  v_cedula := public.normalizar_cedula(p_cedula);
  if v_cedula is null then
    raise exception 'La cédula no es válida' using errcode = '22023';
  end if;
  if p_motivo is null or pg_catalog.length(pg_catalog.btrim(p_motivo)) < 3 then
    raise exception 'El motivo es obligatorio' using errcode = '22023';
  end if;

  update public.personas_vetadas
     set levantado_por = auth.uid(),
         levantado_por_correo = auth.email(),
         levantado_en = now(),
         motivo_levantamiento = pg_catalog.btrim(p_motivo)
   where cedula = v_cedula and levantado_en is null
  returning id into v_id;

  if v_id is null then
    raise exception 'Esta persona no tiene un veto vigente' using errcode = 'P0002';
  end if;
  return v_id;
end;
$$;

-- Lista completa (con motivo) sólo para el administrador.
create or replace function public.listar_personas_vetadas(
  p_incluir_levantados boolean default false
)
returns table (
  id uuid,
  cedula text,
  nombre_referencia text,
  motivo text,
  vetado_por_correo text,
  vetado_en timestamptz,
  levantado_por_correo text,
  levantado_en timestamptz,
  motivo_levantamiento text
)
language plpgsql
stable
security definer
set search_path to ''
as $$
begin
  if not private.es_admin_global() then
    raise exception 'Sólo un administrador puede ver los vetos'
      using errcode = '42501';
  end if;
  return query
    select v.id, v.cedula, v.nombre_referencia, v.motivo, v.vetado_por_correo,
           v.vetado_en, v.levantado_por_correo, v.levantado_en, v.motivo_levantamiento
      from public.personas_vetadas v
     where p_incluir_levantados or v.levantado_en is null
     order by v.vetado_en desc;
end;
$$;

revoke all on function public.vetar_persona(text, text, text) from public, anon;
revoke all on function public.levantar_veto(text, text) from public, anon;
revoke all on function public.listar_personas_vetadas(boolean) from public, anon;
grant execute on function public.vetar_persona(text, text, text) to authenticated;
grant execute on function public.levantar_veto(text, text) to authenticated;
grant execute on function public.listar_personas_vetadas(boolean) to authenticated;

-- ---- Aviso en vivo a todos los sitios ----
--
-- `private.emitir_cambio_nube_sitio` avisa sólo al sitio de la fila; un
-- veto es global, así que se avisa a cada sitio por su canal privado. La
-- fila viaja con lo mínimo (sin motivo), igual que lo que los equipos
-- pueden leer por REST.
create or replace function private.emitir_cambio_nube_personas_vetadas()
returns trigger
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_sitio record;
  v_registro jsonb;
begin
  v_registro := pg_catalog.jsonb_build_object(
    'id', new.id,
    'cedula', new.cedula,
    'levantado_en', new.levantado_en,
    'updated_at', new.updated_at
  );
  for v_sitio in select s.id from public.sitios s loop
    perform realtime.send(
      pg_catalog.jsonb_build_object(
        'schema', tg_table_schema,
        'table', tg_table_name,
        'operation', tg_op,
        'sitio_id', v_sitio.id,
        'dispositivo_id', null,
        'changed_at', pg_catalog.now(),
        'id', new.id,
        'registro', v_registro
      ),
      'cambio_nube',
      'sitio:' || v_sitio.id::text,
      true
    );
  end loop;
  return null;
end;
$$;

create trigger personas_vetadas_emitir_cambio_nube
  after insert or update on public.personas_vetadas
  for each row execute function private.emitir_cambio_nube_personas_vetadas();

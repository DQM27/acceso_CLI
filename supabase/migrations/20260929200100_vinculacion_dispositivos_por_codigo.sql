-- Vinculación de dispositivos por código de un solo uso + clave pública
-- generada en el propio equipo (ver
-- docs/features-futuras/propuesta-registro-dispositivos.md, secciones 4.1 y
-- 4.2). Reemplaza al secreto permanente que el panel mostraba para copiar.
--
-- Flujo:
-- 1. El panel (admin-provision-device / admin-crear-codigo-vinculacion)
--    crea una fila en `codigos_vinculacion` con el SHA-256 del código,
--    vigencia corta y un solo uso.
-- 2. El equipo llama a `device-vincular` con el código y SU clave pública
--    (JWK P-256, sin la parte privada). La Edge Function llama a
--    `canjear_codigo_vinculacion`, que en UNA transacción quema el código y
--    ata la clave al dispositivo: dos equipos no pueden canjear el mismo
--    código ni en carrera.
-- 3. De ahí en adelante `device-auth` autentica al equipo verificando una
--    aserción firmada con la clave privada, que nunca sale del equipo.
--
-- `secret_hash` pasa a ser opcional: sólo lo conservan los equipos que
-- todavía no migraron (camino legado de `device-auth`). Al vincular o migrar
-- se pone en NULL, y ese secreto deja de servir para siempre.

alter table public.dispositivos
  alter column secret_hash drop not null,
  add column if not exists clave_publica_jwk jsonb,
  add column if not exists vinculado_en timestamptz;

-- Una clave pública sin huella (o al revés) sería un estado imposible de
-- autenticar; se exige que viajen juntas.
alter table public.dispositivos
  drop constraint if exists dispositivos_clave_completa;
alter table public.dispositivos
  add constraint dispositivos_clave_completa
  check ((clave_publica_jwk is null) = (clave_huella is null));

create table if not exists public.codigos_vinculacion (
  id uuid primary key default gen_random_uuid(),
  dispositivo_id uuid not null references public.dispositivos (id) on delete cascade,
  codigo_hash text not null unique,
  creado_por text not null,
  creado_en timestamptz not null default now(),
  expira_en timestamptz not null,
  usado_en timestamptz,
  usado_ip text,
  anulado_en timestamptz
);

create index if not exists codigos_vinculacion_dispositivo_idx
  on public.codigos_vinculacion (dispositivo_id);

-- Registro de intentos rechazados y señales raras, visible en el panel
-- (reemplaza al "registro forense" del plan anterior con una tabla simple).
create table if not exists public.eventos_seguridad_dispositivos (
  id bigint generated always as identity primary key,
  ocurrido_en timestamptz not null default now(),
  dispositivo_id uuid references public.dispositivos (id) on delete set null,
  tipo text not null check (tipo in (
    'codigo_inexistente',
    'codigo_usado',
    'codigo_vencido',
    'codigo_anulado',
    'codigo_de_otro_dispositivo',
    'firma_invalida',
    'hardware_distinto'
  )),
  ip text,
  detalle jsonb
);

create index if not exists eventos_seguridad_dispositivos_dispositivo_idx
  on public.eventos_seguridad_dispositivos (dispositivo_id);
create index if not exists eventos_seguridad_dispositivos_ocurrido_idx
  on public.eventos_seguridad_dispositivos (ocurrido_en desc);

-- Sin políticas permisivas a propósito: sólo las Edge Functions (service
-- role, que salta RLS) leen y escriben estas tablas. El panel las ve a
-- través de admin-list-devices.
alter table public.codigos_vinculacion enable row level security;
alter table public.eventos_seguridad_dispositivos enable row level security;

-- Misma política restrictiva que el resto de las tablas (ver
-- revocacion_efectiva_dispositivos.sql).
drop policy if exists "solo dispositivos vigentes" on public.codigos_vinculacion;
create policy "solo dispositivos vigentes" on public.codigos_vinculacion
  as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

drop policy if exists "solo dispositivos vigentes" on public.eventos_seguridad_dispositivos;
create policy "solo dispositivos vigentes" on public.eventos_seguridad_dispositivos
  as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

-- Canje atómico. Devuelve cero filas si el código no sirve (inexistente,
-- usado, vencido, anulado, de otro dispositivo, o el dispositivo ya fue
-- revocado); quien llama consulta el motivo aparte para registrarlo. Una
-- huella repetida (la misma clave pública para dos dispositivos) viola el
-- índice único y revierte todo, incluido el uso del código.
--
-- `p_dispositivo_esperado`: el equipo lo manda al re-vincularse con datos
-- locales ya cargados. Un código de OTRO dispositivo se rechaza sin
-- consumirse, para no atar esos datos a otro dispositivo u otro sitio.
create or replace function public.canjear_codigo_vinculacion(
  p_codigo_hash text,
  p_clave_publica_jwk jsonb,
  p_clave_huella text,
  p_metadata jsonb,
  p_ip text,
  p_dispositivo_esperado uuid default null
)
returns table (dispositivo_id uuid, sitio_id uuid, tipo text)
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_dispositivo_id uuid;
begin
  update public.codigos_vinculacion c
     set usado_en = pg_catalog.now(),
         usado_ip = p_ip
   where c.codigo_hash = p_codigo_hash
     and c.usado_en is null
     and c.anulado_en is null
     and c.expira_en > pg_catalog.now()
     and (p_dispositivo_esperado is null or c.dispositivo_id = p_dispositivo_esperado)
  returning c.dispositivo_id into v_dispositivo_id;

  if v_dispositivo_id is null then
    return;
  end if;

  -- La metadata se reemplaza completa: re-vincular es, por definición,
  -- otro equipo físico (o el mismo reinstalado).
  return query
  update public.dispositivos d
     set clave_publica_jwk = p_clave_publica_jwk,
         clave_huella = p_clave_huella,
         secret_hash = null,
         vinculado_en = pg_catalog.now(),
         last_seen_at = pg_catalog.now(),
         last_ip = p_ip,
         identificador_hardware = p_metadata ->> 'identificador_hardware',
         nombre_dispositivo = p_metadata ->> 'nombre_dispositivo',
         plataforma = p_metadata ->> 'plataforma',
         version_build = p_metadata ->> 'version_build',
         app_version = p_metadata ->> 'app_version'
   where d.id = v_dispositivo_id
     and d.revoked_at is null
  returning d.id, d.sitio_id, d.tipo;
end;
$$;

revoke all on function public.canjear_codigo_vinculacion(text, jsonb, text, jsonb, text, uuid)
  from public, anon, authenticated;
grant execute on function public.canjear_codigo_vinculacion(text, jsonb, text, jsonb, text, uuid)
  to service_role;

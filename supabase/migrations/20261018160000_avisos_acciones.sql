-- Quién archivó o borró un aviso en la bandeja del celular (ver
-- mobile/rust-core/src/avisos.rs). La bandeja vive sólo en el teléfono; esto
-- deja la constancia en el servidor para que el historial del panel
-- (`avisos_enviados`) muestre "borrado por ANA en Celular A1".
--
-- Una fila por (aviso, equipo, acción): el teléfono reintenta mientras no
-- reciba confirmación, y reenviar la misma acción no duplica nada.
--
-- `usuario` es el nombre de quien tenía la sesión abierta en el teléfono; lo
-- manda la app (el JWT del equipo no dice qué persona lo usa). La etiqueta del
-- equipo se copia y el equipo no lleva FK: la constancia tiene que seguir
-- leyéndose aunque el equipo se retire o se borre.
create table public.avisos_acciones (
  id uuid primary key default gen_random_uuid(),
  aviso_id uuid not null references public.avisos_enviados (id) on delete cascade,
  accion text not null check (accion in ('archivado', 'borrado')),
  dispositivo_id uuid not null,
  dispositivo_etiqueta text not null,
  usuario text not null check (pg_catalog.length(usuario) between 1 and 200),
  -- Cuándo lo hizo la persona (reloj del teléfono, nunca después de ahora) y
  -- cuándo lo supo el servidor (puede ser más tarde si no había red).
  ocurrio_en timestamptz not null,
  registrado_en timestamptz not null default now(),
  unique (aviso_id, dispositivo_id, accion)
);

create index avisos_acciones_aviso_idx on public.avisos_acciones (aviso_id);

comment on table public.avisos_acciones is
  'Constancia de quién archivó o borró un aviso en el celular. Sólo la escribe registrar_accion_aviso; la lee el administrador del panel.';

-- Ningún cliente escribe la tabla directo: sólo `registrar_accion_aviso`
-- (security definer, valida al equipo que llama). La lee el administrador.
alter table public.avisos_acciones enable row level security;
revoke all on public.avisos_acciones from anon, authenticated;
grant select on public.avisos_acciones to authenticated;

create policy "admin_global lee las acciones sobre avisos" on public.avisos_acciones
  for select to authenticated
  using ((select private.es_admin_global()));

-- Invariante de revocación efectiva (ver
-- 20260929200000_revocacion_efectiva_dispositivos): toda tabla nueva con RLS
-- la lleva.
create policy "solo dispositivos vigentes" on public.avisos_acciones
  as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

-- Anota la acción del equipo que llama. Un aviso que el servidor no conoce
-- (anterior al historial) no es un error: simplemente no hay a qué atar la
-- acción y el teléfono la da por enviada.
create or replace function public.registrar_accion_aviso(
  p_aviso_id uuid,
  p_accion text,
  p_usuario text,
  p_ocurrio_en timestamptz
)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
  v_usuario text := pg_catalog.btrim(p_usuario);
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede anotar una acción sobre un aviso' using errcode = '42501';
  end if;

  if p_accion is null or p_accion not in ('archivado', 'borrado') then
    raise exception 'Acción sobre aviso inválida' using errcode = '22023';
  end if;
  if v_usuario is null or pg_catalog.length(v_usuario) not between 1 and 200 then
    raise exception 'Usuario inválido' using errcode = '22023';
  end if;

  insert into public.avisos_acciones (aviso_id, accion, dispositivo_id, dispositivo_etiqueta, usuario, ocurrio_en)
  select p_aviso_id, p_accion, v_equipo.id, v_equipo.etiqueta, v_usuario,
         least(coalesce(p_ocurrio_en, pg_catalog.now()), pg_catalog.now())
   where exists (select 1 from public.avisos_enviados where id = p_aviso_id)
  on conflict (aviso_id, dispositivo_id, accion) do nothing;
end;
$$;

revoke all on function public.registrar_accion_aviso(uuid, text, text, timestamptz) from public, anon;
grant execute on function public.registrar_accion_aviso(uuid, text, text, timestamptz) to authenticated;

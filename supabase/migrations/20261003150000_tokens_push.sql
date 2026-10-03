-- Notificaciones push (Firebase Cloud Messaging): el token de cada teléfono,
-- la "dirección" a la que la Edge Function `enviar-push` manda los avisos
-- aunque la app esté cerrada (ver mobile/android/.../NotificacionesPush.kt).
--
-- Un token por equipo (`dispositivo_id` es la clave): FCM lo rota cada tanto
-- (reinstalación, datos borrados) y la app lo vuelve a registrar al iniciar
-- sesión, pisando el anterior. `sitio_id` va copiado del equipo para poder
-- avisar a todos los teléfonos de una unidad sin un join.
create table public.tokens_push (
  dispositivo_id uuid primary key references public.dispositivos (id) on delete cascade,
  sitio_id uuid not null references public.sitios (id) on delete cascade,
  -- Un mismo token no puede quedar en dos equipos (teléfono revinculado como
  -- equipo nuevo): `registrar_token_push` se lo quita al anterior.
  token text not null unique,
  plataforma text not null default 'android' check (plataforma in ('android')),
  actualizado_en timestamptz not null default now()
);

create index tokens_push_sitio_idx on public.tokens_push (sitio_id);

comment on table public.tokens_push is
  'Token FCM de cada equipo para notificaciones push. Sólo lo escribe registrar_token_push; lo lee la Edge Function enviar-push con service_role.';

-- Ningún cliente escribe la tabla directo: sólo `registrar_token_push`
-- (security definer, valida al equipo que llama). La lee el administrador
-- del panel; la Edge Function usa service_role y no pasa por RLS.
alter table public.tokens_push enable row level security;
revoke all on public.tokens_push from anon, authenticated;
grant select on public.tokens_push to authenticated;

create policy "admin_global lee los tokens push" on public.tokens_push
  for select to authenticated
  using ((select private.es_admin_global()));

-- Invariante de revocación efectiva (ver
-- 20260929200000_revocacion_efectiva_dispositivos): toda tabla nueva con RLS
-- la lleva.
create policy "solo dispositivos vigentes" on public.tokens_push
  as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

-- Registra (o actualiza) el token FCM del equipo que llama. Sólo un
-- dispositivo vinculado y vigente (`private.dispositivo_que_llama`).
create or replace function public.registrar_token_push(p_token text)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
  v_token text := pg_catalog.btrim(p_token);
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede registrar un token push' using errcode = '42501';
  end if;

  -- Los tokens de FCM rondan los 150-200 caracteres; el tope evita guardar
  -- basura arbitraria.
  if v_token is null or pg_catalog.length(v_token) not between 20 and 4096 then
    raise exception 'Token push inválido' using errcode = '22023';
  end if;

  delete from public.tokens_push
   where token = v_token and dispositivo_id <> v_equipo.id;

  insert into public.tokens_push (dispositivo_id, sitio_id, token)
  values (v_equipo.id, v_equipo.sitio_id, v_token)
  on conflict (dispositivo_id) do update
    set token = excluded.token,
        sitio_id = excluded.sitio_id,
        actualizado_en = pg_catalog.now();
end;
$$;

revoke all on function public.registrar_token_push(text) from public, anon;
grant execute on function public.registrar_token_push(text) to authenticated;

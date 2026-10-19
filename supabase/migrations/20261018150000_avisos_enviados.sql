-- Historial de los avisos push que manda el panel (Edge Function
-- `admin-enviar-push`): qué se mandó, quién lo mandó, a qué destino y a
-- cuántos equipos llegó. Antes no quedaba registro de nada.
--
-- El `id` viaja también en el mensaje (campo `id` de `data`, ver
-- supabase/functions/_shared/fcm.ts): el teléfono lo usa para no guardar dos
-- veces el mismo aviso en su bandeja.
--
-- `no_recibieron`: etiquetas de los equipos a los que FCM no les pudo
-- entregar (error o token muerto), para saber a qué portería avisarle por
-- otro medio. Se guarda la etiqueta y no el id: el historial tiene que
-- seguir leyéndose aunque el equipo se retire o se borre.
create table public.avisos_enviados (
  id uuid primary key default gen_random_uuid(),
  titulo text not null check (pg_catalog.length(titulo) between 1 and 100),
  cuerpo text not null check (pg_catalog.length(cuerpo) between 1 and 500),
  tipo text not null check (tipo in ('normal', 'emergente')),
  -- Tal cual lo eligió el administrador: {"todos": true} o
  -- {"sitio_ids": [...], "dispositivo_ids": [...]}.
  destino jsonb not null,
  enviado_por text not null,
  enviado_en timestamptz not null default now(),
  destinatarios integer not null default 0 check (destinatarios >= 0),
  enviados integer not null default 0 check (enviados >= 0),
  no_recibieron text[] not null default '{}'
);

create index avisos_enviados_enviado_en_idx on public.avisos_enviados (enviado_en desc);

comment on table public.avisos_enviados is
  'Historial de avisos push del panel. Sólo lo escribe la Edge Function admin-enviar-push con service_role; lo lee el administrador del panel.';

-- Ningún cliente escribe la tabla: sólo la Edge Function (service_role, no
-- pasa por RLS). La lee el administrador del panel.
alter table public.avisos_enviados enable row level security;
revoke all on public.avisos_enviados from anon, authenticated;
grant select on public.avisos_enviados to authenticated;

create policy "admin_global lee los avisos enviados" on public.avisos_enviados
  for select to authenticated
  using ((select private.es_admin_global()));

-- Invariante de revocación efectiva (ver
-- 20260929200000_revocacion_efectiva_dispositivos): toda tabla nueva con RLS
-- la lleva.
create policy "solo dispositivos vigentes" on public.avisos_enviados
  as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

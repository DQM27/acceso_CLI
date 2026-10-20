-- Órdenes de telemetría por equipo (ver
-- docs/features-futuras/plan-telemetria-y-registro-de-errores.md, fase 1).
--
-- El administrador del panel enciende y apaga ESTUDIOS de telemetría
-- (grupos de eventos) en cada equipo, sin sacar un release. Esta migración es
-- sólo la parte de la nube:
--
-- 1. `telemetria_ordenes`: una fila por equipo con los grupos encendidos, el
--    vencimiento y un número de versión. El equipo sólo aplica una versión
--    más nueva que la que ya tiene.
-- 2. `telemetria_ordenes_historial`: quién encendió qué y cuándo, para cruzar
--    un cambio en los datos con la orden que lo causó. Sobrevive al retiro
--    del equipo.
-- 3. Un trigger que avisa por Realtime (`telemetria_config`, canal
--    `sitio:<id>`), igual que `dispositivo_expulsado`. El equipo además lee su
--    fila al sincronizar, por si estaba apagado o sin red cuando se dio la
--    orden.
--
-- Qué NO controla: `base` (estado de la flota y caídas) va siempre y no se
-- guarda acá, y los errores van siempre encendidos y no pasan por esta tabla.
-- Un equipo sin fila mide `base` y `rendimiento`; la fila lista TODOS los
-- grupos además de `base` que el equipo debe medir (el panel la crea con
-- `rendimiento` marcado).
--
-- Seguridad: escribe sólo el administrador del panel (`es_admin_global`) y cada
-- equipo lee únicamente su propia fila. Lo peor que puede hacer una orden es
-- encender mediciones técnicas: no toca datos de operación.

create sequence if not exists public.telemetria_ordenes_version_seq;

-- Sólo la usa el trigger (`security definer`, corre como dueño). Sin esto la
-- secuencia hereda los permisos automáticos de `public` y cualquier rol de la
-- API podría avanzarla y saltar versiones.
revoke all on sequence public.telemetria_ordenes_version_seq from public, anon, authenticated, service_role;

create table public.telemetria_ordenes (
  dispositivo_id uuid primary key references public.dispositivos (id) on delete cascade,
  -- Lista fija; cambiarla es una migración, igual que agregar un grupo a la app.
  grupos text[] not null default '{rendimiento}'
    check (grupos <@ array[
      'rendimiento', 'fluidez', 'sincronizacion', 'recursos', 'escaneo', 'depuracion'
    ]::text[]),
  -- Nulo = sin vencimiento. El equipo lo hace cumplir solo, aunque no tenga red.
  vence_en timestamptz,
  -- La pone el trigger (secuencia global): crece en cada cambio, también si la
  -- fila se borra y se vuelve a crear. El cliente no la elige.
  version bigint not null default 0,
  nota text not null default '' check (pg_catalog.length(nota) <= 200),
  actualizado_en timestamptz not null default now(),
  -- Quién la cambió: la pone el trigger con el correo de la sesión.
  actualizado_por text not null default ''
);

comment on table public.telemetria_ordenes is
  'Estudios de telemetría encendidos por equipo. Los escribe el administrador del panel; cada equipo lee la suya. Sin fila: base y rendimiento. Ver docs/features-futuras/plan-telemetria-y-registro-de-errores.md.';

create table public.telemetria_ordenes_historial (
  id bigint generated always as identity primary key,
  -- Sin clave foránea a propósito: el historial sigue leyéndose aunque el
  -- equipo se retire o se borre (por eso también guarda la etiqueta).
  dispositivo_id uuid not null,
  etiqueta text,
  grupos text[] not null,
  vence_en timestamptz,
  version bigint not null,
  nota text not null,
  cambiado_por text not null,
  cambiado_en timestamptz not null default now()
);

create index telemetria_ordenes_historial_dispositivo_idx
  on public.telemetria_ordenes_historial (dispositivo_id, cambiado_en desc);

comment on table public.telemetria_ordenes_historial is
  'Quién cambió las órdenes de telemetría de cada equipo y cuándo. Lo escribe el trigger; lo lee el administrador del panel.';

-- Version, autor y hora las decide el servidor, nunca el cliente.
create or replace function private.sellar_orden_telemetria()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  new.version := pg_catalog.nextval('public.telemetria_ordenes_version_seq');
  new.actualizado_en := pg_catalog.now();
  new.actualizado_por := coalesce(nullif((select auth.email()), ''), current_user);
  return new;
end;
$$;

revoke all on function private.sellar_orden_telemetria() from public;

create trigger telemetria_ordenes_sellar
  before insert or update on public.telemetria_ordenes
  for each row execute function private.sellar_orden_telemetria();

-- Historial + aviso en vivo. Después de sellar, así el aviso lleva la versión
-- definitiva. El aviso no lleva la nota (es para el panel, no para el equipo).
create or replace function private.registrar_y_avisar_orden_telemetria()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_sitio_id uuid;
  v_etiqueta text;
begin
  select d.sitio_id, d.etiqueta
    into v_sitio_id, v_etiqueta
    from public.dispositivos d
   where d.id = new.dispositivo_id;

  insert into public.telemetria_ordenes_historial
    (dispositivo_id, etiqueta, grupos, vence_en, version, nota, cambiado_por, cambiado_en)
  values
    (new.dispositivo_id, v_etiqueta, new.grupos, new.vence_en, new.version,
     new.nota, new.actualizado_por, new.actualizado_en);

  if v_sitio_id is not null then
    perform realtime.send(
      pg_catalog.jsonb_build_object(
        'dispositivo_id', new.dispositivo_id,
        'grupos', new.grupos,
        'vence_en', new.vence_en,
        'version', new.version
      ),
      'telemetria_config',
      'sitio:' || v_sitio_id::text,
      true
    );
  end if;
  return null;
end;
$$;

revoke all on function private.registrar_y_avisar_orden_telemetria() from public;

create trigger telemetria_ordenes_registrar_y_avisar
  after insert or update on public.telemetria_ordenes
  for each row execute function private.registrar_y_avisar_orden_telemetria();

alter table public.telemetria_ordenes enable row level security;
alter table public.telemetria_ordenes_historial enable row level security;

revoke all on public.telemetria_ordenes from anon, authenticated;
revoke all on public.telemetria_ordenes_historial from anon, authenticated;
-- Sin DELETE: para volver al estado por defecto se actualiza la fila.
grant select, insert, update on public.telemetria_ordenes to authenticated;
grant select on public.telemetria_ordenes_historial to authenticated;
-- Explícito y no heredado de los permisos automáticos de `public`, que
-- Supabase deja de dar a las tablas nuevas: las Edge Functions de
-- administración usan el rol de servicio (salta RLS, pero no los GRANT).
grant select, insert, update on public.telemetria_ordenes to service_role;
grant select on public.telemetria_ordenes_historial to service_role;

-- Lee el administrador del panel, o el equipo dueño de la fila. Una sola
-- política para no sumar políticas permisivas del mismo comando. Para un JWT
-- sin `sitio_id` (sesión humana) el equipo no coincide con nada.
create policy "lee el administrador o la propia unidad" on public.telemetria_ordenes
  for select to authenticated
  using (
    (select private.es_admin_global())
    or case
      when ((select auth.jwt()) ->> 'sitio_id') is null then false
      else dispositivo_id = ((select auth.jwt()) ->> 'sub')::uuid
    end
  );

create policy "admin_global crea ordenes de telemetria" on public.telemetria_ordenes
  for insert to authenticated
  with check ((select private.es_admin_global()));

create policy "admin_global cambia ordenes de telemetria" on public.telemetria_ordenes
  for update to authenticated
  using ((select private.es_admin_global()))
  with check ((select private.es_admin_global()));

create policy "admin_global lee el historial de telemetria" on public.telemetria_ordenes_historial
  for select to authenticated
  using ((select private.es_admin_global()));

-- Invariante de revocación efectiva (ver
-- 20260929200000_revocacion_efectiva_dispositivos): toda tabla nueva con RLS
-- la lleva.
create policy "solo dispositivos vigentes" on public.telemetria_ordenes
  as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

create policy "solo dispositivos vigentes" on public.telemetria_ordenes_historial
  as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

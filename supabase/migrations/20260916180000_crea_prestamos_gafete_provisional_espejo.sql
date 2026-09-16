-- Espejo en la nube de `prestamos_gafete_provisional` (local, MIGRACION_39/40)
-- -- mismo motivo que el espejo de `movimientos_visita`: sin esto, un
-- préstamo de gafete provisional KOF sólo vivía en el SQLite del
-- dispositivo que lo entregó, invisible para otros dispositivos del mismo
-- sitio (que necesitan saber si ese número ya está prestado antes de
-- entregar el mismo) y para el admin que audita todos los sitios. Mismas
-- columnas/garantías/políticas que `movimientos_visita`, adaptado al
-- vocabulario de este módulo (entrega/devolución en vez de entrada/salida).
-- Sin índice único de `gafete_numero` a propósito -- igual que
-- `movimientos_visita`, la unicidad real es sólo local (índice parcial en
-- SQLite) y se "simula" contra la nube con una consulta en vivo
-- (`gafete_provisional_ocupado_en_otro_dispositivo`).

create table public.prestamos_gafete_provisional (
  id uuid primary key,
  sitio_id uuid not null references public.sitios(id),
  dispositivo_entrega_id uuid not null references public.dispositivos(id),
  encargado_id uuid not null references public.encargados_ruta(id),
  encargado_nombre text not null,
  encargado_codigo_empleado text not null,
  gafete_numero bigint not null,
  hora_entrega timestamptz not null,
  usuario_entrega_nombre text not null,
  hora_devolucion timestamptz,
  dispositivo_devolucion_id uuid references public.dispositivos(id),
  usuario_devolucion_nombre text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index idx_prestamos_gafete_provisional_sitio on public.prestamos_gafete_provisional(sitio_id);
create index idx_prestamos_gafete_provisional_encargado on public.prestamos_gafete_provisional(encargado_id);
create index idx_prestamos_gafete_provisional_dispositivo_entrega
  on public.prestamos_gafete_provisional(dispositivo_entrega_id);
create index idx_prestamos_gafete_provisional_dispositivo_devolucion
  on public.prestamos_gafete_provisional(dispositivo_devolucion_id);

alter table public.prestamos_gafete_provisional enable row level security;

-- Mismo criterio que "crear movimientos_visita del propio sitio": un
-- dispositivo sólo escribe en su propio sitio, y un secreto tipo "visor"
-- (sólo lectura) no puede registrar entregas.
create policy "crear prestamos_gafete_provisional del propio sitio"
  on public.prestamos_gafete_provisional
  for insert
  to authenticated
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "leer prestamos_gafete_provisional del propio sitio o admin"
  on public.prestamos_gafete_provisional
  for select
  to authenticated
  using (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or public.es_admin_global()
  );

create policy "actualizar prestamos_gafete_provisional del propio sitio"
  on public.prestamos_gafete_provisional
  for update
  to authenticated
  using (sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid)
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create or replace function public.prestamos_gafete_provisional_actualizar_updated_at()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

-- Mismas dos garantías que ya tiene `movimientos_visita`: los datos de
-- entrega no se editan después de creados, la devolución se registra una
-- sola vez.
create or replace function public.prestamos_gafete_provisional_bloquear_cambios_de_entrega()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  if new.sitio_id is distinct from old.sitio_id
     or new.dispositivo_entrega_id is distinct from old.dispositivo_entrega_id
     or new.encargado_id is distinct from old.encargado_id
     or new.encargado_nombre is distinct from old.encargado_nombre
     or new.encargado_codigo_empleado is distinct from old.encargado_codigo_empleado
     or new.gafete_numero is distinct from old.gafete_numero
     or new.hora_entrega is distinct from old.hora_entrega
     or new.usuario_entrega_nombre is distinct from old.usuario_entrega_nombre
     or new.created_at is distinct from old.created_at
  then
    raise exception 'Los datos de entrega de un prestamo de gafete provisional son inmutables';
  end if;
  return new;
end;
$$;

create or replace function public.prestamos_gafete_provisional_bloquear_doble_devolucion()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  if old.hora_devolucion is not null and new.hora_devolucion is distinct from old.hora_devolucion then
    raise exception 'La devolucion de un prestamo de gafete provisional solo puede registrarse una vez';
  end if;
  return new;
end;
$$;

create trigger prestamos_gafete_provisional_set_updated_at
  before update on public.prestamos_gafete_provisional
  for each row execute function public.prestamos_gafete_provisional_actualizar_updated_at();

create trigger prestamos_gafete_provisional_entrega_inmutable
  before update on public.prestamos_gafete_provisional
  for each row execute function public.prestamos_gafete_provisional_bloquear_cambios_de_entrega();

create trigger prestamos_gafete_provisional_devolucion_unica
  before update on public.prestamos_gafete_provisional
  for each row execute function public.prestamos_gafete_provisional_bloquear_doble_devolucion();

-- Mismo canal realtime que ya usan `ingresos`/`movimientos_visita` -- sin
-- costo extra habilitarlo ahora, aunque este módulo todavía no tenga una
-- vista tipo "Activos" que lo consuma en vivo.
create trigger prestamos_gafete_provisional_emitir_cambio_nube
  after insert or delete or update on public.prestamos_gafete_provisional
  for each row execute function private.emitir_cambio_nube_sitio();

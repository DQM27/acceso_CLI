-- Espejo en la nube de `movimientos_visita` (local, MIGRACION_28/31) --
-- cierra el hueco de trazabilidad: sin esto, un movimiento de visita sólo
-- vivía en el SQLite del dispositivo que hizo el check-in/check-out,
-- invisible para otros dispositivos del mismo sitio y para el admin que
-- audita todos los sitios. Mismas columnas/garantías/políticas que
-- `ingresos` (contratistas), adaptado a visitas: snapshot del visitante en
-- la fila (igual que `ingresos.contratista_nombre`), no un JOIN contra
-- `cita_visitantes`/`citas` en cada lectura.

create table public.movimientos_visita (
  id uuid primary key,
  sitio_id uuid not null references public.sitios(id),
  dispositivo_entrada_id uuid not null references public.dispositivos(id),
  cita_visitante_id uuid not null references public.cita_visitantes(id),
  visitante_cedula text not null,
  visitante_nombre text not null,
  empresa text,
  anfitrion_nombre text,
  motivo text,
  gafete_numero bigint,
  hora_entrada timestamptz not null,
  usuario_entrada_nombre text not null,
  hora_salida timestamptz,
  dispositivo_salida_id uuid references public.dispositivos(id),
  usuario_salida_nombre text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index idx_movimientos_visita_sitio on public.movimientos_visita(sitio_id);
create index idx_movimientos_visita_cita_visitante on public.movimientos_visita(cita_visitante_id);

alter table public.movimientos_visita enable row level security;

-- Mismo criterio que "crear ingresos del propio sitio": un dispositivo
-- sólo escribe en su propio sitio, y un secreto tipo "visor" (sólo
-- lectura) no puede registrar movimientos.
create policy "crear movimientos_visita del propio sitio"
  on public.movimientos_visita
  for insert
  to authenticated
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "leer movimientos_visita del propio sitio o admin"
  on public.movimientos_visita
  for select
  to authenticated
  using (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or public.es_admin_global()
  );

create policy "actualizar movimientos_visita del propio sitio"
  on public.movimientos_visita
  for update
  to authenticated
  using (sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid)
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create or replace function public.movimientos_visita_actualizar_updated_at()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

-- Mismas cuatro garantías que ya tiene `ingresos`: los datos de entrada no
-- se editan después de creados, la salida se registra una sola vez.
create or replace function public.movimientos_visita_bloquear_cambios_de_entrada()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  if new.sitio_id is distinct from old.sitio_id
     or new.dispositivo_entrada_id is distinct from old.dispositivo_entrada_id
     or new.cita_visitante_id is distinct from old.cita_visitante_id
     or new.visitante_cedula is distinct from old.visitante_cedula
     or new.visitante_nombre is distinct from old.visitante_nombre
     or new.gafete_numero is distinct from old.gafete_numero
     or new.hora_entrada is distinct from old.hora_entrada
     or new.usuario_entrada_nombre is distinct from old.usuario_entrada_nombre
     or new.created_at is distinct from old.created_at
  then
    raise exception 'Los datos de entrada de un movimiento de visita son inmutables';
  end if;
  return new;
end;
$$;

create or replace function public.movimientos_visita_bloquear_doble_cierre()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  if old.hora_salida is not null and new.hora_salida is distinct from old.hora_salida then
    raise exception 'La salida de un movimiento de visita solo puede registrarse una vez';
  end if;
  return new;
end;
$$;

create trigger movimientos_visita_set_updated_at
  before update on public.movimientos_visita
  for each row execute function public.movimientos_visita_actualizar_updated_at();

create trigger movimientos_visita_entrada_inmutable
  before update on public.movimientos_visita
  for each row execute function public.movimientos_visita_bloquear_cambios_de_entrada();

create trigger movimientos_visita_salida_unica
  before update on public.movimientos_visita
  for each row execute function public.movimientos_visita_bloquear_doble_cierre();

-- Mismo canal realtime que ya usan `ingresos`/`citas` -- sin costo extra
-- habilitarlo ahora, aunque "Visitas" todavía no tenga una vista tipo
-- "Activos" que lo consuma en vivo.
create trigger movimientos_visita_emitir_cambio_nube
  after insert or delete or update on public.movimientos_visita
  for each row execute function private.emitir_cambio_nube_sitio();

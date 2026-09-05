-- Catálogo de gafetes (espejo): sólo el estado actual -- no el historial de
-- incidentes (gafetes_incidentes local), eso queda para más adelante si
-- hace falta. `numero` único por sitio, no global: dos sitios distintos
-- pueden tener cada uno su propio "gafete #5", son catálogos
-- independientes.
create table gafetes (
  id uuid primary key,
  sitio_id uuid not null references sitios(id),
  dispositivo_origen_id uuid not null references dispositivos(id),
  numero bigint not null,
  estado text not null check (estado in ('DISPONIBLE', 'PERDIDO', 'DE_BAJA')),
  contratista_deudor_id uuid references contratistas(id),
  contratista_deudor_nombre text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create unique index gafetes_sitio_numero_idx on gafetes(sitio_id, numero);
create index gafetes_sitio_idx on gafetes(sitio_id);

alter table gafetes enable row level security;

create policy "leer gafetes del propio sitio"
on gafetes for select to authenticated
using (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

create policy "crear gafetes del propio sitio"
on gafetes for insert to authenticated
with check (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

create policy "actualizar gafetes del propio sitio"
on gafetes for update to authenticated
using (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid)
with check (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

-- Mismo criterio que ingresos: la base pone updated_at, no quien manda el
-- pedido.
create or replace function gafetes_actualizar_updated_at()
returns trigger
language plpgsql
set search_path = public
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create trigger gafetes_set_updated_at
before update on gafetes
for each row execute function gafetes_actualizar_updated_at();

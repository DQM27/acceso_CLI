-- empresas: tabla propia en el espejo, mismo patrón que contratistas.
create table empresas (
  id uuid primary key,
  sitio_id uuid not null references sitios(id),
  dispositivo_origen_id uuid not null references dispositivos(id),
  nombre text not null,
  activa boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create index empresas_sitio_idx on empresas(sitio_id);
alter table empresas enable row level security;

create policy "leer empresas del propio sitio"
on empresas for select to authenticated
using (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

create policy "crear empresas del propio sitio"
on empresas for insert to authenticated
with check (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

create policy "actualizar empresas del propio sitio"
on empresas for update to authenticated
using (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid)
with check (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

alter publication supabase_realtime add table empresas;

-- contratistas: referencia a su empresa (id + nombre, mismo criterio de
-- snapshot que ya usa la base local para no depender de un JOIN en vivo).
alter table contratistas add column empresa_id uuid references empresas(id);
alter table contratistas add column empresa_nombre text;

-- ingresos: nunca se sincroniza la tabla usuarios (contraseñas) -- alcanza
-- con el nombre de quien registró entrada/salida, como texto plano.
alter table ingresos add column usuario_entrada_nombre text;
alter table ingresos add column usuario_salida_nombre text;

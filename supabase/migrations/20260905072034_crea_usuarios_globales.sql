-- Operadores/administradores globales (docs/plan-panel-administrativo-web.md,
-- punto 2 de "usuarios/operadores tampoco se espejan a la nube"). ROOT
-- queda deliberadamente afuera -- "Root inicial y login offline: sin
-- cambios", cada sitio lo sigue creando local con crear_root_inicial. Sin
-- password_hash a propósito: la nube distribuye quién existe y su rol/
-- estado, nunca la contraseña (ver esa misma sección del plan) -- el hash
-- se fija localmente en cada dispositivo (ver
-- AutenticacionError::SinPasswordLocal / SIN_PASSWORD_LOCAL, ya
-- implementado del lado de Rust).
create table public.usuarios (
  id uuid primary key default gen_random_uuid(),
  sitio_id uuid not null references public.sitios(id),
  dispositivo_origen_id uuid references public.dispositivos(id),
  cedula text not null unique,
  nombre text not null,
  rol text not null check (rol in ('ADMINISTRADOR', 'OPERADOR')),
  activo boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

comment on table public.usuarios is
  'Operadores/administradores globales -- ROOT nunca viaja acá (queda 100% local a cada sitio). Sin password_hash: eso se fija por dispositivo, ver SIN_PASSWORD_LOCAL en src/services/password.rs.';

alter table public.usuarios enable row level security;

-- Mismo criterio que contratistas/empresas: cualquier dispositivo
-- autenticado lee/actualiza global (para que una baja propague a todos los
-- sitios), pero sólo crea acotado a su propio sitio (procedencia).
create policy "leer usuarios (global)"
  on public.usuarios
  for select
  to authenticated
  using (true);

create policy "crear usuarios del propio sitio"
  on public.usuarios
  for insert
  to authenticated
  with check (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

create policy "actualizar usuarios (global)"
  on public.usuarios
  for update
  to authenticated
  using (true)
  with check (true);

-- Panel web (admin_global) -- mismo patrón que administradores_panel/
-- contratistas.
create policy "admin_global lee usuarios"
  on public.usuarios
  for select
  to authenticated
  using (public.es_admin_global());

create policy "admin_global gestiona usuarios"
  on public.usuarios
  for update
  to authenticated
  using (public.es_admin_global())
  with check (public.es_admin_global());

create table public.administradores_panel (
  correo text primary key,
  rol text not null check (rol in ('admin_global', 'admin_regional')),
  creado_en timestamptz not null default now()
);

comment on table public.administradores_panel is
  'Quién puede entrar al panel administrativo web (distinto de Root/Administrador/Operador de cada sitio, ver docs/plan-panel-administrativo-web.md). El login con Google solo confirma identidad -- esta tabla decide autorización real.';

alter table public.administradores_panel enable row level security;

-- Cada quien puede leer únicamente su propia fila -- así el panel, ya
-- logueado con Google, puede chequear "¿estoy autorizado?" sin exponer la
-- lista completa de admins a cualquier sesión autenticada.
create policy "cada admin lee su propia fila"
  on public.administradores_panel
  for select
  to authenticated
  using (auth.email() = correo);

-- Alta/baja de admins queda fuera de esta migración a propósito -- sin
-- política de insert/update/delete todavía, se gestiona con service_role
-- (dashboard/SQL directo) hasta que exista una pantalla para eso.
insert into public.administradores_panel (correo, rol)
values ('daniel.bleach11@gmail.com', 'admin_global');

-- Ver docs/plan-autenticacion-supabase-auth.md -- reemplaza el modelo de
-- "contraseña local por dispositivo" (SIN_PASSWORD_LOCAL) por login siempre
-- contra Supabase Auth. El hash real vive en auth.users, nunca en esta tabla
-- -- esta columna sólo enlaza la identidad global con su cuenta de Auth.
alter table public.usuarios
  add column auth_user_id uuid references auth.users(id) on delete set null;

comment on column public.usuarios.auth_user_id is
  'Enlace a auth.users -- la contraseña real vive ahí (Supabase Auth), nunca en esta tabla. NULL sólo transitoriamente durante el backfill de usuarios creados antes de este cambio.';

create unique index usuarios_auth_user_id_key on public.usuarios (auth_user_id)
  where auth_user_id is not null;

comment on table public.usuarios is
  'Usuarios globales (ROOT/ADMINISTRADOR/OPERADOR). Identidad/rol/estado acá; la contraseña vive en Supabase Auth (auth_user_id), ver docs/plan-autenticacion-supabase-auth.md. SIN_PASSWORD_LOCAL queda deprecado.';

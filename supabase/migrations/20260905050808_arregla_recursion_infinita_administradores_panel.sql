-- Las políticas de la migración anterior se preguntaban a sí mismas
-- ("¿este correo es admin_global?") consultando la misma tabla que
-- protegen -- Postgres lo detecta como recursión infinita (42P17) y
-- rompe CUALQUIER select sobre la tabla, no sólo el del admin_global.
-- Fix estándar de Supabase: mover el chequeo a una función
-- `security definer`, que corre sin RLS por dentro, rompiendo el loop.
create or replace function public.es_admin_global()
returns boolean
language sql
security definer
set search_path = public
stable
as $$
  select exists (
    select 1 from public.administradores_panel
    where correo = auth.email() and rol = 'admin_global'
  );
$$;

revoke all on function public.es_admin_global() from public;
grant execute on function public.es_admin_global() to authenticated;

drop policy "admin_global ve todos los admins" on public.administradores_panel;
drop policy "admin_global agrega admins" on public.administradores_panel;
drop policy "admin_global borra otros admins" on public.administradores_panel;

create policy "admin_global ve todos los admins"
  on public.administradores_panel
  for select
  to authenticated
  using (public.es_admin_global());

create policy "admin_global agrega admins"
  on public.administradores_panel
  for insert
  to authenticated
  with check (public.es_admin_global());

create policy "admin_global borra otros admins"
  on public.administradores_panel
  for delete
  to authenticated
  using (correo != auth.email() and public.es_admin_global());

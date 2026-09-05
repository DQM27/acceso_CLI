-- Un admin_global necesita ver/agregar/borrar filas de OTROS admins, no
-- solo la propia (la política de la migración anterior). Postgres evalúa
-- varias políticas permisivas del mismo comando con OR, así que esto se
-- suma a "cada admin lee su propia fila" sin reemplazarla.
create policy "admin_global ve todos los admins"
  on public.administradores_panel
  for select
  to authenticated
  using (
    exists (
      select 1 from public.administradores_panel a
      where a.correo = auth.email() and a.rol = 'admin_global'
    )
  );

create policy "admin_global agrega admins"
  on public.administradores_panel
  for insert
  to authenticated
  with check (
    exists (
      select 1 from public.administradores_panel a
      where a.correo = auth.email() and a.rol = 'admin_global'
    )
  );

-- No se puede borrar la propia fila desde acá -- evita que un admin_global
-- se quede afuera por accidente sin nadie más que pueda re-agregarlo salvo
-- entrando directo a Supabase. Si hace falta bajar al último admin_global,
-- se hace a mano por SQL, a propósito (no vía la UI de todos los días).
create policy "admin_global borra otros admins"
  on public.administradores_panel
  for delete
  to authenticated
  using (
    correo != auth.email()
    and exists (
      select 1 from public.administradores_panel a
      where a.correo = auth.email() and a.rol = 'admin_global'
    )
  );

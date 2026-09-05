-- `contratistas` hoy sólo la leen/escriben los dispositivos (RLS scoped
-- por sitio_id del JWT del dispositivo) -- el panel web necesita su propio
-- acceso, con el mismo criterio que ya usamos en administradores_panel/
-- ingresos/sitios (es_admin_global()). Sin filtro por sitio a propósito:
-- el modelo de contratistas ya se decidió global (ver
-- docs/plan-panel-administrativo-web.md, "Modelo de datos") -- admin_global
-- ve y da de baja desde cualquier sitio.
create policy "admin_global lee contratistas"
  on public.contratistas
  for select
  to authenticated
  using (public.es_admin_global());

create policy "admin_global actualiza contratistas"
  on public.contratistas
  for update
  to authenticated
  using (public.es_admin_global())
  with check (public.es_admin_global());

-- multiple_permissive_policies (Performance Advisor, 4 tablas): cada par de políticas
-- listadas abajo son ambas "permissive" para el mismo role+acción -- Postgres las
-- evalúa como OR implícito, pero paga el costo de planear/ejecutar dos políticas en
-- vez de una. Se fusiona cada par en una sola política con el mismo OR explícito --
-- CERO cambio de semántica, verificado antes y después con los diagnósticos de
-- supabase/tests/dispositivos_autorizacion.sql, usuarios_autorizacion.sql y
-- sitios_autorizacion.sql corridos a mano contra esta misma base (ver commit).
--
-- dispositivos y usuarios están publicadas a Realtime -- fusionar políticas de
-- SELECT/INSERT no cambia qué filas ve o puede escribir cada rol (mismo resultado,
-- una sola evaluación), así que no afecta qué le llega a cada cliente por
-- Postgres Changes.

drop policy "cada anfitrion lee su propia fila" on public.anfitriones;
drop policy "dispositivo del sitio lee el anfitrion de sus citas" on public.anfitriones;
create policy "leer anfitriones (propia fila o dispositivo del sitio de su cita)"
  on public.anfitriones
  for select
  to authenticated
  using (
    ((select auth.email()) = correo and activo)
    or exists (
      select 1 from public.citas c
      join public.cita_sitios cs on cs.cita_id = c.id
      where c.anfitrion_correo = anfitriones.correo
        and cs.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    )
  );

drop policy "admin_global lee dispositivos" on public.dispositivos;
drop policy "leer dispositivos del propio sitio" on public.dispositivos;
create policy "leer dispositivos (propio sitio o admin_global)"
  on public.dispositivos
  for select
  to authenticated
  using (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or public.es_admin_global()
  );

drop policy "admin_global lee sitios" on public.sitios;
drop policy "anfitrion lee sitios" on public.sitios;
create policy "leer sitios (anfitrion o admin_global)"
  on public.sitios
  for select
  to authenticated
  using (
    exists (select 1 from public.anfitriones a where a.correo = (select auth.email()))
    or public.es_admin_global()
  );

drop policy "admin_global crea usuarios" on public.usuarios;
drop policy "crear usuarios del propio sitio" on public.usuarios;
create policy "crear usuarios (propio sitio o admin_global)"
  on public.usuarios
  for insert
  to authenticated
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or public.es_admin_global()
  );

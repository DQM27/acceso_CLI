-- Evita que auth.jwt()/auth.email() se re-evalue fila por fila (advisor
-- auth_rls_initplan). (select auth.jwt()) hace que Postgres lo calcule
-- una sola vez por consulta. Mismo comportamiento, solo mas rapido --
-- verificado con la suite completa de supabase/tests/ y con una prueba de
-- Realtime end-to-end (usuario temporal + suscripcion + insert) despues de
-- aplicar esto, sin dejar restos en la base.

alter policy "leer dispositivos del propio sitio" on public.dispositivos
using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid);

alter policy "crear contratistas del propio sitio" on public.contratistas
with check (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid);

alter policy "crear empresas del propio sitio" on public.empresas
with check (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid);

alter policy "crear usuarios del propio sitio" on public.usuarios
with check (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid);

alter policy "leer gafetes del propio sitio" on public.gafetes
using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid);

alter policy "crear gafetes del propio sitio" on public.gafetes
with check (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid);

alter policy "actualizar gafetes del propio sitio" on public.gafetes
using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid)
with check (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid);

alter policy "leer ingresos del propio sitio" on public.ingresos
using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid);

alter policy "crear ingresos del propio sitio" on public.ingresos
with check (
  sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
  and ((select auth.jwt()) ->> 'tipo') <> 'visor'
);

alter policy "actualizar ingresos del propio sitio" on public.ingresos
using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid)
with check (
  sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
  and ((select auth.jwt()) ->> 'tipo') <> 'visor'
);

alter policy "cada admin lee su propia fila" on public.administradores_panel
using ((select auth.email()) = correo);

alter policy "admin_global borra otros admins" on public.administradores_panel
using (correo <> (select auth.email()) and public.es_admin_global());

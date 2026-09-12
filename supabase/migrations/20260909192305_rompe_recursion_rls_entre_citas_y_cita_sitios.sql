-- Bug reportado por la revisión de seguridad de la web de visitas
-- (docs/reporte-seguridad-web-2026-09-09.md, docs/contrato-web-visitas.md):
-- leer `anfitriones` (o `citas`, o `cita_sitios`) como `authenticated`
-- producía "42P17: infinite recursion detected in policy for relation
-- citas". Causa real: la política SELECT de `citas` hace un EXISTS sobre
-- `cita_sitios` (para el caso "dispositivo del sitio"), y la política
-- SELECT/INSERT/DELETE de `cita_sitios` hace un EXISTS sobre `citas` (para
-- el caso "anfitrión dueño") -- cada EXISTS es una consulta normal sujeta a
-- RLS de la otra tabla, así que evaluar la política de una vuelve a evaluar
-- la política de la otra indefinidamente. La reproducción con un rol
-- `authenticated` real (no `postgres`, que no aplica RLS) es justo lo que
-- lo expone.
--
-- Mismo arreglo que ya usa `es_admin_global()`: una función SECURITY
-- DEFINER, dueña de `postgres`, que lee la tabla del otro lado del ciclo
-- SIN pasar por su RLS (un SECURITY DEFINER corre con los privilegios del
-- dueño, que sí puede leer sin RLS) -- rompe el ciclo sin abrir el acceso a
-- nadie más: las políticas que llaman a estas funciones siguen exigiendo
-- exactamente la misma condición que antes, sólo que ya no la evalúan
-- re-disparando la política ajena.

create or replace function public.sitios_de_cita(p_cita_id uuid)
returns setof uuid
language sql
stable
security definer
set search_path = public
as $$
  select sitio_id from public.cita_sitios where cita_id = p_cita_id;
$$;

create or replace function public.anfitrion_de_cita(p_cita_id uuid)
returns text
language sql
stable
security definer
set search_path = public
as $$
  select anfitrion_correo from public.citas where id = p_cita_id;
$$;

drop policy "leer citas propias, del sitio, o admin" on public.citas;
create policy "leer citas propias, del sitio, o admin"
  on public.citas
  for select
  to authenticated
  using (
    anfitrion_correo = auth.email()
    or public.es_admin_global()
    or ((select auth.jwt()) ->> 'sitio_id')::uuid in (select public.sitios_de_cita(citas.id))
  );

drop policy "leer cita_sitios propios, del sitio, o admin" on public.cita_sitios;
create policy "leer cita_sitios propios, del sitio, o admin"
  on public.cita_sitios
  for select
  to authenticated
  using (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or public.es_admin_global()
    or public.anfitrion_de_cita(cita_sitios.cita_id) = auth.email()
  );

drop policy "anfitrion agrega sitios a sus propias citas" on public.cita_sitios;
create policy "anfitrion agrega sitios a sus propias citas"
  on public.cita_sitios
  for insert
  to authenticated
  with check (public.anfitrion_de_cita(cita_sitios.cita_id) = auth.email());

drop policy "anfitrion quita sitios de sus propias citas" on public.cita_sitios;
create policy "anfitrion quita sitios de sus propias citas"
  on public.cita_sitios
  for delete
  to authenticated
  using (public.anfitrion_de_cita(cita_sitios.cita_id) = auth.email());

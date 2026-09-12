-- El advisor de seguridad marcó `public.sitios_de_cita`/`public.anfitrion_de_cita`
-- (agregadas en la migración anterior para romper la recursión de RLS) como
-- invocables directo vía PostgREST (`/rest/v1/rpc/...`) por `anon` Y por
-- `authenticated` -- eso es un bypass real de RLS: cualquiera con (o sin)
-- sesión podía pedir "a qué sitios aplica la cita X" o "quién es el
-- anfitrión de la cita X" para CUALQUIER uuid, sin pasar por la política
-- "leer citas propias, del sitio, o admin" que se supone las protege. El
-- esquema `private` ya existe en este proyecto para exactamente este caso
-- (ver `private.emitir_cambio_nube_sitio`, un SECURITY DEFINER que tampoco
-- se expone como RPC): PostgREST sólo expone funciones de los esquemas
-- configurados en `db_schemas` (`public`), así que moverlas ahí las deja
-- disponibles para las políticas (que sí pueden referenciar cualquier
-- función a la que su rol tenga EXECUTE) sin quedar accesibles por HTTP.

create or replace function private.sitios_de_cita(p_cita_id uuid)
returns setof uuid
language sql
stable
security definer
set search_path = public
as $$
  select sitio_id from public.cita_sitios where cita_id = p_cita_id;
$$;

create or replace function private.anfitrion_de_cita(p_cita_id uuid)
returns text
language sql
stable
security definer
set search_path = public
as $$
  select anfitrion_correo from public.citas where id = p_cita_id;
$$;

revoke all on function private.sitios_de_cita(uuid) from public;
revoke all on function private.anfitrion_de_cita(uuid) from public;
grant execute on function private.sitios_de_cita(uuid) to authenticated;
grant execute on function private.anfitrion_de_cita(uuid) to authenticated;

drop policy "leer citas propias, del sitio, o admin" on public.citas;
create policy "leer citas propias, del sitio, o admin"
  on public.citas
  for select
  to authenticated
  using (
    anfitrion_correo = auth.email()
    or public.es_admin_global()
    or ((select auth.jwt()) ->> 'sitio_id')::uuid in (select private.sitios_de_cita(citas.id))
  );

drop policy "leer cita_sitios propios, del sitio, o admin" on public.cita_sitios;
create policy "leer cita_sitios propios, del sitio, o admin"
  on public.cita_sitios
  for select
  to authenticated
  using (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or public.es_admin_global()
    or private.anfitrion_de_cita(cita_sitios.cita_id) = auth.email()
  );

drop policy "anfitrion agrega sitios a sus propias citas" on public.cita_sitios;
create policy "anfitrion agrega sitios a sus propias citas"
  on public.cita_sitios
  for insert
  to authenticated
  with check (private.anfitrion_de_cita(cita_sitios.cita_id) = auth.email());

drop policy "anfitrion quita sitios de sus propias citas" on public.cita_sitios;
create policy "anfitrion quita sitios de sus propias citas"
  on public.cita_sitios
  for delete
  to authenticated
  using (private.anfitrion_de_cita(cita_sitios.cita_id) = auth.email());

drop function if exists public.sitios_de_cita(uuid);
drop function if exists public.anfitrion_de_cita(uuid);

-- Hueco encontrado al preparar la sincronización de citas: la única
-- política de `anfitriones` es "cada anfitrión lee su propia fila"
-- (auth.email() = correo) -- un dispositivo (JWT sin email, con sitio_id)
-- nunca matchea eso, así que el embed `anfitrion:anfitriones(nombre)` en
-- una consulta de `citas` le volvía siempre NULL, sin error (PostgREST
-- no falla el resto de la fila si el embed queda vacío por RLS, sólo
-- pone null). Mismo criterio EXISTS-vía-cita_sitios que ya usan las
-- políticas de `citas`/`cita_visitantes`: no expone más de lo que ya se
-- podía ver (el correo del anfitrión ya viaja en `citas.anfitrion_correo`,
-- visible para el mismo dispositivo).
create policy "dispositivo del sitio lee el anfitrion de sus citas"
  on public.anfitriones
  for select
  to authenticated
  using (
    exists (
      select 1 from public.citas c
      join public.cita_sitios cs on cs.cita_id = c.id
      where c.anfitrion_correo = anfitriones.correo
        and cs.sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    )
  );

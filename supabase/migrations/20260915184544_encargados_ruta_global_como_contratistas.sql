-- Corrección (2026-09-15): encargados_ruta (personal KOF) es GLOBAL,
-- igual que contratistas/empresas -- no está acotado a un sitio como
-- vehiculos_ruta/salidas_ruta. El INSERT se queda igual (sigue
-- estampando el sitio_id del dispositivo que lo crea, mismo criterio que
-- "crear empresas del propio sitio"); sólo cambian lectura y
-- actualización, que ahora son globales (cualquier dispositivo
-- autenticado, sin restricción de sitio).

drop policy "leer encargados_ruta del propio sitio o admin" on public.encargados_ruta;
drop policy "actualizar encargados_ruta del propio sitio" on public.encargados_ruta;

create policy "leer encargados_ruta (global)"
  on public.encargados_ruta for select to authenticated
  using (
    ((select auth.jwt()) ->> 'sitio_id') is not null
    or private.es_admin_global()
  );

create policy "actualizar encargados_ruta (global)"
  on public.encargados_ruta for update to authenticated
  using (
    ((select auth.jwt()) ->> 'sitio_id') is not null
    or private.es_admin_global()
  )
  with check (
    ((select auth.jwt()) ->> 'sitio_id') is not null
    or private.es_admin_global()
  );

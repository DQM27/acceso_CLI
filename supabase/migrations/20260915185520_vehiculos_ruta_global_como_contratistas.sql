-- Corrección (2026-09-15): vehiculos_ruta también es GLOBAL, igual que
-- encargados_ruta/contratistas/empresas -- el usuario aclaró que una
-- unidad puede prestar servicio en otro sitio (recurso compartido), y
-- placa/numero_unidad ya son identificadores únicos de por sí, sin
-- riesgo de choque al hacerlo global. Sólo salidas_ruta se queda acotada
-- por sitio (el registro operativo real sí es de un sitio puntual).

drop policy "leer vehiculos_ruta del propio sitio o admin" on public.vehiculos_ruta;
drop policy "actualizar vehiculos_ruta del propio sitio" on public.vehiculos_ruta;

create policy "leer vehiculos_ruta (global)"
  on public.vehiculos_ruta for select to authenticated
  using (
    ((select auth.jwt()) ->> 'sitio_id') is not null
    or private.es_admin_global()
  );

create policy "actualizar vehiculos_ruta (global)"
  on public.vehiculos_ruta for update to authenticated
  using (
    ((select auth.jwt()) ->> 'sitio_id') is not null
    or private.es_admin_global()
  )
  with check (
    ((select auth.jwt()) ->> 'sitio_id') is not null
    or private.es_admin_global()
  );

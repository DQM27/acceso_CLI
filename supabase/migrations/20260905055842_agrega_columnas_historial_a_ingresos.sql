-- Estas columnas ya viajan localmente en `registro_ingresos` pero nunca se
-- mandaban a la nube porque el único consumidor remoto hasta ahora era el
-- cierre cruzado entre dispositivos (sólo necesitaba contratista/horas).
-- Ahora que el panel web quiere mostrar un historial real, hacen falta acá
-- también. Nullable: las filas ya sincronizadas antes de este cambio se
-- quedan sin estos datos (perderlas no vale una migración de backfill para
-- ~20 filas de prueba, ver docs/plan-panel-administrativo-web.md).
alter table public.ingresos
  add column contratista_cedula text,
  add column empresa_nombre text,
  add column tipo_ingreso text,
  add column medio_ingreso text,
  add column gafete_numero bigint;

-- Autorización para el panel web (admin_global, ver es_admin_global() de
-- la migración administradores_panel_gestion_admin_global) -- hoy sin
-- distinción por sitio para admin_regional a propósito: esa tabla todavía
-- no guarda qué sitios administra cada quien (pendiente, ver el plan), así
-- que de momento sólo admin_global puede leer el historial desde la web.
create policy "admin_global lee todo el historial"
  on public.ingresos
  for select
  to authenticated
  using (public.es_admin_global());

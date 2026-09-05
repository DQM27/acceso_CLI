-- La web (postgres_changes) sólo escuchaba contratistas/empresas/ingresos
-- (habilitados en 20260902155434 y 20260902165426, pensados para el cierre
-- cruzado entre dispositivos). Faltaban las tablas que hacían falta para
-- que las pantallas de Dispositivos/Operadores/Administradores del panel
-- web también reciban cambios en vivo, no sólo por polling.
alter publication supabase_realtime add table public.usuarios, public.dispositivos, public.administradores_panel;

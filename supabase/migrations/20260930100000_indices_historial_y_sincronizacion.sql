-- Índices para escalar el historial del panel y la sincronización de los equipos.
--
-- Hoy las tablas de movimientos sólo tienen índices por sitio, dispositivo y
-- los parciales de "abiertos". Faltaban los dos accesos que más se usan al
-- crecer los datos (varias unidades operativas, alguna con ~200 ingresos por día):
--   1. El panel filtra por rango de fecha (y unidad) y ordena por `hora_entrada`
--      descendente, paginando en el servidor.
--   2. Los equipos piden "lo cambiado desde la marca" por `updated_at` dentro de
--      su sitio, y reconcilian los últimos 7 días.
-- Son sólo índices: no cambian datos, políticas ni contratos. Idempotente.

-- Contratistas
create index if not exists ingresos_sitio_hora_entrada_idx
  on public.ingresos (sitio_id, hora_entrada desc);
create index if not exists ingresos_hora_entrada_idx
  on public.ingresos (hora_entrada desc);
create index if not exists ingresos_sitio_updated_at_idx
  on public.ingresos (sitio_id, updated_at);
create index if not exists ingresos_cedula_hora_entrada_idx
  on public.ingresos (contratista_cedula, hora_entrada desc);

-- Proveedores
create index if not exists ingresos_proveedor_sitio_hora_entrada_idx
  on public.ingresos_proveedor (sitio_id, hora_entrada desc);
create index if not exists ingresos_proveedor_hora_entrada_idx
  on public.ingresos_proveedor (hora_entrada desc);
create index if not exists ingresos_proveedor_sitio_updated_at_idx
  on public.ingresos_proveedor (sitio_id, updated_at);

-- Visitas
create index if not exists movimientos_visita_sitio_hora_entrada_idx
  on public.movimientos_visita (sitio_id, hora_entrada desc);
create index if not exists movimientos_visita_sitio_updated_at_idx
  on public.movimientos_visita (sitio_id, updated_at);

-- Gafetes provisionales
create index if not exists prestamos_gafete_provisional_sitio_hora_entrega_idx
  on public.prestamos_gafete_provisional (sitio_id, hora_entrega desc);
create index if not exists prestamos_gafete_provisional_sitio_updated_at_idx
  on public.prestamos_gafete_provisional (sitio_id, updated_at);

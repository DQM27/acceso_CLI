-- Gafetes de contratista y de visita son objetos físicos distintos que
-- repiten la misma numeración (docs/features-futuras/plan-gafetes-compartido-y-realtime-citas.md,
-- mismo criterio ya documentado en
-- docs/planes-implementados/plan-control-visitas.md) -- hasta acá el
-- espejo sólo modelaba el pool de contratistas. Se agrega `tipo` (con
-- 'PROVEEDOR' aceptado a futuro, sin columna de portador propia todavía
-- porque no existe tabla `proveedores`) y la unicidad pasa de
-- `(sitio_id, numero)` a `(sitio_id, numero, tipo)`. El campo "deudor" se
-- renombra a "portador" -- esta app no lleva control de dinero, es sólo
-- trazabilidad de a quién se le asignó el objeto físico la última vez.
--
-- Sin el CHECK cruzado tipo<->columna / estado<->portador que sí tiene la
-- tabla local (`gafetes` en SQLite): el espejo remoto ya no lo tenía para
-- estado<->contratista_deudor_id tampoco (ver `supabase/tests/gafetes_autorizacion.sql`,
-- que inserta PERDIDO sin deudor sólo para probar RLS) -- es estructural
-- (RLS + unicidad para upsert), no de validación de negocio; esa regla
-- vive en SQLite, la fuente de verdad real de este catálogo.
alter table public.gafetes add column tipo text not null default 'CONTRATISTA'
  check (tipo in ('CONTRATISTA', 'VISITA', 'PROVEEDOR'));

alter table public.gafetes rename column contratista_deudor_id to contratista_portador_id;
alter table public.gafetes rename column contratista_deudor_nombre to contratista_portador_nombre;

alter table public.gafetes add column visita_portador_id uuid references public.cita_visitantes(id);
alter table public.gafetes add column visita_portador_nombre text;

alter table public.gafetes drop constraint gafetes_sitio_id_numero_key;
alter table public.gafetes add constraint gafetes_sitio_id_numero_tipo_key unique (sitio_id, numero, tipo);

drop index if exists public.gafetes_contratista_deudor_id_idx;
create index if not exists gafetes_contratista_portador_id_idx on public.gafetes(contratista_portador_id);
create index if not exists gafetes_visita_portador_id_idx on public.gafetes(visita_portador_id);

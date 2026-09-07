-- LEER ANTES DE USAR: contratistas y empresas NO se pueblan con INSERTs
-- de Postgres a mano -- ya existe la herramienta correcta para eso, que
-- reusa el mismo camino que cualquier sincronización real (no bypassea
-- dispositivo_origen_id/sitio_id ni el resto de las columnas NOT NULL):
--
--   cargo run --example importar_catalogo_limpio -- <archivo.sql> [ruta_db]
--
-- Ese comando corre CONTRA LA BASE LOCAL (SQLite) de un dispositivo real
-- ya configurado (con su secreto pegado) -- ver examples/importar_
-- catalogo_limpio.rs: hace un respaldo, corre el <archivo.sql> (formato
-- SQLite, `INSERT INTO empresas/contratistas ... ON CONFLICT`), rellena
-- UUIDs, reconstruye el índice de búsqueda, y ENCOLA todo para subir a la
-- nube (cola_salida). El próximo "Sincronizar" (o el pulso automático de
-- 2 min) empuja los datos a Supabase con el dispositivo_origen_id/sitio_id
-- reales de esa máquina -- exactamente lo que las columnas NOT NULL de
-- contratistas/empresas en Supabase exigen.
--
-- Los datos semi-producción de contratistas/empresas hoy están en
-- `contratistas_base_final_limpia_v15.sql` (raíz del repo) -- ese es el
-- <archivo.sql> a usar:
--
--   cargo run --example importar_catalogo_limpio -- contratistas_base_final_limpia_v15.sql
--   -- (sin ruta_db: usa la misma base que la app de escritorio)
--   -- después: abrir la app y apretar "Sincronizar", o esperar el pulso
--   -- automático de 2 minutos.
--
-- Si en cambio se quiere una base local 100% limpia antes de importar
-- (en vez de fusionar sobre lo que ya haya), agregar `--recrear` antes
-- del nombre del archivo -- hace un respaldo y arranca de una base vacía.
--
-- ============================================================
-- Lo de abajo es sólo para GAFETES -- no hay (todavía) un archivo de
-- datos reales ni una herramienta de import como la de arriba, así que
-- sigue siendo un INSERT directo a Postgres. Reemplazar los números de
-- ejemplo cuando haya un rango real que cargar.
-- ============================================================
begin;

with sitio as (select id from public.sitios where nombre = 'Brisas'),
     dispositivo as (
       -- Cualquier dispositivo real ya provisionado sirve como origen --
       -- a diferencia de contratistas/empresas, acá no hace falta un
       -- dispositivo "semilla" de mentira porque este INSERT es puntual,
       -- no un catálogo completo que además tenga que viajar de vuelta
       -- a un dispositivo local.
       select id from public.dispositivos where sitio_id = (select id from sitio) limit 1
     )
insert into public.gafetes (sitio_id, dispositivo_origen_id, numero, estado)
select sitio.id, dispositivo.id, numero, 'DISPONIBLE'
from sitio, dispositivo,
  (values (1), (2), (3), (4), (5)) as valores(numero)
on conflict (sitio_id, numero) do nothing;

select (select count(*) from public.gafetes) as gafetes;

-- Revisar el resultado del SELECT de arriba antes de decidir.
commit;
-- rollback;

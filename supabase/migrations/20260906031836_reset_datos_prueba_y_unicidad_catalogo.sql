-- Reset de datos de prueba (desarrollo, datos ficticios) pedido explícitamente
-- para "empezar de cero" tras detectar 117 contratistas duplicados: sin una
-- restricción de unicidad real, el upsert por `id` (UUID generado en cada
-- dispositivo) nunca detectaba que dos bases locales distintas mandaban al
-- mismo contratista/empresa/gafete. Se limpia todo lo que es catálogo y
-- movimientos -- sitios/dispositivos/usuarios quedan intactos para que los
-- dispositivos ya configurados no necesiten volver a pegar el secreto.
truncate table ingresos, gafetes, contratistas, empresas restart identity cascade;

alter table contratistas add constraint contratistas_identificacion_key unique (identificacion);
alter table empresas add constraint empresas_nombre_key unique (nombre);
alter table gafetes add constraint gafetes_sitio_id_numero_key unique (sitio_id, numero);

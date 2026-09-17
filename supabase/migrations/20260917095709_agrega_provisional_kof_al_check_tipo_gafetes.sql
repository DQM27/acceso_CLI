-- El local (MIGRACION_39, src/database/schema.rs) ya admite 'PROVISIONAL_KOF'
-- como tipo de gafete desde hace días, pero el espejo en Supabase se quedó
-- atrás: el push de un gafete provisional KOF (POST /rest/v1/gafetes)
-- fallaba con 23514 (check_violation) porque gafetes_tipo_check solo
-- aceptaba CONTRATISTA/VISITA/PROVEEDOR. `ALTER TABLE ... DROP/ADD
-- CONSTRAINT` en vez de recrear la tabla -- Postgres sí lo permite
-- directo, a diferencia de SQLite.
alter table public.gafetes drop constraint gafetes_tipo_check;
alter table public.gafetes add constraint gafetes_tipo_check
  check (tipo = any (array['CONTRATISTA', 'VISITA', 'PROVEEDOR', 'PROVISIONAL_KOF']));

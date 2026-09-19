-- Necesaria para `plegar_texto` (ver la migración siguiente,
-- `unicidad_nombre_empresas_ignora_mayusculas_y_tildes`) -- mismo criterio
-- que `PLEGAR` en el núcleo Rust (`src/database/schema.rs`): "Dos Pinos" y
-- "DOS PINOS" deben detectarse como el mismo nombre.
create extension if not exists unaccent with schema extensions;

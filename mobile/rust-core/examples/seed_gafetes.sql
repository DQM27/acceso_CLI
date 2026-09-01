-- Catálogo de gafetes de desarrollo: 25 gafetes numerados 1-25, todos
-- disponibles. El SQL de contratistas que se usa para sembrar (raíz del
-- repo) es anterior al catálogo de gafetes, por eso no trae ninguno.
WITH RECURSIVE numeros(value) AS (
    SELECT 1
    UNION ALL
    SELECT value + 1 FROM numeros WHERE value < 25
)
INSERT OR IGNORE INTO gafetes (numero, estado)
SELECT value, 'DISPONIBLE' FROM numeros;

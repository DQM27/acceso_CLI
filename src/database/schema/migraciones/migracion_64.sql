-- Carnés PRAIND (docs/features-futuras/plan-fotos-praind.md, fase 3).
--
-- `contratistas.praind_foto`: ruta del carné vigente en R2, tal como llega de
-- la nube con el catálogo. NULL: el contratista no tiene carné cargado. La
-- escribe sólo la sincronización (nunca el equipo: la nube no la acepta).
--
-- `carnes_praind_guardados`: los carnés que ya se abrieron en este equipo,
-- para verlos al instante y sin internet. Se guardan sólo los que alguien
-- abrió (nunca se bajan todos), dentro de esta base cifrada y no como
-- archivos sueltos. La ruta lleva el SHA-256 del archivo, así que un carné
-- guardado nunca queda viejo: si el carné cambia, cambia la ruta.
--
-- Se borra la marca del catálogo para que la próxima sincronización lo baje
-- entero una vez: los contratistas que ya tenían carné cuando este equipo se
-- actualizó no volverían a llegar por la sincronización incremental.
ALTER TABLE contratistas ADD COLUMN praind_foto TEXT;

CREATE TABLE carnes_praind_guardados (
    clave TEXT PRIMARY KEY CHECK (length(clave) BETWEEN 1 AND 200),
    tipo_mime TEXT NOT NULL CHECK (tipo_mime IN ('image/png', 'image/jpeg')),
    bytes BLOB NOT NULL,
    abierto_en TEXT NOT NULL
) STRICT;

CREATE INDEX carnes_praind_guardados_abierto_en ON carnes_praind_guardados (abierto_en);

UPDATE sincronizacion_estado SET catalogo_actualizado_hasta = NULL WHERE id = 1;

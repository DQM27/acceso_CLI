
CREATE TABLE ingresos_remotos (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    contratista_nombre TEXT NOT NULL,
    hora_entrada TEXT NOT NULL,
    usuario_entrada_nombre TEXT,
    dispositivo_entrada_id TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
) STRICT;

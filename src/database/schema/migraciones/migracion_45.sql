
CREATE TABLE prestamos_gafete_provisional_remotos (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    encargado_nombre TEXT NOT NULL,
    encargado_codigo_empleado TEXT NOT NULL,
    gafete_numero INTEGER NOT NULL,
    hora_entrega TEXT NOT NULL,
    usuario_entrega_nombre TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
) STRICT;

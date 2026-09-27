
CREATE TABLE ingresos_proveedor_remotos (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    cedula TEXT NOT NULL,
    nombre TEXT NOT NULL,
    empresa_nombre TEXT NOT NULL,
    placa TEXT,
    gafete_numero INTEGER NOT NULL,
    hora_entrada TEXT NOT NULL,
    usuario_entrada_nombre TEXT NOT NULL,
    dispositivo_entrada_id TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
) STRICT;

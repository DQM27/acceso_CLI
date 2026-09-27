
ALTER TABLE sincronizacion_estado ADD COLUMN gafetes_provisionales_historial_actualizado_hasta TEXT;

CREATE TABLE prestamos_gafete_provisional_historial_sitio (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    encargado_nombre TEXT NOT NULL,
    encargado_codigo_empleado TEXT NOT NULL,
    gafete_numero INTEGER NOT NULL,
    fecha_hora_entrega TEXT NOT NULL,
    usuario_entrega_nombre TEXT NOT NULL,
    fecha_hora_devolucion TEXT,
    usuario_devolucion_nombre TEXT,
    actualizado_en TEXT NOT NULL
) STRICT;

CREATE INDEX idx_prestamos_gafete_provisional_historial_sitio_fecha_entrega
ON prestamos_gafete_provisional_historial_sitio(fecha_hora_entrega);

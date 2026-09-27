
ALTER TABLE sincronizacion_estado ADD COLUMN historial_visitas_actualizado_hasta TEXT;

CREATE TABLE historial_visitas_sitio (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    visitante_cedula TEXT NOT NULL,
    visitante_nombre TEXT NOT NULL,
    empresa TEXT,
    anfitrion_nombre TEXT,
    motivo TEXT,
    gafete_numero INTEGER,
    hora_entrada TEXT NOT NULL,
    hora_salida TEXT,
    usuario_entrada_nombre TEXT,
    usuario_salida_nombre TEXT,
    dispositivo_entrada_id TEXT NOT NULL,
    dispositivo_salida_id TEXT,
    actualizado_en TEXT NOT NULL
) STRICT;

CREATE INDEX idx_historial_visitas_sitio_hora_entrada ON historial_visitas_sitio(hora_entrada);

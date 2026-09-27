
ALTER TABLE sincronizacion_estado ADD COLUMN historial_actualizado_hasta TEXT;

CREATE TABLE historial_sitio (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    contratista_cedula TEXT,
    contratista_nombre TEXT NOT NULL,
    empresa_nombre TEXT,
    tipo_ingreso TEXT,
    medio_ingreso TEXT,
    hora_entrada TEXT NOT NULL,
    hora_salida TEXT,
    gafete_numero INTEGER,
    usuario_entrada_nombre TEXT,
    usuario_salida_nombre TEXT,
    resultado_acceso TEXT,
    motivo_resultado TEXT,
    reglas_version INTEGER,
    empresa_activa_snapshot INTEGER,
    dispositivo_entrada_id TEXT NOT NULL,
    dispositivo_salida_id TEXT,
    actualizado_en TEXT NOT NULL
) STRICT;

CREATE INDEX idx_historial_sitio_hora_entrada ON historial_sitio(hora_entrada);

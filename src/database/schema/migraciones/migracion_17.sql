
CREATE TABLE cola_salida (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (entidad IN ('contratista', 'ingreso')),
    entidad_uuid TEXT NOT NULL,
    operacion TEXT NOT NULL CHECK (operacion IN ('crear', 'actualizar', 'cerrar')),
    estado TEXT NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'enviado', 'fallido')),
    intentos INTEGER NOT NULL DEFAULT 0 CHECK (intentos >= 0),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL,
    ultimo_error TEXT
) STRICT;

-- Vaciar la cola recorre lo pendiente en orden de creación.
CREATE INDEX idx_cola_salida_pendientes
ON cola_salida(creado_en)
WHERE estado = 'pendiente';

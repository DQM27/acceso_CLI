
CREATE TABLE cola_salida_nueva (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (entidad IN ('contratista', 'ingreso', 'empresa', 'gafete', 'usuario')),
    entidad_uuid TEXT NOT NULL,
    operacion TEXT NOT NULL CHECK (operacion IN ('crear', 'actualizar', 'cerrar')),
    estado TEXT NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'enviado', 'fallido')),
    intentos INTEGER NOT NULL DEFAULT 0 CHECK (intentos >= 0),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL,
    ultimo_error TEXT,
    proximo_intento_en TEXT GENERATED ALWAYS AS (
        datetime(actualizado_en, '+' || MIN(intentos * 15, 1440) || ' minutes')
    ) STORED
) STRICT;
INSERT INTO cola_salida_nueva (
    id, entidad, entidad_uuid, operacion, estado, intentos,
    creado_en, actualizado_en, ultimo_error
)
SELECT
    id, entidad, entidad_uuid, operacion, estado, intentos,
    creado_en, actualizado_en, ultimo_error
FROM cola_salida;
DROP TABLE cola_salida;
ALTER TABLE cola_salida_nueva RENAME TO cola_salida;
CREATE INDEX idx_cola_salida_pendientes
ON cola_salida(proximo_intento_en)
WHERE estado = 'pendiente';

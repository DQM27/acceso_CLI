
ALTER TABLE empresas ADD COLUMN uuid TEXT;
UPDATE empresas SET uuid = (
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
    || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2)
    || '-' || lower(hex(randomblob(6)))
) WHERE uuid IS NULL;
CREATE UNIQUE INDEX idx_empresas_uuid ON empresas(uuid);

CREATE TABLE cola_salida_nueva (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (entidad IN ('contratista', 'ingreso', 'empresa')),
    entidad_uuid TEXT NOT NULL,
    operacion TEXT NOT NULL CHECK (operacion IN ('crear', 'actualizar', 'cerrar')),
    estado TEXT NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'enviado', 'fallido')),
    intentos INTEGER NOT NULL DEFAULT 0 CHECK (intentos >= 0),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL,
    ultimo_error TEXT
) STRICT;
INSERT INTO cola_salida_nueva SELECT * FROM cola_salida;
DROP TABLE cola_salida;
ALTER TABLE cola_salida_nueva RENAME TO cola_salida;
CREATE INDEX idx_cola_salida_pendientes
ON cola_salida(creado_en)
WHERE estado = 'pendiente';

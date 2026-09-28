
CREATE TABLE personas_vetadas (
    uuid TEXT PRIMARY KEY,
    cedula TEXT NOT NULL,
    vigente INTEGER NOT NULL CHECK (vigente IN (0, 1)),
    actualizado_en TEXT NOT NULL
) STRICT;
CREATE INDEX idx_personas_vetadas_cedula_vigente
ON personas_vetadas(cedula) WHERE vigente = 1;
ALTER TABLE sincronizacion_estado ADD COLUMN vetos_actualizado_hasta TEXT;


CREATE TABLE gafetes (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL UNIQUE,
    estado TEXT NOT NULL CHECK (estado IN ('DISPONIBLE', 'PERDIDO', 'DE_BAJA')),
    contratista_deudor_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    CHECK (
        (estado = 'PERDIDO' AND contratista_deudor_id IS NOT NULL)
        OR (estado <> 'PERDIDO' AND contratista_deudor_id IS NULL)
    )
);
CREATE INDEX idx_gafetes_estado ON gafetes(estado);
CREATE INDEX idx_gafetes_contratista_deudor
ON gafetes(contratista_deudor_id) WHERE contratista_deudor_id IS NOT NULL;

CREATE TABLE gafetes_incidentes (
    id INTEGER PRIMARY KEY,
    gafete_id INTEGER NOT NULL REFERENCES gafetes(id) ON DELETE RESTRICT,
    tipo TEXT NOT NULL CHECK (tipo IN ('PERDIDO', 'RESUELTO')),
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    motivo_resolucion TEXT CHECK (
        motivo_resolucion IS NULL OR motivo_resolucion IN ('PAGADO', 'APARECIDO')
    ),
    CHECK (
        (tipo = 'PERDIDO' AND contratista_id IS NOT NULL AND motivo_resolucion IS NULL)
        OR (tipo = 'RESUELTO' AND contratista_id IS NULL AND motivo_resolucion IS NOT NULL)
    )
);
CREATE INDEX idx_gafetes_incidentes_gafete ON gafetes_incidentes(gafete_id, id DESC);
CREATE INDEX idx_gafetes_incidentes_fecha ON gafetes_incidentes(fecha_hora DESC, id DESC);

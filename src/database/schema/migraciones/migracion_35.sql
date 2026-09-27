
CREATE TABLE gafetes_nueva (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL,
    tipo TEXT NOT NULL CHECK (tipo IN ('CONTRATISTA', 'VISITA', 'PROVEEDOR')),
    estado TEXT NOT NULL CHECK (estado IN ('DISPONIBLE', 'PERDIDO', 'DE_BAJA')),
    contratista_portador_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    visita_portador_id INTEGER REFERENCES cita_visitantes(id) ON DELETE RESTRICT,
    uuid TEXT,
    CHECK (
        (tipo = 'CONTRATISTA' AND visita_portador_id IS NULL)
        OR (tipo = 'VISITA' AND contratista_portador_id IS NULL)
        OR (tipo = 'PROVEEDOR' AND contratista_portador_id IS NULL AND visita_portador_id IS NULL)
    ),
    CHECK (
        (estado = 'PERDIDO' AND (contratista_portador_id IS NOT NULL OR visita_portador_id IS NOT NULL))
        OR (estado <> 'PERDIDO' AND contratista_portador_id IS NULL AND visita_portador_id IS NULL)
    )
) STRICT;
INSERT INTO gafetes_nueva (id, numero, tipo, estado, contratista_portador_id, visita_portador_id, uuid)
SELECT id, numero, 'CONTRATISTA', estado, contratista_deudor_id, NULL, uuid FROM gafetes;
DROP TABLE gafetes;
ALTER TABLE gafetes_nueva RENAME TO gafetes;
CREATE UNIQUE INDEX idx_gafetes_numero_tipo ON gafetes(numero, tipo);
CREATE UNIQUE INDEX idx_gafetes_uuid ON gafetes(uuid);
CREATE INDEX idx_gafetes_estado ON gafetes(estado);
CREATE INDEX idx_gafetes_contratista_portador
ON gafetes(contratista_portador_id) WHERE contratista_portador_id IS NOT NULL;
CREATE INDEX idx_gafetes_visita_portador
ON gafetes(visita_portador_id) WHERE visita_portador_id IS NOT NULL;

CREATE TABLE gafetes_incidentes_nueva (
    id INTEGER PRIMARY KEY,
    gafete_id INTEGER NOT NULL REFERENCES gafetes(id) ON DELETE RESTRICT,
    tipo TEXT NOT NULL CHECK (tipo IN ('PERDIDO', 'RESUELTO')),
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    visita_portador_id INTEGER REFERENCES cita_visitantes(id) ON DELETE RESTRICT,
    motivo_resolucion TEXT CHECK (
        motivo_resolucion IS NULL OR motivo_resolucion IN ('PAGADO', 'APARECIDO')
    ),
    CHECK (
        (tipo = 'PERDIDO' AND (contratista_id IS NOT NULL OR visita_portador_id IS NOT NULL)
            AND motivo_resolucion IS NULL)
        OR (tipo = 'RESUELTO' AND contratista_id IS NULL AND visita_portador_id IS NULL
            AND motivo_resolucion IS NOT NULL)
    )
) STRICT;
INSERT INTO gafetes_incidentes_nueva (id, gafete_id, tipo, fecha_hora, usuario_id, contratista_id, visita_portador_id, motivo_resolucion)
SELECT id, gafete_id, tipo, fecha_hora, usuario_id, contratista_id, NULL, motivo_resolucion FROM gafetes_incidentes;
DROP TABLE gafetes_incidentes;
ALTER TABLE gafetes_incidentes_nueva RENAME TO gafetes_incidentes;
CREATE INDEX idx_gafetes_incidentes_gafete ON gafetes_incidentes(gafete_id, id DESC);
CREATE INDEX idx_gafetes_incidentes_fecha ON gafetes_incidentes(fecha_hora DESC, id DESC);

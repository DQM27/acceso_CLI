
CREATE TABLE auditoria_contratistas_nueva (
    id INTEGER PRIMARY KEY,
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER NOT NULL REFERENCES contratistas(id) ON DELETE RESTRICT,
    campo TEXT NOT NULL CHECK (
        campo IN ('tipo_ingreso', 'fecha_vencimiento_praind', 'tiene_acceso')
    ),
    valor_anterior TEXT,
    valor_nuevo TEXT,
    CHECK (valor_anterior IS NOT valor_nuevo)
);

INSERT INTO auditoria_contratistas_nueva(
    id, fecha_hora, usuario_id, contratista_id, campo, valor_anterior, valor_nuevo
)
SELECT id, fecha_hora, usuario_id, contratista_id, campo, valor_anterior, valor_nuevo
FROM auditoria_contratistas;

DROP TABLE auditoria_contratistas;
ALTER TABLE auditoria_contratistas_nueva RENAME TO auditoria_contratistas;

CREATE INDEX idx_auditoria_contratistas_fecha
ON auditoria_contratistas(fecha_hora DESC, id DESC);

CREATE INDEX idx_auditoria_contratistas_contratista
ON auditoria_contratistas(contratista_id, id DESC);


CREATE TABLE auditoria_contratistas (
    id INTEGER PRIMARY KEY,
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER NOT NULL REFERENCES contratistas(id) ON DELETE RESTRICT,
    campo TEXT NOT NULL CHECK (campo IN ('tipo_ingreso', 'fecha_vencimiento_praind')),
    valor_anterior TEXT,
    valor_nuevo TEXT,
    CHECK (valor_anterior IS NOT valor_nuevo)
);

CREATE INDEX idx_auditoria_contratistas_fecha
ON auditoria_contratistas(fecha_hora DESC, id DESC);

CREATE INDEX idx_auditoria_contratistas_contratista
ON auditoria_contratistas(contratista_id, id DESC);

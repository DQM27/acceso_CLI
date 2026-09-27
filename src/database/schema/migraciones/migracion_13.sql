
DROP TABLE auditoria_contratistas;

CREATE TABLE auditoria_cambios (
    id INTEGER PRIMARY KEY,
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    usuario_nombre TEXT NOT NULL,
    entidad TEXT NOT NULL CHECK (entidad IN ('contratista', 'empresa', 'usuario')),
    entidad_id INTEGER NOT NULL,
    entidad_nombre TEXT NOT NULL,
    campo TEXT NOT NULL,
    valor_anterior TEXT,
    valor_nuevo TEXT,
    -- Bloquea registrar un "cambio" sin cambio real (anterior == nuevo,
    -- ambos no nulos) — mismo backstop que tenía `auditoria_contratistas`.
    -- La excepción es `valor_anterior IS NULL`: cubre tanto el caso ya
    -- existente (ej. PRAIND sin fecha previa) como un marcador de evento
    -- sin valores (ej. "se cambió la contraseña", donde sólo importa la
    -- fecha — no hay antes/después que registrar).
    CHECK (valor_anterior IS NOT valor_nuevo OR valor_anterior IS NULL)
);

CREATE INDEX idx_auditoria_cambios_fecha
ON auditoria_cambios(fecha_hora DESC, id DESC);

CREATE INDEX idx_auditoria_cambios_entidad
ON auditoria_cambios(entidad, entidad_id, id DESC);

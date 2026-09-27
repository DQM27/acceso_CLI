
CREATE TABLE gafetes_nueva (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL,
    tipo TEXT NOT NULL CHECK (tipo IN ('CONTRATISTA', 'VISITA', 'PROVEEDOR', 'PROVISIONAL_KOF')),
    estado TEXT NOT NULL CHECK (estado IN ('DISPONIBLE', 'PERDIDO', 'DE_BAJA')),
    contratista_portador_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    visita_portador_id INTEGER REFERENCES cita_visitantes(id) ON DELETE RESTRICT,
    encargado_portador_id INTEGER REFERENCES encargados_ruta(id) ON DELETE RESTRICT,
    uuid TEXT,
    CHECK (
        (tipo = 'CONTRATISTA' AND visita_portador_id IS NULL AND encargado_portador_id IS NULL)
        OR (tipo = 'VISITA' AND contratista_portador_id IS NULL AND encargado_portador_id IS NULL)
        OR (tipo = 'PROVEEDOR'
            AND contratista_portador_id IS NULL AND visita_portador_id IS NULL
            AND encargado_portador_id IS NULL)
        OR (tipo = 'PROVISIONAL_KOF' AND contratista_portador_id IS NULL AND visita_portador_id IS NULL)
    ),
    CHECK (
        (estado = 'PERDIDO'
            AND (contratista_portador_id IS NOT NULL OR visita_portador_id IS NOT NULL
                OR encargado_portador_id IS NOT NULL))
        OR (estado <> 'PERDIDO'
            AND contratista_portador_id IS NULL AND visita_portador_id IS NULL
            AND encargado_portador_id IS NULL)
    )
) STRICT;
INSERT INTO gafetes_nueva (
    id, numero, tipo, estado, contratista_portador_id, visita_portador_id, encargado_portador_id, uuid
)
SELECT id, numero, tipo, estado, contratista_portador_id, visita_portador_id, NULL, uuid FROM gafetes;
DROP TABLE gafetes;
ALTER TABLE gafetes_nueva RENAME TO gafetes;
CREATE UNIQUE INDEX idx_gafetes_numero_tipo ON gafetes(numero, tipo);
CREATE UNIQUE INDEX idx_gafetes_uuid ON gafetes(uuid);
CREATE INDEX idx_gafetes_estado ON gafetes(estado);
CREATE INDEX idx_gafetes_contratista_portador
ON gafetes(contratista_portador_id) WHERE contratista_portador_id IS NOT NULL;
CREATE INDEX idx_gafetes_visita_portador
ON gafetes(visita_portador_id) WHERE visita_portador_id IS NOT NULL;
CREATE INDEX idx_gafetes_encargado_portador
ON gafetes(encargado_portador_id) WHERE encargado_portador_id IS NOT NULL;

CREATE TABLE gafetes_incidentes_nueva (
    id INTEGER PRIMARY KEY,
    gafete_id INTEGER NOT NULL REFERENCES gafetes(id) ON DELETE RESTRICT,
    tipo TEXT NOT NULL CHECK (tipo IN ('PERDIDO', 'RESUELTO')),
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    visita_portador_id INTEGER REFERENCES cita_visitantes(id) ON DELETE RESTRICT,
    encargado_portador_id INTEGER REFERENCES encargados_ruta(id) ON DELETE RESTRICT,
    motivo_resolucion TEXT CHECK (
        motivo_resolucion IS NULL OR motivo_resolucion IN ('PAGADO', 'APARECIDO')
    ),
    CHECK (
        (tipo = 'PERDIDO'
            AND (contratista_id IS NOT NULL OR visita_portador_id IS NOT NULL
                OR encargado_portador_id IS NOT NULL)
            AND motivo_resolucion IS NULL)
        OR (tipo = 'RESUELTO'
            AND contratista_id IS NULL AND visita_portador_id IS NULL AND encargado_portador_id IS NULL
            AND motivo_resolucion IS NOT NULL)
    )
) STRICT;
INSERT INTO gafetes_incidentes_nueva (
    id, gafete_id, tipo, fecha_hora, usuario_id, contratista_id, visita_portador_id,
    encargado_portador_id, motivo_resolucion
)
SELECT id, gafete_id, tipo, fecha_hora, usuario_id, contratista_id, visita_portador_id, NULL, motivo_resolucion
FROM gafetes_incidentes;
DROP TABLE gafetes_incidentes;
ALTER TABLE gafetes_incidentes_nueva RENAME TO gafetes_incidentes;
CREATE INDEX idx_gafetes_incidentes_gafete ON gafetes_incidentes(gafete_id, id DESC);
CREATE INDEX idx_gafetes_incidentes_fecha ON gafetes_incidentes(fecha_hora DESC, id DESC);

CREATE TABLE prestamos_gafete_provisional (
    id INTEGER PRIMARY KEY,
    encargado_id INTEGER NOT NULL REFERENCES encargados_ruta(id) ON DELETE RESTRICT,
    encargado_nombre TEXT NOT NULL,
    encargado_codigo_empleado TEXT NOT NULL,
    gafete_numero INTEGER NOT NULL,
    fecha_hora_entrega TEXT NOT NULL,
    usuario_entrega_id INTEGER NOT NULL REFERENCES usuarios(id),
    usuario_entrega_nombre TEXT NOT NULL,
    fecha_hora_devolucion TEXT,
    usuario_devolucion_id INTEGER REFERENCES usuarios(id),
    usuario_devolucion_nombre TEXT,
    uuid TEXT NOT NULL,
    CHECK (
        (fecha_hora_devolucion IS NULL
            AND usuario_devolucion_id IS NULL AND usuario_devolucion_nombre IS NULL)
        OR
        (fecha_hora_devolucion IS NOT NULL
            AND usuario_devolucion_id IS NOT NULL AND usuario_devolucion_nombre IS NOT NULL)
    ),
    CHECK (fecha_hora_devolucion IS NULL OR fecha_hora_devolucion >= fecha_hora_entrega)
) STRICT;
CREATE UNIQUE INDEX idx_prestamos_gafete_provisional_uuid ON prestamos_gafete_provisional(uuid);
-- Un mismo encargado no puede tener dos prestamos abiertos a la vez.
CREATE UNIQUE INDEX idx_prestamos_gafete_provisional_encargado_activo
ON prestamos_gafete_provisional(encargado_id) WHERE fecha_hora_devolucion IS NULL;
-- Ni el mismo numero de gafete puede estar prestado a dos personas a la vez
-- (mismo criterio que `idx_salidas_ruta_placa_activa`).
CREATE UNIQUE INDEX idx_prestamos_gafete_provisional_numero_activo
ON prestamos_gafete_provisional(gafete_numero) WHERE fecha_hora_devolucion IS NULL;
CREATE INDEX idx_prestamos_gafete_provisional_encargado ON prestamos_gafete_provisional(encargado_id);

CREATE TRIGGER prestamos_gafete_provisional_no_eliminar
BEFORE DELETE ON prestamos_gafete_provisional
BEGIN
    SELECT RAISE(ABORT, 'Los prestamos de gafete provisional no se pueden eliminar');
END;
CREATE TRIGGER prestamos_gafete_provisional_entrega_inmutable
BEFORE UPDATE OF
    encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero,
    fecha_hora_entrega, usuario_entrega_id, usuario_entrega_nombre, uuid
ON prestamos_gafete_provisional
WHEN
    NEW.encargado_id IS NOT OLD.encargado_id
    OR NEW.encargado_nombre IS NOT OLD.encargado_nombre
    OR NEW.encargado_codigo_empleado IS NOT OLD.encargado_codigo_empleado
    OR NEW.gafete_numero IS NOT OLD.gafete_numero
    OR NEW.fecha_hora_entrega IS NOT OLD.fecha_hora_entrega
    OR NEW.usuario_entrega_id IS NOT OLD.usuario_entrega_id
    OR NEW.usuario_entrega_nombre IS NOT OLD.usuario_entrega_nombre
    OR NEW.uuid IS NOT OLD.uuid
BEGIN
    SELECT RAISE(ABORT, 'Los datos de entrega del prestamo son inmutables');
END;
CREATE TRIGGER prestamos_gafete_provisional_devolucion_unica
BEFORE UPDATE OF fecha_hora_devolucion, usuario_devolucion_id, usuario_devolucion_nombre
ON prestamos_gafete_provisional
WHEN
    OLD.fecha_hora_devolucion IS NOT NULL
    OR NEW.fecha_hora_devolucion IS NULL
    OR NEW.usuario_devolucion_nombre IS NULL
BEGIN
    SELECT RAISE(ABORT, 'La devolucion solo puede registrarse una vez');
END;
CREATE TRIGGER prestamos_gafete_provisional_fecha_utc_insert
BEFORE INSERT ON prestamos_gafete_provisional
WHEN
    strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_entrega) IS NOT NEW.fecha_hora_entrega
    OR (
        NEW.fecha_hora_devolucion IS NOT NULL
        AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_devolucion) IS NOT NEW.fecha_hora_devolucion
    )
BEGIN
    SELECT RAISE(ABORT, 'Las fechas del prestamo deben estar normalizadas en UTC');
END;
CREATE TRIGGER prestamos_gafete_provisional_devolucion_utc
BEFORE UPDATE OF fecha_hora_devolucion ON prestamos_gafete_provisional
WHEN
    NEW.fecha_hora_devolucion IS NOT NULL
    AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_devolucion) IS NOT NEW.fecha_hora_devolucion
BEGIN
    SELECT RAISE(ABORT, 'La fecha de devolucion debe estar normalizada en UTC');
END;

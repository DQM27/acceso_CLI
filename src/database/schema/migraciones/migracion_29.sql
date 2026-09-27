
CREATE TABLE citas (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL,
    motivo TEXT,
    fecha_desde TEXT NOT NULL,
    fecha_hasta TEXT NOT NULL,
    anfitrion_nombre TEXT NOT NULL,
    anfitrion_correo TEXT NOT NULL,
    estado TEXT NOT NULL CHECK (estado IN ('VIGENTE', 'CANCELADA')),
    creado_en TEXT NOT NULL,
    CHECK (fecha_hasta >= fecha_desde)
) STRICT;
CREATE UNIQUE INDEX idx_citas_uuid ON citas(uuid);
CREATE INDEX idx_citas_vigencia ON citas(fecha_desde, fecha_hasta);

CREATE TABLE cita_visitantes (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL,
    cita_id INTEGER NOT NULL REFERENCES citas(id) ON DELETE RESTRICT,
    cedula TEXT NOT NULL,
    nombre TEXT NOT NULL,
    empresa TEXT,
    placa_vehiculo TEXT
) STRICT;
CREATE UNIQUE INDEX idx_cita_visitantes_uuid ON cita_visitantes(uuid);
CREATE INDEX idx_cita_visitantes_cita ON cita_visitantes(cita_id);
CREATE INDEX idx_cita_visitantes_cedula ON cita_visitantes(cedula);

-- `usuario_entrada_id`/`usuario_salida_id` con `ON DELETE RESTRICT`
-- (mismo criterio que `registro_ingresos`): un usuario nunca se borra de
-- verdad si tiene movimientos asociados.
CREATE TABLE movimientos_visita (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL,
    cita_visitante_id INTEGER NOT NULL REFERENCES cita_visitantes(id) ON DELETE RESTRICT,
    gafete_numero INTEGER,
    fecha_hora_entrada TEXT NOT NULL,
    fecha_hora_salida TEXT,
    usuario_entrada_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    usuario_entrada_nombre TEXT NOT NULL,
    usuario_salida_id INTEGER REFERENCES usuarios(id) ON DELETE RESTRICT,
    usuario_salida_nombre TEXT
) STRICT;
CREATE UNIQUE INDEX idx_movimientos_visita_uuid ON movimientos_visita(uuid);
CREATE INDEX idx_movimientos_visita_cita_visitante ON movimientos_visita(cita_visitante_id);
-- Un mismo visitante no puede tener dos movimientos abiertos a la vez
-- (mismo criterio que `idx_registro_ingresos_contratista_activo`).
CREATE UNIQUE INDEX idx_movimientos_visita_visitante_activo
ON movimientos_visita(cita_visitante_id) WHERE fecha_hora_salida IS NULL;
CREATE INDEX idx_movimientos_visita_gafete ON movimientos_visita(gafete_numero);
CREATE UNIQUE INDEX idx_movimientos_visita_gafete_activo
ON movimientos_visita(gafete_numero)
WHERE gafete_numero IS NOT NULL AND fecha_hora_salida IS NULL;

-- Mismas cuatro garantías que ya tiene `registro_ingresos`: no se borra,
-- los datos de entrada no se editan después de creados, la salida se
-- registra una sola vez, y toda fecha queda en UTC normalizado.
CREATE TRIGGER movimientos_visita_no_eliminar
BEFORE DELETE ON movimientos_visita
BEGIN
    SELECT RAISE(ABORT, 'Los movimientos de visita no se pueden eliminar');
END;
CREATE TRIGGER movimientos_visita_entrada_inmutable
BEFORE UPDATE OF
    cita_visitante_id, gafete_numero, fecha_hora_entrada,
    usuario_entrada_id, usuario_entrada_nombre, uuid
ON movimientos_visita
WHEN
    NEW.cita_visitante_id IS NOT OLD.cita_visitante_id
    OR NEW.gafete_numero IS NOT OLD.gafete_numero
    OR NEW.fecha_hora_entrada IS NOT OLD.fecha_hora_entrada
    OR NEW.usuario_entrada_id IS NOT OLD.usuario_entrada_id
    OR NEW.usuario_entrada_nombre IS NOT OLD.usuario_entrada_nombre
    OR NEW.uuid IS NOT OLD.uuid
BEGIN
    SELECT RAISE(ABORT, 'Los datos de entrada del movimiento son inmutables');
END;
CREATE TRIGGER movimientos_visita_salida_unica
BEFORE UPDATE OF fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
ON movimientos_visita
WHEN
    OLD.fecha_hora_salida IS NOT NULL
    OR NEW.fecha_hora_salida IS NULL
    OR NEW.usuario_salida_nombre IS NULL
BEGIN
    SELECT RAISE(ABORT, 'La salida solo puede registrarse una vez');
END;
CREATE TRIGGER movimientos_visita_fecha_utc_insert
BEFORE INSERT ON movimientos_visita
WHEN
    strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_entrada) IS NOT NEW.fecha_hora_entrada
    OR (
        NEW.fecha_hora_salida IS NOT NULL
        AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
    )
BEGIN
    SELECT RAISE(ABORT, 'Las fechas de movimientos deben estar normalizadas en UTC');
END;
CREATE TRIGGER movimientos_visita_salida_utc
BEFORE UPDATE OF fecha_hora_salida ON movimientos_visita
WHEN
    NEW.fecha_hora_salida IS NOT NULL
    AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
BEGIN
    SELECT RAISE(ABORT, 'La fecha de salida debe estar normalizada en UTC');
END;

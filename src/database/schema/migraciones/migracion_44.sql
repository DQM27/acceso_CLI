
CREATE TABLE registro_ingresos_proveedor_nueva (
    id INTEGER PRIMARY KEY,
    cedula TEXT NOT NULL,
    nombre TEXT NOT NULL,
    empresa_id INTEGER NOT NULL REFERENCES empresas_proveedor(id) ON DELETE RESTRICT,
    empresa_nombre TEXT NOT NULL,
    placa TEXT,
    gafete_numero INTEGER NOT NULL,
    fecha_hora_ingreso TEXT NOT NULL,
    usuario_ingreso_id INTEGER NOT NULL REFERENCES usuarios(id),
    usuario_ingreso_nombre TEXT NOT NULL,
    fecha_hora_salida TEXT,
    usuario_salida_id INTEGER REFERENCES usuarios(id),
    usuario_salida_nombre TEXT,
    uuid TEXT NOT NULL,
    CHECK (
        (fecha_hora_salida IS NULL AND usuario_salida_id IS NULL AND usuario_salida_nombre IS NULL)
        OR
        (fecha_hora_salida IS NOT NULL AND usuario_salida_nombre IS NOT NULL)
    ),
    CHECK (fecha_hora_salida IS NULL OR fecha_hora_salida >= fecha_hora_ingreso)
) STRICT;
INSERT INTO registro_ingresos_proveedor_nueva SELECT * FROM registro_ingresos_proveedor;
DROP TABLE registro_ingresos_proveedor;
ALTER TABLE registro_ingresos_proveedor_nueva RENAME TO registro_ingresos_proveedor;

CREATE UNIQUE INDEX idx_registro_ingresos_proveedor_uuid ON registro_ingresos_proveedor(uuid);
CREATE UNIQUE INDEX idx_registro_ingresos_proveedor_cedula_activa
ON registro_ingresos_proveedor(cedula) WHERE fecha_hora_salida IS NULL;
CREATE UNIQUE INDEX idx_registro_ingresos_proveedor_gafete_activo
ON registro_ingresos_proveedor(gafete_numero) WHERE fecha_hora_salida IS NULL;
CREATE INDEX idx_registro_ingresos_proveedor_empresa ON registro_ingresos_proveedor(empresa_id);

CREATE TRIGGER registro_ingresos_proveedor_no_eliminar
BEFORE DELETE ON registro_ingresos_proveedor
BEGIN
    SELECT RAISE(ABORT, 'Los ingresos de proveedor no se pueden eliminar');
END;
CREATE TRIGGER registro_ingresos_proveedor_ingreso_inmutable
BEFORE UPDATE OF
    cedula, nombre, empresa_id, empresa_nombre, placa, gafete_numero,
    fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre, uuid
ON registro_ingresos_proveedor
WHEN
    NEW.cedula IS NOT OLD.cedula
    OR NEW.nombre IS NOT OLD.nombre
    OR NEW.empresa_id IS NOT OLD.empresa_id
    OR NEW.empresa_nombre IS NOT OLD.empresa_nombre
    OR NEW.placa IS NOT OLD.placa
    OR NEW.gafete_numero IS NOT OLD.gafete_numero
    OR NEW.fecha_hora_ingreso IS NOT OLD.fecha_hora_ingreso
    OR NEW.usuario_ingreso_id IS NOT OLD.usuario_ingreso_id
    OR NEW.usuario_ingreso_nombre IS NOT OLD.usuario_ingreso_nombre
    OR NEW.uuid IS NOT OLD.uuid
BEGIN
    SELECT RAISE(ABORT, 'Los datos de ingreso del proveedor son inmutables');
END;
CREATE TRIGGER registro_ingresos_proveedor_salida_unica
BEFORE UPDATE OF fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
ON registro_ingresos_proveedor
WHEN
    OLD.fecha_hora_salida IS NOT NULL
    OR NEW.fecha_hora_salida IS NULL
    OR NEW.usuario_salida_nombre IS NULL
BEGIN
    SELECT RAISE(ABORT, 'La salida solo puede registrarse una vez');
END;
CREATE TRIGGER registro_ingresos_proveedor_fecha_utc_insert
BEFORE INSERT ON registro_ingresos_proveedor
WHEN
    strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_ingreso) IS NOT NEW.fecha_hora_ingreso
    OR (
        NEW.fecha_hora_salida IS NOT NULL
        AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
    )
BEGIN
    SELECT RAISE(ABORT, 'Las fechas del ingreso de proveedor deben estar normalizadas en UTC');
END;
CREATE TRIGGER registro_ingresos_proveedor_salida_utc
BEFORE UPDATE OF fecha_hora_salida ON registro_ingresos_proveedor
WHEN
    NEW.fecha_hora_salida IS NOT NULL
    AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
BEGIN
    SELECT RAISE(ABORT, 'La fecha de salida debe estar normalizada en UTC');
END;

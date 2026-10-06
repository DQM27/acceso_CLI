CREATE TABLE registro_ingresos_correo (
    id INTEGER PRIMARY KEY,
    cedula TEXT NOT NULL,
    nombre TEXT NOT NULL,
    motivo TEXT NOT NULL CHECK (trim(motivo) <> ''),
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

CREATE UNIQUE INDEX idx_registro_ingresos_correo_uuid ON registro_ingresos_correo(uuid);
CREATE UNIQUE INDEX idx_registro_ingresos_correo_cedula_activa
ON registro_ingresos_correo(cedula) WHERE fecha_hora_salida IS NULL;
CREATE UNIQUE INDEX idx_registro_ingresos_correo_gafete_activo
ON registro_ingresos_correo(gafete_numero) WHERE fecha_hora_salida IS NULL;
CREATE INDEX idx_registro_ingresos_correo_fecha
ON registro_ingresos_correo(fecha_hora_ingreso DESC);

CREATE TRIGGER registro_ingresos_correo_no_eliminar
BEFORE DELETE ON registro_ingresos_correo
BEGIN
    SELECT RAISE(ABORT, 'Los ingresos por correo no se pueden eliminar');
END;
CREATE TRIGGER registro_ingresos_correo_ingreso_inmutable
BEFORE UPDATE OF
    cedula, nombre, motivo, placa, gafete_numero,
    fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre, uuid
ON registro_ingresos_correo
WHEN
    NEW.cedula IS NOT OLD.cedula
    OR NEW.nombre IS NOT OLD.nombre
    OR NEW.motivo IS NOT OLD.motivo
    OR NEW.placa IS NOT OLD.placa
    OR NEW.gafete_numero IS NOT OLD.gafete_numero
    OR NEW.fecha_hora_ingreso IS NOT OLD.fecha_hora_ingreso
    OR NEW.usuario_ingreso_id IS NOT OLD.usuario_ingreso_id
    OR NEW.usuario_ingreso_nombre IS NOT OLD.usuario_ingreso_nombre
    OR NEW.uuid IS NOT OLD.uuid
BEGIN
    SELECT RAISE(ABORT, 'Los datos de un ingreso por correo son inmutables');
END;
CREATE TRIGGER registro_ingresos_correo_salida_unica
BEFORE UPDATE OF fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
ON registro_ingresos_correo
WHEN
    OLD.fecha_hora_salida IS NOT NULL
    OR NEW.fecha_hora_salida IS NULL
    OR NEW.usuario_salida_nombre IS NULL
BEGIN
    SELECT RAISE(ABORT, 'La salida solo puede registrarse una vez');
END;
CREATE TRIGGER registro_ingresos_correo_fecha_utc_insert
BEFORE INSERT ON registro_ingresos_correo
WHEN
    strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_ingreso) IS NOT NEW.fecha_hora_ingreso
    OR (
        NEW.fecha_hora_salida IS NOT NULL
        AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
    )
BEGIN
    SELECT RAISE(ABORT, 'Las fechas del ingreso por correo deben estar normalizadas en UTC');
END;
CREATE TRIGGER registro_ingresos_correo_salida_utc
BEFORE UPDATE OF fecha_hora_salida ON registro_ingresos_correo
WHEN
    NEW.fecha_hora_salida IS NOT NULL
    AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
BEGIN
    SELECT RAISE(ABORT, 'La fecha de salida debe estar normalizada en UTC');
END;

CREATE TABLE ingresos_correo_remotos (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    cedula TEXT NOT NULL,
    nombre TEXT NOT NULL,
    motivo TEXT NOT NULL,
    placa TEXT,
    gafete_numero INTEGER NOT NULL,
    hora_entrada TEXT NOT NULL,
    usuario_entrada_nombre TEXT NOT NULL,
    dispositivo_entrada_id TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
) STRICT;

CREATE TABLE historial_ingresos_correo_sitio (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    cedula TEXT NOT NULL,
    nombre TEXT NOT NULL,
    motivo TEXT,
    placa TEXT,
    gafete_numero INTEGER,
    hora_entrada TEXT NOT NULL,
    hora_salida TEXT,
    usuario_entrada_nombre TEXT,
    usuario_salida_nombre TEXT,
    dispositivo_entrada_id TEXT NOT NULL,
    dispositivo_salida_id TEXT,
    actualizado_en TEXT NOT NULL
) STRICT;
CREATE INDEX idx_historial_ingresos_correo_sitio_hora_entrada
ON historial_ingresos_correo_sitio(hora_entrada);
ALTER TABLE sincronizacion_estado ADD COLUMN historial_ingresos_correo_actualizado_hasta TEXT;

CREATE TABLE cola_salida_nueva (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (
        entidad IN (
            'contratista', 'ingreso', 'empresa', 'gafete', 'usuario', 'movimiento_visita',
            'vehiculo_ruta', 'encargado_ruta', 'salida_ruta', 'ruta', 'prestamo_gafete_provisional',
            'empresa_proveedor', 'ingreso_proveedor', 'ingreso_correo'
        )
    ),
    entidad_uuid TEXT NOT NULL,
    operacion TEXT NOT NULL CHECK (operacion IN ('crear', 'actualizar', 'cerrar')),
    estado TEXT NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'enviado', 'fallido')),
    intentos INTEGER NOT NULL DEFAULT 0 CHECK (intentos >= 0),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL,
    ultimo_error TEXT,
    proximo_intento_en TEXT GENERATED ALWAYS AS (
        datetime(actualizado_en, '+' || MIN(intentos * 15, 1440) || ' minutes')
    ) STORED
) STRICT;
INSERT INTO cola_salida_nueva (
    id, entidad, entidad_uuid, operacion, estado, intentos,
    creado_en, actualizado_en, ultimo_error
)
SELECT
    id, entidad, entidad_uuid, operacion, estado, intentos,
    creado_en, actualizado_en, ultimo_error
FROM cola_salida;
DROP TABLE cola_salida;
ALTER TABLE cola_salida_nueva RENAME TO cola_salida;
CREATE INDEX idx_cola_salida_pendientes
ON cola_salida(proximo_intento_en)
WHERE estado = 'pendiente';

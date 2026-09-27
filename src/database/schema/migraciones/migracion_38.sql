
CREATE TABLE rutas (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL UNIQUE CHECK (numero > 0),
    activo INTEGER NOT NULL DEFAULT 1 CHECK (activo IN (0, 1)),
    uuid TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX idx_rutas_uuid ON rutas(uuid);

DROP TABLE salidas_ruta;
CREATE TABLE salidas_ruta (
    id INTEGER PRIMARY KEY,
    vehiculo_id INTEGER REFERENCES vehiculos_ruta(id) ON DELETE RESTRICT,
    vehiculo_placa TEXT NOT NULL,
    vehiculo_numero_unidad TEXT,
    encargado_id INTEGER REFERENCES encargados_ruta(id) ON DELETE RESTRICT,
    encargado_nombre TEXT NOT NULL,
    ruta_id INTEGER NOT NULL REFERENCES rutas(id) ON DELETE RESTRICT,
    numero_ruta INTEGER NOT NULL,
    sub_numero INTEGER NOT NULL CHECK (sub_numero >= 1),
    numero_documento TEXT NOT NULL,
    fecha_documento TEXT NOT NULL,
    resultado TEXT NOT NULL CHECK (resultado IN ('PERMITIDO', 'PERMITIDO_CON_AUTORIZACION')),
    motivo_resultado TEXT CHECK (
        motivo_resultado IS NULL OR motivo_resultado IN ('DOCUMENTO_FECHA_DISTINTA')
    ),
    fecha_hora_salida TEXT NOT NULL,
    usuario_salida_id INTEGER NOT NULL REFERENCES usuarios(id),
    usuario_salida_nombre TEXT NOT NULL,
    fecha_hora_retorno TEXT,
    usuario_retorno_id INTEGER REFERENCES usuarios(id),
    usuario_retorno_nombre TEXT,
    uuid TEXT NOT NULL,
    CHECK (
        (resultado = 'PERMITIDO' AND motivo_resultado IS NULL)
        OR (resultado = 'PERMITIDO_CON_AUTORIZACION' AND motivo_resultado = 'DOCUMENTO_FECHA_DISTINTA')
    ),
    CHECK (
        (fecha_hora_retorno IS NULL
            AND usuario_retorno_id IS NULL
            AND usuario_retorno_nombre IS NULL)
        OR
        (fecha_hora_retorno IS NOT NULL
            AND usuario_retorno_nombre IS NOT NULL)
    ),
    CHECK (fecha_hora_retorno IS NULL OR fecha_hora_retorno >= fecha_hora_salida)
) STRICT;
CREATE UNIQUE INDEX idx_salidas_ruta_uuid ON salidas_ruta(uuid);
CREATE UNIQUE INDEX idx_salidas_ruta_numero_documento ON salidas_ruta(numero_documento);
CREATE INDEX idx_salidas_ruta_vehiculo ON salidas_ruta(vehiculo_id) WHERE vehiculo_id IS NOT NULL;
CREATE INDEX idx_salidas_ruta_encargado ON salidas_ruta(encargado_id) WHERE encargado_id IS NOT NULL;
CREATE INDEX idx_salidas_ruta_ruta ON salidas_ruta(ruta_id);
-- Un mismo vehículo (por placa, no por id -- vale incluso sin match de
-- catálogo) no puede tener dos salidas abiertas a la vez.
CREATE UNIQUE INDEX idx_salidas_ruta_placa_activa
ON salidas_ruta(vehiculo_placa) WHERE fecha_hora_retorno IS NULL;

CREATE TRIGGER salidas_ruta_no_eliminar
BEFORE DELETE ON salidas_ruta
BEGIN
    SELECT RAISE(ABORT, 'Las salidas de ruta no se pueden eliminar');
END;
CREATE TRIGGER salidas_ruta_apertura_inmutable
BEFORE UPDATE OF
    vehiculo_id, vehiculo_placa, vehiculo_numero_unidad, encargado_id, encargado_nombre,
    ruta_id, numero_ruta, sub_numero, numero_documento, fecha_documento, resultado,
    motivo_resultado, fecha_hora_salida, usuario_salida_id, usuario_salida_nombre, uuid
ON salidas_ruta
WHEN
    NEW.vehiculo_id IS NOT OLD.vehiculo_id
    OR NEW.vehiculo_placa IS NOT OLD.vehiculo_placa
    OR NEW.vehiculo_numero_unidad IS NOT OLD.vehiculo_numero_unidad
    OR NEW.encargado_id IS NOT OLD.encargado_id
    OR NEW.encargado_nombre IS NOT OLD.encargado_nombre
    OR NEW.ruta_id IS NOT OLD.ruta_id
    OR NEW.numero_ruta IS NOT OLD.numero_ruta
    OR NEW.sub_numero IS NOT OLD.sub_numero
    OR NEW.numero_documento IS NOT OLD.numero_documento
    OR NEW.fecha_documento IS NOT OLD.fecha_documento
    OR NEW.resultado IS NOT OLD.resultado
    OR NEW.motivo_resultado IS NOT OLD.motivo_resultado
    OR NEW.fecha_hora_salida IS NOT OLD.fecha_hora_salida
    OR NEW.usuario_salida_id IS NOT OLD.usuario_salida_id
    OR NEW.usuario_salida_nombre IS NOT OLD.usuario_salida_nombre
    OR NEW.uuid IS NOT OLD.uuid
BEGIN
    SELECT RAISE(ABORT, 'Los datos de salida de la ruta son inmutables');
END;
CREATE TRIGGER salidas_ruta_retorno_unico
BEFORE UPDATE OF fecha_hora_retorno, usuario_retorno_id, usuario_retorno_nombre
ON salidas_ruta
WHEN
    OLD.fecha_hora_retorno IS NOT NULL
    OR NEW.fecha_hora_retorno IS NULL
    OR NEW.usuario_retorno_nombre IS NULL
BEGIN
    SELECT RAISE(ABORT, 'El retorno solo puede registrarse una vez');
END;
CREATE TRIGGER salidas_ruta_fecha_utc_insert
BEFORE INSERT ON salidas_ruta
WHEN
    strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
    OR (
        NEW.fecha_hora_retorno IS NOT NULL
        AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_retorno) IS NOT NEW.fecha_hora_retorno
    )
BEGIN
    SELECT RAISE(ABORT, 'Las fechas de la salida de ruta deben estar normalizadas en UTC');
END;
CREATE TRIGGER salidas_ruta_retorno_utc
BEFORE UPDATE OF fecha_hora_retorno ON salidas_ruta
WHEN
    NEW.fecha_hora_retorno IS NOT NULL
    AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_retorno) IS NOT NEW.fecha_hora_retorno
BEGIN
    SELECT RAISE(ABORT, 'La fecha de retorno debe estar normalizada en UTC');
END;

-- Suma 'ruta' al CHECK de `cola_salida.entidad` -- mismo patrón que
-- MIGRACION_30/36 (SQLite no permite `ALTER TABLE ... CHECK`, la tabla se
-- recrea entera).
CREATE TABLE cola_salida_nueva (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (
        entidad IN (
            'contratista', 'ingreso', 'empresa', 'gafete', 'usuario', 'movimiento_visita',
            'vehiculo_ruta', 'encargado_ruta', 'salida_ruta', 'ruta'
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

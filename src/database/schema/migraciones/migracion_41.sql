
CREATE TABLE empresas_proveedor (
    id INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL UNIQUE,
    activo INTEGER NOT NULL DEFAULT 1 CHECK (activo IN (0, 1)),
    uuid TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX idx_empresas_proveedor_uuid ON empresas_proveedor(uuid);

CREATE TABLE registro_ingresos_proveedor (
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
        (fecha_hora_salida IS NOT NULL AND usuario_salida_id IS NOT NULL
            AND usuario_salida_nombre IS NOT NULL)
    ),
    CHECK (fecha_hora_salida IS NULL OR fecha_hora_salida >= fecha_hora_ingreso)
) STRICT;
CREATE UNIQUE INDEX idx_registro_ingresos_proveedor_uuid ON registro_ingresos_proveedor(uuid);
-- Ni la misma cédula ni el mismo gafete pueden tener dos ingresos de
-- proveedor abiertos a la vez.
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

CREATE TABLE gafetes_nueva (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL,
    tipo TEXT NOT NULL CHECK (tipo IN ('CONTRATISTA', 'VISITA', 'PROVEEDOR', 'PROVISIONAL_KOF')),
    estado TEXT NOT NULL CHECK (estado IN ('DISPONIBLE', 'PERDIDO', 'DE_BAJA')),
    contratista_portador_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    visita_portador_id INTEGER REFERENCES cita_visitantes(id) ON DELETE RESTRICT,
    encargado_portador_id INTEGER REFERENCES encargados_ruta(id) ON DELETE RESTRICT,
    proveedor_portador_id INTEGER REFERENCES registro_ingresos_proveedor(id) ON DELETE RESTRICT,
    uuid TEXT,
    CHECK (
        (tipo = 'CONTRATISTA'
            AND visita_portador_id IS NULL AND encargado_portador_id IS NULL
            AND proveedor_portador_id IS NULL)
        OR (tipo = 'VISITA'
            AND contratista_portador_id IS NULL AND encargado_portador_id IS NULL
            AND proveedor_portador_id IS NULL)
        OR (tipo = 'PROVEEDOR'
            AND contratista_portador_id IS NULL AND visita_portador_id IS NULL
            AND encargado_portador_id IS NULL)
        OR (tipo = 'PROVISIONAL_KOF'
            AND contratista_portador_id IS NULL AND visita_portador_id IS NULL
            AND proveedor_portador_id IS NULL)
    ),
    CHECK (
        (estado = 'PERDIDO'
            AND (contratista_portador_id IS NOT NULL OR visita_portador_id IS NOT NULL
                OR encargado_portador_id IS NOT NULL OR proveedor_portador_id IS NOT NULL))
        OR (estado <> 'PERDIDO'
            AND contratista_portador_id IS NULL AND visita_portador_id IS NULL
            AND encargado_portador_id IS NULL AND proveedor_portador_id IS NULL)
    )
) STRICT;
INSERT INTO gafetes_nueva (
    id, numero, tipo, estado, contratista_portador_id, visita_portador_id,
    encargado_portador_id, proveedor_portador_id, uuid
)
SELECT id, numero, tipo, estado, contratista_portador_id, visita_portador_id,
    encargado_portador_id, NULL, uuid
FROM gafetes;
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
CREATE INDEX idx_gafetes_proveedor_portador
ON gafetes(proveedor_portador_id) WHERE proveedor_portador_id IS NOT NULL;

CREATE TABLE gafetes_incidentes_nueva (
    id INTEGER PRIMARY KEY,
    gafete_id INTEGER NOT NULL REFERENCES gafetes(id) ON DELETE RESTRICT,
    tipo TEXT NOT NULL CHECK (tipo IN ('PERDIDO', 'RESUELTO')),
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    visita_portador_id INTEGER REFERENCES cita_visitantes(id) ON DELETE RESTRICT,
    encargado_portador_id INTEGER REFERENCES encargados_ruta(id) ON DELETE RESTRICT,
    proveedor_portador_id INTEGER REFERENCES registro_ingresos_proveedor(id) ON DELETE RESTRICT,
    motivo_resolucion TEXT CHECK (
        motivo_resolucion IS NULL OR motivo_resolucion IN ('PAGADO', 'APARECIDO')
    ),
    CHECK (
        (tipo = 'PERDIDO'
            AND (contratista_id IS NOT NULL OR visita_portador_id IS NOT NULL
                OR encargado_portador_id IS NOT NULL OR proveedor_portador_id IS NOT NULL)
            AND motivo_resolucion IS NULL)
        OR (tipo = 'RESUELTO'
            AND contratista_id IS NULL AND visita_portador_id IS NULL
            AND encargado_portador_id IS NULL AND proveedor_portador_id IS NULL
            AND motivo_resolucion IS NOT NULL)
    )
) STRICT;
INSERT INTO gafetes_incidentes_nueva (
    id, gafete_id, tipo, fecha_hora, usuario_id, contratista_id, visita_portador_id,
    encargado_portador_id, proveedor_portador_id, motivo_resolucion
)
SELECT id, gafete_id, tipo, fecha_hora, usuario_id, contratista_id, visita_portador_id,
    encargado_portador_id, NULL, motivo_resolucion
FROM gafetes_incidentes;
DROP TABLE gafetes_incidentes;
ALTER TABLE gafetes_incidentes_nueva RENAME TO gafetes_incidentes;
CREATE INDEX idx_gafetes_incidentes_gafete ON gafetes_incidentes(gafete_id, id DESC);
CREATE INDEX idx_gafetes_incidentes_fecha ON gafetes_incidentes(fecha_hora DESC, id DESC);

CREATE TABLE cola_salida_nueva (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (
        entidad IN (
            'contratista', 'ingreso', 'empresa', 'gafete', 'usuario', 'movimiento_visita',
            'vehiculo_ruta', 'encargado_ruta', 'salida_ruta', 'ruta', 'prestamo_gafete_provisional',
            'empresa_proveedor', 'ingreso_proveedor'
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

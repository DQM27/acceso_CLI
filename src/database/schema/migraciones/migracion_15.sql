
CREATE TABLE empresas_nueva (
    id INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL UNIQUE,
    activo INTEGER NOT NULL DEFAULT 1 CHECK (activo IN (0, 1))
) STRICT;
INSERT INTO empresas_nueva SELECT * FROM empresas;
DROP TABLE empresas;
ALTER TABLE empresas_nueva RENAME TO empresas;
CREATE TRIGGER empresas_fts_ad AFTER DELETE ON empresas BEGIN
    INSERT INTO empresas_fts(empresas_fts, rowid, nombre)
    VALUES ('delete', old.id, old.nombre);
END;
CREATE TRIGGER empresas_fts_ai AFTER INSERT ON empresas BEGIN
    INSERT INTO empresas_fts(rowid, nombre) VALUES (new.id, new.nombre);
END;
CREATE TRIGGER empresas_fts_au AFTER UPDATE ON empresas BEGIN
    INSERT INTO empresas_fts(empresas_fts, rowid, nombre)
    VALUES ('delete', old.id, old.nombre);
    INSERT INTO empresas_fts(rowid, nombre) VALUES (new.id, new.nombre);
END;

CREATE TABLE usuarios_nueva (
    id INTEGER PRIMARY KEY,
    cedula TEXT NOT NULL UNIQUE,
    nombre TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    rol TEXT NOT NULL CHECK (rol IN ('ROOT', 'ADMINISTRADOR', 'OPERADOR')),
    activo INTEGER NOT NULL CHECK (activo IN (0, 1))
) STRICT;
INSERT INTO usuarios_nueva SELECT * FROM usuarios;
DROP TABLE usuarios;
ALTER TABLE usuarios_nueva RENAME TO usuarios;
CREATE TRIGGER usuarios_fts_ad AFTER DELETE ON usuarios BEGIN
    INSERT INTO usuarios_fts(usuarios_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
END;
CREATE TRIGGER usuarios_fts_ai AFTER INSERT ON usuarios BEGIN
    INSERT INTO usuarios_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;
CREATE TRIGGER usuarios_fts_au AFTER UPDATE ON usuarios BEGIN
    INSERT INTO usuarios_fts(usuarios_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
    INSERT INTO usuarios_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;

CREATE TABLE contratistas_nueva (
    id INTEGER PRIMARY KEY,
    cedula TEXT NOT NULL UNIQUE,
    nombre TEXT NOT NULL,
    empresa_id INTEGER NOT NULL,
    tipo_ingreso TEXT NOT NULL CHECK (
        tipo_ingreso IN ('PRAIND', 'IN_HOUSE', 'POR_CORREO', 'SWAT')
    ),
    fecha_vencimiento_praind TEXT,
    es_personal_ruta INTEGER NOT NULL DEFAULT 0 CHECK (es_personal_ruta IN (0, 1)),
    tiene_acceso INTEGER NOT NULL CHECK (tiene_acceso IN (0, 1)),
    FOREIGN KEY (empresa_id) REFERENCES empresas(id)
) STRICT;
INSERT INTO contratistas_nueva SELECT * FROM contratistas;
DROP TABLE contratistas;
ALTER TABLE contratistas_nueva RENAME TO contratistas;
CREATE INDEX idx_contratistas_empresa ON contratistas(empresa_id);
CREATE TRIGGER contratistas_fts_ad AFTER DELETE ON contratistas BEGIN
    INSERT INTO contratistas_fts(contratistas_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
END;
CREATE TRIGGER contratistas_fts_ai AFTER INSERT ON contratistas BEGIN
    INSERT INTO contratistas_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;
CREATE TRIGGER contratistas_fts_au AFTER UPDATE ON contratistas BEGIN
    INSERT INTO contratistas_fts(contratistas_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
    INSERT INTO contratistas_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;

CREATE TABLE gafetes_nueva (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL UNIQUE,
    estado TEXT NOT NULL CHECK (estado IN ('DISPONIBLE', 'PERDIDO', 'DE_BAJA')),
    contratista_deudor_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    CHECK (
        (estado = 'PERDIDO' AND contratista_deudor_id IS NOT NULL)
        OR (estado <> 'PERDIDO' AND contratista_deudor_id IS NULL)
    )
) STRICT;
INSERT INTO gafetes_nueva SELECT * FROM gafetes;
DROP TABLE gafetes;
ALTER TABLE gafetes_nueva RENAME TO gafetes;
CREATE INDEX idx_gafetes_estado ON gafetes(estado);
CREATE INDEX idx_gafetes_contratista_deudor
ON gafetes(contratista_deudor_id) WHERE contratista_deudor_id IS NOT NULL;

CREATE TABLE gafetes_incidentes_nueva (
    id INTEGER PRIMARY KEY,
    gafete_id INTEGER NOT NULL REFERENCES gafetes(id) ON DELETE RESTRICT,
    tipo TEXT NOT NULL CHECK (tipo IN ('PERDIDO', 'RESUELTO')),
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    motivo_resolucion TEXT CHECK (
        motivo_resolucion IS NULL OR motivo_resolucion IN ('PAGADO', 'APARECIDO')
    ),
    CHECK (
        (tipo = 'PERDIDO' AND contratista_id IS NOT NULL AND motivo_resolucion IS NULL)
        OR (tipo = 'RESUELTO' AND contratista_id IS NULL AND motivo_resolucion IS NOT NULL)
    )
) STRICT;
INSERT INTO gafetes_incidentes_nueva SELECT * FROM gafetes_incidentes;
DROP TABLE gafetes_incidentes;
ALTER TABLE gafetes_incidentes_nueva RENAME TO gafetes_incidentes;
CREATE INDEX idx_gafetes_incidentes_gafete ON gafetes_incidentes(gafete_id, id DESC);
CREATE INDEX idx_gafetes_incidentes_fecha ON gafetes_incidentes(fecha_hora DESC, id DESC);

CREATE TABLE registro_ingresos_nueva (
    id INTEGER PRIMARY KEY,
    contratista_id INTEGER NOT NULL,
    empresa_id INTEGER NOT NULL,
    fecha_hora_ingreso TEXT NOT NULL,
    medio_ingreso TEXT NOT NULL CHECK (medio_ingreso IN ('CAMINANDO', 'VEHICULO')),
    tipo_ingreso TEXT NOT NULL CHECK (
        tipo_ingreso IN ('PRAIND', 'IN_HOUSE', 'POR_CORREO', 'SWAT')
    ),
    gafete_numero INTEGER,
    usuario_ingreso_id INTEGER NOT NULL,
    fecha_hora_salida TEXT,
    usuario_salida_id INTEGER,
    contratista_cedula TEXT NOT NULL,
    contratista_nombre TEXT NOT NULL,
    empresa_nombre TEXT NOT NULL,
    usuario_ingreso_nombre TEXT NOT NULL,
    usuario_salida_nombre TEXT,
    fecha_vencimiento_praind TEXT,
    es_personal_ruta INTEGER NOT NULL CHECK (es_personal_ruta IN (0, 1)),
    tiene_acceso INTEGER NOT NULL CHECK (tiene_acceso IN (0, 1)),
    resultado_acceso TEXT NOT NULL CHECK (
        resultado_acceso IN ('PERMITIDO', 'PERMITIDO_CON_ADVERTENCIA', 'MIGRADO')
    ),
    motivo_resultado TEXT CHECK (
        motivo_resultado IS NULL
        OR motivo_resultado IN ('PRAIND_PROXIMO_VENCER', 'DATOS_RECONSTRUIDOS')
    ),
    reglas_version INTEGER NOT NULL CHECK (reglas_version >= 0),
    empresa_activa_snapshot INTEGER NOT NULL DEFAULT 1
        CHECK (empresa_activa_snapshot IN (0, 1)),
    CHECK (
        (fecha_hora_salida IS NULL
            AND usuario_salida_id IS NULL
            AND usuario_salida_nombre IS NULL)
        OR
        (fecha_hora_salida IS NOT NULL
            AND usuario_salida_id IS NOT NULL
            AND usuario_salida_nombre IS NOT NULL)
    ),
    CHECK (fecha_hora_salida IS NULL OR fecha_hora_salida >= fecha_hora_ingreso),
    CHECK (
        (resultado_acceso = 'PERMITIDO' AND motivo_resultado IS NULL AND reglas_version > 0)
        OR
        (resultado_acceso = 'PERMITIDO_CON_ADVERTENCIA'
            AND motivo_resultado = 'PRAIND_PROXIMO_VENCER'
            AND reglas_version > 0)
        OR
        (resultado_acceso = 'MIGRADO'
            AND motivo_resultado = 'DATOS_RECONSTRUIDOS'
            AND reglas_version = 0)
    ),
    FOREIGN KEY (contratista_id) REFERENCES contratistas(id),
    FOREIGN KEY (empresa_id) REFERENCES empresas(id),
    FOREIGN KEY (usuario_ingreso_id) REFERENCES usuarios(id),
    FOREIGN KEY (usuario_salida_id) REFERENCES usuarios(id)
) STRICT;
INSERT INTO registro_ingresos_nueva SELECT * FROM registro_ingresos;
DROP TABLE registro_ingresos;
ALTER TABLE registro_ingresos_nueva RENAME TO registro_ingresos;
CREATE INDEX idx_registro_ingresos_contratista ON registro_ingresos(contratista_id);
CREATE INDEX idx_registro_ingresos_empresa ON registro_ingresos(empresa_id);
CREATE INDEX idx_registro_ingresos_fecha_ingreso ON registro_ingresos(fecha_hora_ingreso);
CREATE INDEX idx_registro_ingresos_fecha_salida
ON registro_ingresos(fecha_hora_salida)
WHERE fecha_hora_salida IS NOT NULL;
CREATE INDEX idx_registro_ingresos_gafete ON registro_ingresos(gafete_numero);
CREATE UNIQUE INDEX idx_registro_ingresos_contratista_activo
ON registro_ingresos(contratista_id) WHERE fecha_hora_salida IS NULL;
CREATE UNIQUE INDEX idx_registro_ingresos_gafete_activo
ON registro_ingresos(gafete_numero)
WHERE gafete_numero IS NOT NULL AND fecha_hora_salida IS NULL;
CREATE TRIGGER registro_ingresos_no_eliminar
BEFORE DELETE ON registro_ingresos
BEGIN
    SELECT RAISE(ABORT, 'Los movimientos de acceso no se pueden eliminar');
END;
CREATE TRIGGER registro_ingresos_entrada_inmutable
BEFORE UPDATE OF
    contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
    gafete_numero, usuario_ingreso_id, contratista_cedula, contratista_nombre,
    empresa_nombre, usuario_ingreso_nombre, fecha_vencimiento_praind,
    es_personal_ruta, tiene_acceso, resultado_acceso, motivo_resultado,
    reglas_version, empresa_activa_snapshot
ON registro_ingresos
WHEN
    NEW.contratista_id IS NOT OLD.contratista_id
    OR NEW.empresa_id IS NOT OLD.empresa_id
    OR NEW.fecha_hora_ingreso IS NOT OLD.fecha_hora_ingreso
    OR NEW.medio_ingreso IS NOT OLD.medio_ingreso
    OR NEW.tipo_ingreso IS NOT OLD.tipo_ingreso
    OR NEW.gafete_numero IS NOT OLD.gafete_numero
    OR NEW.usuario_ingreso_id IS NOT OLD.usuario_ingreso_id
    OR NEW.contratista_cedula IS NOT OLD.contratista_cedula
    OR NEW.contratista_nombre IS NOT OLD.contratista_nombre
    OR NEW.empresa_nombre IS NOT OLD.empresa_nombre
    OR NEW.usuario_ingreso_nombre IS NOT OLD.usuario_ingreso_nombre
    OR NEW.fecha_vencimiento_praind IS NOT OLD.fecha_vencimiento_praind
    OR NEW.es_personal_ruta IS NOT OLD.es_personal_ruta
    OR NEW.tiene_acceso IS NOT OLD.tiene_acceso
    OR NEW.resultado_acceso IS NOT OLD.resultado_acceso
    OR NEW.motivo_resultado IS NOT OLD.motivo_resultado
    OR NEW.reglas_version IS NOT OLD.reglas_version
    OR NEW.empresa_activa_snapshot IS NOT OLD.empresa_activa_snapshot
BEGIN
    SELECT RAISE(ABORT, 'Los datos historicos del ingreso son inmutables');
END;
CREATE TRIGGER registro_ingresos_salida_unica
BEFORE UPDATE OF fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
ON registro_ingresos
WHEN
    OLD.fecha_hora_salida IS NOT NULL
    OR NEW.fecha_hora_salida IS NULL
    OR NEW.usuario_salida_id IS NULL
    OR NEW.usuario_salida_nombre IS NULL
BEGIN
    SELECT RAISE(ABORT, 'La salida solo puede registrarse una vez');
END;
CREATE TRIGGER registro_ingresos_fecha_utc_insert
BEFORE INSERT ON registro_ingresos
WHEN
    strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_ingreso) IS NOT NEW.fecha_hora_ingreso
    OR (
        NEW.fecha_hora_salida IS NOT NULL
        AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
    )
BEGIN
    SELECT RAISE(ABORT, 'Las fechas de movimientos deben estar normalizadas en UTC');
END;
CREATE TRIGGER registro_ingresos_salida_utc
BEFORE UPDATE OF fecha_hora_salida ON registro_ingresos
WHEN
    NEW.fecha_hora_salida IS NOT NULL
    AND strftime('%Y-%m-%dT%H:%M:%SZ', NEW.fecha_hora_salida) IS NOT NEW.fecha_hora_salida
BEGIN
    SELECT RAISE(ABORT, 'La fecha de salida debe estar normalizada en UTC');
END;
CREATE TRIGGER registro_ingresos_fts_ad AFTER DELETE ON registro_ingresos BEGIN
    INSERT INTO registro_ingresos_fts(
        registro_ingresos_fts, rowid, contratista_cedula,
        contratista_nombre, empresa_nombre
    ) VALUES (
        'delete', old.id, old.contratista_cedula,
        old.contratista_nombre, old.empresa_nombre
    );
END;
CREATE TRIGGER registro_ingresos_fts_ai AFTER INSERT ON registro_ingresos BEGIN
    INSERT INTO registro_ingresos_fts(
        rowid, contratista_cedula, contratista_nombre, empresa_nombre
    ) VALUES (
        new.id, new.contratista_cedula, new.contratista_nombre, new.empresa_nombre
    );
END;

CREATE TABLE auditoria_cambios_nueva (
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
    CHECK (valor_anterior IS NOT valor_nuevo OR valor_anterior IS NULL)
) STRICT;
INSERT INTO auditoria_cambios_nueva SELECT * FROM auditoria_cambios;
DROP TABLE auditoria_cambios;
ALTER TABLE auditoria_cambios_nueva RENAME TO auditoria_cambios;
CREATE INDEX idx_auditoria_cambios_fecha ON auditoria_cambios(fecha_hora DESC, id DESC);
CREATE INDEX idx_auditoria_cambios_entidad
ON auditoria_cambios(entidad, entidad_id, id DESC);

use rusqlite::{Connection, Transaction, TransactionBehavior};

pub const SCHEMA_VERSION: i64 = 4;

pub fn initialize_database(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;

    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    let mut version: i64 = transaction.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if version == 0 {
        aplicar_migracion(&transaction, MIGRACION_1, 1)?;
        version = 1;
    }

    if version == 1 {
        aplicar_migracion(&transaction, MIGRACION_2, 2)?;
        version = 2;
    }

    if version == 2 {
        aplicar_migracion(&transaction, MIGRACION_3, 3)?;
        version = 3;
    }

    if version == 3 {
        aplicar_migracion(&transaction, MIGRACION_4, 4)?;
        version = 4;
    }

    if version != SCHEMA_VERSION {
        return Err(rusqlite::Error::InvalidQuery);
    }

    transaction.commit()
}

fn aplicar_migracion(
    transaction: &Transaction<'_>,
    sql: &str,
    nueva_version: i64,
) -> rusqlite::Result<()> {
    transaction.execute_batch(sql)?;
    transaction.execute_batch(&format!("PRAGMA user_version = {nueva_version}"))
}

const MIGRACION_1: &str = r#"
CREATE TABLE IF NOT EXISTS empresas (
    id INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS usuarios (
    id INTEGER PRIMARY KEY,
    cedula TEXT NOT NULL UNIQUE,
    nombre TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    rol TEXT NOT NULL CHECK (rol IN ('ROOT', 'ADMINISTRADOR', 'OPERADOR')),
    activo INTEGER NOT NULL CHECK (activo IN (0, 1))
);

CREATE TABLE IF NOT EXISTS contratistas (
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
);

CREATE INDEX IF NOT EXISTS idx_contratistas_empresa
ON contratistas(empresa_id);

CREATE TABLE IF NOT EXISTS registro_ingresos (
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
    FOREIGN KEY (contratista_id) REFERENCES contratistas(id),
    FOREIGN KEY (empresa_id) REFERENCES empresas(id),
    FOREIGN KEY (usuario_ingreso_id) REFERENCES usuarios(id),
    FOREIGN KEY (usuario_salida_id) REFERENCES usuarios(id)
);

CREATE INDEX IF NOT EXISTS idx_registro_ingresos_contratista
ON registro_ingresos(contratista_id);
CREATE INDEX IF NOT EXISTS idx_registro_ingresos_empresa
ON registro_ingresos(empresa_id);
CREATE INDEX IF NOT EXISTS idx_registro_ingresos_fecha_ingreso
ON registro_ingresos(fecha_hora_ingreso);
CREATE UNIQUE INDEX IF NOT EXISTS idx_registro_ingresos_contratista_activo
ON registro_ingresos(contratista_id) WHERE fecha_hora_salida IS NULL;
CREATE INDEX IF NOT EXISTS idx_registro_ingresos_gafete
ON registro_ingresos(gafete_numero);
CREATE UNIQUE INDEX IF NOT EXISTS idx_registro_ingresos_gafete_activo
ON registro_ingresos(gafete_numero)
WHERE gafete_numero IS NOT NULL AND fecha_hora_salida IS NULL;
"#;

const MIGRACION_2: &str = r#"
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
    CHECK (
        (fecha_hora_salida IS NULL AND usuario_salida_id IS NULL)
        OR
        (fecha_hora_salida IS NOT NULL AND usuario_salida_id IS NOT NULL)
    ),
    FOREIGN KEY (contratista_id) REFERENCES contratistas(id),
    FOREIGN KEY (empresa_id) REFERENCES empresas(id),
    FOREIGN KEY (usuario_ingreso_id) REFERENCES usuarios(id),
    FOREIGN KEY (usuario_salida_id) REFERENCES usuarios(id)
);

INSERT INTO registro_ingresos_nueva (
    id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso,
    tipo_ingreso, gafete_numero, usuario_ingreso_id, fecha_hora_salida,
    usuario_salida_id
)
SELECT
    id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso,
    tipo_ingreso, gafete_numero, usuario_ingreso_id, fecha_hora_salida,
    usuario_salida_id
FROM registro_ingresos;

DROP TABLE registro_ingresos;
ALTER TABLE registro_ingresos_nueva RENAME TO registro_ingresos;

CREATE INDEX idx_registro_ingresos_contratista
ON registro_ingresos(contratista_id);
CREATE INDEX idx_registro_ingresos_empresa
ON registro_ingresos(empresa_id);
CREATE INDEX idx_registro_ingresos_fecha_ingreso
ON registro_ingresos(fecha_hora_ingreso);
CREATE UNIQUE INDEX idx_registro_ingresos_contratista_activo
ON registro_ingresos(contratista_id) WHERE fecha_hora_salida IS NULL;
CREATE INDEX idx_registro_ingresos_gafete
ON registro_ingresos(gafete_numero);
CREATE UNIQUE INDEX idx_registro_ingresos_gafete_activo
ON registro_ingresos(gafete_numero)
WHERE gafete_numero IS NOT NULL AND fecha_hora_salida IS NULL;
"#;

const MIGRACION_3: &str = r#"
CREATE VIRTUAL TABLE contratistas_fts USING fts5(
    cedula, nombre,
    content='contratistas', content_rowid='id',
    tokenize='trigram case_sensitive 0 remove_diacritics 1'
);
CREATE VIRTUAL TABLE empresas_fts USING fts5(
    nombre,
    content='empresas', content_rowid='id',
    tokenize='trigram case_sensitive 0 remove_diacritics 1'
);
CREATE VIRTUAL TABLE usuarios_fts USING fts5(
    cedula, nombre,
    content='usuarios', content_rowid='id',
    tokenize='trigram case_sensitive 0 remove_diacritics 1'
);

CREATE TRIGGER contratistas_fts_ai AFTER INSERT ON contratistas BEGIN
    INSERT INTO contratistas_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;
CREATE TRIGGER contratistas_fts_ad AFTER DELETE ON contratistas BEGIN
    INSERT INTO contratistas_fts(contratistas_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
END;
CREATE TRIGGER contratistas_fts_au AFTER UPDATE ON contratistas BEGIN
    INSERT INTO contratistas_fts(contratistas_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
    INSERT INTO contratistas_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;

CREATE TRIGGER empresas_fts_ai AFTER INSERT ON empresas BEGIN
    INSERT INTO empresas_fts(rowid, nombre) VALUES (new.id, new.nombre);
END;
CREATE TRIGGER empresas_fts_ad AFTER DELETE ON empresas BEGIN
    INSERT INTO empresas_fts(empresas_fts, rowid, nombre)
    VALUES ('delete', old.id, old.nombre);
END;
CREATE TRIGGER empresas_fts_au AFTER UPDATE ON empresas BEGIN
    INSERT INTO empresas_fts(empresas_fts, rowid, nombre)
    VALUES ('delete', old.id, old.nombre);
    INSERT INTO empresas_fts(rowid, nombre) VALUES (new.id, new.nombre);
END;

CREATE TRIGGER usuarios_fts_ai AFTER INSERT ON usuarios BEGIN
    INSERT INTO usuarios_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;
CREATE TRIGGER usuarios_fts_ad AFTER DELETE ON usuarios BEGIN
    INSERT INTO usuarios_fts(usuarios_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
END;
CREATE TRIGGER usuarios_fts_au AFTER UPDATE ON usuarios BEGIN
    INSERT INTO usuarios_fts(usuarios_fts, rowid, cedula, nombre)
    VALUES ('delete', old.id, old.cedula, old.nombre);
    INSERT INTO usuarios_fts(rowid, cedula, nombre)
    VALUES (new.id, new.cedula, new.nombre);
END;

INSERT INTO contratistas_fts(contratistas_fts) VALUES ('rebuild');
INSERT INTO empresas_fts(empresas_fts) VALUES ('rebuild');
INSERT INTO usuarios_fts(usuarios_fts) VALUES ('rebuild');
"#;

const MIGRACION_4: &str = r#"
CREATE TRIGGER contratistas_cedula_inmutable
BEFORE UPDATE OF cedula ON contratistas
WHEN NEW.cedula <> OLD.cedula
BEGIN
    SELECT RAISE(ABORT, 'La cedula del contratista es inmutable');
END;
"#;

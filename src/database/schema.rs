use chrono::NaiveDateTime;
use rusqlite::functions::FunctionFlags;
use rusqlite::{Connection, Transaction, TransactionBehavior, params};

use crate::texto::plegar_para_busqueda;
use crate::tiempo::{local_costa_rica_a_utc, parsear_utc, serializar_utc};

pub const SCHEMA_VERSION: i64 = 21;

/// Identifica un archivo `SQLite` como propio de Control Acceso (bytes de
/// "BRIS" como entero de 32 bits). `0` es el valor que trae por defecto
/// cualquier base nueva o creada antes de este cambio; sólo se rechaza un
/// `application_id` que sea de un tercero (ni `0` ni el nuestro).
pub const APPLICATION_ID: i32 = 0x4252_4953;

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("Error de SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// El archivo abierto es un `SQLite` válido pero no es una base de
    /// Control Acceso (`application_id` pertenece a otra aplicación).
    #[error("El archivo no es una base de datos de Control Acceso")]
    BaseAjena,
    /// `PRAGMA quick_check` encontró un problema estructural.
    #[error("La base de datos falló la verificación de integridad: {0}")]
    IntegridadInvalida(String),
    /// Hay una migración de esquema pendiente pero el respaldo obligatorio
    /// previo (`TipoRespaldo::PreMigracion`) falló — el arranque se detiene
    /// antes de tocar el esquema.
    #[error("No se pudo crear el respaldo obligatorio antes de migrar el esquema: {0}")]
    RespaldoPreMigracionFallido(String),
    /// Invariante interno: tras aplicar todas las migraciones conocidas, la
    /// versión resultante no es `SCHEMA_VERSION`. No es un error de `SQLite` —
    /// sólo puede pasar si la cadena de migraciones de este archivo tiene un
    /// hueco (una versión sin `if version == N` que la maneje).
    #[error(
        "Error interno: la base quedó en la versión de esquema {encontrada} tras migrar, se esperaba {SCHEMA_VERSION}"
    )]
    VersionInesperadaTrasMigrar { encontrada: i64 },
    /// `PRAGMA foreign_key_check` tras `MIGRACION_15` encontró filas con una
    /// clave foránea inválida — no debería poder pasar (las 7 tablas se
    /// recrean copiando exactamente los mismos datos), pero se verifica
    /// igual antes de reactivar `foreign_keys`, ya que la migración corre
    /// con la validación apagada (ver `aplicar_migracion_15`).
    #[error("La migración a tablas STRICT dejó filas con una clave foránea inválida")]
    MigracionStrictReferenciasInvalidas,
}

pub fn initialize_database(connection: &Connection) -> Result<(), SchemaError> {
    registrar_funcion_plegar(connection)?;

    // `foreign_keys`, `journal_mode` y `trusted_schema` no pueden cambiarse
    // dentro de una transacción activa, así que se fijan antes de abrir la
    // transacción de migración.
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        PRAGMA journal_mode = DELETE;
        PRAGMA synchronous = EXTRA;
        PRAGMA trusted_schema = OFF;
        PRAGMA secure_delete = FAST;
        ",
    )?;

    rechazar_archivo_ajeno(connection)?;
    verificar_integridad_rapida(connection)?;
    adoptar_application_id(connection)?;

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

    if version == 4 {
        aplicar_migracion(&transaction, MIGRACION_5, 5)?;
        version = 5;
    }

    if version == 5 {
        aplicar_migracion_6(&transaction)?;
        version = 6;
    }

    if version == 6 {
        aplicar_migracion(&transaction, MIGRACION_7, 7)?;
        version = 7;
    }

    if version == 7 {
        aplicar_migracion(&transaction, MIGRACION_8, 8)?;
        version = 8;
    }

    if version == 8 {
        aplicar_migracion(&transaction, MIGRACION_9, 9)?;
        version = 9;
    }

    if version == 9 {
        aplicar_migracion(&transaction, MIGRACION_10, 10)?;
        version = 10;
    }

    if version == 10 {
        aplicar_migracion(&transaction, MIGRACION_11, 11)?;
        version = 11;
    }

    if version == 11 {
        aplicar_migracion(&transaction, MIGRACION_12, 12)?;
        version = 12;
    }

    if version == 12 {
        aplicar_migracion(&transaction, MIGRACION_13, 13)?;
        version = 13;
    }

    if version == 13 {
        aplicar_migracion(&transaction, MIGRACION_14, 14)?;
        version = 14;
    }

    transaction.commit()?;

    // MIGRACION_15 recrea las 7 tablas normales con STRICT y no puede
    // compartir la transacción de arriba: necesita `foreign_keys = OFF`
    // (`DROP TABLE` sobre una tabla con hijos dispara `ON DELETE RESTRICT`
    // en cada uno, como si borrara todas las filas antes de eliminarla), y
    // ese pragma es un no-op dentro de una transacción activa — sólo surte
    // efecto entre transacciones. Ver `aplicar_migracion_15`.
    if version == 14 {
        aplicar_migracion_15(connection)?;
        version = 15;
    }

    aplicar_migraciones_posteriores_a_15(connection, &mut version)?;

    if version != SCHEMA_VERSION {
        return Err(SchemaError::VersionInesperadaTrasMigrar {
            encontrada: version,
        });
    }

    Ok(())
}

/// `MIGRACION_16` en adelante: ninguna comparte transacción con las de arriba
/// por prolijidad únicamente (a diferencia de `MIGRACION_15`, ninguna necesita
/// `foreign_keys = OFF`) — sólo agregan una columna nullable y la rellenan,
/// o crean una tabla nueva; ninguna recrea una tabla existente. Separado de
/// `initialize_database` sólo para no pasar el límite de líneas de esa
/// función.
fn aplicar_migraciones_posteriores_a_15(
    connection: &Connection,
    version: &mut i64,
) -> Result<(), SchemaError> {
    if *version == 15 {
        aplicar_migracion_16(connection)?;
        *version = 16;
    }

    if *version == 16 {
        aplicar_migracion_17(connection)?;
        *version = 17;
    }

    if *version == 17 {
        aplicar_migracion_18(connection)?;
        *version = 18;
    }

    if *version == 18 {
        aplicar_migracion_19(connection)?;
        *version = 19;
    }

    if *version == 19 {
        aplicar_migracion_20(connection)?;
        *version = 20;
    }

    if *version == 20 {
        aplicar_migracion_21(connection)?;
        *version = 21;
    }

    Ok(())
}

fn aplicar_migracion_16(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_16)?;
    transaction.execute_batch("PRAGMA user_version = 16")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_17(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_17)?;
    transaction.execute_batch("PRAGMA user_version = 17")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_18(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_18)?;
    transaction.execute_batch("PRAGMA user_version = 18")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_19(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_19)?;
    transaction.execute_batch("PRAGMA user_version = 19")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_20(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_20)?;
    transaction.execute_batch("PRAGMA user_version = 20")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_21(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_21)?;
    transaction.execute_batch("PRAGMA user_version = 21")?;
    transaction.commit()?;
    Ok(())
}

/// `foreign_keys = OFF` mientras corre `MIGRACION_15` (recrea 7 tablas con
/// `DROP`+`RENAME`, varias con hijos que usan `ON DELETE RESTRICT`) y se
/// reactiva al final, éxito o error — nunca debe quedar la conexión con la
/// validación apagada más allá de esta función. `PRAGMA foreign_key_check`
/// confirma, ya con los datos migrados, que ninguna fila quedó apuntando a
/// un id inexistente antes de dar la migración por buena.
fn aplicar_migracion_15(connection: &Connection) -> Result<(), SchemaError> {
    connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let resultado = ejecutar_migracion_15(connection);
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    resultado
}

fn ejecutar_migracion_15(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_15)?;
    transaction.execute_batch("PRAGMA user_version = 15")?;
    transaction.commit()?;
    if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(SchemaError::MigracionStrictReferenciasInvalidas);
    }
    Ok(())
}

/// Rechaza un archivo ajeno o corrupto antes de cualquier otra operación —
/// en particular antes del respaldo obligatorio pre-migración
/// (`connection::respaldar_antes_de_migrar`), para no terminar copiando a
/// `backups/` un archivo que ni siquiera es nuestro.
pub(crate) fn verificar_archivo_propio(connection: &Connection) -> Result<(), SchemaError> {
    rechazar_archivo_ajeno(connection)?;
    verificar_integridad_rapida(connection)
}

/// Rechaza un archivo `SQLite` ajeno (`application_id` de otra app). No
/// escribe nada — sólo lee, a propósito: adoptar el sello (`PRAGMA
/// application_id = ...`) es responsabilidad de [`adoptar_application_id`],
/// que debe correr después de `quick_check`, no antes.
fn rechazar_archivo_ajeno(connection: &Connection) -> Result<(), SchemaError> {
    let id: i32 = connection.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    if id != 0 && id != APPLICATION_ID {
        return Err(SchemaError::BaseAjena);
    }
    Ok(())
}

/// Adopta `APPLICATION_ID` en una base nueva o en una creada antes de que
/// existiera esta comprobación (`id == 0`). Sólo se llama tras confirmar,
/// vía `quick_check`, que el archivo no está dañado — de lo contrario un
/// archivo corrupto o ajeno con `application_id == 0` por casualidad
/// quedaría "adoptado" como propio antes de rechazarlo.
fn adoptar_application_id(connection: &Connection) -> Result<(), SchemaError> {
    let id: i32 = connection.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    if id == 0 {
        connection.execute_batch(&format!("PRAGMA application_id = {APPLICATION_ID};"))?;
    }
    Ok(())
}

/// Función SQL `PLEGAR(texto)`: minúsculas + diacríticos plegados
/// (`crate::texto::plegar_para_busqueda`), usada por las búsquedas cortas
/// (`< 3` caracteres) en vez de `LIKE ... COLLATE NOCASE`. `COLLATE NOCASE`
/// sólo pliega ASCII A-Z — no encuentra "Óscar" buscando "os" — y a
/// diferencia de una `COLLATE` personalizada (que `SQLite` no aplica al
/// operador `LIKE`, sólo a comparaciones de igualdad; verificado antes de
/// implementar esto), una función SQL sí participa en `LIKE` porque el
/// plegado ocurre antes de comparar, no durante. Se registra en cada
/// apertura (no sobrevive a un `ATTACH`/reconexión) — determinista y sin
/// acceso a memoria compartida entre threads, así que es segura para `SQLite`.
///
/// `NULL` de entrada produce `NULL` de salida (semántica SQL estándar, igual
/// que cualquier función de `SQLite`) en vez de fallar — necesario para
/// columnas opcionales como `usuario_salida_nombre`, donde antes
/// `LIKE ... COLLATE NOCASE` excluía la fila sin error al comparar contra
/// `NULL` y `PLEGAR(NULL)` sin este manejo rompía la consulta entera.
fn registrar_funcion_plegar(connection: &Connection) -> Result<(), SchemaError> {
    connection
        .create_scalar_function(
            "PLEGAR",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            |contexto| {
                let texto: Option<String> = contexto.get(0)?;
                Ok(texto.map(|texto| plegar_para_busqueda(&texto)))
            },
        )
        .map_err(SchemaError::from)
}

/// Chequeo estructural barato en cada apertura (`quick_check`, no
/// `integrity_check`/`foreign_key_check` completos — esos se reservan para
/// la validación de respaldos, donde el costo mayor es aceptable).
fn verificar_integridad_rapida(connection: &Connection) -> Result<(), SchemaError> {
    let resultado: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if resultado != "ok" {
        return Err(SchemaError::IntegridadInvalida(resultado));
    }
    Ok(())
}

fn aplicar_migracion(
    transaction: &Transaction<'_>,
    sql: &str,
    nueva_version: i64,
) -> rusqlite::Result<()> {
    transaction.execute_batch(sql)?;
    transaction.execute_batch(&format!("PRAGMA user_version = {nueva_version}"))
}

/// Carga toda `registro_ingresos` en memoria para normalizar sus fechas
/// (tabla de solo-inserción, crece indefinidamente). Aceptable aquí porque
/// ya corrió y quedó fijada — una migración, una vez publicada, no se
/// reescribe (cualquier base que ya esté en `user_version >= 6` nunca vuelve
/// a ejecutar esta función). **No repetir este patrón en una migración
/// nueva** sobre esta misma tabla u otra que pueda crecer sin límite: usar
/// lotes (`LIMIT`/`OFFSET` o un cursor) en vez de cargar todo de una vez.
fn aplicar_migracion_6(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    let movimientos = {
        let mut statement = transaction
            .prepare("SELECT id, fecha_hora_ingreso, fecha_hora_salida FROM registro_ingresos")?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
    };
    let movimientos = movimientos
        .into_iter()
        .map(|(id, ingreso, salida)| {
            Ok((
                id,
                normalizar_fecha_utc_legacy(&ingreso)?,
                salida
                    .map(|valor| normalizar_fecha_utc_legacy(&valor))
                    .transpose()?,
            ))
        })
        .collect::<rusqlite::Result<Vec<_>>>()?;

    transaction.execute_batch(MIGRACION_6_INICIO)?;
    for (id, ingreso, salida) in movimientos {
        transaction.execute(
            "UPDATE registro_ingresos
             SET fecha_hora_ingreso = ?1, fecha_hora_salida = ?2
             WHERE id = ?3",
            params![ingreso, salida, id],
        )?;
    }
    transaction.execute_batch(MIGRACION_6_FINAL)?;
    transaction.execute_batch("PRAGMA user_version = 6")
}

fn normalizar_fecha_utc_legacy(valor: &str) -> rusqlite::Result<String> {
    if let Ok(instante) = parsear_utc(valor) {
        return Ok(serializar_utc(instante));
    }
    let local = NaiveDateTime::parse_from_str(valor, "%Y-%m-%d %H:%M:%S")
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    local_costa_rica_a_utc(local)
        .map(serializar_utc)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

const MIGRACION_1: &str = r"
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
";

const MIGRACION_2: &str = r"
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
";

const MIGRACION_3: &str = r"
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
";

const MIGRACION_4: &str = r"
CREATE TRIGGER contratistas_cedula_inmutable
BEFORE UPDATE OF cedula ON contratistas
WHEN NEW.cedula <> OLD.cedula
BEGIN
    SELECT RAISE(ABORT, 'La cedula del contratista es inmutable');
END;
";

const MIGRACION_5: &str = r"
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
);

INSERT INTO registro_ingresos_nueva (
    id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso,
    tipo_ingreso, gafete_numero, usuario_ingreso_id, fecha_hora_salida,
    usuario_salida_id, contratista_cedula, contratista_nombre, empresa_nombre,
    usuario_ingreso_nombre, usuario_salida_nombre, fecha_vencimiento_praind,
    es_personal_ruta, tiene_acceso, resultado_acceso, motivo_resultado,
    reglas_version
)
SELECT
    r.id, r.contratista_id, r.empresa_id, r.fecha_hora_ingreso, r.medio_ingreso,
    r.tipo_ingreso, r.gafete_numero, r.usuario_ingreso_id, r.fecha_hora_salida,
    r.usuario_salida_id, c.cedula, c.nombre, e.nombre, ui.nombre, us.nombre,
    c.fecha_vencimiento_praind, c.es_personal_ruta, c.tiene_acceso,
    'MIGRADO', 'DATOS_RECONSTRUIDOS', 0
FROM registro_ingresos AS r
INNER JOIN contratistas AS c ON c.id = r.contratista_id
INNER JOIN empresas AS e ON e.id = r.empresa_id
INNER JOIN usuarios AS ui ON ui.id = r.usuario_ingreso_id
LEFT JOIN usuarios AS us ON us.id = r.usuario_salida_id;

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

CREATE VIRTUAL TABLE registro_ingresos_fts USING fts5(
    contratista_cedula, contratista_nombre, empresa_nombre,
    content='registro_ingresos', content_rowid='id',
    tokenize='trigram case_sensitive 0 remove_diacritics 1'
);
CREATE TRIGGER registro_ingresos_fts_ai AFTER INSERT ON registro_ingresos BEGIN
    INSERT INTO registro_ingresos_fts(
        rowid, contratista_cedula, contratista_nombre, empresa_nombre
    ) VALUES (
        new.id, new.contratista_cedula, new.contratista_nombre, new.empresa_nombre
    );
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
INSERT INTO registro_ingresos_fts(registro_ingresos_fts) VALUES ('rebuild');

CREATE TRIGGER registro_ingresos_entrada_inmutable
BEFORE UPDATE OF
    contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
    gafete_numero, usuario_ingreso_id, contratista_cedula, contratista_nombre,
    empresa_nombre, usuario_ingreso_nombre, fecha_vencimiento_praind,
    es_personal_ruta, tiene_acceso, resultado_acceso, motivo_resultado,
    reglas_version
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

CREATE TRIGGER registro_ingresos_no_eliminar
BEFORE DELETE ON registro_ingresos
BEGIN
    SELECT RAISE(ABORT, 'Los movimientos de acceso no se pueden eliminar');
END;
";

// Hasta la versión 5 las fechas se persistían sin zona y correspondían a la hora local
// de Costa Rica. Desde la versión 6 todos los instantes se guardan en UTC canónico.
const MIGRACION_6_INICIO: &str = r"
DROP TRIGGER registro_ingresos_entrada_inmutable;
DROP TRIGGER registro_ingresos_salida_unica;
";

const MIGRACION_6_FINAL: &str = r"
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

CREATE TRIGGER registro_ingresos_entrada_inmutable
BEFORE UPDATE OF
    contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
    gafete_numero, usuario_ingreso_id, contratista_cedula, contratista_nombre,
    empresa_nombre, usuario_ingreso_nombre, fecha_vencimiento_praind,
    es_personal_ruta, tiene_acceso, resultado_acceso, motivo_resultado,
    reglas_version
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
";

// Da de baja una empresa sin tocar el acceso individual de sus contratistas:
// `domain::acceso::verificar_acceso` deniega a todos los suyos mientras esté
// inactiva. Las empresas existentes quedan activas (DEFAULT 1) — nadie pierde
// acceso por el simple hecho de migrar.
const MIGRACION_7: &str = r"
ALTER TABLE empresas ADD COLUMN activo INTEGER NOT NULL DEFAULT 1 CHECK (activo IN (0, 1));
";

// Guarda si la empresa estaba activa al momento del ingreso, junto al resto
// de la fotografía histórica (`docs/auditoria-dominio-2026-08-20.md`,
// hallazgo #7). Antes `verificar_acceso` (Regla 0, desde la migración 7)
// influía en la decisión sin que ese factor quedara registrado explícitamente
// — quedaba implícito ("si se guardó el ingreso, la empresa estaba activa"),
// pero no como un dato propio, a diferencia de PRAIND/personal de
// ruta/tiene_acceso, que sí se snapshot-ean. `DEFAULT 1` es correcto para
// todas las filas existentes, no sólo "la mejor reconstrucción posible": las
// anteriores a la migración 7 son de una época en la que no existía el
// concepto de empresa inactiva (toda empresa era, por definición, activa), y
// las posteriores sólo pudieron persistirse si pasaron la Regla 0 primero.
const MIGRACION_8: &str = r"
ALTER TABLE registro_ingresos
ADD COLUMN empresa_activa_snapshot INTEGER NOT NULL DEFAULT 1
CHECK (empresa_activa_snapshot IN (0, 1));

DROP TRIGGER registro_ingresos_entrada_inmutable;
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
";

// Registro acotado a los dos campos de contratistas cuya trazabilidad es
// operativamente crítica. Los valores son texto nullable para representar
// correctamente una fecha PRAIND ausente, sin inventar sentinelas.
const MIGRACION_9: &str = r"
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
";

// Amplía la trazabilidad operativa para incluir la habilitación o
// deshabilitación de acceso. SQLite no permite modificar un CHECK existente,
// por eso se reconstruye la tabla conservando todas las filas anteriores.
const MIGRACION_10: &str = r"
CREATE TABLE auditoria_contratistas_nueva (
    id INTEGER PRIMARY KEY,
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER NOT NULL REFERENCES contratistas(id) ON DELETE RESTRICT,
    campo TEXT NOT NULL CHECK (
        campo IN ('tipo_ingreso', 'fecha_vencimiento_praind', 'tiene_acceso')
    ),
    valor_anterior TEXT,
    valor_nuevo TEXT,
    CHECK (valor_anterior IS NOT valor_nuevo)
);

INSERT INTO auditoria_contratistas_nueva(
    id, fecha_hora, usuario_id, contratista_id, campo, valor_anterior, valor_nuevo
)
SELECT id, fecha_hora, usuario_id, contratista_id, campo, valor_anterior, valor_nuevo
FROM auditoria_contratistas;

DROP TABLE auditoria_contratistas;
ALTER TABLE auditoria_contratistas_nueva RENAME TO auditoria_contratistas;

CREATE INDEX idx_auditoria_contratistas_fecha
ON auditoria_contratistas(fecha_hora DESC, id DESC);

CREATE INDEX idx_auditoria_contratistas_contratista
ON auditoria_contratistas(contratista_id, id DESC);
";

// Acelera la búsqueda de la salida más reciente sin indexar las filas todavía
// activas. La propia clave del índice cubre por completo `MAX(fecha_hora_salida)`.
const MIGRACION_11: &str = r"
CREATE INDEX idx_registro_ingresos_fecha_salida
ON registro_ingresos(fecha_hora_salida)
WHERE fecha_hora_salida IS NOT NULL;
";

// La identidad del contratista puede corregirse desde la aplicación con una
// sesión administrativa. Se retira la prohibición absoluta de SQLite y se
// incorpora la cédula a la auditoría para conservar quién hizo la corrección
// y sus valores anterior y nuevo.
const MIGRACION_12: &str = r"
DROP TRIGGER IF EXISTS contratistas_cedula_inmutable;

CREATE TABLE auditoria_contratistas_nueva (
    id INTEGER PRIMARY KEY,
    fecha_hora TEXT NOT NULL,
    usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    contratista_id INTEGER NOT NULL REFERENCES contratistas(id) ON DELETE RESTRICT,
    campo TEXT NOT NULL CHECK (
        campo IN ('cedula', 'tipo_ingreso', 'fecha_vencimiento_praind', 'tiene_acceso')
    ),
    valor_anterior TEXT,
    valor_nuevo TEXT,
    CHECK (valor_anterior IS NOT valor_nuevo)
);

INSERT INTO auditoria_contratistas_nueva(
    id, fecha_hora, usuario_id, contratista_id, campo, valor_anterior, valor_nuevo
)
SELECT id, fecha_hora, usuario_id, contratista_id, campo, valor_anterior, valor_nuevo
FROM auditoria_contratistas;

DROP TABLE auditoria_contratistas;
ALTER TABLE auditoria_contratistas_nueva RENAME TO auditoria_contratistas;

CREATE INDEX idx_auditoria_contratistas_fecha
ON auditoria_contratistas(fecha_hora DESC, id DESC);

CREATE INDEX idx_auditoria_contratistas_contratista
ON auditoria_contratistas(contratista_id, id DESC);
";

// Generaliza la auditoría de "sólo contratistas" a las tres entidades de
// catálogo (contratistas, empresas, usuarios) en una tabla única en vez de
// triplicar `auditoria_contratistas` por entidad — la GUI necesita mostrar
// "todos los cambios" en una sola grilla. `usuario_nombre`/`entidad_nombre`
// quedan como snapshot en la propia fila (a diferencia de antes, que
// resolvía `contratista_nombre` con un JOIN en vivo a `contratistas`): así
// un contratista/usuario renombrado o dado de baja no le hace perder
// sentido a una fila de auditoría vieja. Decisión explícita del usuario:
// los datos existentes en `auditoria_contratistas` son de prueba, no hace
// falta migrarlos — se descartan.
const MIGRACION_13: &str = r#"
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
"#;

// Catálogo real de gafetes físicos (`docs/plan-gafetes.md`). Hasta acá
// `registro_ingresos.gafete_numero` era un INTEGER libre sin relación con
// los gafetes físicos reales — cualquier número servía, sin forma de sacar
// de circulación uno perdido ni de saber quién lo debe. `gafetes` guarda
// sólo el estado vigente; `gafetes_incidentes` es el historial
// append-only (mismo patrón que `auditoria_cambios`), necesario para
// "quién debe qué" y para no perder el rastro de cuándo se marcó/resolvió
// un incidente. `registro_ingresos.gafete_numero` NO se vuelve FK a
// `gafetes.numero` a propósito: hay filas históricas con números que el
// catálogo (creado vacío) no tiene por qué conocer.
const MIGRACION_14: &str = r"
CREATE TABLE gafetes (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL UNIQUE,
    estado TEXT NOT NULL CHECK (estado IN ('DISPONIBLE', 'PERDIDO', 'DE_BAJA')),
    contratista_deudor_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    CHECK (
        (estado = 'PERDIDO' AND contratista_deudor_id IS NOT NULL)
        OR (estado <> 'PERDIDO' AND contratista_deudor_id IS NULL)
    )
);
CREATE INDEX idx_gafetes_estado ON gafetes(estado);
CREATE INDEX idx_gafetes_contratista_deudor
ON gafetes(contratista_deudor_id) WHERE contratista_deudor_id IS NOT NULL;

CREATE TABLE gafetes_incidentes (
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
);
CREATE INDEX idx_gafetes_incidentes_gafete ON gafetes_incidentes(gafete_id, id DESC);
CREATE INDEX idx_gafetes_incidentes_fecha ON gafetes_incidentes(fecha_hora DESC, id DESC);
";

// Tablas `STRICT` (`docs/pendientes.md`, "Evaluar tablas STRICT"): SQLite no
// permite `ALTER TABLE ... STRICT`, así que cada tabla se recrea con el
// patrón ya usado en `MIGRACION_5` (crear `_nueva`, copiar, `DROP`,
// `RENAME`) — en orden de dependencia de FK (padres antes que hijos) para
// que ninguna copia intente validar contra una tabla que todavía no existe.
// `DROP TABLE` también elimina los triggers definidos sobre esa tabla, así
// que cada uno se recrea después del `RENAME`, con el mismo texto que ya
// tenían. Los índices se recrean por el mismo motivo. Las tablas FTS5
// (`*_fts` y sus tablas sombra `*_fts_config/_data/_docsize/_idx`) no
// admiten `STRICT` y no se tocan — como los `id` se preservan exactos en el
// `INSERT ... SELECT`, sus índices externos (`content_rowid='id'`) siguen
// apuntando a filas válidas sin reconstruir nada.
//
// `INTEGER PRIMARY KEY` sigue siendo alias de `rowid` en una tabla
// `STRICT` (exento del chequeo de tipo estricto, es la única excepción
// documentada por SQLite) — no cambia el comportamiento de autoasignación
// que ya usan los repositorios.
const MIGRACION_15: &str = r"
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
";

// Identidad estable para la persistencia en la nube
// (`docs/plan-persistencia-nube.md`): el `id` local es autoincremental por
// dispositivo, así que el mismo número existe sin relación en cada sitio —
// no sirve para identificar una fila una vez que conviven datos de varios
// dispositivos en el receptor. `uuid` es esa segunda identidad, generada al
// azar, sin relación con el `id` local (que sigue existiendo igual, sin
// tocarse).
//
// Nullable a propósito: `ALTER TABLE ... ADD COLUMN` de `SQLite` rechaza un
// `DEFAULT` no constante ("Cannot add a column with non-constant default",
// verificado antes de escribir esto) y un trigger no puede modificar la fila
// que se está insertando — no hay forma de que el propio esquema rellene un
// UUID nuevo por sí solo sin recrear la tabla entera. Se resuelve distinto
// según el caso: las filas ya existentes se rellenan acá mismo, una vez, con
// el `UPDATE`; las filas nuevas lo reciben del código Rust que arma la
// bandeja de salida (todavía sin escribir) al crearlas — no de SQL.
const MIGRACION_16: &str = r"
ALTER TABLE contratistas ADD COLUMN uuid TEXT;
UPDATE contratistas SET uuid = (
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
    || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2)
    || '-' || lower(hex(randomblob(6)))
) WHERE uuid IS NULL;
CREATE UNIQUE INDEX idx_contratistas_uuid ON contratistas(uuid);

ALTER TABLE registro_ingresos ADD COLUMN uuid TEXT;
UPDATE registro_ingresos SET uuid = (
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
    || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2)
    || '-' || lower(hex(randomblob(6)))
) WHERE uuid IS NULL;
CREATE UNIQUE INDEX idx_registro_ingresos_uuid ON registro_ingresos(uuid);
";

// Bandeja de salida hacia el receptor en la nube
// (`docs/plan-persistencia-nube.md`): cada fila es "esto hay que mandarlo,
// todavía no se mandó". La llena el código Rust de los servicios (no un
// trigger de SQL -- ver el comentario de MIGRACION_16 sobre por qué SQL no
// puede generar el UUID de la fila nueva por sí solo), en la misma
// transacción que crea/actualiza la fila real, así que nunca puede quedar
// un cambio real sin su fila de cola correspondiente.
const MIGRACION_17: &str = r"
CREATE TABLE cola_salida (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (entidad IN ('contratista', 'ingreso')),
    entidad_uuid TEXT NOT NULL,
    operacion TEXT NOT NULL CHECK (operacion IN ('crear', 'actualizar', 'cerrar')),
    estado TEXT NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'enviado', 'fallido')),
    intentos INTEGER NOT NULL DEFAULT 0 CHECK (intentos >= 0),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL,
    ultimo_error TEXT
) STRICT;

-- Vaciar la cola recorre lo pendiente en orden de creación.
CREATE INDEX idx_cola_salida_pendientes
ON cola_salida(creado_en)
WHERE estado = 'pendiente';
";

// Espejo de sólo lectura de "lo que está abierto en mi sitio, creado por
// el otro dispositivo" (`docs/plan-persistencia-nube.md`). A propósito NO
// es una fila de `registro_ingresos`: esa tabla tiene triggers de
// inmutabilidad y llaves foráneas atadas a `usuarios`/`contratistas` *de
// este mismo dispositivo* -- un ingreso creado en la PC referencia un
// `usuario_ingreso_id` que, en el celular, puede no existir o ser otra
// persona. Forzarlo ahí rompería exactamente las garantías que protegen el
// historial real. Este espejo sólo cachea lo necesario para mostrarlo y
// cerrarlo (contra la nube directamente, ver `nube::sincronizacion`) --
// nunca es la fuente de verdad de nada.
const MIGRACION_18: &str = r"
CREATE TABLE ingresos_remotos (
    uuid TEXT PRIMARY KEY,
    sitio_id TEXT NOT NULL,
    contratista_nombre TEXT NOT NULL,
    hora_entrada TEXT NOT NULL,
    usuario_entrada_nombre TEXT,
    dispositivo_entrada_id TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
) STRICT;
";

// Empresas se suma al espejo de la nube (antes sólo viajaba su nombre como
// texto suelto dentro de cada contratista) -- mismo criterio de identidad
// que ya tienen contratistas/registro_ingresos: `uuid` nuevo, nullable,
// backfill para lo existente, las filas nuevas lo reciben del código Rust
// al crearlas (ver comentario de MIGRACION_16 sobre por qué SQL no puede
// generar el UUID por sí solo). `cola_salida` se recrea porque SQLite no
// permite modificar un CHECK existente -- mismo patrón que MIGRACION_10/12
// en su momento con `auditoria_contratistas`.
const MIGRACION_19: &str = r"
ALTER TABLE empresas ADD COLUMN uuid TEXT;
UPDATE empresas SET uuid = (
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
    || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2)
    || '-' || lower(hex(randomblob(6)))
) WHERE uuid IS NULL;
CREATE UNIQUE INDEX idx_empresas_uuid ON empresas(uuid);

CREATE TABLE cola_salida_nueva (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (entidad IN ('contratista', 'ingreso', 'empresa')),
    entidad_uuid TEXT NOT NULL,
    operacion TEXT NOT NULL CHECK (operacion IN ('crear', 'actualizar', 'cerrar')),
    estado TEXT NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'enviado', 'fallido')),
    intentos INTEGER NOT NULL DEFAULT 0 CHECK (intentos >= 0),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL,
    ultimo_error TEXT
) STRICT;
INSERT INTO cola_salida_nueva SELECT * FROM cola_salida;
DROP TABLE cola_salida;
ALTER TABLE cola_salida_nueva RENAME TO cola_salida;
CREATE INDEX idx_cola_salida_pendientes
ON cola_salida(creado_en)
WHERE estado = 'pendiente';
";

// Gafetes se suma al espejo de la nube -- mismo criterio que empresas en
// MIGRACION_19: sólo el estado actual del gafete (número, estado, a quién
// se lo debe), no el historial de incidentes (`gafetes_incidentes` sigue
// siendo puramente local). `cola_salida` se recrea de nuevo porque SQLite
// no permite modificar un CHECK existente.
const MIGRACION_20: &str = r"
ALTER TABLE gafetes ADD COLUMN uuid TEXT;
UPDATE gafetes SET uuid = (
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
    || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2)
    || '-' || lower(hex(randomblob(6)))
) WHERE uuid IS NULL;
CREATE UNIQUE INDEX idx_gafetes_uuid ON gafetes(uuid);

CREATE TABLE cola_salida_nueva (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (entidad IN ('contratista', 'ingreso', 'empresa', 'gafete')),
    entidad_uuid TEXT NOT NULL,
    operacion TEXT NOT NULL CHECK (operacion IN ('crear', 'actualizar', 'cerrar')),
    estado TEXT NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'enviado', 'fallido')),
    intentos INTEGER NOT NULL DEFAULT 0 CHECK (intentos >= 0),
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL,
    ultimo_error TEXT
) STRICT;
INSERT INTO cola_salida_nueva SELECT * FROM cola_salida;
DROP TABLE cola_salida;
ALTER TABLE cola_salida_nueva RENAME TO cola_salida;
CREATE INDEX idx_cola_salida_pendientes
ON cola_salida(creado_en)
WHERE estado = 'pendiente';
";

// Realtime permite que un dispositivo cierre en la nube un ingreso abierto
// por otro. Si ese ingreso nació en esta base local, la sincronización debe
// reflejar el cierre sin inventar un `usuario_salida_id` local. El nombre
// de salida queda como snapshot textual; las salidas registradas localmente
// siguen escribiendo ambos campos como antes.
const MIGRACION_21: &str = r"
DROP TRIGGER registro_ingresos_no_eliminar;
DROP TRIGGER registro_ingresos_entrada_inmutable;
DROP TRIGGER registro_ingresos_salida_unica;
DROP TRIGGER registro_ingresos_fecha_utc_insert;
DROP TRIGGER registro_ingresos_salida_utc;
DROP TRIGGER registro_ingresos_fts_ad;
DROP TRIGGER registro_ingresos_fts_ai;
DROP INDEX idx_registro_ingresos_contratista;
DROP INDEX idx_registro_ingresos_empresa;
DROP INDEX idx_registro_ingresos_fecha_ingreso;
DROP INDEX idx_registro_ingresos_fecha_salida;
DROP INDEX idx_registro_ingresos_gafete;
DROP INDEX idx_registro_ingresos_contratista_activo;
DROP INDEX idx_registro_ingresos_gafete_activo;
DROP INDEX idx_registro_ingresos_uuid;

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
    uuid TEXT,
    CHECK (
        (fecha_hora_salida IS NULL
            AND usuario_salida_id IS NULL
            AND usuario_salida_nombre IS NULL)
        OR
        (fecha_hora_salida IS NOT NULL
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
CREATE UNIQUE INDEX idx_registro_ingresos_uuid ON registro_ingresos(uuid);

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
    reglas_version, empresa_activa_snapshot, uuid
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
    OR NEW.uuid IS NOT OLD.uuid
BEGIN
    SELECT RAISE(ABORT, 'Los datos historicos del ingreso son inmutables');
END;
CREATE TRIGGER registro_ingresos_salida_unica
BEFORE UPDATE OF fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
ON registro_ingresos
WHEN
    OLD.fecha_hora_salida IS NOT NULL
    OR NEW.fecha_hora_salida IS NULL
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
";

use chrono::NaiveDateTime;
use rusqlite::functions::FunctionFlags;
use rusqlite::{Connection, Transaction, TransactionBehavior, params};

use crate::texto::plegar_para_busqueda;
use crate::tiempo::{local_costa_rica_a_utc, parsear_utc, serializar_utc};

pub const SCHEMA_VERSION: i64 = 56;

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

/// `PRAGMA journal_mode = WAL` sobre un archivo que TODAVÍA no está en WAL
/// exige convertirlo (reescribir el encabezado, crear el `-shm`) -- a
/// diferencia del resto de transacciones normales bajo WAL, esta
/// conversión puntual puede devolver "database is locked" de forma
/// inmediata si otra conexión intenta la misma conversión al mismo tiempo,
/// SIN pasar por el reintento automático de `busy_timeout` (reproducido en
/// vivo: `dos_conexiones_migran_una_base_vacia_sin_reaplicar_pasos`, dos
/// conexiones nuevas abriendo el mismo archivo recién creado a la vez).
/// Una vez que cualquiera de las dos ya lo dejó en WAL, la misma pragma en
/// la otra es un no-op instantáneo -- de ahí que reintentar unas pocas
/// veces con una espera corta alcance, sin necesitar coordinación real
/// entre conexiones.
fn fijar_pragmas_iniciales(connection: &Connection) -> Result<(), SchemaError> {
    const PRAGMAS: &str = "
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = EXTRA;
        PRAGMA trusted_schema = OFF;
        PRAGMA secure_delete = FAST;
        ";
    const REINTENTOS: u32 = 20;
    const ESPERA_ENTRE_REINTENTOS: std::time::Duration = std::time::Duration::from_millis(50);

    for intento in 1..=REINTENTOS {
        match connection.execute_batch(PRAGMAS) {
            Ok(()) => return Ok(()),
            Err(rusqlite::Error::SqliteFailure(codigo, _))
                if intento < REINTENTOS
                    && matches!(
                        codigo.code,
                        rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                    ) =>
            {
                std::thread::sleep(ESPERA_ENTRE_REINTENTOS);
            }
            Err(error) => return Err(error.into()),
        }
    }
    unreachable!("el bucle de arriba siempre retorna en el último intento (Ok o Err)")
}

pub fn initialize_database(connection: &Connection) -> Result<(), SchemaError> {
    registrar_funciones_propias(connection)?;

    // `foreign_keys`, `journal_mode` y `trusted_schema` no pueden cambiarse
    // dentro de una transacción activa, así que se fijan antes de abrir la
    // transacción de migración.
    // WAL en vez de DELETE (rollback journal clásico): con DELETE, cualquier
    // transacción de escritura toma un lock exclusivo del archivo completo y
    // bloquea toda lectura concurrente hasta que termina o vence
    // `busy_timeout` -- eso ya no es aceptable ahora que el escritorio abre
    // una segunda conexión a propósito (`GuiState::conexion_secundaria`)
    // para leer/sincronizar sin retener el candado de `AppCore`, y el
    // celular sincroniza en segundo plano cada 2 minutos mientras el guardia
    // sigue buscando. WAL persiste en el propio archivo (no es una pragma
    // por conexión): una vez que cualquier conexión lo activa acá, todas las
    // conexiones que abran después el mismo archivo -- incluida
    // `conexion_secundaria`, que nunca vuelve a llamar `initialize_database`
    // -- lo heredan solas. `synchronous = EXTRA` se deja igual a propósito:
    // cambiar journal y durabilidad en el mismo paso complica diagnosticar
    // cuál de los dos causó un problema si aparece uno.
    fijar_pragmas_iniciales(connection)?;

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

    if *version == 21 {
        aplicar_migracion_22(connection)?;
        *version = 22;
    }

    if *version == 22 {
        aplicar_migracion_23(connection)?;
        *version = 23;
    }

    if *version == 23 {
        aplicar_migracion_24(connection)?;
        *version = 24;
    }

    if *version == 24 {
        aplicar_migracion_25(connection)?;
        *version = 25;
    }

    if *version == 25 {
        aplicar_migracion_26(connection)?;
        *version = 26;
    }

    if *version == 26 {
        aplicar_migracion_27(connection)?;
        *version = 27;
    }

    if *version == 27 {
        aplicar_migracion_28(connection)?;
        *version = 28;
    }

    if *version == 28 {
        aplicar_migracion_29(connection)?;
        *version = 29;
    }

    aplicar_migraciones_posteriores_a_29(connection, version)
}

/// Continuación de `aplicar_migraciones_posteriores_a_15` -- mismo motivo
/// que esa (no pasar el límite de líneas de una sola función), separada acá
/// en vez de en `initialize_database` porque ya era esta la que crecía.
fn aplicar_migraciones_posteriores_a_29(
    connection: &Connection,
    version: &mut i64,
) -> Result<(), SchemaError> {
    if *version == 29 {
        aplicar_migracion_30(connection)?;
        *version = 30;
    }

    if *version == 30 {
        aplicar_migracion_31(connection)?;
        *version = 31;
    }

    if *version == 31 {
        aplicar_migracion_32(connection)?;
        *version = 32;
    }

    if *version == 32 {
        aplicar_migracion_33(connection)?;
        *version = 33;
    }

    if *version == 33 {
        aplicar_migracion_34(connection)?;
        *version = 34;
    }

    if *version == 34 {
        aplicar_migracion_35(connection)?;
        *version = 35;
    }

    if *version == 35 {
        aplicar_migracion_36(connection)?;
        *version = 36;
    }

    if *version == 36 {
        aplicar_migracion_37(connection)?;
        *version = 37;
    }

    if *version == 37 {
        aplicar_migracion_38(connection)?;
        *version = 38;
    }

    if *version == 38 {
        aplicar_migracion_39(connection)?;
        *version = 39;
    }

    if *version == 39 {
        aplicar_migracion_40(connection)?;
        *version = 40;
    }

    if *version == 40 {
        aplicar_migracion_41(connection)?;
        *version = 41;
    }

    if *version == 41 {
        aplicar_migracion_42(connection)?;
        *version = 42;
    }

    if *version == 42 {
        aplicar_migracion_43(connection)?;
        *version = 43;
    }

    if *version == 43 {
        aplicar_migracion_44(connection)?;
        *version = 44;
    }

    if *version == 44 {
        aplicar_migracion_45(connection)?;
        *version = 45;
    }

    if *version == 45 {
        aplicar_migracion_46(connection)?;
        *version = 46;
    }

    if *version == 46 {
        aplicar_migracion_47(connection)?;
        *version = 47;
    }

    if *version == 47 {
        aplicar_migracion_48(connection)?;
        *version = 48;
    }

    if *version == 48 {
        aplicar_migracion_49(connection)?;
        *version = 49;
    }

    if *version == 49 {
        aplicar_migracion_50(connection)?;
        *version = 50;
    }

    if *version == 50 {
        aplicar_migracion_51(connection)?;
        *version = 51;
    }

    if *version == 51 {
        aplicar_migracion_52(connection)?;
        *version = 52;
    }

    aplicar_migraciones_posteriores_a_52(connection, version)
}

/// Continuación de `aplicar_migraciones_posteriores_a_29`, por el mismo
/// motivo (no pasar el límite de líneas de una sola función).
fn aplicar_migraciones_posteriores_a_52(
    connection: &Connection,
    version: &mut i64,
) -> Result<(), SchemaError> {
    if *version == 52 {
        aplicar_migracion_53(connection)?;
        *version = 53;
    }

    if *version == 53 {
        aplicar_migracion_54(connection)?;
        *version = 54;
    }

    if *version == 54 {
        aplicar_migracion_55(connection)?;
        *version = 55;
    }

    if *version == 55 {
        aplicar_migracion_56(connection)?;
        *version = 56;
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

fn aplicar_migracion_22(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_22)?;
    transaction.execute_batch("PRAGMA user_version = 22")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_23(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_23)?;
    transaction.execute_batch("PRAGMA user_version = 23")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_24(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_24)?;
    transaction.execute_batch("PRAGMA user_version = 24")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_25(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_25)?;
    transaction.execute_batch("PRAGMA user_version = 25")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_26(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_26)?;
    transaction.execute_batch("PRAGMA user_version = 26")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_27(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_27)?;
    transaction.execute_batch("PRAGMA user_version = 27")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_28(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_28)?;
    transaction.execute_batch("PRAGMA user_version = 28")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_29(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_29)?;
    transaction.execute_batch("PRAGMA user_version = 29")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_30(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_30)?;
    transaction.execute_batch("PRAGMA user_version = 30")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_31(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_31)?;
    transaction.execute_batch("PRAGMA user_version = 31")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_32(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_32)?;
    transaction.execute_batch("PRAGMA user_version = 32")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_33(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_33)?;
    transaction.execute_batch("PRAGMA user_version = 33")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_34(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_34)?;
    transaction.execute_batch("PRAGMA user_version = 34")?;
    transaction.commit()?;
    Ok(())
}

/// Mismo criterio que `aplicar_migracion_15`: `gafetes` y
/// `gafetes_incidentes` se recrean juntas (la segunda es hija de la
/// primera vía `ON DELETE RESTRICT`) para poder cambiar la unicidad de
/// `gafetes` de `numero` a `(numero, tipo)` -- `SQLite` no permite alterar
/// un `UNIQUE`/`CHECK` existente.
fn aplicar_migracion_35(connection: &Connection) -> Result<(), SchemaError> {
    connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let resultado = ejecutar_migracion_35(connection);
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    resultado
}

fn ejecutar_migracion_35(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_35)?;
    transaction.execute_batch("PRAGMA user_version = 35")?;
    transaction.commit()?;
    if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(SchemaError::MigracionStrictReferenciasInvalidas);
    }
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

fn aplicar_migracion_36(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_36)?;
    transaction.execute_batch("PRAGMA user_version = 36")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_37(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_37)?;
    transaction.execute_batch("PRAGMA user_version = 37")?;
    transaction.commit()?;
    Ok(())
}

fn aplicar_migracion_38(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_38)?;
    transaction.execute_batch("PRAGMA user_version = 38")?;
    transaction.commit()?;
    Ok(())
}

/// Mismo criterio que `aplicar_migracion_35`: `gafetes` y `gafetes_incidentes`
/// se recrean juntas (la segunda es hija de la primera vía `ON DELETE
/// RESTRICT`) para poder sumar `PROVISIONAL_KOF` al `CHECK` de `tipo` y una
/// tercera columna de portador -- `SQLite` no permite alterar un `CHECK`
/// existente.
fn aplicar_migracion_39(connection: &Connection) -> Result<(), SchemaError> {
    connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let resultado = ejecutar_migracion_39(connection);
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    resultado
}

fn ejecutar_migracion_39(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_39)?;
    transaction.execute_batch("PRAGMA user_version = 39")?;
    transaction.commit()?;
    if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(SchemaError::MigracionStrictReferenciasInvalidas);
    }
    Ok(())
}

/// Suma `'prestamo_gafete_provisional'` al `CHECK` de `cola_salida.entidad`
/// -- mismo patrón que `MIGRACION_30`/`36`/`38` (`SQLite` no permite
/// `ALTER TABLE ... CHECK`, se recrea la tabla entera). Habilita el sync a
/// la nube del módulo de gafetes provisionales KOF -- ver
/// `docs/features-futuras/plan-gafetes-provisionales-kof.md`.
fn aplicar_migracion_40(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_40)?;
    transaction.execute_batch("PRAGMA user_version = 40")?;
    transaction.commit()?;
    Ok(())
}

/// Mismo criterio que `aplicar_migracion_39`: `gafetes`/`gafetes_incidentes`
/// se recrean para sumar `proveedor_portador_id` -- completa la cadena de
/// control de proveedores (`docs/features-futuras/plan-control-proveedores.md`).
fn aplicar_migracion_41(connection: &Connection) -> Result<(), SchemaError> {
    connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let resultado = ejecutar_migracion_41(connection);
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    resultado
}

fn ejecutar_migracion_41(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_41)?;
    transaction.execute_batch("PRAGMA user_version = 41")?;
    transaction.commit()?;
    if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(SchemaError::MigracionStrictReferenciasInvalidas);
    }
    Ok(())
}

/// Agrega `ingresos_proveedor_remotos` -- tabla nueva, sin recrear nada
/// existente, así que no hace falta el paréntesis `foreign_keys = OFF/ON`
/// que sí necesitan las migraciones que recrean una tabla con FKs activas.
fn aplicar_migracion_42(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_42)?;
    transaction.execute_batch("PRAGMA user_version = 42")?;
    transaction.commit()?;
    Ok(())
}

/// Agrega `historial_ingresos_proveedor_sitio` -- tabla nueva + una columna
/// en `sincronizacion_estado`, sin recrear nada existente.
fn aplicar_migracion_43(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_43)?;
    transaction.execute_batch("PRAGMA user_version = 43")?;
    transaction.commit()?;
    Ok(())
}

/// Mismo criterio que `aplicar_migracion_39`/`41`: `registro_ingresos_proveedor`
/// se recrea para relajar su `CHECK` de salida (ver el comentario de
/// `MIGRACION_44`).
fn aplicar_migracion_44(connection: &Connection) -> Result<(), SchemaError> {
    connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let resultado = ejecutar_migracion_44(connection);
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    resultado
}

fn ejecutar_migracion_44(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_44)?;
    transaction.execute_batch("PRAGMA user_version = 44")?;
    transaction.commit()?;
    if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(SchemaError::MigracionStrictReferenciasInvalidas);
    }
    Ok(())
}

/// Agrega `prestamos_gafete_provisional_remotos` -- tabla nueva, sin
/// recrear nada existente. Faltaba por completo desde que se creó el
/// módulo de gafetes provisionales KOF (`MIGRACION_39/40`): sin esta caché
/// no había forma de que un dispositivo se enterara de un préstamo que
/// OTRO entregó -- ver el comentario de `recibir_prestamos_gafete_provisional_abiertos`
/// en `nube::sincronizacion`. Bug reportado en pruebas reales, 2026-09-17.
fn aplicar_migracion_45(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_45)?;
    transaction.execute_batch("PRAGMA user_version = 45")?;
    transaction.commit()?;
    Ok(())
}

/// Mismo criterio que `aplicar_migracion_44`: `prestamos_gafete_provisional`
/// se recrea para relajar su `CHECK` de devolución -- exigía
/// `usuario_devolucion_id IS NOT NULL` junto con la fecha/nombre, cosa que
/// `registro_ingresos`/`registro_ingresos_proveedor` nunca exigieron (un
/// cierre remoto no siempre trae un id de usuario LOCAL válido, sólo el
/// nombre). Mismo bug, mismo fix, tabla distinta -- bug reportado en
/// pruebas reales, 2026-09-17.
fn aplicar_migracion_46(connection: &Connection) -> Result<(), SchemaError> {
    connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let resultado = ejecutar_migracion_46(connection);
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    resultado
}

fn ejecutar_migracion_46(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_46)?;
    transaction.execute_batch("PRAGMA user_version = 46")?;
    transaction.commit()?;
    if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(SchemaError::MigracionStrictReferenciasInvalidas);
    }
    Ok(())
}

/// `UNIQUE(nombre)` en `empresas`/`empresas_proveedor` sólo bloqueaba un
/// choque exacto -- "Dos Pinos" y "DOS PINOS" se colaban como dos empresas
/// distintas (hallazgo del usuario, 2026-09-18). Un índice único sobre
/// `PLEGAR(nombre)` (misma función que ya usan los buscadores, ver
/// `registrar_funciones_propias`) cierra el hueco sin tocar el valor guardado
/// -- el nombre se sigue mostrando tal como se escribió, sólo la
/// comparación de unicidad ignora mayúsculas y diacríticos. Sin recrear
/// tablas: un índice nuevo no necesita el patrón `_nueva`/copiar/`DROP` de
/// otras migraciones. Si alguna instalación ya tuviera dos nombres que sólo
/// difieren en mayúsculas/tildes, esta migración falla a propósito en vez
/// de elegir en silencio cuál de las dos filas "gana" -- un caso así
/// necesita revisión manual, no una regla automática.
fn aplicar_migracion_47(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_47)?;
    transaction.execute_batch("PRAGMA user_version = 47")?;
    transaction.commit()?;
    Ok(())
}

/// Columna nueva para cachear localmente el login de un usuario global
/// (Administrador/Operador, autenticado contra Supabase Auth) y poder
/// operar sin internet ante un corte -- ver
/// `docs/decisiones-tecnicas.md`, entrada 2026-09-18, y el doc-comment de
/// `Usuario::password_hash_confirmado_en`. `NULL` por defecto en todas las
/// filas existentes (ROOT y cuentas locales de antes de esa migración):
/// mismo significado que ya tenían, hash permanente sin vencimiento.
fn aplicar_migracion_48(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_48)?;
    transaction.execute_batch("PRAGMA user_version = 48")?;
    transaction.commit()?;
    Ok(())
}

/// Agrega `placa` a `registro_ingresos` -- la placa del vehículo cuando el
/// contratista entra en `MedioIngreso::Vehiculo` (pedido explícito del
/// usuario, 2026-09-21: hoy se elige "Vehículo" como medio pero la placa no
/// se captura en ningún lado). `NULL` cuando el medio es `CAMINANDO` -- un
/// `CHECK` de columna cruzada (`(medio_ingreso = 'VEHICULO') OR (placa IS
/// NULL)`) hace esa regla imposible de romper en la base, igual que ya hace
/// el `CHECK` de fecha/usuario de salida un poco más abajo. Mismo criterio
/// que `MIGRACION_44`/`46`: `SQLite` no permite `ALTER TABLE ... ADD COLUMN`
/// con un `CHECK` que referencia otra columna, así que la tabla se recrea
/// completa (recreate-and-swap) en vez de un simple `ADD COLUMN`. Se
/// desactivan las foreign keys durante el swap por el mismo motivo que en
/// esas dos migraciones: `registro_ingresos` referencia `contratistas`/
/// `empresas`/`usuarios`, y `DROP TABLE` + `RENAME` intermedios dispararían
/// `PRAGMA foreign_key_check` en un estado transitorio inválido si quedaran
/// activas.
fn aplicar_migracion_49(connection: &Connection) -> Result<(), SchemaError> {
    connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let resultado = ejecutar_migracion_49(connection);
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    resultado
}

fn ejecutar_migracion_49(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_49)?;
    transaction.execute_batch("PRAGMA user_version = 49")?;
    transaction.commit()?;
    if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(SchemaError::MigracionStrictReferenciasInvalidas);
    }
    Ok(())
}

/// Mismo criterio que `MIGRACION_25` (`historial_sitio`), pero para
/// préstamos de gafete provisional KOF -- falencia detectada por el
/// usuario 2026-09-21: la pantalla de escritorio no tenía ninguna vista de
/// historial, sólo "Activos". Un préstamo que OTRO dispositivo entregó Y
/// devolvió nunca queda guardado localmente (sólo pasa por la caché
/// `prestamos_gafete_provisional_remotos` mientras está abierto, ver
/// `MIGRACION_45`), así que el historial completo del sitio sólo existe en
/// Supabase -- esta tabla es el mismo espejo local que ya existe para
/// ingresos. `ADD COLUMN` simple (no recrea nada): a diferencia de
/// `MIGRACION_44/46/49`, ninguna columna nueva participa de un `CHECK`.
fn aplicar_migracion_50(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_50)?;
    transaction.execute_batch("PRAGMA user_version = 50")?;
    transaction.commit()?;
    Ok(())
}

/// Columna nueva para distinguir un hash CACHEADO (`password_hash_confirmado_en`
/// no nulo) que corresponde a una contraseña TEMPORAL todavía sin cambiar
/// (`debe_cambiar_password` en `true` cuando se cacheó, ver
/// `AppCore::cachear_password_local`) de uno que ya es la contraseña real
/// del usuario. Hallazgo de auditoría 2026-09-24 (MV-01/DF-03): antes de
/// esta columna, un login online con la contraseña temporal la dejaba
/// cacheada igual que cualquier otra, y el siguiente login sin conexión
/// devolvía `debe_cambiar_password = false` siempre -- el cambio
/// obligatorio se podía esquivar del todo quedándose sin internet. `0`
/// (falso) por defecto en todas las filas existentes: un hash ya cacheado
/// antes de esta migración no tiene forma de saber si era temporal, y
/// tratarlo como si ya no lo fuera es lo mismo que pasaba hasta ahora --
/// no empeora nada, sólo dejan de arrastrar el problema los cacheos
/// nuevos, que sí van a marcar esto correctamente desde el próximo login
/// online.
fn aplicar_migracion_51(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_51)?;
    transaction.execute_batch("PRAGMA user_version = 51")?;
    transaction.commit()?;
    Ok(())
}

/// Guarda el último desfase medido entre el reloj del equipo y la hora del
/// servidor (ver `tiempo::RelojCorregido`). Antes vivía sólo en memoria: al
/// abrir la app, hasta la primera respuesta de la nube (o durante todo el
/// turno si no había internet), el equipo volvía a sellar movimientos con
/// su propio reloj, desfasado. `NULL` = nunca se midió; se usa el reloj
/// del equipo tal cual, como hasta ahora.
fn aplicar_migracion_52(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_52)?;
    transaction.execute_batch("PRAGMA user_version = 52")?;
    transaction.commit()?;
    Ok(())
}

/// Guarda el nombre de la unidad y la etiqueta con que el panel registró el
/// equipo (llegan en cada token, ver `TokenDispositivo::sitio_nombre`), para
/// mostrarlos en el login y la barra de estado también sin conexión. `NULL`
/// = todavía no llegaron (servidor anterior o equipo sin vincular).
fn aplicar_migracion_53(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_53)?;
    transaction.execute_batch("PRAGMA user_version = 53")?;
    transaction.commit()?;
    Ok(())
}

/// Guarda el ancla del reloj confiable (hora del servidor + contador de
/// arranque, ver `tiempo::Ancla`), para que al reabrir la app en el mismo
/// arranque del equipo la hora siga sin depender del reloj de Windows o
/// Android. `NULL` = sin ancla todavía.
fn aplicar_migracion_54(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_54)?;
    transaction.execute_batch("PRAGMA user_version = 54")?;
    transaction.commit()?;
    Ok(())
}

/// Ingreso "por correo" (visita autorizada por correo, comodín previo al
/// módulo de Visitas): `registro_ingresos_correo` (mismo armazón que
/// `registro_ingresos_proveedor`, con `motivo` en vez de empresa y gafete de
/// visita), su caché del otro dispositivo `ingresos_correo_remotos`, el
/// historial del sitio que sólo llena el escritorio
/// (`historial_ingresos_correo_sitio`, con su marca de agua en
/// `sincronizacion_estado`), y
/// `'ingreso_correo'` en el `CHECK` de `cola_salida.entidad` (se recrea,
/// igual que en `MIGRACION_40`/`41`). Ninguna tabla nueva tiene hijos y
/// `cola_salida` no tiene claves foráneas, así que no hace falta apagar
/// `foreign_keys`.
fn aplicar_migracion_55(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_55)?;
    transaction.execute_batch("PRAGMA user_version = 55")?;
    transaction.commit()?;
    Ok(())
}

/// Lápidas de los registros del otro dispositivo que ESTE equipo cerró a mano
/// (`remotos_cerrados_aca`): `cerrar_*_remoto` borra la fila de la caché
/// `*_remotos` y anota su uuid en la misma transacción, y la recepción de
/// abiertos no la vuelve a insertar mientras la lápida exista. Sin esto, una
/// sincronización que leyó la nube un instante antes del cierre la volvía a
/// meter en la caché y la persona reaparecía "adentro" hasta la siguiente
/// pasada (revisión del 2026-10-04). Tabla nueva sin hijos ni padres.
fn aplicar_migracion_56(connection: &Connection) -> Result<(), SchemaError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    transaction.execute_batch(MIGRACION_56)?;
    transaction.execute_batch("PRAGMA user_version = 56")?;
    transaction.commit()?;
    Ok(())
}

/// Rechaza un archivo ajeno o corrupto antes de cualquier otra operación —
/// se corre antes de tocar el esquema, para no terminar migrando (o
/// mostrando como propio) un archivo que ni siquiera es nuestro.
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
///
/// `SQLITE_INNOCUOUS` (desde `MIGRACION_47`): con `PRAGMA trusted_schema =
/// OFF` (ver `fijar_pragmas_iniciales`), `SQLite` rechaza cualquier función
/// de aplicación dentro de un objeto del esquema persistido (índice sobre
/// expresión, `CHECK`, columna generada) salvo que esté marcada así --
/// "unsafe use of `PLEGAR()`" es el error exacto que tira sin esta bandera.
/// Es correcto marcarla: `PLEGAR` no lee ni escribe nada fuera de su
/// argumento, mismo motivo por el que ya es seguro registrarla sin
/// sincronización entre threads.
pub(crate) fn registrar_funciones_propias(connection: &Connection) -> Result<(), SchemaError> {
    let banderas = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    connection.create_scalar_function("PLEGAR", 1, banderas, |contexto| {
        let texto: Option<String> = contexto.get(0)?;
        Ok(texto.map(|texto| plegar_para_busqueda(&texto)))
    })?;
    // Forma única de una cédula (`domain::cedula::Cedula::normalizar`), para
    // comparar contra valores guardados antes de que existiera (con guiones,
    // con el cero del TSE). Lo que no se puede normalizar queda tal cual:
    // nunca coincide con una cédula válida, pero no rompe la consulta.
    connection.create_scalar_function("NORMALIZAR_CEDULA", 1, banderas, |contexto| {
        let texto: Option<String> = contexto.get(0)?;
        Ok(texto.map(|texto| {
            crate::domain::cedula::Cedula::normalizar(&texto)
                .map_or(texto, crate::domain::cedula::Cedula::into_string)
        }))
    })?;
    Ok(())
}

/// Chequeo estructural barato en cada apertura (`quick_check`, no
/// `integrity_check`/`foreign_key_check` completos — mucho más lentos,
/// no justificados en cada arranque).
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

const MIGRACION_1: &str = include_str!("migraciones/migracion_1.sql");

const MIGRACION_2: &str = include_str!("migraciones/migracion_2.sql");

const MIGRACION_3: &str = include_str!("migraciones/migracion_3.sql");

const MIGRACION_4: &str = include_str!("migraciones/migracion_4.sql");

const MIGRACION_5: &str = include_str!("migraciones/migracion_5.sql");

// Hasta la versión 5 las fechas se persistían sin zona y correspondían a la hora local
// de Costa Rica. Desde la versión 6 todos los instantes se guardan en UTC canónico.
const MIGRACION_6_INICIO: &str = include_str!("migraciones/migracion_6_inicio.sql");

const MIGRACION_6_FINAL: &str = include_str!("migraciones/migracion_6_final.sql");

// Da de baja una empresa sin tocar el acceso individual de sus contratistas:
// `domain::acceso::verificar_acceso` deniega a todos los suyos mientras esté
// inactiva. Las empresas existentes quedan activas (DEFAULT 1) — nadie pierde
// acceso por el simple hecho de migrar.
const MIGRACION_7: &str = include_str!("migraciones/migracion_7.sql");

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
const MIGRACION_8: &str = include_str!("migraciones/migracion_8.sql");

// Registro acotado a los dos campos de contratistas cuya trazabilidad es
// operativamente crítica. Los valores son texto nullable para representar
// correctamente una fecha PRAIND ausente, sin inventar sentinelas.
const MIGRACION_9: &str = include_str!("migraciones/migracion_9.sql");

// Amplía la trazabilidad operativa para incluir la habilitación o
// deshabilitación de acceso. SQLite no permite modificar un CHECK existente,
// por eso se reconstruye la tabla conservando todas las filas anteriores.
const MIGRACION_10: &str = include_str!("migraciones/migracion_10.sql");

// Acelera la búsqueda de la salida más reciente sin indexar las filas todavía
// activas. La propia clave del índice cubre por completo `MAX(fecha_hora_salida)`.
const MIGRACION_11: &str = include_str!("migraciones/migracion_11.sql");

// La identidad del contratista puede corregirse desde la aplicación con una
// sesión administrativa. Se retira la prohibición absoluta de SQLite y se
// incorpora la cédula a la auditoría para conservar quién hizo la corrección
// y sus valores anterior y nuevo.
const MIGRACION_12: &str = include_str!("migraciones/migracion_12.sql");

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
const MIGRACION_13: &str = include_str!("migraciones/migracion_13.sql");

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
const MIGRACION_14: &str = include_str!("migraciones/migracion_14.sql");

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
const MIGRACION_15: &str = include_str!("migraciones/migracion_15.sql");

// Identidad estable para la persistencia en la nube
// (`docs/planes-implementados/plan-persistencia-nube.md`): el `id` local es autoincremental por
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
const MIGRACION_16: &str = include_str!("migraciones/migracion_16.sql");

// Bandeja de salida hacia el receptor en la nube
// (`docs/planes-implementados/plan-persistencia-nube.md`): cada fila es "esto hay que mandarlo,
// todavía no se mandó". La llena el código Rust de los servicios (no un
// trigger de SQL -- ver el comentario de MIGRACION_16 sobre por qué SQL no
// puede generar el UUID de la fila nueva por sí solo), en la misma
// transacción que crea/actualiza la fila real, así que nunca puede quedar
// un cambio real sin su fila de cola correspondiente.
const MIGRACION_17: &str = include_str!("migraciones/migracion_17.sql");

// Espejo de sólo lectura de "lo que está abierto en mi sitio, creado por
// el otro dispositivo" (`docs/planes-implementados/plan-persistencia-nube.md`). A propósito NO
// es una fila de `registro_ingresos`: esa tabla tiene triggers de
// inmutabilidad y llaves foráneas atadas a `usuarios`/`contratistas` *de
// este mismo dispositivo* -- un ingreso creado en la PC referencia un
// `usuario_ingreso_id` que, en el celular, puede no existir o ser otra
// persona. Forzarlo ahí rompería exactamente las garantías que protegen el
// historial real. Este espejo sólo cachea lo necesario para mostrarlo y
// cerrarlo (contra la nube directamente, ver `nube::sincronizacion`) --
// nunca es la fuente de verdad de nada.
const MIGRACION_18: &str = include_str!("migraciones/migracion_18.sql");

// Empresas se suma al espejo de la nube (antes sólo viajaba su nombre como
// texto suelto dentro de cada contratista) -- mismo criterio de identidad
// que ya tienen contratistas/registro_ingresos: `uuid` nuevo, nullable,
// backfill para lo existente, las filas nuevas lo reciben del código Rust
// al crearlas (ver comentario de MIGRACION_16 sobre por qué SQL no puede
// generar el UUID por sí solo). `cola_salida` se recrea porque SQLite no
// permite modificar un CHECK existente -- mismo patrón que MIGRACION_10/12
// en su momento con `auditoria_contratistas`.
const MIGRACION_19: &str = include_str!("migraciones/migracion_19.sql");

// Gafetes se suma al espejo de la nube -- mismo criterio que empresas en
// MIGRACION_19: sólo el estado actual del gafete (número, estado, a quién
// se lo debe), no el historial de incidentes (`gafetes_incidentes` sigue
// siendo puramente local). `cola_salida` se recrea de nuevo porque SQLite
// no permite modificar un CHECK existente.
const MIGRACION_20: &str = include_str!("migraciones/migracion_20.sql");

// Realtime permite que un dispositivo cierre en la nube un ingreso abierto
// por otro. Si ese ingreso nació en esta base local, la sincronización debe
// reflejar el cierre sin inventar un `usuario_salida_id` local. El nombre
// de salida queda como snapshot textual; las salidas registradas localmente
// siguen escribiendo ambos campos como antes.
const MIGRACION_21: &str = include_str!("migraciones/migracion_21.sql");

// Usuarios/operadores globales (docs/planes-implementados/plan-panel-administrativo-web.md) --
// mismo patrón que MIGRACION_20 para gafetes: agrega `uuid` (identidad
// estable para el upsert contra Supabase, ver `enviar_usuario`/
// `recibir_usuarios`) y suma 'usuario' al CHECK de `cola_salida.entidad`.
const MIGRACION_22: &str = include_str!("migraciones/migracion_22.sql");

// Sync incremental del catálogo (contratistas/empresas/usuarios): antes
// `recibir_catalogo_del_sitio` traía las tres tablas COMPLETAS en cada
// ciclo (cada 2 minutos, para siempre), sin importar si algo cambió.
// Esta fila única guarda desde cuándo pedir "sólo lo que cambió"
// (`updated_at > catalogo_actualizado_hasta`) -- si es NULL, todavía no
// hubo un primer sync y se trae todo (caso de sembrado inicial).
const MIGRACION_23: &str = include_str!("migraciones/migracion_23.sql");

// `ingresos_remotos` sólo traía lo mínimo para identificar/cerrar un
// ingreso abierto en otro dispositivo -- al fusionarse con la grilla de
// Ingreso Activo (`FilaRemota`, `desktop/src/pantallas/Activos.tsx`) esas
// filas se ven junto a las locales, que sí muestran empresa/tipo/medio/
// gafete/cédula. La tabla `ingresos` de Supabase ya tiene esas columnas
// (`recibir_ingresos_abiertos` sólo no las pedía) -- nada del otro lado
// cambia, sólo se agregan columnas nullable acá para cachearlas también.
const MIGRACION_24: &str = include_str!("migraciones/migracion_24.sql");

// Historial multi-dispositivo: decisión explícita del usuario -- "es la
// misma operación vista desde dos dispositivos distintos", no una versión
// resumida. `historial_sitio` espeja TODO movimiento del sitio (abierto o
// cerrado, de cualquier dispositivo, incluido este) con los mismos campos
// que ya muestra Historial local (`MovimientoIngresoResumen`) -- la nube
// (`ingresos`, ver migración `agrega_auditoria_completa_a_ingresos` del
// lado de Supabase) ahora también carga `resultado_acceso`/
// `motivo_resultado`/`reglas_version`/`empresa_activa_snapshot`, así que
// no hay hueco de datos entre un movimiento local y uno espejado. Nunca
// reemplaza `registro_ingresos` (misma razón que `ingresos_remotos`: sin
// FKs reales a `contratistas`/`usuarios` de este dispositivo, todo texto
// suelto) -- la pantalla de Historial combina ambas fuentes.
//
// `historial_actualizado_hasta` es la marca de agua del sync incremental
// (mismo mecanismo que `catalogo_actualizado_hasta`, columna separada
// porque son dos ritmos de sync independientes) -- sin esto, cada ciclo
// traería el historial completo del sitio para siempre.
const MIGRACION_25: &str = include_str!("migraciones/migracion_25.sql");

// Pedido del usuario tras no poder diferenciar de un vistazo si un
// movimiento de `historial_sitio` vino de la PC o del celular del mismo
// sitio -- `dispositivo_entrada_id` ya viajaba, pero es un UUID sin
// significado visible. Queda `NULL` para filas ya sincronizadas antes de
// esta migración (el sync incremental no las vuelve a tocar a menos que
// cambien) -- la pantalla debe mostrar algo neutro ("—"/ícono genérico)
// para ese caso, no asumir un valor.
const MIGRACION_26: &str = include_str!("migraciones/migracion_26.sql");

// `nube::sincronizacion::pendientes()` decidía si una fila ya podía
// reintentarse con `datetime(actualizado_en, '+' || MIN(intentos * 15,
// 1440) || ' minutes') <= datetime('now')`, calculado en cada consulta --
// SQLite no puede usar ningún índice para eso (es una expresión, no una
// columna), así que cada `drenar_cola` escaneaba TODA `cola_salida`
// pendiente para evaluarla fila por fila. Hallazgo R-06 de
// `docs/auditorias/auditoria-rendimiento-core-rust-2026-09-10.md`.
//
// `proximo_intento_en` es la misma fórmula, pero como columna generada
// (`GENERATED ALWAYS AS (...) STORED`): SQLite la recalcula sola cada vez
// que `actualizado_en`/`intentos` cambian, así que ni `cola_salida.rs`
// (al encolar) ni `marcar()` (al reintentar) necesitan tocarla a mano --
// no hay forma de que se desincronice de la fórmula real. Al ser una
// columna de verdad (no una expresión ad-hoc en el `WHERE`), sí admite un
// índice.
//
// `cola_salida` se recrea una vez más (ya pasó con MIGRACION_19/20/22)
// porque SQLite no permite agregar una columna `STORED` a una tabla que ya
// tiene filas vía `ALTER TABLE ADD COLUMN` -- sólo `VIRTUAL`, y acá
// conviene `STORED` para que el índice no tenga que recalcularla en cada
// consulta.
const MIGRACION_27: &str = include_str!("migraciones/migracion_27.sql");

// Watermark propio de gafetes, mismo mecanismo que
// `catalogo_actualizado_hasta`/`historial_actualizado_hasta` (columna
// separada, no reutiliza la de catálogo) -- antes `recibir_catalogo_del_sitio`
// bajaba TODOS los gafetes del sitio en cada sync, para siempre, a
// propósito (ver el doc-comment viejo de `descargar_catalogo_remoto`: el
// cursor compartido de catálogo podía ser anterior a que gafetes se sumara
// al pull, y bajar todo de nuevo reintentaba solo deudores que no habían
// podido resolverse localmente). Una columna propia resuelve el primer
// motivo sin ayuda (nace en `NULL`, primer sync siempre completo);
// `descargar_catalogo_remoto` resuelve el segundo sin volver a bajar todo:
// la marca sólo avanza hasta el `updated_at` más nuevo entre los gafetes
// que sí se pudieron guardar, nunca más allá de uno que se salteó por
// deudor no resuelto -- ese sigue pidiéndose en cada sync hasta que
// resuelva, igual que antes, pero sin arrastrar el resto del catálogo de
// gafetes que ya no cambió.
const MIGRACION_28: &str = include_str!("migraciones/migracion_28.sql");

// Control de visitas (docs/planes-implementados/plan-control-visitas.md) -- primer corte: solo
// esquema local, sin sincronización todavía. Patrón header-detail: `citas`
// es la autorización con vigencia (puede agendarse para un grupo -- ver
// `cita_visitantes`), `movimientos_visita` es el cruce real en el punto de
// acceso (equivalente a `registro_ingresos`, pero sin ningún campo de PRAIND/SWAT
// que no le pertenece a una visita).
//
// A propósito son TRES tablas locales, no las cuatro del documento de
// plan: `cita_sitios` (el puente muchos-a-muchos que permite una cita
// "tour" con varios sitios) vive sólo en Supabase. Cada dispositivo ya
// sabe a qué sitio pertenece (`contexto.sitio_id`, del token, no una fila
// local -- no existe ninguna tabla `sitios` local, mismo motivo que
// `registro_ingresos` nunca guarda su propio `sitio_id`: toda la base es
// de un solo sitio) y el pull desde la nube ya viene filtrado a "citas que
// incluyen mi sitio" -- replicar el puente acá no aportaría nada que este
// dispositivo pueda usar, sólo sitios ajenos que nunca le interesan.
//
// `estado` no incluye 'VENCIDA' como valor guardado -- si una cita ya
// pasó su `fecha_hasta` se calcula comparando fechas en el momento de la
// consulta, no se persiste (evita necesitar un proceso de fondo que vaya
// actualizando filas sólo para que caduquen solas). Sólo 'VIGENTE'/
// 'CANCELADA' son estados reales que alguien decide.
const MIGRACION_29: &str = include_str!("migraciones/migracion_29.sql");

// Suma 'movimiento_visita' al CHECK de `cola_salida.entidad` -- mismo
// patrón que MIGRACION_20/22 para gafetes/usuarios: SQLite no permite
// `ALTER TABLE ... CHECK`, así que la tabla se recrea entera. Sin esto,
// `MovimientoVisitaRepository` (siguiente corte) no puede encolar sus
// propios `crear`/`cerrar` -- el `INSERT` violaría el `CHECK` viejo.
//
// La tabla ya trae `proximo_intento_en` (columna generada, MIGRACION_27) --
// se conserva la misma definición acá para no perder el índice de
// reintentos al recrear la tabla una vez más; sólo cambia el `CHECK` de
// `entidad`.
const MIGRACION_30: &str = include_str!("migraciones/migracion_30.sql");

// Watermark propio de citas, mismo mecanismo que
// `catalogo_actualizado_hasta`/`historial_actualizado_hasta`/
// `gafetes_actualizado_hasta` (columna separada, ritmo de sync
// independiente) -- ver `nube::sincronizacion::recibir_citas_del_sitio`.
const MIGRACION_31: &str = include_str!("migraciones/migracion_31.sql");

// Snapshot al momento del check-in -- mismo criterio que
// `registro_ingresos` (que ya guarda `contratista_nombre`/`empresa_nombre`
// propios, no un JOIN en cada lectura): la trazabilidad para auditoría
// (docs/planes-implementados/plan-control-visitas.md) necesita mostrar quién era el visitante,
// de qué empresa y quién lo recibía TAL COMO ERAN al momento del cruce, sin
// depender de que `cita_visitantes`/`citas` todavía existan sin cambios
// más adelante. Nullable a propósito (igual que `dispositivo_entrada_tipo`
// en MIGRACION_26): filas creadas antes de esta migración quedan sin estos
// datos, no hay forma de reconstruirlos retroactivamente.
const MIGRACION_32: &str = include_str!("migraciones/migracion_32.sql");

// Caché de lectura del historial de visitas del sitio -- mismo rol que
// `historial_sitio` para contratistas (MIGRACION_25): trae TODO movimiento
// (abierto o cerrado) del sitio, de cualquier dispositivo, vía sync
// incremental con su propia marca de agua (`historial_visitas_actualizado_hasta`,
// ritmo independiente del resto). Sin esto, un movimiento de visita
// registrado en un dispositivo era invisible para cualquier otro (y para
// el admin que audita todos los sitios) -- vivía y moría sólo en el
// `movimientos_visita` local de quien hizo el check-in/check-out.
const MIGRACION_33: &str = include_str!("migraciones/migracion_33.sql");

// Gafetes de contratista y de visita son objetos físicos distintos que
// repiten la misma numeración (docs/planes-implementados/plan-control-visitas.md) --
// hasta acá el catálogo sólo modelaba el pool de contratistas
// ("contratista_deudor_id" a secas). Se agrega `tipo` (con `PROVEEDOR`
// aceptado a futuro, sin columna de portador propia todavía porque no
// existe tabla `proveedores`) y la unicidad pasa de `numero` a
// `(numero, tipo)`. El campo "deudor" se renombra a "portador" -- esta
// app no lleva control de dinero, es sólo trazabilidad de a quién se le
// asignó el objeto físico la última vez. `gafetes_incidentes` gana el
// mismo segundo portador porque es tabla hija y porque un mismo servicio
// compartido tiene que poder registrar la pérdida de cualquier fila del
// catálogo, sea del tipo que sea.
const MIGRACION_35: &str = include_str!("migraciones/migracion_35.sql");

// Hora aproximada de llegada -- puramente informativa a propósito
// (decisión explícita del usuario): "esta visita llega a las 10:00" no
// significa que a las 11:00 se le niegue el paso, `domain::cita::verificar_cita`
// no la toca para nada, sólo decide por `fecha_desde`/`fecha_hasta`. Texto
// libre tipo "HH:MM" (no una hora real de SQLite, que no tiene ese tipo)
// -- si el día de mañana el cliente pide bloquear por hora de verdad, eso
// es una regla de negocio nueva, no algo que este campo ya debería estar
// hoy validando.
const MIGRACION_34: &str = include_str!("migraciones/migracion_34.sql");

// Control de rutas (docs/planes-implementados/plan-control-rutas.md) -- primer
// corte del núcleo, sólo la ruta PRINCIPAL (pedido explícito del usuario,
// 2026-09-15: "primero dejemos las rutas principales montadas" -- H2/H3/H4
// y su modelado como `tipo_ruta` quedan para una migración aparte, una vez
// decidido si son el mismo tipo o no). Por eso `salidas_ruta` es UNA fila
// por salida (un solo documento), no header+detalle todavía -- el atajo "+
// Documento (H)" que ya existe en el mock de `PantallaRutas.kt` no tiene
// tabla propia hasta esa decisión.
//
// Espejo deliberado de `registro_ingresos` (pedido explícito del usuario:
// "toma como modelo mejor... ingreso, que es el que está más fino"), con
// una diferencia central: `vehiculo_id`/`encargado_id` son **opcionales**
// (pedido explícito 2026-09-15) -- a diferencia de `contratista_id`
// (obligatorio, bloquea el ingreso si no existe en el catálogo), acá el
// snapshot de texto (`vehiculo_placa`, `encargado_nombre`) es la fuente
// real y el link al catálogo es sólo un plus si hay coincidencia. Motivo:
// el OCR/entrada manual "nunca es obligatorio" (ya establecido en el plan)
// y la administración del catálogo todavía no tiene dueño confirmado
// (¿mobile, desktop, o ambos? -- pendiente).
//
// `vehiculos_ruta`/`encargados_ruta`: catálogos livianos, mismo molde que
// `empresas` (sin FTS -- son catálogos de código exacto, no de búsqueda de
// texto libre como el historial).
const MIGRACION_36: &str = include_str!("migraciones/migracion_36.sql");

// Watermark propio del catálogo de rutas (vehículos + encargados KOF),
// mismo mecanismo que `catalogo_actualizado_hasta`/`gafetes_actualizado_hasta`
// -- ver `nube::sincronizacion::recibir_catalogo_rutas_del_sitio`. Faltaba
// desde MIGRACION_36: esas dos tablas sólo tenían push (local -> nube),
// nunca el "pull" que las trae de vuelta -- un dispositivo que no las creó
// él mismo (ej. el catálogo KOF de 1438 filas, importado directo en
// Supabase) las veía siempre vacías.
const MIGRACION_37: &str = include_str!("migraciones/migracion_37.sql");

// Catálogo de números de ruta válidos (pedido explícito del usuario,
// 2026-09-15: "sin restricción podrías poner la ruta 222 y no existe,
// sino un número acotado de rutas") -- a diferencia de
// `vehiculos_ruta`/`encargados_ruta` (consultivos), este catálogo SÍ
// restringe: `salidas_ruta.numero_ruta` pasa de texto libre a un
// `ruta_id` obligatorio contra esta tabla, mismo criterio que
// `contratista_id` en `registro_ingresos`. Por sitio (como `gafetes`, sin
// columna `sitio_id` local -- toda la base ya es de un sitio), no global
// como vehículos/encargados: una ruta pertenece a un sitio a la vez
// (reasignable administrativamente, ver el plan).
//
// `salidas_ruta` se recrea entera (SQLite no permite cambiar el tipo de
// una columna ni sumarle una `FOREIGN KEY` con `ALTER TABLE`) -- sin
// migrar filas existentes: el texto libre viejo (ej. "CRR079") no tiene
// forma segura de mapearse a un `ruta_id` real sin intervención humana, y
// esta tabla nació en MIGRACION_36 de la misma sesión, todavía sin datos
// reales de producción (confirmado: 0 filas en Supabase al momento de
// escribir esta migración).
const MIGRACION_38: &str = include_str!("migraciones/migracion_38.sql");

// Gafetes provisionales KOF (docs/features-futuras/plan-gafetes-provisionales-kof.md)
// -- un colaborador interno de KOF que olvida su gafete permanente recibe
// uno de esta categoría física aparte mientras está en el sitio. A
// diferencia de PROVEEDOR (que sólo tiene el valor aceptado en el CHECK,
// sin columna de portador ni tabla propia todavía), acá sí se completa la
// cadena entera porque el catálogo de personas ya existe: `encargados_ruta`
// (importado para el módulo de rutas). El portador es un id de catálogo
// real, no transaccional -- la persona sí se repite.
//
// `prestamos_gafete_provisional` es un ciclo entrega/devolución, mismo
// espíritu que `salidas_ruta`/`movimientos_visita`: snapshot desnormalizado
// de nombre/código de empleado (lo que el guardia corrobora de palabra),
// trío nullable todo-o-nada para la devolución, triggers de inmutabilidad,
// e índice único parcial que impide dos préstamos abiertos a la vez para el
// mismo encargado. Sin ningún campo de "resultado" -- pedido explícito del
// usuario: no hay más verificación que la humana (cotejar la cédula física
// contra el nombre, algo que este sistema no captura ni valida). Sin
// integración a `cola_salida`/sync todavía -- fuera de alcance de este
// primer corte, sólo local.
const MIGRACION_39: &str = include_str!("migraciones/migracion_39.sql");

// Control de proveedores (docs/features-futuras/plan-control-proveedores.md)
// -- completa la cadena que el esquema sólo anticipaba a medias para
// PROVEEDOR (ver el comentario de MIGRACION_35/39): ahora sí existe la
// entidad, así que gafetes/gafetes_incidentes ganan su columna de portador
// real. A diferencia de PROVISIONAL_KOF (portador = catálogo real,
// encargados_ruta), acá el portador apunta a un registro TRANSACCIONAL
// (`registro_ingresos_proveedor`, la visita puntual) -- pedido explícito
// del usuario: no hay catálogo de personas, cada colaborador es distinto.
//
// `empresas_proveedor`: catálogo separado a propósito de `empresas` (el
// usuario fue explícito: "son empresas aparte de la de los colaboradores")
// -- mismo shape mínimo que `vehiculos_ruta`/`encargados_ruta` (sin FTS5,
// catálogo chico, `PLEGAR(nombre) LIKE ...` alcanza para el buscador).
//
// `registro_ingresos_proveedor`: mismo espíritu que `movimientos_visita` --
// snapshot puro (cédula/nombre vienen del OCR o tecleados, sin FK a ningún
// catálogo de personas), sin PRAIND/SWAT/tipo_ingreso (eso es exclusivo de
// contratistas). `gafete_numero` es NOT NULL sin condición (a diferencia de
// contratistas, acá siempre es obligatorio -- pedido explícito del
// usuario). `placa` nullable: la ausencia ya comunica "llegó a pie", no
// hace falta un `medio_ingreso` aparte. Dos índices únicos parciales (no
// uno) porque acá hay DOS invariantes independientes que proteger a la vez:
// ni la misma cédula ni el mismo número de gafete pueden tener dos ingresos
// abiertos simultáneos.
const MIGRACION_41: &str = include_str!("migraciones/migracion_41.sql");

const MIGRACION_40: &str = include_str!("migraciones/migracion_40.sql");

/// Caché local de ingresos de proveedor abiertos por OTRO dispositivo del
/// mismo sitio -- mismo criterio que `ingresos_remotos` (contratistas):
/// `nube::recibir_ingresos_proveedor_abiertos` reemplaza su contenido
/// entero en cada sync, `nube::cerrar_ingreso_proveedor_remoto` borra la
/// fila puntual al cerrarla. Sin FK a ningún catálogo local a propósito --
/// un ingreso remoto no vive en `registro_ingresos_proveedor` de este
/// dispositivo, es sólo lo mínimo para mostrarlo en "Proveedores" y poder
/// cerrarlo. Tabla nueva (no una recreación de otra existente), por eso no
/// hace falta el patrón `_nueva`/`DROP`/`RENAME` de otras migraciones.
const MIGRACION_42: &str = include_str!("migraciones/migracion_42.sql");

/// Caché de lectura del historial de ingresos de proveedor del sitio --
/// mismo rol que `historial_visitas_sitio` (`MIGRACION_33`): trae TODO
/// ingreso de proveedor (abierto o cerrado) del sitio, de cualquier
/// dispositivo, vía sync incremental con su propia marca de agua. Sólo se
/// llena en escritorio -- mismo criterio explícito que
/// `historial_visitas_sitio` (`mobile/rust-core/src/lib.rs`: "el celular es
/// para acciones rápidas, auditar historial le corresponde a la PC").
const MIGRACION_43: &str = include_str!("migraciones/migracion_43.sql");

// Relaja el CHECK de `registro_ingresos_proveedor` para permitir
// `usuario_salida_id IS NULL` con salida ya registrada -- mismo criterio que
// ya tiene `registro_ingresos` para contratistas desde MIGRACION_15 (ver esa
// migración: cuando la nube confirma una salida cerrada en OTRO dispositivo,
// `nube::recibir_cierres_de_ingresos_propios_proveedor` no tiene ningún
// `usuario_salida_id` LOCAL válido para esa persona -- sólo el nombre que
// viajó en el `UPDATE` remoto. El CHECK original de MIGRACION_41 exigía
// `usuario_salida_id IS NOT NULL` junto con la fecha/nombre, cosa que
// contratistas nunca tuvo -- bug encontrado en pruebas reales, 2026-09-17:
// un ingreso de proveedor cerrado por OTRO dispositivo nunca se reflejaba en
// el dispositivo dueño porque el `UPDATE` violaba este CHECK. `SQLite` no
// permite `ALTER TABLE ... CHECK`, se recrea la tabla entera (mismo patrón
// que MIGRACION_39/41: `PRAGMA foreign_keys OFF/ON` porque
// `registro_ingresos_proveedor` referencia `empresas_proveedor`/`usuarios`).
const MIGRACION_44: &str = include_str!("migraciones/migracion_44.sql");

/// Caché de préstamos de gafete provisional KOF abiertos por OTRO
/// dispositivo del sitio -- mismo rol que `ingresos_proveedor_remotos`
/// (`MIGRACION_41`). Ver el comentario de
/// `recibir_prestamos_gafete_provisional_abiertos` en `nube::sincronizacion`.
const MIGRACION_45: &str = include_str!("migraciones/migracion_45.sql");

// Relaja el CHECK de `prestamos_gafete_provisional` -- ver el comentario de
// `aplicar_migracion_46` arriba.
const MIGRACION_46: &str = include_str!("migraciones/migracion_46.sql");

// Cierra el hueco de "Dos Pinos" vs "DOS PINOS" -- ver el comentario de
// `aplicar_migracion_47` arriba.
const MIGRACION_47: &str = include_str!("migraciones/migracion_47.sql");

// Caché de login offline para usuarios globales -- ver el comentario de
// `aplicar_migracion_48` arriba.
const MIGRACION_48: &str = include_str!("migraciones/migracion_48.sql");

// Agrega `placa` a `registro_ingresos` -- ver el comentario de
// `aplicar_migracion_49` arriba. Recrea la tabla completa (columnas,
// índices, triggers) igual que `MIGRACION_21`, sólo sumando la columna
// nueva y su `CHECK` cruzado con `medio_ingreso`.
const MIGRACION_49: &str = include_str!("migraciones/migracion_49.sql");

const MIGRACION_50: &str = include_str!("migraciones/migracion_50.sql");

const MIGRACION_51: &str = include_str!("migraciones/migracion_51.sql");

const MIGRACION_52: &str = include_str!("migraciones/migracion_52.sql");

const MIGRACION_53: &str = include_str!("migraciones/migracion_53.sql");

const MIGRACION_54: &str = include_str!("migraciones/migracion_54.sql");

const MIGRACION_55: &str = include_str!("migraciones/migracion_55.sql");

const MIGRACION_56: &str = include_str!("migraciones/migracion_56.sql");

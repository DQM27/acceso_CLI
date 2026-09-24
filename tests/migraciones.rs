use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use rusqlite::Connection;

use control_acceso::database::schema::{SCHEMA_VERSION, initialize_database};

static SECUENCIA: AtomicU64 = AtomicU64::new(0);

fn base_temporal(nombre: &str) -> PathBuf {
    let numero = SECUENCIA.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "control_acceso_migracion_{nombre}_{}_{numero}.db",
        std::process::id()
    ))
}

fn limpiar_base(ruta: &Path) {
    let _ = fs::remove_file(ruta);
    let _ = fs::remove_file(ruta.with_extension("db-journal"));
    let _ = fs::remove_file(ruta.with_extension("db-wal"));
    let _ = fs::remove_file(ruta.with_extension("db-shm"));
}

fn version(connection: &Connection) -> i64 {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap()
}

fn crear_trigger_cedula_inmutable(connection: &Connection) {
    connection
        .execute_batch(
            "CREATE TRIGGER contratistas_cedula_inmutable
             BEFORE UPDATE OF cedula ON contratistas
             WHEN NEW.cedula <> OLD.cedula
             BEGIN
                SELECT RAISE(ABORT, 'La cedula del contratista es inmutable');
             END;",
        )
        .unwrap();
}

/// Deshace, para el fixture de pruebas, lo que `MIGRACION_16` (agrega
/// `uuid`) y `MIGRACION_49` (agrega `placa` + el `CHECK` cruzado contra
/// `medio_ingreso`) le hicieron a `registro_ingresos` -- de un solo saque,
/// en vez de dos pasos separados (que era el plan original: recrear sin
/// `placa` acá y soltar `uuid` aparte con un simple `ALTER TABLE ... DROP
/// COLUMN uuid`, como ya hacía `rebobinar_trigger_entrada_inmutable_sin_uuid`
/// para el trigger). Ese plan en dos pasos resultó frágil: `SQLite`
/// reconstruye TODOS los índices de la tabla al ejecutar `DROP COLUMN`
/// (no sólo los que mencionan la columna soltada) y, contra esta tabla en
/// particular, la reconstrucción chocaba con "index ... already exists" --
/// aparentemente una interacción entre el `DROP COLUMN` y el propio
/// recreate-and-swap que acababa de hacer esta función. Recrear la tabla
/// una sola vez, ya sin `uuid` ni `placa` ni sus objetos dependientes,
/// evita point por completo la maquinaria de `DROP COLUMN` de `SQLite`
/// para esta tabla. `SQLite` rechaza igual `DROP COLUMN placa` mientras el
/// `CHECK` siga mencionándola ("no such column: placa" durante la
/// validación posterior al drop) -- un `CHECK` no se puede quitar con
/// `ALTER TABLE`, así que de cualquier forma hacía falta recrear la tabla
/// entera (mismo patrón recreate-and-swap que la migración real).
fn rebobinar_registro_ingresos_sin_placa_ni_uuid(connection: &Connection) {
    connection
        .execute_batch(SQL_REBOBINAR_REGISTRO_INGRESOS_SIN_PLACA_NI_UUID)
        .unwrap();
}

// Separado en un `const` (en vez de un literal inline dentro de la función
// de arriba) puramente para que clippy no la cuente como una función de 176
// líneas (`too_many_lines`, en `deny` -- ver `[lints.clippy]` de
// `Cargo.toml`) -- es texto SQL, no lógica, mismo criterio que las
// constantes `MIGRACION_N` de `src/database/schema.rs`.
const SQL_REBOBINAR_REGISTRO_INGRESOS_SIN_PLACA_NI_UUID: &str = "
            CREATE TABLE registro_ingresos_v_sin_placa (
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

            INSERT INTO registro_ingresos_v_sin_placa (
                id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
                gafete_numero, usuario_ingreso_id, fecha_hora_salida, usuario_salida_id,
                contratista_cedula, contratista_nombre, empresa_nombre, usuario_ingreso_nombre,
                usuario_salida_nombre, fecha_vencimiento_praind, es_personal_ruta, tiene_acceso,
                resultado_acceso, motivo_resultado, reglas_version, empresa_activa_snapshot
            )
            SELECT
                id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
                gafete_numero, usuario_ingreso_id, fecha_hora_salida, usuario_salida_id,
                contratista_cedula, contratista_nombre, empresa_nombre, usuario_ingreso_nombre,
                usuario_salida_nombre, fecha_vencimiento_praind, es_personal_ruta, tiene_acceso,
                resultado_acceso, motivo_resultado, reglas_version, empresa_activa_snapshot
            FROM registro_ingresos;

            DROP TABLE registro_ingresos;
            ALTER TABLE registro_ingresos_v_sin_placa RENAME TO registro_ingresos;

            CREATE INDEX idx_registro_ingresos_contratista ON registro_ingresos(contratista_id);
            CREATE INDEX idx_registro_ingresos_empresa ON registro_ingresos(empresa_id);
            CREATE INDEX idx_registro_ingresos_fecha_ingreso ON registro_ingresos(fecha_hora_ingreso);
            -- Sin idx_registro_ingresos_fecha_salida a proposito: nace
            -- recien en MIGRACION_11, que todavia no corrio en el punto al
            -- que rebobinan los llamadores de este helper (v9/v10) -- si ya
            -- existiera aca, el CREATE INDEX de esa migracion chocaria con
            -- un error de indice duplicado al querer crearlo de nuevo.
            CREATE INDEX idx_registro_ingresos_gafete ON registro_ingresos(gafete_numero);
            CREATE UNIQUE INDEX idx_registro_ingresos_contratista_activo
            ON registro_ingresos(contratista_id) WHERE fecha_hora_salida IS NULL;
            CREATE UNIQUE INDEX idx_registro_ingresos_gafete_activo
            ON registro_ingresos(gafete_numero)
            WHERE gafete_numero IS NOT NULL AND fecha_hora_salida IS NULL;
            -- Sin `idx_registro_ingresos_uuid` a propósito: ese índice (y la
            -- columna `uuid` misma) nace en `MIGRACION_16`, todavía no
            -- corrió en el punto al que rebobinan los llamadores de este
            -- helper.

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

fn crear_esquema_version_1(connection: &Connection) {
    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE empresas (id INTEGER PRIMARY KEY, nombre TEXT NOT NULL UNIQUE);
            CREATE TABLE usuarios (
                id INTEGER PRIMARY KEY, cedula TEXT NOT NULL UNIQUE, nombre TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                rol TEXT NOT NULL CHECK (rol IN ('ROOT','ADMINISTRADOR','OPERADOR')),
                activo INTEGER NOT NULL CHECK (activo IN (0,1))
            );
            CREATE TABLE contratistas (
                id INTEGER PRIMARY KEY, cedula TEXT NOT NULL UNIQUE, nombre TEXT NOT NULL,
                empresa_id INTEGER NOT NULL,
                tipo_ingreso TEXT NOT NULL CHECK (tipo_ingreso IN ('PRAIND','IN_HOUSE','POR_CORREO','SWAT')),
                fecha_vencimiento_praind TEXT, es_personal_ruta INTEGER NOT NULL DEFAULT 0,
                tiene_acceso INTEGER NOT NULL,
                FOREIGN KEY (empresa_id) REFERENCES empresas(id)
            );
            CREATE TABLE registro_ingresos (
                id INTEGER PRIMARY KEY, contratista_id INTEGER NOT NULL, empresa_id INTEGER NOT NULL,
                fecha_hora_ingreso TEXT NOT NULL, medio_ingreso TEXT NOT NULL,
                tipo_ingreso TEXT NOT NULL, gafete_numero INTEGER,
                usuario_ingreso_id INTEGER NOT NULL, fecha_hora_salida TEXT, usuario_salida_id INTEGER,
                FOREIGN KEY (contratista_id) REFERENCES contratistas(id),
                FOREIGN KEY (empresa_id) REFERENCES empresas(id),
                FOREIGN KEY (usuario_ingreso_id) REFERENCES usuarios(id),
                FOREIGN KEY (usuario_salida_id) REFERENCES usuarios(id)
            );
            CREATE UNIQUE INDEX idx_registro_ingresos_contratista_activo
            ON registro_ingresos(contratista_id) WHERE fecha_hora_salida IS NULL;
            CREATE UNIQUE INDEX idx_registro_ingresos_gafete_activo
            ON registro_ingresos(gafete_numero)
            WHERE gafete_numero IS NOT NULL AND fecha_hora_salida IS NULL;
            CREATE INDEX idx_registro_ingresos_contratista ON registro_ingresos(contratista_id);
            CREATE INDEX idx_registro_ingresos_empresa ON registro_ingresos(empresa_id);
            CREATE INDEX idx_registro_ingresos_fecha_ingreso ON registro_ingresos(fecha_hora_ingreso);
            CREATE INDEX idx_registro_ingresos_gafete ON registro_ingresos(gafete_numero);
            PRAGMA user_version = 1;
            ",
        )
        .unwrap();
}

/// Usada tanto contra el esquema crudo de la versión 1 (`crear_esquema_version_1`,
/// sin `empresas.activo`, `contratistas.uuid` ni `usuarios.uuid`) como contra el
/// esquema actual ya migrado — de ahí que `empresas`, `usuarios` y `contratistas`
/// listen sus columnas explícitamente en vez de depender del orden posicional: así
/// las columnas que sólo existen en uno de los dos esquemas (`activo`, `uuid`,
/// todas con `DEFAULT`/nullable) no rompen el INSERT en ninguno de los dos casos.
fn insertar_referencias(connection: &Connection) {
    connection
        .execute_batch(
            "
            INSERT INTO empresas (id, nombre) VALUES (1, 'Empresa');
            INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
            INSERT INTO contratistas
                (id, cedula, nombre, empresa_id, tipo_ingreso, fecha_vencimiento_praind, es_personal_ruta, tiene_acceso)
                VALUES (1, '2001', 'Persona', 1, 'PRAIND', '2030-01-01', 0, 1);
            ",
        )
        .unwrap();
}

fn insertar_movimiento_actual(
    connection: &Connection,
    id: i64,
    ingreso: &str,
    salida: Option<&str>,
    usuario_salida_id: Option<i64>,
    usuario_salida_nombre: Option<&str>,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO registro_ingresos(
            id,contratista_id,empresa_id,fecha_hora_ingreso,medio_ingreso,
            tipo_ingreso,gafete_numero,usuario_ingreso_id,fecha_hora_salida,
            usuario_salida_id,contratista_cedula,contratista_nombre,
            empresa_nombre,usuario_ingreso_nombre,usuario_salida_nombre,
            fecha_vencimiento_praind,es_personal_ruta,tiene_acceso,
            resultado_acceso,motivo_resultado,reglas_version
         ) VALUES (?1,1,1,?2,'CAMINANDO','PRAIND',NULL,1,?3,?4,
            '2001','Persona','Empresa','Operador',?5,'2030-01-01',0,1,
            'PERMITIDO',NULL,1)",
        rusqlite::params![
            id,
            ingreso,
            salida,
            usuario_salida_id,
            usuario_salida_nombre
        ],
    )?;
    Ok(())
}

#[test]
fn base_vacia_llega_a_version_actual_y_es_idempotente() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
    initialize_database(&connection).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
}

#[test]
// Rebobina el esquema a v9 a mano, SQL por SQL -- crece por diseño con cada
// migración nueva que haya que deshacer (mismo criterio que
// `#[allow(clippy::too_many_lines)]` en `tests/contratista_queries.rs` y en
// `migracion_11_...` más abajo).
#[allow(clippy::too_many_lines)]
fn migracion_10_procesa_auditoria_vieja_sin_perder_el_resto_del_esquema() {
    // MIGRACION_13 (más reciente que ésta) termina descartando
    // `auditoria_contratistas` por completo (reemplazada por
    // `auditoria_cambios`, ver el comentario junto a `MIGRACION_13` en
    // `schema.rs`) — así que ya no tiene sentido afirmar que esta fila
    // "se conserva" hasta el final de la cadena. Lo que sigue valiendo la
    // pena probar es que una base real congelada en v9, con filas ya
    // escritas en la forma vieja de la tabla, atraviesa el resto de la
    // cadena de migraciones (10 → 11 → 12 → 13) sin errores de SQL.
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    crear_trigger_cedula_inmutable(&connection);
    connection
        .execute_batch(
            "DROP INDEX idx_registro_ingresos_fecha_salida;
             DROP TABLE auditoria_cambios;
             CREATE TABLE auditoria_contratistas (
                id INTEGER PRIMARY KEY,
                fecha_hora TEXT NOT NULL,
                usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
                contratista_id INTEGER NOT NULL REFERENCES contratistas(id) ON DELETE RESTRICT,
                campo TEXT NOT NULL CHECK (
                    campo IN ('tipo_ingreso', 'fecha_vencimiento_praind')
                ),
                valor_anterior TEXT,
                valor_nuevo TEXT,
                CHECK (valor_anterior IS NOT valor_nuevo)
             );
             INSERT INTO auditoria_contratistas(
                fecha_hora,usuario_id,contratista_id,campo,valor_anterior,valor_nuevo
             ) VALUES(
                '2026-08-20T22:00:00Z',1,1,'tipo_ingreso','SWAT','PRAIND'
             );
             -- MIGRACION_14 (que corre al final al rebobinar) recrea
             -- `gafetes`/`gafetes_incidentes` — el `initialize_database` de
             -- arriba ya las creó, hay que soltarlas antes de simular v9.
             DROP TABLE gafetes_incidentes;
             DROP TABLE gafetes;
             -- MIGRACION_39/41/42/43/45 (que corren al final al rebobinar)
             -- crean el módulo de gafetes provisionales KOF y el de
             -- proveedores desde cero -- mismo motivo que gafetes/
             -- gafetes_incidentes arriba. Orden de FK: el hijo primero
             -- (`registro_ingresos_proveedor` referencia `empresas_proveedor`).
             DROP TABLE prestamos_gafete_provisional;
             DROP TABLE prestamos_gafete_provisional_remotos;
             DROP TABLE registro_ingresos_proveedor;
             DROP TABLE empresas_proveedor;
             DROP TABLE ingresos_proveedor_remotos;
             DROP TABLE historial_ingresos_proveedor_sitio;
             -- Mismo motivo con MIGRACION_17/18, que crean `cola_salida` e
             -- `ingresos_remotos` desde cero -- ya existen por el
             -- `initialize_database` de arriba.
             DROP TABLE cola_salida;
             DROP TABLE ingresos_remotos;
             -- MIGRACION_36 (que corre al final al rebobinar) crea
             -- `salidas_ruta`/`vehiculos_ruta`/`encargados_ruta` desde cero,
             -- y MIGRACION_38 suma `rutas` -- mismo motivo que
             -- cola_salida/ingresos_remotos/gafetes arriba. Orden de FK: el
             -- hijo primero.
             DROP TABLE salidas_ruta;
             DROP TABLE rutas;
             DROP TABLE vehiculos_ruta;
             DROP TABLE encargados_ruta;
             -- MIGRACION_16/19 (que corren después de ésta al rebobinar) le
             -- agregan `uuid` a contratistas/registro_ingresos/empresas -- el
             -- `initialize_database` de arriba ya las dejó con esas columnas,
             -- hay que soltarlas antes de simular v9 para que el `SELECT *`
             -- de MIGRACION_15 (contra la forma que tenía la tabla en v9)
             -- tenga la misma cantidad de columnas de un lado y del otro.
             DROP INDEX idx_empresas_uuid;
             ALTER TABLE empresas DROP COLUMN uuid;
             DROP INDEX idx_contratistas_uuid;
             ALTER TABLE contratistas DROP COLUMN uuid;
             -- MIGRACION_22 (que corre al final al rebobinar) le agrega
             -- `uuid` a usuarios -- mismo motivo que las tres de arriba.
             DROP INDEX idx_usuarios_uuid;
             ALTER TABLE usuarios DROP COLUMN uuid;
             -- MIGRACION_48 (que corre al final al rebobinar) le agrega
             -- `password_hash_confirmado_en` a usuarios -- mismo motivo: sin
             -- soltarla acá, el `SELECT *` de MIGRACION_15 (más abajo en la
             -- cadena de re-aplicación) encuentra una columna de más contra
             -- la forma que `usuarios_nueva` esperaba en ese punto histórico.
             ALTER TABLE usuarios DROP COLUMN password_hash_confirmado_en;
             DROP INDEX idx_registro_ingresos_uuid;
             -- MIGRACION_23 (que corre al final al rebobinar) crea
             -- `sincronizacion_estado` desde cero -- mismo motivo que
             -- cola_salida/ingresos_remotos/gafetes arriba.
             DROP TABLE sincronizacion_estado;
             -- MIGRACION_25 (que corre al final al rebobinar) crea
             -- `historial_sitio` desde cero -- mismo motivo.
             DROP TABLE historial_sitio;
             -- MIGRACION_50 (que corre al final al rebobinar) crea
             -- `prestamos_gafete_provisional_historial_sitio` desde cero --
             -- mismo motivo.
             DROP TABLE prestamos_gafete_provisional_historial_sitio;
             -- MIGRACION_32 (que corre al final al rebobinar) crea
             -- `historial_visitas_sitio` desde cero -- mismo motivo.
             DROP TABLE historial_visitas_sitio;
             -- MIGRACION_28 (que corre al final al rebobinar) crea
             -- citas/cita_visitantes/movimientos_visita desde cero -- mismo
             -- motivo que historial_sitio arriba. Orden de FK: el hijo
             -- primero.
             DROP TABLE movimientos_visita;
             DROP TABLE cita_visitantes;
             DROP TABLE citas;",
        )
        .unwrap();
    rebobinar_registro_ingresos_sin_placa_ni_uuid(&connection);
    connection
        .execute_batch("PRAGMA user_version = 9;")
        .unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='auditoria_contratistas'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect_err("auditoria_contratistas debe haber desaparecido tras MIGRACION_13");
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM auditoria_cambios", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 0);
}

#[test]
// Rebobina el esquema a v10 a mano, SQL por SQL -- crece por diseño con
// cada migración nueva que haya que deshacer (mismo criterio que
// `#[allow(clippy::too_many_lines)]` en `tests/contratista_queries.rs`).
#[allow(clippy::too_many_lines)]
fn migracion_11_crea_indice_parcial_sin_perder_movimientos() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    crear_trigger_cedula_inmutable(&connection);
    insertar_movimiento_actual(
        &connection,
        1,
        "2026-08-21T10:00:00Z",
        Some("2026-08-21T11:00:00Z"),
        Some(1),
        Some("Operador"),
    )
    .unwrap();
    connection
        .execute_batch(
            "DROP INDEX idx_registro_ingresos_fecha_salida;
             -- MIGRACION_12 (que corre después de ésta al rebobinar a v10)
             -- necesita `auditoria_contratistas` en pie — el `initialize_database`
             -- de arriba ya la reemplazó por `auditoria_cambios` (MIGRACION_13),
             -- así que hay que recrearla en la forma que dejó MIGRACION_10 antes
             -- de simular que la base está congelada en v10.
             DROP TABLE auditoria_cambios;
             CREATE TABLE auditoria_contratistas (
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
             -- Mismo motivo que en `migracion_10_...`: soltar lo que
             -- MIGRACION_14 ya creó antes de simular v10.
             DROP TABLE gafetes_incidentes;
             DROP TABLE gafetes;
             -- Mismo motivo que en `migracion_10_...`: soltar lo que
             -- MIGRACION_39/41/42/43/45 ya crearon antes de simular v10.
             -- Orden de FK: el hijo primero (`registro_ingresos_proveedor`
             -- referencia `empresas_proveedor`).
             DROP TABLE prestamos_gafete_provisional;
             DROP TABLE prestamos_gafete_provisional_remotos;
             DROP TABLE registro_ingresos_proveedor;
             DROP TABLE empresas_proveedor;
             DROP TABLE ingresos_proveedor_remotos;
             DROP TABLE historial_ingresos_proveedor_sitio;
             -- Mismo motivo que en `migracion_10_...`: soltar lo que
             -- MIGRACION_17/18 ya crearon antes de simular v10.
             DROP TABLE cola_salida;
             DROP TABLE ingresos_remotos;
             -- Mismo motivo que en `migracion_10_...`: soltar lo que
             -- MIGRACION_36 ya creó antes de simular v10. Orden de FK: el
             -- hijo primero.
             DROP TABLE salidas_ruta;
             DROP TABLE rutas;
             DROP TABLE vehiculos_ruta;
             DROP TABLE encargados_ruta;
             -- Mismo motivo que en `migracion_10_...`: soltar `uuid` de
             -- contratistas/registro_ingresos/empresas antes de simular v10.
             DROP INDEX idx_empresas_uuid;
             ALTER TABLE empresas DROP COLUMN uuid;
             DROP INDEX idx_contratistas_uuid;
             ALTER TABLE contratistas DROP COLUMN uuid;
             -- MIGRACION_22 (que corre al final al rebobinar) le agrega
             -- `uuid` a usuarios -- mismo motivo que las tres de arriba.
             DROP INDEX idx_usuarios_uuid;
             ALTER TABLE usuarios DROP COLUMN uuid;
             -- MIGRACION_48 (que corre al final al rebobinar) le agrega
             -- `password_hash_confirmado_en` a usuarios -- mismo motivo: sin
             -- soltarla acá, el `SELECT *` de MIGRACION_15 (más abajo en la
             -- cadena de re-aplicación) encuentra una columna de más contra
             -- la forma que `usuarios_nueva` esperaba en ese punto histórico.
             ALTER TABLE usuarios DROP COLUMN password_hash_confirmado_en;
             DROP INDEX idx_registro_ingresos_uuid;
             -- MIGRACION_23 (que corre al final al rebobinar) crea
             -- `sincronizacion_estado` desde cero -- mismo motivo que
             -- cola_salida/ingresos_remotos/gafetes arriba.
             DROP TABLE sincronizacion_estado;
             -- MIGRACION_25 (que corre al final al rebobinar) crea
             -- `historial_sitio` desde cero -- mismo motivo.
             DROP TABLE historial_sitio;
             -- MIGRACION_50 (que corre al final al rebobinar) crea
             -- `prestamos_gafete_provisional_historial_sitio` desde cero --
             -- mismo motivo.
             DROP TABLE prestamos_gafete_provisional_historial_sitio;
             -- MIGRACION_32 (que corre al final al rebobinar) crea
             -- `historial_visitas_sitio` desde cero -- mismo motivo.
             DROP TABLE historial_visitas_sitio;
             -- MIGRACION_28 (que corre al final al rebobinar) crea
             -- citas/cita_visitantes/movimientos_visita desde cero -- mismo
             -- motivo que historial_sitio arriba. Orden de FK: el hijo
             -- primero.
             DROP TABLE movimientos_visita;
             DROP TABLE cita_visitantes;
             DROP TABLE citas;",
        )
        .unwrap();
    rebobinar_registro_ingresos_sin_placa_ni_uuid(&connection);
    connection
        .execute_batch("PRAGMA user_version = 10;")
        .unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    let definicion: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'index'
               AND name = 'idx_registro_ingresos_fecha_salida'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(definicion.contains("ON registro_ingresos(fecha_hora_salida)"));
    assert!(definicion.contains("WHERE fecha_hora_salida IS NOT NULL"));
    let movimiento: (String, String) = connection
        .query_row(
            "SELECT fecha_hora_ingreso, fecha_hora_salida
             FROM registro_ingresos WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        movimiento,
        (
            "2026-08-21T10:00:00Z".to_owned(),
            "2026-08-21T11:00:00Z".to_owned()
        )
    );
    initialize_database(&connection).unwrap();
}

#[test]
// Rebobina el esquema a v11 a mano, SQL por SQL -- mismo motivo que
// `migracion_10_...`/`migracion_11_...` arriba.
#[allow(clippy::too_many_lines)]
fn migracion_12_habilita_cambio_de_cedula() {
    // Igual comentario que en `migracion_10_...`: MIGRACION_13 termina
    // reemplazando `auditoria_contratistas` por `auditoria_cambios`, así que
    // ya no tiene sentido afirmar "se conserva la auditoría" al final de la
    // cadena — lo que sigue probando esta prueba es lo que le da nombre: que
    // el trigger que bloqueaba cambiar la cédula queda eliminado.
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    crear_trigger_cedula_inmutable(&connection);
    connection
        .execute_batch(
            "DROP TABLE auditoria_cambios;
             CREATE TABLE auditoria_contratistas (
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
             INSERT INTO auditoria_contratistas(
                fecha_hora,usuario_id,contratista_id,campo,valor_anterior,valor_nuevo
             ) VALUES(
                '2026-08-21T12:00:00Z',1,1,'tipo_ingreso','SWAT','PRAIND'
             );
             CREATE INDEX idx_auditoria_contratistas_fecha
             ON auditoria_contratistas(fecha_hora DESC, id DESC);
             CREATE INDEX idx_auditoria_contratistas_contratista
             ON auditoria_contratistas(contratista_id, id DESC);
             -- Mismo motivo que en `migracion_10_...`: soltar lo que
             -- MIGRACION_14 ya creó antes de simular v11.
             DROP TABLE gafetes_incidentes;
             DROP TABLE gafetes;
             -- Mismo motivo que en `migracion_10_...`: soltar lo que
             -- MIGRACION_39/41/42/43/45 ya crearon antes de simular v11.
             -- Orden de FK: el hijo primero (`registro_ingresos_proveedor`
             -- referencia `empresas_proveedor`).
             DROP TABLE prestamos_gafete_provisional;
             DROP TABLE prestamos_gafete_provisional_remotos;
             DROP TABLE registro_ingresos_proveedor;
             DROP TABLE empresas_proveedor;
             DROP TABLE ingresos_proveedor_remotos;
             DROP TABLE historial_ingresos_proveedor_sitio;
             -- Mismo motivo que en `migracion_10_...`: soltar lo que
             -- MIGRACION_17/18 ya crearon antes de simular v11.
             DROP TABLE cola_salida;
             DROP TABLE ingresos_remotos;
             -- Mismo motivo que en `migracion_10_...`: soltar lo que
             -- MIGRACION_36 ya creó antes de simular v11. Orden de FK: el
             -- hijo primero.
             DROP TABLE salidas_ruta;
             DROP TABLE rutas;
             DROP TABLE vehiculos_ruta;
             DROP TABLE encargados_ruta;
             -- Mismo motivo que en `migracion_10_...`: soltar `uuid` de
             -- contratistas/registro_ingresos/empresas antes de simular v11.
             DROP INDEX idx_empresas_uuid;
             ALTER TABLE empresas DROP COLUMN uuid;
             DROP INDEX idx_contratistas_uuid;
             ALTER TABLE contratistas DROP COLUMN uuid;
             -- MIGRACION_22 (que corre al final al rebobinar) le agrega
             -- `uuid` a usuarios -- mismo motivo que las tres de arriba.
             DROP INDEX idx_usuarios_uuid;
             ALTER TABLE usuarios DROP COLUMN uuid;
             -- MIGRACION_48 (que corre al final al rebobinar) le agrega
             -- `password_hash_confirmado_en` a usuarios -- mismo motivo: sin
             -- soltarla acá, el `SELECT *` de MIGRACION_15 (más abajo en la
             -- cadena de re-aplicación) encuentra una columna de más contra
             -- la forma que `usuarios_nueva` esperaba en ese punto histórico.
             ALTER TABLE usuarios DROP COLUMN password_hash_confirmado_en;
             DROP INDEX idx_registro_ingresos_uuid;
             -- MIGRACION_23 (que corre al final al rebobinar) crea
             -- `sincronizacion_estado` desde cero -- mismo motivo que
             -- cola_salida/ingresos_remotos/gafetes arriba.
             DROP TABLE sincronizacion_estado;
             -- MIGRACION_25 (que corre al final al rebobinar) crea
             -- `historial_sitio` desde cero -- mismo motivo.
             DROP TABLE historial_sitio;
             -- MIGRACION_50 (que corre al final al rebobinar) crea
             -- `prestamos_gafete_provisional_historial_sitio` desde cero --
             -- mismo motivo.
             DROP TABLE prestamos_gafete_provisional_historial_sitio;
             -- MIGRACION_32 (que corre al final al rebobinar) crea
             -- `historial_visitas_sitio` desde cero -- mismo motivo.
             DROP TABLE historial_visitas_sitio;
             -- MIGRACION_28 (que corre al final al rebobinar) crea
             -- citas/cita_visitantes/movimientos_visita desde cero -- mismo
             -- motivo que historial_sitio arriba. Orden de FK: el hijo
             -- primero.
             DROP TABLE movimientos_visita;
             DROP TABLE cita_visitantes;
             DROP TABLE citas;",
        )
        .unwrap();
    rebobinar_registro_ingresos_sin_placa_ni_uuid(&connection);
    connection
        .execute_batch("PRAGMA user_version = 11;")
        .unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    connection
        .execute("UPDATE contratistas SET cedula = 'OTRA' WHERE id = 1", [])
        .unwrap();
    let cedula: String = connection
        .query_row("SELECT cedula FROM contratistas WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(cedula, "OTRA");
}

#[test]
fn migracion_13_reemplaza_auditoria_contratistas_por_auditoria_cambios() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);

    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='auditoria_contratistas'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect_err("auditoria_contratistas no debe existir en una base nueva");

    connection
        .execute(
            "INSERT INTO auditoria_cambios(
                fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,entidad_nombre,
                campo,valor_anterior,valor_nuevo
             ) VALUES(
                '2026-08-28T12:00:00Z',1,'Operador','contratista',1,'Persona',
                'nombre','Persona','Persona Nueva'
             )",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO auditoria_cambios(
                fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,entidad_nombre,
                campo,valor_anterior,valor_nuevo
             ) VALUES(
                '2026-08-28T12:01:00Z',1,'Operador','empresa',1,'Empresa',
                'activo','1','0'
             )",
            [],
        )
        .unwrap();
    // Cambio de contraseña: sólo la fecha importa, sin valores.
    connection
        .execute(
            "INSERT INTO auditoria_cambios(
                fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,entidad_nombre,campo
             ) VALUES(
                '2026-08-28T12:02:00Z',1,'Operador','usuario',1,'Operador','password'
             )",
            [],
        )
        .unwrap();

    let error = connection
        .execute(
            "INSERT INTO auditoria_cambios(
                fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,entidad_nombre,campo
             ) VALUES(
                '2026-08-28T12:03:00Z',1,'Operador','otra_cosa',1,'Lo que sea','campo'
             )",
            [],
        )
        .unwrap_err();
    assert!(
        error.to_string().to_lowercase().contains("check"),
        "{error}"
    );

    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM auditoria_cambios", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 3);
}

#[test]
fn esquema_existente_con_version_cero_migra_sin_perder_datos() {
    let connection = Connection::open_in_memory().unwrap();
    crear_esquema_version_1(&connection);
    insertar_referencias(&connection);
    connection
        .execute(
            "INSERT INTO registro_ingresos VALUES (1,1,1,'2026-08-11 08:00:00','CAMINANDO','PRAIND',5,1,NULL,NULL)",
            [],
        )
        .unwrap();
    connection.execute_batch("PRAGMA user_version = 0").unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    let gafete: i64 = connection
        .query_row(
            "SELECT gafete_numero FROM registro_ingresos WHERE id=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(gafete, 5);
}

#[test]
fn migra_version_1_y_conserva_datos_validos_e_indices() {
    let connection = Connection::open_in_memory().unwrap();
    crear_esquema_version_1(&connection);
    insertar_referencias(&connection);
    connection.execute(
        "INSERT INTO registro_ingresos VALUES (1,1,1,'2026-08-11 08:00:00','CAMINANDO','PRAIND',5,1,NULL,NULL)",
        [],
    ).unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM registro_ingresos", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total, 1);
    let snapshot: (String, String, String, i64, String) = connection
        .query_row(
            "SELECT contratista_cedula, contratista_nombre,
                    resultado_acceso, reglas_version, fecha_hora_ingreso
             FROM registro_ingresos WHERE id=1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        snapshot,
        (
            "2001".into(),
            "Persona".into(),
            "MIGRADO".into(),
            0,
            "2026-08-11T14:00:00Z".into()
        )
    );
    for indice in [
        "idx_registro_ingresos_contratista_activo",
        "idx_registro_ingresos_gafete_activo",
    ] {
        let existe: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                [indice],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(existe, 1);
    }
}

/// Regresión de `MIGRACION_15` (tablas `STRICT`, `docs/pendientes.md`
/// "Evaluar tablas STRICT"): confirma que las 7 tablas recreadas de verdad
/// quedan `STRICT` (rechazan un tipo incorrecto, algo que la versión
/// anterior aceptaba en silencio por tipado dinámico) y que la migración en
/// sí no dejó ninguna clave foránea rota — `DROP TABLE` sobre una tabla con
/// hijos (`empresas`, `usuarios`, `contratistas`, `gafetes`) dispara
/// `ON DELETE RESTRICT` en cada uno si `foreign_keys` sigue activo durante
/// el recambio; `aplicar_migracion_15` lo apaga a propósito para esto.
#[test]
fn migracion_15_deja_tablas_strict_sin_romper_claves_foraneas() {
    let connection = Connection::open_in_memory().unwrap();
    crear_esquema_version_1(&connection);
    insertar_referencias(&connection);
    connection
        .execute(
            "INSERT INTO registro_ingresos VALUES (1,1,1,'2026-08-11 08:00:00','CAMINANDO','PRAIND',5,1,NULL,NULL)",
            [],
        )
        .unwrap();

    initialize_database(&connection).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);

    // `foreign_keys` debe seguir activo después de migrar — no es que
    // `aplicar_migracion_15` lo haya dejado apagado.
    let fk_activas: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk_activas, 1);

    // El propio `PRAGMA foreign_key_check` (mismo que usa `aplicar_migracion_15`
    // antes de dar la migración por buena) no debe encontrar nada roto.
    assert!(
        !connection
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap()
    );

    // STRICT en acción: `activo` es INTEGER — antes de MIGRACION_15, SQLite
    // (tipado dinámico) habría aceptado guardar texto ahí sin quejarse.
    let error = connection
        .execute(
            "UPDATE empresas SET activo = 'no-es-un-entero' WHERE id = 1",
            [],
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("cannot store TEXT value in INTEGER column"),
        "{error}"
    );

    // Los datos reales de todas las tablas con FK sobrevivieron el recambio.
    for (tabla, esperado) in [
        ("empresas", 1),
        ("usuarios", 1),
        ("contratistas", 1),
        ("registro_ingresos", 1),
    ] {
        let total: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {tabla}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, esperado, "tabla {tabla}");
    }
}

/// DDL de `gafetes`/`gafetes_incidentes` (forma anterior a `MIGRACION_35`) +
/// `cola_salida`/`sincronizacion_estado` (formas acumuladas hasta v34) --
/// separado de `base_version_34_con_gafete_perdido` sólo para mantenerla
/// bajo el tope de líneas de Clippy (`too_many_lines`); sin cambio de
/// contenido.
const DDL_GAFETES_Y_COLAS_V34: &str = "
CREATE TABLE gafetes (
    id INTEGER PRIMARY KEY,
    numero INTEGER NOT NULL UNIQUE,
    estado TEXT NOT NULL CHECK (estado IN ('DISPONIBLE', 'PERDIDO', 'DE_BAJA')),
    contratista_deudor_id INTEGER REFERENCES contratistas(id) ON DELETE RESTRICT,
    uuid TEXT,
    CHECK (
        (estado = 'PERDIDO' AND contratista_deudor_id IS NOT NULL)
        OR (estado <> 'PERDIDO' AND contratista_deudor_id IS NULL)
    )
) STRICT;
CREATE UNIQUE INDEX idx_gafetes_uuid ON gafetes(uuid);

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
) STRICT;

-- MIGRACION_36 (que corre al final, después de MIGRACION_35, al migrar
-- desde v34) recrea `cola_salida` (le suma 3 entidades nuevas al CHECK) --
-- esta base minimalista nunca la creó, a diferencia de una base real que
-- ya la tendría desde MIGRACION_17/18. Forma exacta de MIGRACION_30 (la
-- última que la tocó antes de v34).
CREATE TABLE cola_salida (
    id INTEGER PRIMARY KEY,
    entidad TEXT NOT NULL CHECK (
        entidad IN ('contratista', 'ingreso', 'empresa', 'gafete', 'usuario', 'movimiento_visita')
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
CREATE INDEX idx_cola_salida_pendientes
ON cola_salida(proximo_intento_en)
WHERE estado = 'pendiente';

-- MIGRACION_37 (después de MIGRACION_36, al migrar desde v34) agrega una
-- columna a `sincronizacion_estado` -- esta base minimalista tampoco la
-- tenía, mismo motivo que `cola_salida` arriba. Forma exacta acumulada
-- hasta MIGRACION_33 (la última que la tocó antes de v34).
CREATE TABLE sincronizacion_estado (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    catalogo_actualizado_hasta TEXT,
    historial_actualizado_hasta TEXT,
    gafetes_actualizado_hasta TEXT,
    citas_actualizado_hasta TEXT,
    historial_visitas_actualizado_hasta TEXT
) STRICT;
INSERT INTO sincronizacion_estado (id, catalogo_actualizado_hasta) VALUES (1, NULL);

PRAGMA user_version = 34;
";

/// Regresión de `MIGRACION_35` (`gafetes` gana `tipo` + portador de visita,
/// la unicidad pasa de `numero` a `(numero, tipo)`): una base congelada en
/// versión 34 con un gafete `PERDIDO` real (contratista deudor incluido)
/// migra sin perder ese dato, y la nueva unicidad por tipo queda vigente.
/// `contratistas`/`usuarios`/`empresas`/`cita_visitantes` se toman del DDL
/// real de una conexión ya migrada (en vez de reconstruir a mano el
/// historial de `ALTER TABLE` de esas tres tablas, que esta migración no
/// toca) -- sólo `gafetes`/`gafetes_incidentes` se rebobinan a su forma
/// anterior a esta migración.
/// Reconstruye una base congelada en versión 34 (`gafetes`/`gafetes_incidentes`
/// en su forma anterior a `MIGRACION_35`) con un gafete `PERDIDO` real ya
/// cargado. `contratistas`/`usuarios`/`empresas`/`cita_visitantes` se toman
/// del DDL real de una conexión ya migrada (en vez de reconstruir a mano el
/// historial de `ALTER TABLE` de esas tres tablas, que esta migración no
/// toca).
fn base_version_34_con_gafete_perdido() -> Connection {
    let referencia = Connection::open_in_memory().unwrap();
    initialize_database(&referencia).unwrap();
    let ddl_de = |tabla: &str| -> String {
        referencia
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name = ?1",
                [tabla],
                |row| row.get(0),
            )
            .unwrap()
    };

    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(&ddl_de("empresas")).unwrap();
    connection.execute_batch(&ddl_de("usuarios")).unwrap();
    // `ddl_de` trae la forma ACTUAL de `usuarios` (el `sql` de `sqlite_master`
    // ya refleja cualquier `ALTER TABLE ADD COLUMN` aplicado desde que se
    // creó) -- hasta MIGRACION_48 eso coincidía por casualidad con la forma
    // de v34 porque ninguna migración entre la 23 y la 47 tocaba `usuarios`.
    // MIGRACION_48 (`password_hash_confirmado_en`) rompe esa coincidencia:
    // sin soltarla acá, `initialize_database` más abajo choca con "duplicate
    // column name" al querer agregar una columna que ya está.
    connection
        .execute_batch("ALTER TABLE usuarios DROP COLUMN password_hash_confirmado_en;")
        .unwrap();
    connection.execute_batch(&ddl_de("contratistas")).unwrap();
    // `registro_ingresos` no lo toca ninguna migración entre la 21 y la 49
    // (la próxima que la recrea) -- tomar la forma ACTUAL de la referencia
    // es equivalente a reconstruir la de v34 a mano, mismo criterio que
    // `contratistas`/`citas`/`cita_visitantes` arriba. Sin esto,
    // `MIGRACION_49` (agrega `placa`, recrea la tabla) no encuentra
    // `registro_ingresos` en este fixture -- nunca se creó.
    connection
        .execute_batch(&ddl_de("registro_ingresos"))
        .unwrap();
    // Mismo motivo: `ingresos_remotos` (MIGRACION_17) e `historial_sitio`
    // (MIGRACION_25) ya existían en v34, pero ninguna migración entre esa y
    // la 48 las tocaba -- `MIGRACION_49` sí (les agrega `placa` con un simple
    // `ALTER TABLE ... ADD COLUMN`), así que este fixture necesita crearlas
    // para que la cadena de migraciones no encuentre "no such table" al
    // llegar ahí. A diferencia de `registro_ingresos` arriba, acá sí hace
    // falta soltar `placa` de nuevo después de copiar el DDL actual --
    // `ddl_de` ya la incluye (una columna agregada con `ALTER TABLE ADD
    // COLUMN` sí queda reflejada en `sqlite_master`, a diferencia de un
    // `DROP COLUMN`) y un simple `ALTER TABLE ... ADD COLUMN placa` (sin
    // `CHECK` cruzado, a diferencia de `registro_ingresos`) no tolera un
    // nombre duplicado -- mismo criterio que ya usa este fixture con
    // `usuarios.password_hash_confirmado_en` un poco más abajo.
    connection
        .execute_batch(&ddl_de("ingresos_remotos"))
        .unwrap();
    connection
        .execute_batch("ALTER TABLE ingresos_remotos DROP COLUMN placa;")
        .unwrap();
    connection
        .execute_batch(&ddl_de("historial_sitio"))
        .unwrap();
    connection
        .execute_batch("ALTER TABLE historial_sitio DROP COLUMN placa;")
        .unwrap();
    connection.execute_batch(&ddl_de("citas")).unwrap();
    connection
        .execute_batch(&ddl_de("cita_visitantes"))
        .unwrap();
    connection.execute_batch(DDL_GAFETES_Y_COLAS_V34).unwrap();
    insertar_referencias(&connection);
    connection
        .execute(
            "INSERT INTO gafetes (id, numero, estado, contratista_deudor_id, uuid)
             VALUES (1, 7, 'PERDIDO', 1, 'uuid-gafete-7')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO gafetes_incidentes (gafete_id, tipo, fecha_hora, usuario_id, contratista_id)
             VALUES (1, 'PERDIDO', '2026-08-01T00:00:00Z', 1, 1)",
            [],
        )
        .unwrap();
    connection
}

#[test]
fn migracion_35_agrega_tipo_y_portador_visita_preservando_datos_existentes() {
    let connection = base_version_34_con_gafete_perdido();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    assert!(
        !connection
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap()
    );

    let (tipo, estado, contratista_portador_id, visita_portador_id): (
        String,
        String,
        Option<i64>,
        Option<i64>,
    ) = connection
        .query_row(
            "SELECT tipo, estado, contratista_portador_id, visita_portador_id
             FROM gafetes WHERE numero = 7",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(tipo, "CONTRATISTA");
    assert_eq!(estado, "PERDIDO");
    assert_eq!(contratista_portador_id, Some(1));
    assert_eq!(visita_portador_id, None);

    let contratista_id_incidente: Option<i64> = connection
        .query_row(
            "SELECT contratista_id FROM gafetes_incidentes WHERE gafete_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(contratista_id_incidente, Some(1));

    // Mismo número, tipo distinto -- ya no colisiona (antes de esta
    // migración, `UNIQUE(numero)` lo habría rechazado).
    connection
        .execute(
            "INSERT INTO gafetes (numero, tipo, estado) VALUES (7, 'VISITA', 'DISPONIBLE')",
            [],
        )
        .unwrap();
    // Mismo número Y mismo tipo -- sigue colisionando.
    let error = connection
        .execute(
            "INSERT INTO gafetes (numero, tipo, estado) VALUES (7, 'CONTRATISTA', 'DISPONIBLE')",
            [],
        )
        .unwrap_err();
    assert!(error.to_string().to_lowercase().contains("unique"));

    // El CHECK tipo<->columna rechaza un gafete VISITA con portador de
    // contratista.
    assert!(
        connection
            .execute(
                "INSERT INTO gafetes (numero, tipo, estado, contratista_portador_id, visita_portador_id)
                 VALUES (8, 'VISITA', 'PERDIDO', 1, NULL)",
                [],
            )
            .is_err()
    );
}

#[test]
fn esquema_actual_solo_admite_fechas_utc_normalizadas() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);

    assert!(
        insertar_movimiento_actual(&connection, 1, "2026-08-11 08:00:00", None, None, None)
            .is_err()
    );
    insertar_movimiento_actual(&connection, 1, "2026-08-11T14:00:00Z", None, None, None).unwrap();
}

#[test]
fn check_de_salida_acepta_pares_coherentes_y_rechaza_incoherentes() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);

    insertar_movimiento_actual(&connection, 1, "2026-08-11T14:00:00Z", None, None, None).unwrap();
    insertar_movimiento_actual(
        &connection,
        2,
        "2026-08-10T14:00:00Z",
        Some("2026-08-10T23:00:00Z"),
        Some(1),
        Some("Operador"),
    )
    .unwrap();
    assert!(
        insertar_movimiento_actual(
            &connection,
            3,
            "2026-08-09T14:00:00Z",
            Some("2026-08-09T23:00:00Z"),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        insertar_movimiento_actual(
            &connection,
            4,
            "2026-08-09T14:00:00Z",
            None,
            Some(1),
            Some("Operador"),
        )
        .is_err()
    );
}

#[test]
fn migracion_fallida_revierte_tabla_y_version() {
    let connection = Connection::open_in_memory().unwrap();
    crear_esquema_version_1(&connection);
    insertar_referencias(&connection);
    connection.execute(
        "INSERT INTO registro_ingresos VALUES (1,1,1,'2026-08-11 08:00:00','CAMINANDO','PRAIND',NULL,1,'2026-08-11 17:00:00',NULL)", []
    ).unwrap();

    assert!(initialize_database(&connection).is_err());
    assert_eq!(version(&connection), 1);
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM registro_ingresos", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total, 1);
    let nueva: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='registro_ingresos_nueva'", [], |r| r.get(0)
    ).unwrap();
    assert_eq!(nueva, 0);
}

#[test]
fn fallo_tardio_revierte_todas_las_migraciones_pendientes() {
    let connection = Connection::open_in_memory().unwrap();
    crear_esquema_version_1(&connection);
    insertar_referencias(&connection);
    connection
        .execute_batch(
            "
            DROP INDEX idx_registro_ingresos_gafete;
            CREATE TABLE contratistas_fts (id INTEGER PRIMARY KEY);
            PRAGMA user_version = 0;
            ",
        )
        .unwrap();

    assert!(initialize_database(&connection).is_err());

    assert_eq!(version(&connection), 0);
    let indice_recreado: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_registro_ingresos_gafete'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(indice_recreado, 0);

    let definicion_registro: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type='table' AND name='registro_ingresos'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!definicion_registro.contains("fecha_hora_salida IS NULL"));

    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM contratistas", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total, 1);
}

#[test]
fn dos_conexiones_migran_una_base_vacia_sin_reaplicar_pasos() {
    let ruta = base_temporal("concurrente");
    limpiar_base(&ruta);
    let barrera = Arc::new(Barrier::new(3));

    let hilos: Vec<_> = (0..2)
        .map(|_| {
            let ruta = ruta.clone();
            let barrera = Arc::clone(&barrera);
            thread::spawn(move || -> Result<(), String> {
                let connection = Connection::open(&ruta).map_err(|error| error.to_string())?;
                connection
                    .busy_timeout(Duration::from_secs(5))
                    .map_err(|error| error.to_string())?;
                barrera.wait();
                // `busy_timeout` sólo cubre SQLITE_BUSY (la otra conexión
                // tiene el lock de escritura) -- las migraciones son DDL, y
                // dos conexiones haciendo DDL a la vez sobre el mismo
                // archivo, recién creado por ambas al mismo instante
                // (forzado por la barrera), pueden chocar con errores que
                // `busy_timeout` no cubre y que tampoco se reintentan solos
                // -- confirmado reproduciendo dos formas distintas:
                // SQLITE_LOCKED ("database is locked") y una carrera real de
                // `CREATE VIRTUAL TABLE ... fts5` cuando dos conexiones
                // crean la misma tabla FTS5 en el mismo instante
                // ("vtable constructor failed"). Ninguno es un bug de esta
                // base de código -- es un escenario sintético más agresivo
                // que la realidad (`InstanciaGuard` garantiza una sola
                // conexión por archivo en la app real), así que se
                // reintenta a mano acá, no en el código de producción.
                for intento in 0..20 {
                    match initialize_database(&connection) {
                        Ok(()) => return Ok(()),
                        Err(_) if intento < 19 => {
                            thread::sleep(Duration::from_millis(50));
                        }
                        Err(error) => return Err(error.to_string()),
                    }
                }
                unreachable!()
            })
        })
        .collect();

    barrera.wait();
    for hilo in hilos {
        hilo.join().unwrap().unwrap();
    }

    // Conexión nueva sobre una base ya migrada -- `initialize_database` es
    // idempotente (no reaplica nada, ver `version(&connection) ==
    // SCHEMA_VERSION` abajo) pero igual hace falta llamarla para registrar
    // `PLEGAR`, la función detrás de `idx_empresas_nombre_plegado`
    // (migración 47): sin ella, hasta `PRAGMA integrity_check` -- que
    // valida las expresiones de todo índice persistido -- tira "unknown
    // function: PLEGAR()".
    let connection = Connection::open(&ruta).unwrap();
    initialize_database(&connection).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
    let tablas_fts: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='table'
             AND name IN ('contratistas_fts', 'empresas_fts', 'usuarios_fts')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tablas_fts, 3);
    let integridad: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integridad, "ok");

    drop(connection);
    limpiar_base(&ruta);
}

#[test]
fn claves_foraneas_permanecen_activas() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let activas: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(activas, 1);
    assert!(connection.execute(
        "INSERT INTO contratistas (cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso)
         VALUES ('2001','Persona',999,'SWAT',0,1)", []
    ).is_err());
}

// MIGRACION_29 — control de visitas (docs/planes-implementados/plan-control-visitas.md), primer
// corte de esquema: `citas` (autorización con vigencia) → `cita_visitantes`
// (una fila por persona del grupo) → `movimientos_visita` (el cruce real en
// el punto de acceso). Mismas garantías que ya tiene `registro_ingresos`, verificadas acá
// con el mismo criterio que `check_de_salida_acepta_pares_coherentes_y_rechaza_incoherentes`/
// `esquema_actual_solo_admite_fechas_utc_normalizadas` de arriba, pero para
// las tablas nuevas.

fn insertar_cita(connection: &Connection, id: i64, desde: &str, hasta: &str) {
    connection
        .execute(
            "INSERT INTO citas (id, uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                anfitrion_correo, estado, creado_en)
             VALUES (?1, ?2, ?3, ?4, 'Anfitrión de prueba', 'anfitrion@ejemplo.com',
                'VIGENTE', '2026-08-01T00:00:00Z')",
            rusqlite::params![id, format!("uuid-cita-{id}"), desde, hasta],
        )
        .unwrap();
}

fn insertar_cita_visitante(connection: &Connection, id: i64, cita_id: i64, cedula: &str) {
    connection
        .execute(
            "INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre)
             VALUES (?1, ?2, ?3, ?4, 'Visitante de prueba')",
            rusqlite::params![id, format!("uuid-visitante-{id}"), cita_id, cedula],
        )
        .unwrap();
}

#[test]
fn citas_exige_fecha_hasta_no_anterior_a_fecha_desde() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    insertar_cita(&connection, 1, "2026-08-10", "2026-08-15");
    let resultado = connection.execute(
        "INSERT INTO citas (id, uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
            anfitrion_correo, estado, creado_en)
         VALUES (2, 'uuid-cita-2', '2026-08-15', '2026-08-10', 'A', 'a@a.com', 'VIGENTE',
            '2026-08-01T00:00:00Z')",
        [],
    );
    assert!(resultado.is_err());
}

#[test]
fn cita_visitantes_exige_una_cita_existente() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    let resultado = connection.execute(
        "INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre)
         VALUES (1, 'uuid-v1', 999, '1-2345', 'Alguien')",
        [],
    );
    assert!(resultado.is_err());

    insertar_cita(&connection, 1, "2026-08-10", "2026-08-15");
    insertar_cita_visitante(&connection, 1, 1, "1-2345");
    let visitantes: i64 = connection
        .query_row("SELECT COUNT(*) FROM cita_visitantes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(visitantes, 1);
}

#[test]
fn movimientos_visita_exige_visitante_y_usuario_existentes() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    insertar_cita(&connection, 1, "2026-08-10", "2026-08-15");
    insertar_cita_visitante(&connection, 1, 1, "1-2345");

    // Usuario inexistente.
    assert!(
        connection
            .execute(
                "INSERT INTO movimientos_visita
                    (id, uuid, cita_visitante_id, fecha_hora_entrada, usuario_entrada_id,
                     usuario_entrada_nombre)
                 VALUES (1, 'uuid-m1', 1, '2026-08-11T14:00:00Z', 999, 'Nadie')",
                [],
            )
            .is_err()
    );
    // Visitante inexistente.
    assert!(
        connection
            .execute(
                "INSERT INTO movimientos_visita
                    (id, uuid, cita_visitante_id, fecha_hora_entrada, usuario_entrada_id,
                     usuario_entrada_nombre)
                 VALUES (1, 'uuid-m1', 999, '2026-08-11T14:00:00Z', 1, 'Operador')",
                [],
            )
            .is_err()
    );

    connection
        .execute(
            "INSERT INTO movimientos_visita
                (id, uuid, cita_visitante_id, fecha_hora_entrada, usuario_entrada_id,
                 usuario_entrada_nombre)
             VALUES (1, 'uuid-m1', 1, '2026-08-11T14:00:00Z', 1, 'Operador')",
            [],
        )
        .unwrap();
}

#[test]
fn movimientos_visita_no_admite_dos_abiertos_para_el_mismo_visitante() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    insertar_cita(&connection, 1, "2026-08-10", "2026-08-15");
    insertar_cita_visitante(&connection, 1, 1, "1-2345");

    connection
        .execute(
            "INSERT INTO movimientos_visita
                (id, uuid, cita_visitante_id, fecha_hora_entrada, usuario_entrada_id,
                 usuario_entrada_nombre)
             VALUES (1, 'uuid-m1', 1, '2026-08-11T14:00:00Z', 1, 'Operador')",
            [],
        )
        .unwrap();
    let resultado = connection.execute(
        "INSERT INTO movimientos_visita
            (id, uuid, cita_visitante_id, fecha_hora_entrada, usuario_entrada_id,
             usuario_entrada_nombre)
         VALUES (2, 'uuid-m2', 1, '2026-08-11T15:00:00Z', 1, 'Operador')",
        [],
    );
    assert!(resultado.is_err());
}

#[test]
fn movimientos_visita_no_admite_el_mismo_gafete_activo_dos_veces() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    insertar_cita(&connection, 1, "2026-08-10", "2026-08-15");
    insertar_cita_visitante(&connection, 1, 1, "1-2345");
    insertar_cita_visitante(&connection, 2, 1, "6-7890");

    connection
        .execute(
            "INSERT INTO movimientos_visita
                (id, uuid, cita_visitante_id, gafete_numero, fecha_hora_entrada,
                 usuario_entrada_id, usuario_entrada_nombre)
             VALUES (1, 'uuid-m1', 1, 5, '2026-08-11T14:00:00Z', 1, 'Operador')",
            [],
        )
        .unwrap();
    let resultado = connection.execute(
        "INSERT INTO movimientos_visita
            (id, uuid, cita_visitante_id, gafete_numero, fecha_hora_entrada,
             usuario_entrada_id, usuario_entrada_nombre)
         VALUES (2, 'uuid-m2', 2, 5, '2026-08-11T15:00:00Z', 1, 'Operador')",
        [],
    );
    assert!(resultado.is_err());
}

#[test]
fn movimientos_visita_solo_admite_fechas_utc_normalizadas() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    insertar_cita(&connection, 1, "2026-08-10", "2026-08-15");
    insertar_cita_visitante(&connection, 1, 1, "1-2345");

    assert!(
        connection
            .execute(
                "INSERT INTO movimientos_visita
                    (id, uuid, cita_visitante_id, fecha_hora_entrada, usuario_entrada_id,
                     usuario_entrada_nombre)
                 VALUES (1, 'uuid-m1', 1, '2026-08-11 14:00:00', 1, 'Operador')",
                [],
            )
            .is_err()
    );
}

#[test]
fn movimientos_visita_entrada_es_inmutable_y_salida_se_registra_una_sola_vez() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    insertar_cita(&connection, 1, "2026-08-10", "2026-08-15");
    insertar_cita_visitante(&connection, 1, 1, "1-2345");
    connection
        .execute(
            "INSERT INTO movimientos_visita
                (id, uuid, cita_visitante_id, fecha_hora_entrada, usuario_entrada_id,
                 usuario_entrada_nombre)
             VALUES (1, 'uuid-m1', 1, '2026-08-11T14:00:00Z', 1, 'Operador')",
            [],
        )
        .unwrap();

    assert!(
        connection
            .execute(
                "UPDATE movimientos_visita SET fecha_hora_entrada = '2026-08-12T14:00:00Z'
                 WHERE id = 1",
                [],
            )
            .is_err(),
        "los datos de entrada no deberían poder editarse"
    );

    connection
        .execute(
            "UPDATE movimientos_visita
             SET fecha_hora_salida = '2026-08-11T18:00:00Z', usuario_salida_id = 1,
                 usuario_salida_nombre = 'Operador'
             WHERE id = 1",
            [],
        )
        .unwrap();
    assert!(
        connection
            .execute(
                "UPDATE movimientos_visita SET fecha_hora_salida = '2026-08-11T19:00:00Z'
                 WHERE id = 1",
                [],
            )
            .is_err(),
        "la salida no debería poder registrarse dos veces"
    );

    assert!(
        connection
            .execute("DELETE FROM movimientos_visita WHERE id = 1", [])
            .is_err(),
        "un movimiento de visita no debería poder eliminarse"
    );
}

// MIGRACION_30 -- suma 'movimiento_visita' al CHECK de `cola_salida.entidad`.
#[test]
fn cola_salida_acepta_movimiento_visita_y_conserva_los_valores_viejos() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    for (entidad, operacion) in [
        ("contratista", "crear"),
        ("ingreso", "crear"),
        ("empresa", "crear"),
        ("gafete", "crear"),
        ("usuario", "crear"),
        ("movimiento_visita", "crear"),
    ] {
        connection
            .execute(
                "INSERT INTO cola_salida (entidad, entidad_uuid, operacion, creado_en, actualizado_en)
                 VALUES (?1, 'uuid-x', ?2, '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z')",
                rusqlite::params![entidad, operacion],
            )
            .unwrap_or_else(|error| panic!("{entidad} debería seguir siendo válido: {error}"));
    }

    assert!(
        connection
            .execute(
                "INSERT INTO cola_salida (entidad, entidad_uuid, operacion, creado_en, actualizado_en)
                 VALUES ('inventado', 'uuid-x', 'crear', '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z')",
                [],
            )
            .is_err(),
        "un valor de entidad fuera del CHECK debería seguir rechazándose"
    );
}

// MIGRACION_49 -- agrega `placa` a `registro_ingresos`, con un CHECK cruzado
// contra `medio_ingreso` (sólo `VEHICULO` puede llevar placa).
#[test]
fn migracion_49_corre_limpia_y_llega_a_schema_version() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    assert!(
        !connection
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap(),
        "la migración no debería dejar referencias huérfanas"
    );

    let columnas: Vec<String> = connection
        .prepare("SELECT name FROM pragma_table_info('registro_ingresos')")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(
        columnas.iter().any(|c| c == "placa"),
        "registro_ingresos debería tener la columna placa: {columnas:?}"
    );
}

#[test]
fn migracion_50_corre_limpia_y_llega_a_schema_version() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);

    let columnas: Vec<String> = connection
        .prepare(
            "SELECT name FROM pragma_table_info('prestamos_gafete_provisional_historial_sitio')",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(
        columnas.iter().any(|c| c == "fecha_hora_devolucion"),
        "prestamos_gafete_provisional_historial_sitio debería existir con sus columnas: {columnas:?}"
    );

    let columnas_sync: Vec<String> = connection
        .prepare("SELECT name FROM pragma_table_info('sincronizacion_estado')")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(
        columnas_sync
            .iter()
            .any(|c| c == "gafetes_provisionales_historial_actualizado_hasta"),
        "sincronizacion_estado debería tener la marca de agua nueva: {columnas_sync:?}"
    );
}

/// `contratista_id` distinto en cada llamada a propósito -- el índice único
/// `idx_registro_ingresos_contratista_activo` sólo permite un ingreso ACTIVO
/// por contratista a la vez, y este test no le importa la salida, sólo el
/// `CHECK` de placa.
fn insertar_ingreso_base(
    connection: &Connection,
    contratista_id: i64,
    medio_ingreso: &str,
    placa: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    connection.execute(
        "INSERT INTO registro_ingresos (
            contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
            gafete_numero, usuario_ingreso_id, contratista_cedula, contratista_nombre,
            empresa_nombre, usuario_ingreso_nombre, es_personal_ruta, tiene_acceso,
            resultado_acceso, reglas_version, uuid, placa
         ) VALUES (
            ?1, 1, '2026-08-11T08:00:00Z', ?2, 'PRAIND', NULL, 1, '1-1111', 'Persona',
            'Empresa', 'Operador', 0, 1, 'PERMITIDO', 1, ?3, ?4
         )",
        rusqlite::params![
            contratista_id,
            medio_ingreso,
            format!("uuid-{contratista_id}-{medio_ingreso}"),
            placa
        ],
    )?;
    Ok(connection.last_insert_rowid())
}

/// El `CHECK ((medio_ingreso = 'VEHICULO') OR (placa IS NULL))` de
/// `MIGRACION_49` es la única garantía dura de que `CAMINANDO` nunca lleva
/// placa -- sin esto, la validación del servicio (`tests/registro_ingreso_service.rs`)
/// sería la única barrera, y un `INSERT` directo (ej. un import) podría
/// colarse.
#[test]
fn check_de_placa_rechaza_caminando_con_placa_y_acepta_el_resto() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas (id, nombre) VALUES (1, 'Empresa')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
             VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
            [],
        )
        .unwrap();
    for id in 1..=4i64 {
        connection
            .execute(
                "INSERT INTO contratistas (
                    id, cedula, nombre, empresa_id, tipo_ingreso,
                    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
                 ) VALUES (?1, ?2, 'Persona', 1, 'PRAIND', '2030-01-01', 0, 1)",
                rusqlite::params![id, format!("1-{id}")],
            )
            .unwrap();
    }

    assert!(
        insertar_ingreso_base(&connection, 1, "CAMINANDO", Some("ABC123")).is_err(),
        "CAMINANDO con placa debería violar el CHECK"
    );
    assert!(
        insertar_ingreso_base(&connection, 2, "CAMINANDO", None).is_ok(),
        "CAMINANDO sin placa sigue siendo válido"
    );
    assert!(
        insertar_ingreso_base(&connection, 3, "VEHICULO", Some("ABC123")).is_ok(),
        "VEHICULO con placa es el caso normal"
    );
    assert!(
        insertar_ingreso_base(&connection, 4, "VEHICULO", None).is_ok(),
        "VEHICULO sin placa (dato viejo pre-migración) sigue siendo válido"
    );
}

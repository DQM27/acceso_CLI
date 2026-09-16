//! Escritura de préstamos de gafete provisional KOF
//! (`docs/features-futuras/plan-gafetes-provisionales-kof.md`) -- ciclo
//! entrega/devolución, mismo armazón que `MovimientoVisitaRepository` pero
//! sin ningún campo de resultado/validación: pedido explícito del usuario,
//! no hay más verificación que la humana (cotejar la cédula física contra
//! el nombre, algo que este sistema no captura ni valida). Sincroniza a la
//! nube igual que `movimiento_visita` -- ciclo abrir/cerrar, `cola_salida`
//! con `'prestamo_gafete_provisional'` (`MIGRACION_40`), nunca por lote (ver
//! `nube::sincronizacion::procesar_fila_individual`).

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, named_params, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::prestamo_gafete_provisional::{
    DevolucionPrestamoGafeteProvisional, NuevoPrestamoGafeteProvisional,
    PrestamoGafeteProvisional, PrestamoGafeteProvisionalActivoResumen,
};
use crate::tiempo::{parsear_utc, serializar_utc};

pub trait PrestamoGafeteProvisionalRepository {
    fn crear(&self, prestamo: &NuevoPrestamoGafeteProvisional) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<PrestamoGafeteProvisional>, DatabaseError>;

    fn buscar_activo_por_encargado(
        &self,
        encargado_id: i64,
    ) -> Result<Option<PrestamoGafeteProvisional>, DatabaseError>;

    /// Mismo motivo que `MovimientoVisitaRepository::buscar_activo_por_gafete`:
    /// evitar que un número de gafete quede prestado a dos personas a la vez
    /// -- el `CHECK` ya lo impide (`idx_prestamos_gafete_provisional_numero_activo`),
    /// esto es la consulta que deja mostrar el mensaje ANTES de intentar el
    /// `INSERT` y chocar contra esa restricción.
    fn buscar_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<PrestamoGafeteProvisional>, DatabaseError>;

    fn registrar_devolucion(
        &self,
        id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
    ) -> Result<(), DatabaseError>;

    /// Fila aplanada para la pantalla "Activos" -- análoga a
    /// `MovimientoVisitaRepository::listar_activos`.
    fn listar_activos(&self) -> Result<Vec<PrestamoGafeteProvisionalActivoResumen>, DatabaseError>;
}

pub struct SqlitePrestamoGafeteProvisionalRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqlitePrestamoGafeteProvisionalRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<PrestamoGafeteProvisional> {
    let fecha_hora_entrega_texto: String = row.get(4)?;
    let fecha_hora_entrega = parsear_utc(&fecha_hora_entrega_texto).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(error))
    })?;

    let fecha_hora_devolucion_texto: Option<String> = row.get(6)?;
    let fecha_hora_devolucion = fecha_hora_devolucion_texto
        .map(|fecha| {
            parsear_utc(&fecha).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;
    let usuario_devolucion_id: Option<i64> = row.get(7)?;
    // `CHECK (fecha_hora_devolucion IS NULL) = (usuario_devolucion_id IS NULL)`
    // ya lo garantiza el esquema (ver MIGRACION_39).
    let devolucion = fecha_hora_devolucion.zip(usuario_devolucion_id).map(
        |(fecha_hora, usuario_id)| DevolucionPrestamoGafeteProvisional {
            fecha_hora,
            usuario_id,
        },
    );

    Ok(PrestamoGafeteProvisional {
        id: row.get(0)?,
        encargado_id: row.get(1)?,
        encargado_nombre: row.get(2)?,
        encargado_codigo_empleado: row.get(3)?,
        gafete_numero: row.get(5)?,
        fecha_hora_entrega,
        usuario_entrega_id: row.get(8)?,
        devolucion,
    })
}

const SELECT_PRESTAMO: &str = "
    SELECT id, encargado_id, encargado_nombre, encargado_codigo_empleado,
           fecha_hora_entrega, gafete_numero, fecha_hora_devolucion,
           usuario_devolucion_id, usuario_entrega_id
    FROM prestamos_gafete_provisional
";

impl PrestamoGafeteProvisionalRepository for SqlitePrestamoGafeteProvisionalRepository<'_> {
    fn crear(&self, prestamo: &NuevoPrestamoGafeteProvisional) -> Result<i64, DatabaseError> {
        let fecha_hora_entrega = serializar_utc(prestamo.fecha_hora_entrega);
        let uuid = generar_uuid_v4();

        let filas = self.connection.execute(
            "
            INSERT INTO prestamos_gafete_provisional (
                encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero,
                fecha_hora_entrega, usuario_entrega_id, usuario_entrega_nombre, uuid
            )
            SELECT :encargado_id, :encargado_nombre, :encargado_codigo_empleado, :gafete_numero,
                   :fecha_hora_entrega, :usuario_entrega_id, u.nombre, :uuid
            FROM usuarios AS u
            WHERE u.id = :usuario_entrega_id
            ",
            named_params! {
                ":encargado_id": prestamo.encargado_id,
                ":encargado_nombre": prestamo.encargado_nombre,
                ":encargado_codigo_empleado": prestamo.encargado_codigo_empleado,
                ":gafete_numero": prestamo.gafete_numero,
                ":fecha_hora_entrega": fecha_hora_entrega,
                ":usuario_entrega_id": prestamo.usuario_entrega_id,
                ":uuid": uuid,
            },
        )?;

        if filas == 0 {
            return Err(DatabaseError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "prestamo_gafete_provisional", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<PrestamoGafeteProvisional>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_PRESTAMO} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(prestamo) => Ok(Some(prestamo)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_activo_por_encargado(
        &self,
        encargado_id: i64,
    ) -> Result<Option<PrestamoGafeteProvisional>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_PRESTAMO} WHERE encargado_id = ?1 AND fecha_hora_devolucion IS NULL
             ORDER BY fecha_hora_entrega DESC LIMIT 1"
        ))?;
        match statement.query_row(params![encargado_id], convertir_fila) {
            Ok(prestamo) => Ok(Some(prestamo)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<PrestamoGafeteProvisional>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_PRESTAMO} WHERE gafete_numero = ?1 AND fecha_hora_devolucion IS NULL
             ORDER BY fecha_hora_entrega DESC LIMIT 1"
        ))?;
        match statement.query_row(params![gafete_numero], convertir_fila) {
            Ok(prestamo) => Ok(Some(prestamo)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn registrar_devolucion(
        &self,
        id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
    ) -> Result<(), DatabaseError> {
        let fecha_hora_texto = serializar_utc(fecha_hora);

        let filas_afectadas = self.connection.execute(
            "
            UPDATE prestamos_gafete_provisional
            SET
                fecha_hora_devolucion = ?1,
                usuario_devolucion_id = ?2,
                usuario_devolucion_nombre = (SELECT nombre FROM usuarios WHERE id = ?2)
            WHERE id = ?3
              AND fecha_hora_devolucion IS NULL
            ",
            params![fecha_hora_texto, usuario_id, id],
        )?;

        if filas_afectadas == 0 {
            return Err(DatabaseError::PrestamoGafeteProvisionalNoActivo);
        }

        let uuid: String = self.connection.query_row(
            "SELECT uuid FROM prestamos_gafete_provisional WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        cola_salida::encolar(self.connection, "prestamo_gafete_provisional", &uuid, "cerrar")?;

        Ok(())
    }

    fn listar_activos(&self) -> Result<Vec<PrestamoGafeteProvisionalActivoResumen>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, encargado_nombre, encargado_codigo_empleado, gafete_numero, fecha_hora_entrega
            FROM prestamos_gafete_provisional
            WHERE fecha_hora_devolucion IS NULL
            ORDER BY fecha_hora_entrega ASC
            ",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        filas
            .into_iter()
            .map(
                |(id, encargado_nombre, encargado_codigo_empleado, gafete_numero, fecha_hora_texto)| {
                    let fecha_hora_entrega = parsear_utc(&fecha_hora_texto)
                        .map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))?;
                    Ok(PrestamoGafeteProvisionalActivoResumen {
                        id,
                        encargado_nombre,
                        encargado_codigo_empleado,
                        gafete_numero,
                        fecha_hora_entrega,
                    })
                },
            )
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    fn conexion_con_encargado() -> (Connection, i64) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                    VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
                 INSERT INTO encargados_ruta (id, codigo_empleado, nombre, activo, uuid)
                    VALUES (1, '5040017', 'Michael Araya Retana', 1, 'uuid-encargado-1');",
            )
            .unwrap();
        (connection, 1)
    }

    fn nuevo(encargado_id: i64, gafete_numero: i64) -> NuevoPrestamoGafeteProvisional {
        NuevoPrestamoGafeteProvisional {
            encargado_id,
            encargado_nombre: "Michael Araya Retana".to_string(),
            encargado_codigo_empleado: "5040017".to_string(),
            gafete_numero,
            fecha_hora_entrega: Utc::now(),
            usuario_entrega_id: 1,
        }
    }

    #[test]
    fn crear_y_buscar_por_id_redondea_el_viaje() {
        let (connection, encargado_id) = conexion_con_encargado();
        let repo = SqlitePrestamoGafeteProvisionalRepository::new(&connection);

        let id = repo.crear(&nuevo(encargado_id, 7)).unwrap();
        let prestamo = repo.buscar_por_id(id).unwrap().unwrap();

        assert_eq!(prestamo.encargado_id, encargado_id);
        assert_eq!(prestamo.gafete_numero, 7);
        assert!(prestamo.devolucion.is_none());
    }

    #[test]
    fn crear_con_usuario_inexistente_falla() {
        let (connection, encargado_id) = conexion_con_encargado();
        let repo = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let mut prestamo = nuevo(encargado_id, 7);
        prestamo.usuario_entrega_id = 999;

        assert!(repo.crear(&prestamo).is_err());
    }

    #[test]
    fn buscar_activo_por_encargado_ignora_prestamos_ya_devueltos() {
        let (connection, encargado_id) = conexion_con_encargado();
        let repo = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let id = repo.crear(&nuevo(encargado_id, 7)).unwrap();

        assert!(
            repo.buscar_activo_por_encargado(encargado_id)
                .unwrap()
                .is_some()
        );

        repo.registrar_devolucion(id, Utc::now(), 1).unwrap();

        assert!(
            repo.buscar_activo_por_encargado(encargado_id)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn buscar_activo_por_gafete_encuentra_el_prestamo_abierto() {
        let (connection, encargado_id) = conexion_con_encargado();
        let repo = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        repo.crear(&nuevo(encargado_id, 3)).unwrap();

        let activo = repo.buscar_activo_por_gafete(3).unwrap();

        assert!(activo.is_some());
        assert_eq!(activo.unwrap().gafete_numero, 3);
        assert!(repo.buscar_activo_por_gafete(99).unwrap().is_none());
    }

    #[test]
    fn registrar_devolucion_dos_veces_falla_la_segunda() {
        let (connection, encargado_id) = conexion_con_encargado();
        let repo = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let id = repo.crear(&nuevo(encargado_id, 7)).unwrap();

        repo.registrar_devolucion(id, Utc::now(), 1).unwrap();

        assert!(matches!(
            repo.registrar_devolucion(id, Utc::now(), 1),
            Err(DatabaseError::PrestamoGafeteProvisionalNoActivo)
        ));
    }

    #[test]
    fn dos_prestamos_abiertos_a_la_vez_con_el_mismo_encargado_falla() {
        let (connection, encargado_id) = conexion_con_encargado();
        let repo = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        repo.crear(&nuevo(encargado_id, 5)).unwrap();

        assert!(repo.crear(&nuevo(encargado_id, 9)).is_err());
    }

    #[test]
    fn dos_prestamos_abiertos_a_la_vez_con_el_mismo_gafete_falla() {
        let (connection, encargado_id) = conexion_con_encargado();
        connection
            .execute(
                "INSERT INTO encargados_ruta (id, codigo_empleado, nombre, activo, uuid)
                 VALUES (2, '77851', 'Ramon Rodriguez', 1, 'uuid-encargado-2')",
                [],
            )
            .unwrap();
        let repo = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        repo.crear(&nuevo(encargado_id, 5)).unwrap();

        assert!(repo.crear(&nuevo(2, 5)).is_err());
    }

    #[test]
    fn listar_activos_trae_el_prestamo_abierto_y_omite_el_ya_devuelto() {
        let (connection, encargado_id) = conexion_con_encargado();
        connection
            .execute(
                "INSERT INTO encargados_ruta (id, codigo_empleado, nombre, activo, uuid)
                 VALUES (2, '77851', 'Ramon Rodriguez', 1, 'uuid-encargado-2')",
                [],
            )
            .unwrap();
        let repo = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let activo_id = repo.crear(&nuevo(encargado_id, 3)).unwrap();
        let cerrado_id = repo.crear(&nuevo(2, 8)).unwrap();
        repo.registrar_devolucion(cerrado_id, Utc::now(), 1)
            .unwrap();

        let activos = repo.listar_activos().unwrap();

        assert_eq!(activos.len(), 1);
        let fila = &activos[0];
        assert_eq!(fila.id, activo_id);
        assert_eq!(fila.encargado_nombre, "Michael Araya Retana");
        assert_eq!(fila.gafete_numero, 3);
    }
}

//! Ingreso/salida de proveedores
//! (`docs/features-futuras/plan-control-proveedores.md`) -- ciclo
//! entrada/salida, mismo armazón que `MovimientoVisitaRepository`: sin
//! catálogo de personas (snapshot puro de cédula/nombre), sin PRAIND/SWAT/
//! `tipo_ingreso` (exclusivo de contratistas). Sincroniza a la nube igual que
//! `movimiento_visita`/`prestamo_gafete_provisional` -- ciclo abrir/cerrar,
//! `cola_salida` con `'ingreso_proveedor'` (`MIGRACION_41`), nunca por lote
//! (ver `nube::sincronizacion::procesar_fila_individual`).

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, named_params, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::registro_ingreso_proveedor::{
    NuevoRegistroIngresoProveedor, RegistroIngresoProveedor, RegistroIngresoProveedorActivoResumen,
    SalidaRegistroIngresoProveedor,
};
use crate::tiempo::{parsear_utc, serializar_utc};

pub trait RegistroIngresoProveedorRepository {
    fn crear(&self, registro: &NuevoRegistroIngresoProveedor) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<RegistroIngresoProveedor>, DatabaseError>;

    /// Ingreso abierto (sin salida) para esta cédula -- mismo motivo que
    /// `RegistroIngresoRepository::buscar_ingreso_activo`: evitar que la
    /// misma persona quede con dos ingresos abiertos a la vez.
    fn buscar_ingreso_activo(
        &self,
        cedula: &str,
    ) -> Result<Option<RegistroIngresoProveedor>, DatabaseError>;

    /// Mismo motivo que `RegistroIngresoRepository::buscar_ingreso_activo_por_gafete`:
    /// evitar que un número de gafete quede asignado a dos ingresos de
    /// proveedor a la vez -- el `CHECK`/índice único del esquema ya lo
    /// impide, esto es la consulta que deja mostrar el mensaje ANTES de
    /// intentar el `INSERT` y chocar contra esa restricción.
    fn buscar_ingreso_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<RegistroIngresoProveedor>, DatabaseError>;

    fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError>;

    fn listar(&self) -> Result<Vec<RegistroIngresoProveedor>, DatabaseError>;

    /// Fila aplanada para la pantalla "Activos" -- análoga a
    /// `MovimientoVisitaRepository::listar_activos`/
    /// `PrestamoGafeteProvisionalRepository::listar_activos`.
    fn listar_activos(&self) -> Result<Vec<RegistroIngresoProveedorActivoResumen>, DatabaseError>;
}

pub struct SqliteRegistroIngresoProveedorRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteRegistroIngresoProveedorRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<RegistroIngresoProveedor> {
    let fecha_hora_ingreso_texto: String = row.get(7)?;
    let fecha_hora_ingreso = parsear_utc(&fecha_hora_ingreso_texto).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(error))
    })?;

    let fecha_hora_salida_texto: Option<String> = row.get(9)?;
    let fecha_hora_salida = fecha_hora_salida_texto
        .map(|fecha| {
            parsear_utc(&fecha).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    9,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;
    let usuario_salida_id: Option<i64> = row.get(10)?;
    // `CHECK (fecha_hora_salida IS NULL) = (usuario_salida_id IS NULL)` en el
    // esquema (MIGRACION_41) garantiza que ambos vienen juntos o ninguno.
    let salida = fecha_hora_salida
        .zip(usuario_salida_id)
        .map(|(fecha_hora, usuario_id)| SalidaRegistroIngresoProveedor {
            fecha_hora,
            usuario_id,
        });

    Ok(RegistroIngresoProveedor {
        id: row.get(0)?,
        cedula: row.get(1)?,
        nombre: row.get(2)?,
        empresa_id: row.get(3)?,
        empresa_nombre: row.get(4)?,
        placa: row.get(5)?,
        gafete_numero: row.get(6)?,
        fecha_hora_ingreso,
        usuario_ingreso_id: row.get(8)?,
        salida,
    })
}

const SELECT_REGISTRO: &str = "
    SELECT id, cedula, nombre, empresa_id, empresa_nombre, placa, gafete_numero,
           fecha_hora_ingreso, usuario_ingreso_id, fecha_hora_salida, usuario_salida_id
    FROM registro_ingresos_proveedor
";

impl RegistroIngresoProveedorRepository for SqliteRegistroIngresoProveedorRepository<'_> {
    fn crear(&self, registro: &NuevoRegistroIngresoProveedor) -> Result<i64, DatabaseError> {
        let fecha_hora_ingreso = serializar_utc(registro.fecha_hora_ingreso);
        let uuid = generar_uuid_v4();

        let filas = self.connection.execute(
            "
            INSERT INTO registro_ingresos_proveedor (
                cedula, nombre, empresa_id, empresa_nombre, placa, gafete_numero,
                fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre, uuid
            )
            SELECT :cedula, :nombre, :empresa_id, :empresa_nombre, :placa, :gafete_numero,
                   :fecha_hora_ingreso, :usuario_ingreso_id, u.nombre, :uuid
            FROM usuarios AS u
            WHERE u.id = :usuario_ingreso_id
            ",
            named_params! {
                ":cedula": registro.cedula,
                ":nombre": registro.nombre,
                ":empresa_id": registro.empresa_id,
                ":empresa_nombre": registro.empresa_nombre,
                ":placa": registro.placa,
                ":gafete_numero": registro.gafete_numero,
                ":fecha_hora_ingreso": fecha_hora_ingreso,
                ":usuario_ingreso_id": registro.usuario_ingreso_id,
                ":uuid": uuid,
            },
        )?;

        if filas == 0 {
            return Err(DatabaseError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "ingreso_proveedor", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<RegistroIngresoProveedor>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_REGISTRO} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(registro) => Ok(Some(registro)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_ingreso_activo(
        &self,
        cedula: &str,
    ) -> Result<Option<RegistroIngresoProveedor>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_REGISTRO} WHERE cedula = ?1 AND fecha_hora_salida IS NULL
             ORDER BY fecha_hora_ingreso DESC LIMIT 1"
        ))?;
        match statement.query_row(params![cedula], convertir_fila) {
            Ok(registro) => Ok(Some(registro)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_ingreso_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<RegistroIngresoProveedor>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_REGISTRO} WHERE gafete_numero = ?1 AND fecha_hora_salida IS NULL
             ORDER BY fecha_hora_ingreso DESC LIMIT 1"
        ))?;
        match statement.query_row(params![gafete_numero], convertir_fila) {
            Ok(registro) => Ok(Some(registro)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError> {
        let fecha_hora_salida = serializar_utc(fecha_hora_salida);

        let filas_afectadas = self.connection.execute(
            "
            UPDATE registro_ingresos_proveedor
            SET
                fecha_hora_salida = ?1,
                usuario_salida_id = ?2,
                usuario_salida_nombre = (SELECT nombre FROM usuarios WHERE id = ?2)
            WHERE id = ?3
              AND fecha_hora_salida IS NULL
            ",
            params![fecha_hora_salida, usuario_salida_id, id],
        )?;

        if filas_afectadas == 0 {
            return Err(DatabaseError::RegistroProveedorNoActivo);
        }

        let uuid: String = self.connection.query_row(
            "SELECT uuid FROM registro_ingresos_proveedor WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        cola_salida::encolar(self.connection, "ingreso_proveedor", &uuid, "cerrar")?;

        Ok(())
    }

    fn listar(&self) -> Result<Vec<RegistroIngresoProveedor>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_REGISTRO} ORDER BY fecha_hora_ingreso DESC"
        ))?;
        let registros = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(registros)
    }

    fn listar_activos(&self) -> Result<Vec<RegistroIngresoProveedorActivoResumen>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, cedula, nombre, empresa_nombre, placa, gafete_numero, fecha_hora_ingreso,
                   usuario_ingreso_nombre
            FROM registro_ingresos_proveedor
            WHERE fecha_hora_salida IS NULL
            ORDER BY fecha_hora_ingreso ASC
            ",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        filas
            .into_iter()
            .map(
                |(
                    id,
                    cedula,
                    nombre,
                    empresa_nombre,
                    placa,
                    gafete_numero,
                    fecha_hora_texto,
                    usuario_ingreso_nombre,
                )| {
                    let fecha_hora_ingreso = parsear_utc(&fecha_hora_texto)
                        .map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))?;
                    Ok(RegistroIngresoProveedorActivoResumen {
                        id,
                        cedula,
                        nombre,
                        empresa_nombre,
                        placa,
                        gafete_numero,
                        fecha_hora_ingreso,
                        usuario_ingreso_nombre,
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

    fn conexion_con_empresa() -> (Connection, i64) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                    VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
                 INSERT INTO empresas_proveedor (id, nombre, activo, uuid)
                    VALUES (1, 'Maika', 1, 'uuid-empresa-proveedor-1');",
            )
            .unwrap();
        (connection, 1)
    }

    fn nuevo(empresa_id: i64, cedula: &str, gafete_numero: i64) -> NuevoRegistroIngresoProveedor {
        NuevoRegistroIngresoProveedor {
            cedula: cedula.to_string(),
            nombre: "Juan Perez".to_string(),
            empresa_id,
            empresa_nombre: "Maika".to_string(),
            placa: None,
            gafete_numero,
            fecha_hora_ingreso: Utc::now(),
            usuario_ingreso_id: 1,
        }
    }

    #[test]
    fn crear_y_buscar_por_id_redondea_el_viaje() {
        let (connection, empresa_id) = conexion_con_empresa();
        let repo = SqliteRegistroIngresoProveedorRepository::new(&connection);

        let id = repo.crear(&nuevo(empresa_id, "1-1111", 7)).unwrap();
        let registro = repo.buscar_por_id(id).unwrap().unwrap();

        assert_eq!(registro.cedula, "1-1111");
        assert_eq!(registro.gafete_numero, 7);
        assert!(registro.salida.is_none());
    }

    #[test]
    fn buscar_ingreso_activo_ignora_ingresos_ya_cerrados() {
        let (connection, empresa_id) = conexion_con_empresa();
        let repo = SqliteRegistroIngresoProveedorRepository::new(&connection);
        let id = repo.crear(&nuevo(empresa_id, "1-1111", 7)).unwrap();

        assert!(repo.buscar_ingreso_activo("1-1111").unwrap().is_some());

        repo.registrar_salida(id, Utc::now(), 1).unwrap();

        assert!(repo.buscar_ingreso_activo("1-1111").unwrap().is_none());
    }

    #[test]
    fn dos_ingresos_abiertos_a_la_vez_con_la_misma_cedula_falla() {
        let (connection, empresa_id) = conexion_con_empresa();
        let repo = SqliteRegistroIngresoProveedorRepository::new(&connection);
        repo.crear(&nuevo(empresa_id, "1-1111", 5)).unwrap();

        assert!(repo.crear(&nuevo(empresa_id, "1-1111", 9)).is_err());
    }

    #[test]
    fn dos_ingresos_abiertos_a_la_vez_con_el_mismo_gafete_falla() {
        let (connection, empresa_id) = conexion_con_empresa();
        let repo = SqliteRegistroIngresoProveedorRepository::new(&connection);
        repo.crear(&nuevo(empresa_id, "1-1111", 5)).unwrap();

        assert!(repo.crear(&nuevo(empresa_id, "2-2222", 5)).is_err());
    }

    #[test]
    fn registrar_salida_dos_veces_falla_la_segunda() {
        let (connection, empresa_id) = conexion_con_empresa();
        let repo = SqliteRegistroIngresoProveedorRepository::new(&connection);
        let id = repo.crear(&nuevo(empresa_id, "1-1111", 7)).unwrap();

        repo.registrar_salida(id, Utc::now(), 1).unwrap();

        assert!(matches!(
            repo.registrar_salida(id, Utc::now(), 1),
            Err(DatabaseError::RegistroProveedorNoActivo)
        ));
    }

    #[test]
    fn listar_activos_trae_el_ingreso_abierto_y_omite_el_ya_cerrado() {
        let (connection, empresa_id) = conexion_con_empresa();
        let repo = SqliteRegistroIngresoProveedorRepository::new(&connection);
        let activo_id = repo.crear(&nuevo(empresa_id, "1-1111", 3)).unwrap();
        let cerrado_id = repo.crear(&nuevo(empresa_id, "2-2222", 8)).unwrap();
        repo.registrar_salida(cerrado_id, Utc::now(), 1).unwrap();

        let activos = repo.listar_activos().unwrap();

        assert_eq!(activos.len(), 1);
        let fila = &activos[0];
        assert_eq!(fila.id, activo_id);
        assert_eq!(fila.cedula, "1-1111");
        assert_eq!(fila.gafete_numero, 3);
        assert_eq!(fila.usuario_ingreso_nombre, "Operador");
    }
}

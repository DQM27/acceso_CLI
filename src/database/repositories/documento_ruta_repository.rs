use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::documento_ruta::{DocumentoRuta, NuevoDocumentoRuta};
use crate::tiempo::serializar_utc;

/// Ver `docs/planes-implementados/plan-control-rutas.md`, sección
/// "Rediseño del núcleo de rutas -- documento/tramo/viaje". Sin
/// `actualizar` a propósito -- un documento es inmutable una vez creado
/// (mismo criterio que `salidas_ruta`), la base lo garantiza con un
/// trigger propio (`documentos_ruta_inmutable`).
pub trait DocumentoRutaRepository {
    fn crear(
        &self,
        documento: &NuevoDocumentoRuta,
        fecha_hora_creacion: DateTime<Utc>,
    ) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<DocumentoRuta>, DatabaseError>;

    /// El punto de reuso de un documento en una recarga: antes de crear
    /// uno nuevo, `RutaService` consulta si el número ya existe -- si sí,
    /// reusa esa fila en vez de duplicarla (`numero_documento` es único).
    fn buscar_por_numero(
        &self,
        numero_documento: &str,
    ) -> Result<Option<DocumentoRuta>, DatabaseError>;
}

pub struct SqliteDocumentoRutaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteDocumentoRutaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<DocumentoRuta> {
    let fecha_documento_texto: String = row.get(3)?;
    let fecha_documento =
        NaiveDate::parse_from_str(&fecha_documento_texto, "%Y-%m-%d").map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;

    Ok(DocumentoRuta {
        id: row.get(0)?,
        numero_documento: row.get(1)?,
        ruta_id: row.get(2)?,
        sub_numero: row.get(4)?,
        fecha_documento,
    })
}

const SELECT_DOCUMENTO: &str =
    "SELECT id, numero_documento, ruta_id, fecha_documento, sub_numero FROM documentos_ruta";

impl DocumentoRutaRepository for SqliteDocumentoRutaRepository<'_> {
    fn crear(
        &self,
        documento: &NuevoDocumentoRuta,
        fecha_hora_creacion: DateTime<Utc>,
    ) -> Result<i64, DatabaseError> {
        let fecha_documento = documento.fecha_documento.format("%Y-%m-%d").to_string();
        let fecha_hora_creacion = serializar_utc(fecha_hora_creacion);
        let uuid = generar_uuid_v4();

        self.connection.execute(
            "
            INSERT INTO documentos_ruta (
                numero_documento, ruta_id, sub_numero, fecha_documento,
                fecha_hora_creacion, uuid
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                documento.numero_documento,
                documento.ruta_id,
                documento.sub_numero,
                fecha_documento,
                fecha_hora_creacion,
                uuid,
            ],
        )?;

        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "documento_ruta", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<DocumentoRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_DOCUMENTO} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(documento) => Ok(Some(documento)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_numero(
        &self,
        numero_documento: &str,
    ) -> Result<Option<DocumentoRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_DOCUMENTO} WHERE numero_documento = ?1"))?;
        match statement.query_row(params![numero_documento], convertir_fila) {
            Ok(documento) => Ok(Some(documento)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;
    use chrono::TimeZone;

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    fn nuevo(numero_documento: &str, ruta_id: Option<i64>) -> NuevoDocumentoRuta {
        NuevoDocumentoRuta {
            numero_documento: numero_documento.to_string(),
            ruta_id,
            sub_numero: 1,
            fecha_documento: "2026-09-19".parse().unwrap(),
        }
    }

    fn ahora() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 19, 12, 0, 0).unwrap()
    }

    #[test]
    fn crear_y_buscar_por_numero_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteDocumentoRutaRepository::new(&connection);

        let id = repo.crear(&nuevo("700101452", None), ahora()).unwrap();
        let documento = repo.buscar_por_numero("700101452").unwrap().unwrap();

        assert_eq!(documento.id, id);
        assert!(documento.ruta_id.is_none());
    }

    #[test]
    fn documento_sin_ruta_de_catalogo_es_valido() {
        // Caso tercero: hay documento pero no hay una ruta de catálogo
        // real detrás de la carga -- confirmado explícitamente por el
        // usuario que esto puede pasar.
        let connection = conexion();
        let repo = SqliteDocumentoRutaRepository::new(&connection);

        let id = repo.crear(&nuevo("999888777", None), ahora()).unwrap();

        assert!(repo.buscar_por_id(id).unwrap().unwrap().ruta_id.is_none());
    }

    #[test]
    fn numero_documento_duplicado_falla() {
        let connection = conexion();
        let repo = SqliteDocumentoRutaRepository::new(&connection);
        repo.crear(&nuevo("700101452", None), ahora()).unwrap();

        let error = repo.crear(&nuevo("700101452", None), ahora()).unwrap_err();
        assert!(error.es_constraint_unique());
    }

    #[test]
    fn buscar_por_numero_inexistente_devuelve_none() {
        let connection = conexion();
        let repo = SqliteDocumentoRutaRepository::new(&connection);

        assert!(repo.buscar_por_numero("nada").unwrap().is_none());
    }
}

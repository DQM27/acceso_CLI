//! Escritura del catálogo de gafetes (`docs/plan-gafetes.md`). Un solo
//! trait para lectura+escritura puntual (a diferencia de Empresas/
//! Contratistas, que separan `*Query` de `*Repository`) porque el catálogo
//! es chico y no hay una proyección cara que justifique separarlos —
//! `GafetesQuery` (`queries/gafetes.rs`) sigue aparte sólo para la lista
//! completa con datos del deudor, que sí es una proyección propia.

use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::gafete::{EstadoGafete, Gafete, PortadorGafete, TipoGafete};

pub trait GafeteRepository {
    fn crear(&self, numero: i64, tipo: TipoGafete) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<Gafete>, DatabaseError>;

    fn buscar_por_numero(
        &self,
        numero: i64,
        tipo: TipoGafete,
    ) -> Result<Option<Gafete>, DatabaseError>;

    fn dar_de_baja(&self, id: i64) -> Result<(), DatabaseError>;

    fn marcar_perdido(&self, id: i64, portador: PortadorGafete) -> Result<(), DatabaseError>;

    fn resolver(&self, id: i64) -> Result<(), DatabaseError>;

    /// Números de los gafetes que un contratista debe actualmente
    /// (`estado = 'PERDIDO'` con `contratista_portador_id` apuntándolo). Un
    /// `Vec` y no `Option<i64>`: nada impide más de una deuda simultánea.
    fn deuda_de_contratista(&self, contratista_id: i64) -> Result<Vec<i64>, DatabaseError>;
}

pub struct SqliteGafeteRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteGafeteRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<Gafete> {
    let tipo_texto: String = row.get(2)?;
    let Some(tipo) = TipoGafete::from_str_sql(&tipo_texto) else {
        return Err(rusqlite::Error::InvalidColumnType(
            2,
            "tipo".to_string(),
            rusqlite::types::Type::Text,
        ));
    };
    let estado_texto: String = row.get(3)?;
    let Some(estado) = EstadoGafete::from_str_sql(&estado_texto) else {
        return Err(rusqlite::Error::InvalidColumnType(
            3,
            "estado".to_string(),
            rusqlite::types::Type::Text,
        ));
    };

    Ok(Gafete {
        id: row.get(0)?,
        numero: row.get(1)?,
        tipo,
        estado,
        contratista_portador_id: row.get(4)?,
        visita_portador_id: row.get(5)?,
        encargado_portador_id: row.get(6)?,
        proveedor_portador_id: row.get(7)?,
    })
}

const SELECT_GAFETE: &str = "SELECT id, numero, tipo, estado, contratista_portador_id, \
     visita_portador_id, encargado_portador_id, proveedor_portador_id FROM gafetes";

fn encolar_actualizacion(connection: &Connection, id: i64) -> Result<(), DatabaseError> {
    let uuid: Option<String> = connection.query_row(
        "SELECT uuid FROM gafetes WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    if let Some(uuid) = uuid {
        cola_salida::encolar(connection, "gafete", &uuid, "actualizar")?;
    }
    Ok(())
}

impl GafeteRepository for SqliteGafeteRepository<'_> {
    fn crear(&self, numero: i64, tipo: TipoGafete) -> Result<i64, DatabaseError> {
        let uuid = generar_uuid_v4();
        self.connection.execute(
            "INSERT INTO gafetes (numero, tipo, estado, uuid) VALUES (?1, ?2, 'DISPONIBLE', ?3)",
            params![numero, tipo.as_str_sql(), uuid],
        )?;

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "gafete", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<Gafete>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_GAFETE} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(gafete) => Ok(Some(gafete)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_numero(
        &self,
        numero: i64,
        tipo: TipoGafete,
    ) -> Result<Option<Gafete>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_GAFETE} WHERE numero = ?1 AND tipo = ?2"))?;
        match statement.query_row(params![numero, tipo.as_str_sql()], convertir_fila) {
            Ok(gafete) => Ok(Some(gafete)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn dar_de_baja(&self, id: i64) -> Result<(), DatabaseError> {
        self.connection.execute(
            "UPDATE gafetes SET estado = 'DE_BAJA' WHERE id = ?1",
            params![id],
        )?;
        encolar_actualizacion(self.connection, id)
    }

    fn marcar_perdido(&self, id: i64, portador: PortadorGafete) -> Result<(), DatabaseError> {
        match portador {
            PortadorGafete::Contratista(contratista_id) => self.connection.execute(
                "UPDATE gafetes SET estado = 'PERDIDO', contratista_portador_id = ?1 WHERE id = ?2",
                params![contratista_id, id],
            ),
            PortadorGafete::Visita(cita_visitante_id) => self.connection.execute(
                "UPDATE gafetes SET estado = 'PERDIDO', visita_portador_id = ?1 WHERE id = ?2",
                params![cita_visitante_id, id],
            ),
            PortadorGafete::ProvisionalKof(encargado_id) => self.connection.execute(
                "UPDATE gafetes SET estado = 'PERDIDO', encargado_portador_id = ?1 WHERE id = ?2",
                params![encargado_id, id],
            ),
            PortadorGafete::Proveedor(registro_ingreso_proveedor_id) => self.connection.execute(
                "UPDATE gafetes SET estado = 'PERDIDO', proveedor_portador_id = ?1 WHERE id = ?2",
                params![registro_ingreso_proveedor_id, id],
            ),
        }?;
        encolar_actualizacion(self.connection, id)
    }

    fn resolver(&self, id: i64) -> Result<(), DatabaseError> {
        self.connection.execute(
            "UPDATE gafetes SET estado = 'DISPONIBLE',
                contratista_portador_id = NULL, visita_portador_id = NULL,
                encargado_portador_id = NULL, proveedor_portador_id = NULL WHERE id = ?1",
            params![id],
        )?;
        encolar_actualizacion(self.connection, id)
    }

    fn deuda_de_contratista(&self, contratista_id: i64) -> Result<Vec<i64>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT numero FROM gafetes
             WHERE contratista_portador_id = ?1 AND estado = 'PERDIDO'
             ORDER BY numero",
        )?;
        let numeros = statement
            .query_map(params![contratista_id], |row| row.get(0))?
            .collect::<Result<Vec<i64>, _>>()?;
        Ok(numeros)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    #[test]
    fn crear_y_buscar_por_numero_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteGafeteRepository::new(&connection);

        let id = repo.crear(5, TipoGafete::Contratista).unwrap();
        let gafete = repo
            .buscar_por_numero(5, TipoGafete::Contratista)
            .unwrap()
            .unwrap();

        assert_eq!(gafete.id, id);
        assert_eq!(gafete.tipo, TipoGafete::Contratista);
        assert_eq!(gafete.estado, EstadoGafete::Disponible);
        assert_eq!(gafete.contratista_portador_id, None);
        assert_eq!(gafete.visita_portador_id, None);
    }

    #[test]
    fn marcar_perdido_y_resolver_limpian_al_portador() {
        let connection = conexion();
        connection
            .execute("INSERT INTO empresas (nombre) VALUES ('Acme')", params![])
            .unwrap();
        connection
            .execute(
                "INSERT INTO contratistas (cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso)
                 VALUES ('1', 'Juan', 1, 'PRAIND', 0, 1)",
                params![],
            )
            .unwrap();
        let repo = SqliteGafeteRepository::new(&connection);
        let id = repo.crear(1, TipoGafete::Contratista).unwrap();

        repo.marcar_perdido(id, PortadorGafete::Contratista(1))
            .unwrap();
        let perdido = repo.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(perdido.estado, EstadoGafete::Perdido);
        assert_eq!(perdido.contratista_portador_id, Some(1));
        assert_eq!(perdido.portador(), Some(PortadorGafete::Contratista(1)));
        assert_eq!(repo.deuda_de_contratista(1).unwrap(), vec![1]);

        repo.resolver(id).unwrap();
        let resuelto = repo.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(resuelto.estado, EstadoGafete::Disponible);
        assert_eq!(resuelto.contratista_portador_id, None);
        assert_eq!(resuelto.portador(), None);
        assert!(repo.deuda_de_contratista(1).unwrap().is_empty());
    }

    #[test]
    fn numero_duplicado_viola_unique_dentro_del_mismo_tipo() {
        let connection = conexion();
        let repo = SqliteGafeteRepository::new(&connection);
        repo.crear(1, TipoGafete::Contratista).unwrap();

        let error = repo.crear(1, TipoGafete::Contratista).unwrap_err();
        assert!(error.es_constraint_unique());
    }

    #[test]
    fn mismo_numero_coexiste_entre_tipos_distintos() {
        let connection = conexion();
        let repo = SqliteGafeteRepository::new(&connection);

        let id_contratista = repo.crear(7, TipoGafete::Contratista).unwrap();
        let id_visita = repo.crear(7, TipoGafete::Visita).unwrap();

        assert_ne!(id_contratista, id_visita);
        assert_eq!(
            repo.buscar_por_numero(7, TipoGafete::Contratista)
                .unwrap()
                .unwrap()
                .tipo,
            TipoGafete::Contratista
        );
        assert_eq!(
            repo.buscar_por_numero(7, TipoGafete::Visita)
                .unwrap()
                .unwrap()
                .tipo,
            TipoGafete::Visita
        );
    }

    #[test]
    fn marcar_perdido_de_gafete_de_visita_setea_el_portador_correcto() {
        let connection = conexion();
        connection
            .execute(
                "INSERT INTO citas (uuid, fecha_desde, fecha_hasta, anfitrion_nombre, anfitrion_correo, estado, creado_en)
                 VALUES ('c1', '2026-08-01', '2026-08-08', 'Ana', 'ana@acme.com', 'VIGENTE', '2026-08-01T00:00:00Z')",
                params![],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO cita_visitantes (uuid, cita_id, cedula, nombre) VALUES ('v1', 1, '1-2345', 'Jenna')",
                params![],
            )
            .unwrap();
        let cita_visitante_id = connection.last_insert_rowid();
        let repo = SqliteGafeteRepository::new(&connection);
        let id = repo.crear(9, TipoGafete::Visita).unwrap();

        repo.marcar_perdido(id, PortadorGafete::Visita(cita_visitante_id))
            .unwrap();

        let perdido = repo.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(perdido.visita_portador_id, Some(cita_visitante_id));
        assert_eq!(perdido.contratista_portador_id, None);
        assert_eq!(
            perdido.portador(),
            Some(PortadorGafete::Visita(cita_visitante_id))
        );
    }
}

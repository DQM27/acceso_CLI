//! Historial append-only de incidentes de gafetes (`docs/plan-gafetes.md`),
//! mismo patrón que `queries::auditoria`: `gafetes` guarda sólo el estado
//! vigente, acá queda el rastro de cuándo se marcó perdido y cuándo se
//! resolvió, y con qué motivo.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};

use crate::database::error::DatabaseError;
use crate::models::gafete::{MotivoResolucionGafete, PortadorGafete, TipoIncidenteGafete};
use crate::tiempo::{parsear_utc, serializar_utc};

/// Una fila del historial de un gafete puntual — `usuario_nombre`/
/// `contratista_nombre`/`visita_portador_nombre` quedan como snapshot
/// (mismo criterio que `CambioAuditado` en `queries::auditoria`), así un
/// usuario/contratista/visitante renombrado después no le hace perder
/// sentido a una fila vieja.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IncidenteGafete {
    pub id: i64,
    pub tipo: TipoIncidenteGafete,
    pub fecha_hora: DateTime<Utc>,
    pub usuario_nombre: String,
    pub contratista_nombre: Option<String>,
    pub visita_portador_nombre: Option<String>,
    pub motivo_resolucion: Option<MotivoResolucionGafete>,
    /// Número del gafete al que pertenece — no hace falta para el historial
    /// de un gafete puntual (el llamador ya lo conoce), pero es
    /// indispensable para `historial_completo` (todos los gafetes juntos).
    pub gafete_numero: i64,
}

pub trait GafetesIncidentesQuery {
    fn historial(&self, gafete_id: i64) -> Result<Vec<IncidenteGafete>, DatabaseError>;

    /// Historial de todos los gafetes junto, para la pantalla general de
    /// Auditoría (`application::catalogos::buscar_auditoria_gafetes`) — a
    /// diferencia de `historial`, sin filtrar por `gafete_id`.
    fn historial_completo(&self) -> Result<Vec<IncidenteGafete>, DatabaseError>;
}

pub trait GafetesIncidentesWriter {
    fn registrar_perdido(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        portador: PortadorGafete,
    ) -> Result<(), DatabaseError>;

    fn registrar_resuelto(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        motivo: MotivoResolucionGafete,
    ) -> Result<(), DatabaseError>;
}

pub struct SqliteGafetesIncidentes<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteGafetesIncidentes<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl GafetesIncidentesWriter for SqliteGafetesIncidentes<'_> {
    fn registrar_perdido(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        portador: PortadorGafete,
    ) -> Result<(), DatabaseError> {
        match portador {
            PortadorGafete::Contratista(contratista_id) => self.connection.execute(
                "INSERT INTO gafetes_incidentes (gafete_id, tipo, fecha_hora, usuario_id, contratista_id)
                 VALUES (?1, 'PERDIDO', ?2, ?3, ?4)",
                params![gafete_id, serializar_utc(fecha_hora), usuario_id, contratista_id],
            ),
            PortadorGafete::Visita(cita_visitante_id) => self.connection.execute(
                "INSERT INTO gafetes_incidentes (gafete_id, tipo, fecha_hora, usuario_id, visita_portador_id)
                 VALUES (?1, 'PERDIDO', ?2, ?3, ?4)",
                params![gafete_id, serializar_utc(fecha_hora), usuario_id, cita_visitante_id],
            ),
            PortadorGafete::ProvisionalKof(encargado_id) => self.connection.execute(
                "INSERT INTO gafetes_incidentes (gafete_id, tipo, fecha_hora, usuario_id, encargado_portador_id)
                 VALUES (?1, 'PERDIDO', ?2, ?3, ?4)",
                params![gafete_id, serializar_utc(fecha_hora), usuario_id, encargado_id],
            ),
        }?;
        Ok(())
    }

    fn registrar_resuelto(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        motivo: MotivoResolucionGafete,
    ) -> Result<(), DatabaseError> {
        self.connection.execute(
            "INSERT INTO gafetes_incidentes (gafete_id, tipo, fecha_hora, usuario_id, motivo_resolucion)
             VALUES (?1, 'RESUELTO', ?2, ?3, ?4)",
            params![
                gafete_id,
                serializar_utc(fecha_hora),
                usuario_id,
                motivo.as_str_sql()
            ],
        )?;
        Ok(())
    }
}

impl GafetesIncidentesQuery for SqliteGafetesIncidentes<'_> {
    fn historial(&self, gafete_id: i64) -> Result<Vec<IncidenteGafete>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT gi.id, gi.tipo, gi.fecha_hora, u.nombre, c.nombre, cv.nombre, gi.motivo_resolucion, g.numero
             FROM gafetes_incidentes gi
             JOIN gafetes g ON g.id = gi.gafete_id
             JOIN usuarios u ON u.id = gi.usuario_id
             LEFT JOIN contratistas c ON c.id = gi.contratista_id
             LEFT JOIN cita_visitantes cv ON cv.id = gi.visita_portador_id
             WHERE gi.gafete_id = ?1
             ORDER BY gi.id DESC",
        )?;
        Ok(statement
            .query_map(params![gafete_id], convertir_fila_incidente)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn historial_completo(&self) -> Result<Vec<IncidenteGafete>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT gi.id, gi.tipo, gi.fecha_hora, u.nombre, c.nombre, cv.nombre, gi.motivo_resolucion, g.numero
             FROM gafetes_incidentes gi
             JOIN gafetes g ON g.id = gi.gafete_id
             JOIN usuarios u ON u.id = gi.usuario_id
             LEFT JOIN contratistas c ON c.id = gi.contratista_id
             LEFT JOIN cita_visitantes cv ON cv.id = gi.visita_portador_id
             ORDER BY gi.fecha_hora DESC, gi.id DESC",
        )?;
        Ok(statement
            .query_map([], convertir_fila_incidente)?
            .collect::<Result<Vec<_>, _>>()?)
    }
}

fn convertir_fila_incidente(row: &Row<'_>) -> rusqlite::Result<IncidenteGafete> {
    let tipo_texto: String = row.get(1)?;
    let Some(tipo) = TipoIncidenteGafete::from_str_sql(&tipo_texto) else {
        return Err(rusqlite::Error::InvalidColumnType(
            1,
            "tipo".to_string(),
            rusqlite::types::Type::Text,
        ));
    };
    let fecha_texto: String = row.get(2)?;
    let fecha_hora = parsear_utc(&fecha_texto).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let motivo_texto: Option<String> = row.get(6)?;
    let motivo_resolucion = motivo_texto
        .map(|texto| {
            MotivoResolucionGafete::from_str_sql(&texto).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    format!("motivo_resolucion desconocido: {texto}").into(),
                )
            })
        })
        .transpose()?;

    Ok(IncidenteGafete {
        id: row.get(0)?,
        tipo,
        fecha_hora,
        usuario_nombre: row.get(3)?,
        contratista_nombre: row.get(4)?,
        visita_portador_nombre: row.get(5)?,
        motivo_resolucion,
        gafete_numero: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;
    use rusqlite::Connection;

    fn conexion_con_gafete_y_partes() -> (Connection, i64) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO empresas (nombre) VALUES ('Acme');
                 INSERT INTO contratistas (cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso)
                 VALUES ('1', 'Juan', 1, 'PRAIND', 0, 1);
                 INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
                 VALUES ('9', 'Root', 'hash', 'ROOT', 1);
                 INSERT INTO gafetes (numero, tipo, estado) VALUES (1, 'CONTRATISTA', 'DISPONIBLE');",
            )
            .unwrap();
        let gafete_id: i64 = connection
            .query_row("SELECT id FROM gafetes WHERE numero = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        (connection, gafete_id)
    }

    #[test]
    fn registra_perdido_y_resuelto_como_filas_distintas() {
        let (connection, gafete_id) = conexion_con_gafete_y_partes();
        let writer = SqliteGafetesIncidentes::new(&connection);
        let ahora = Utc::now();

        writer
            .registrar_perdido(gafete_id, ahora, 1, PortadorGafete::Contratista(1))
            .unwrap();
        writer
            .registrar_resuelto(gafete_id, ahora, 1, MotivoResolucionGafete::Aparecido)
            .unwrap();

        let total: i64 = connection
            .query_row("SELECT COUNT(*) FROM gafetes_incidentes", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, 2);
    }

    #[test]
    fn historial_trae_perdido_y_resuelto_mas_reciente_primero() {
        let (connection, gafete_id) = conexion_con_gafete_y_partes();
        let writer = SqliteGafetesIncidentes::new(&connection);
        let ahora = Utc::now();

        writer
            .registrar_perdido(gafete_id, ahora, 1, PortadorGafete::Contratista(1))
            .unwrap();
        writer
            .registrar_resuelto(gafete_id, ahora, 1, MotivoResolucionGafete::Aparecido)
            .unwrap();

        let historial = writer.historial(gafete_id).unwrap();

        assert_eq!(historial.len(), 2);
        assert_eq!(historial[0].tipo, TipoIncidenteGafete::Resuelto);
        assert_eq!(historial[0].usuario_nombre, "Root");
        assert_eq!(
            historial[0].motivo_resolucion,
            Some(MotivoResolucionGafete::Aparecido)
        );
        assert_eq!(historial[0].contratista_nombre, None);
        assert_eq!(historial[0].gafete_numero, 1);
        assert_eq!(historial[1].tipo, TipoIncidenteGafete::Perdido);
        assert_eq!(historial[1].contratista_nombre, Some("Juan".to_string()));
        assert_eq!(historial[1].motivo_resolucion, None);
        assert_eq!(historial[1].gafete_numero, 1);
    }

    #[test]
    fn historial_completo_trae_incidentes_de_todos_los_gafetes() {
        let (connection, gafete_id) = conexion_con_gafete_y_partes();
        connection
            .execute_batch("INSERT INTO gafetes (numero, tipo, estado) VALUES (2, 'CONTRATISTA', 'DISPONIBLE')")
            .unwrap();
        let otro_gafete_id: i64 = connection
            .query_row("SELECT id FROM gafetes WHERE numero = 2", [], |row| {
                row.get(0)
            })
            .unwrap();
        let writer = SqliteGafetesIncidentes::new(&connection);
        let ahora = Utc::now();

        writer
            .registrar_perdido(gafete_id, ahora, 1, PortadorGafete::Contratista(1))
            .unwrap();
        writer
            .registrar_perdido(otro_gafete_id, ahora, 1, PortadorGafete::Contratista(1))
            .unwrap();

        let historial = writer.historial_completo().unwrap();

        assert_eq!(historial.len(), 2);
        assert_eq!(
            historial
                .iter()
                .map(|i| i.gafete_numero)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
    }

    #[test]
    fn registra_perdido_de_gafete_de_visita_con_portador_visita() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
                 VALUES ('9', 'Root', 'hash', 'ROOT', 1);
                 INSERT INTO citas (uuid, fecha_desde, fecha_hasta, anfitrion_nombre, anfitrion_correo, estado, creado_en)
                 VALUES ('c1', '2026-08-01', '2026-08-08', 'Ana', 'ana@acme.com', 'VIGENTE', '2026-08-01T00:00:00Z');
                 INSERT INTO cita_visitantes (uuid, cita_id, cedula, nombre) VALUES ('v1', 1, '1-2345', 'Jenna');
                 INSERT INTO gafetes (numero, tipo, estado) VALUES (9, 'VISITA', 'DISPONIBLE');",
            )
            .unwrap();
        let gafete_id: i64 = connection
            .query_row("SELECT id FROM gafetes WHERE numero = 9", [], |row| {
                row.get(0)
            })
            .unwrap();
        let writer = SqliteGafetesIncidentes::new(&connection);

        writer
            .registrar_perdido(gafete_id, Utc::now(), 1, PortadorGafete::Visita(1))
            .unwrap();

        let historial = writer.historial(gafete_id).unwrap();
        assert_eq!(historial.len(), 1);
        assert_eq!(
            historial[0].visita_portador_nombre,
            Some("Jenna".to_string())
        );
        assert_eq!(historial[0].contratista_nombre, None);
    }
}

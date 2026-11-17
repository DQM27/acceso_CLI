//! Historial append-only de reportes de gafetes, mismo patrón que
//! `queries::auditoria`: `gafetes` guarda sólo el estado vigente, acá queda
//! el rastro de cada reporte (perdido, pagado, recuperado), quién lo hizo,
//! cuándo y qué escribió.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};

use crate::database::error::DatabaseError;
use crate::models::gafete::{MotivoResolucionGafete, PortadorGafete, TipoIncidenteGafete};
use crate::tiempo::{parsear_utc, serializar_utc};

/// Una fila del historial de un gafete — `usuario_nombre`/
/// `responsable_nombre` se leen por `JOIN` contra la persona actual.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IncidenteGafete {
    pub id: i64,
    pub tipo: TipoIncidenteGafete,
    pub fecha_hora: DateTime<Utc>,
    /// Quién hizo el reporte: responde por lo que escribió.
    pub usuario_nombre: String,
    /// La persona marcada (`Perdido`) o a la que se le quitó la marca
    /// (`Pagado`), de cualquier tipo de gafete. `None` si no se sabe quién
    /// lo tenía, o en los reportes que no llevan responsable.
    pub responsable_nombre: Option<String>,
    /// Texto libre de quien reportó.
    pub detalle: Option<String>,
    /// Sólo en las filas `Resuelto`, anteriores a `MIGRACION_65`.
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
        portador: Option<PortadorGafete>,
        detalle: Option<&str>,
    ) -> Result<(), DatabaseError>;

    fn registrar_pagado(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        portador: PortadorGafete,
        detalle: Option<&str>,
    ) -> Result<(), DatabaseError>;

    fn registrar_recuperado(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        detalle: &str,
    ) -> Result<(), DatabaseError>;
}

pub struct SqliteGafetesIncidentes<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteGafetesIncidentes<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    /// Un reporte con su responsable (si hay) en la columna que le toca.
    fn insertar(
        &self,
        gafete_id: i64,
        tipo: TipoIncidenteGafete,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        portador: Option<PortadorGafete>,
        detalle: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let (mut contratista, mut visita, mut encargado, mut proveedor) = (None, None, None, None);
        match portador {
            Some(PortadorGafete::Contratista(id)) => contratista = Some(id),
            Some(PortadorGafete::Visita(id)) => visita = Some(id),
            Some(PortadorGafete::ProvisionalKof(id)) => encargado = Some(id),
            Some(PortadorGafete::Proveedor(id)) => proveedor = Some(id),
            None => {}
        }
        self.connection.execute(
            "INSERT INTO gafetes_incidentes (
                gafete_id, tipo, fecha_hora, usuario_id, contratista_id, visita_portador_id,
                encargado_portador_id, proveedor_portador_id, detalle
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                gafete_id,
                tipo.as_str_sql(),
                serializar_utc(fecha_hora),
                usuario_id,
                contratista,
                visita,
                encargado,
                proveedor,
                detalle
            ],
        )?;
        Ok(())
    }
}

impl GafetesIncidentesWriter for SqliteGafetesIncidentes<'_> {
    fn registrar_perdido(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        portador: Option<PortadorGafete>,
        detalle: Option<&str>,
    ) -> Result<(), DatabaseError> {
        self.insertar(
            gafete_id,
            TipoIncidenteGafete::Perdido,
            fecha_hora,
            usuario_id,
            portador,
            detalle,
        )
    }

    fn registrar_pagado(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        portador: PortadorGafete,
        detalle: Option<&str>,
    ) -> Result<(), DatabaseError> {
        self.insertar(
            gafete_id,
            TipoIncidenteGafete::Pagado,
            fecha_hora,
            usuario_id,
            Some(portador),
            detalle,
        )
    }

    fn registrar_recuperado(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        detalle: &str,
    ) -> Result<(), DatabaseError> {
        self.insertar(
            gafete_id,
            TipoIncidenteGafete::Recuperado,
            fecha_hora,
            usuario_id,
            None,
            Some(detalle),
        )
    }
}

const SELECT_INCIDENTES: &str = "
    SELECT gi.id, gi.tipo, gi.fecha_hora, u.nombre,
           COALESCE(c.nombre, cv.nombre, er.nombre, rp.nombre),
           gi.detalle, gi.motivo_resolucion, g.numero
    FROM gafetes_incidentes gi
    JOIN gafetes g ON g.id = gi.gafete_id
    JOIN usuarios u ON u.id = gi.usuario_id
    LEFT JOIN contratistas c ON c.id = gi.contratista_id
    LEFT JOIN cita_visitantes cv ON cv.id = gi.visita_portador_id
    LEFT JOIN encargados_ruta er ON er.id = gi.encargado_portador_id
    LEFT JOIN registro_ingresos_proveedor rp ON rp.id = gi.proveedor_portador_id";

impl GafetesIncidentesQuery for SqliteGafetesIncidentes<'_> {
    fn historial(&self, gafete_id: i64) -> Result<Vec<IncidenteGafete>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_INCIDENTES} WHERE gi.gafete_id = ?1 ORDER BY gi.id DESC"
        ))?;
        Ok(statement
            .query_map(params![gafete_id], convertir_fila_incidente)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn historial_completo(&self) -> Result<Vec<IncidenteGafete>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_INCIDENTES} ORDER BY gi.fecha_hora DESC, gi.id DESC"
        ))?;
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
        responsable_nombre: row.get(4)?,
        detalle: row.get(5)?,
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
    fn historial_trae_cada_reporte_mas_reciente_primero() {
        let (connection, gafete_id) = conexion_con_gafete_y_partes();
        let writer = SqliteGafetesIncidentes::new(&connection);
        let ahora = Utc::now();

        writer
            .registrar_perdido(
                gafete_id,
                ahora,
                1,
                Some(PortadorGafete::Contratista(1)),
                None,
            )
            .unwrap();
        writer
            .registrar_pagado(
                gafete_id,
                ahora,
                1,
                PortadorGafete::Contratista(1),
                Some("Recibo 9"),
            )
            .unwrap();
        writer
            .registrar_recuperado(gafete_id, ahora, 1, "Llegó reimpreso")
            .unwrap();

        let historial = writer.historial(gafete_id).unwrap();

        assert_eq!(historial.len(), 3);
        assert_eq!(historial[0].tipo, TipoIncidenteGafete::Recuperado);
        assert_eq!(historial[0].detalle.as_deref(), Some("Llegó reimpreso"));
        assert_eq!(historial[0].responsable_nombre, None);
        assert_eq!(historial[0].usuario_nombre, "Root");
        assert_eq!(historial[1].tipo, TipoIncidenteGafete::Pagado);
        assert_eq!(historial[1].responsable_nombre.as_deref(), Some("Juan"));
        assert_eq!(historial[1].detalle.as_deref(), Some("Recibo 9"));
        assert_eq!(historial[2].tipo, TipoIncidenteGafete::Perdido);
        assert_eq!(historial[2].responsable_nombre.as_deref(), Some("Juan"));
        assert_eq!(historial[2].gafete_numero, 1);
    }

    #[test]
    fn un_perdido_sin_responsable_y_sin_detalle_lo_rechaza_la_base() {
        let (connection, gafete_id) = conexion_con_gafete_y_partes();
        let writer = SqliteGafetesIncidentes::new(&connection);

        assert!(
            writer
                .registrar_perdido(gafete_id, Utc::now(), 1, None, None)
                .is_err()
        );
        writer
            .registrar_perdido(gafete_id, Utc::now(), 1, None, Some("No estaba al recibir"))
            .unwrap();
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
            .registrar_perdido(
                gafete_id,
                ahora,
                1,
                Some(PortadorGafete::Contratista(1)),
                None,
            )
            .unwrap();
        writer
            .registrar_perdido(otro_gafete_id, ahora, 1, None, Some("no está"))
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
            .registrar_perdido(
                gafete_id,
                Utc::now(),
                1,
                Some(PortadorGafete::Visita(1)),
                None,
            )
            .unwrap();

        let historial = writer.historial(gafete_id).unwrap();
        assert_eq!(historial.len(), 1);
        assert_eq!(historial[0].responsable_nombre.as_deref(), Some("Jenna"));
    }
}

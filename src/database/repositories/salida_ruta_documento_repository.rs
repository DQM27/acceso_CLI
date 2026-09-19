use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::domain::resultado_salida_ruta::ResultadoSalidaRuta;
use crate::models::salida_ruta_documento::SalidaRutaDocumento;

fn resultado_a_texto(resultado: ResultadoSalidaRuta) -> (&'static str, Option<&'static str>) {
    match resultado {
        ResultadoSalidaRuta::Permitido => ("PERMITIDO", None),
        ResultadoSalidaRuta::PermitidoConAutorizacion => (
            "PERMITIDO_CON_AUTORIZACION",
            Some("DOCUMENTO_FECHA_DISTINTA"),
        ),
    }
}

fn resultado_de_texto(texto: &str) -> Option<ResultadoSalidaRuta> {
    match texto {
        "PERMITIDO" => Some(ResultadoSalidaRuta::Permitido),
        "PERMITIDO_CON_AUTORIZACION" => Some(ResultadoSalidaRuta::PermitidoConAutorizacion),
        _ => None,
    }
}

/// El vínculo N a N entre tramo y documento -- ver
/// `docs/planes-implementados/plan-control-rutas.md`, sección "Rediseño
/// del núcleo de rutas -- documento/tramo/viaje". Sin `actualizar` ni
/// `eliminar` a propósito -- inmutable una vez creado, la base lo
/// garantiza con triggers propios.
pub trait SalidaRutaDocumentoRepository {
    /// Crea el vínculo con su veredicto de fecha ya evaluado (por
    /// `RutaService`, contra la fecha del tramo al que se está
    /// vinculando -- ver el doc-comment de `SalidaRutaDocumento`).
    fn vincular(
        &self,
        salida_id: i64,
        documento_id: i64,
        resultado: ResultadoSalidaRuta,
    ) -> Result<(), DatabaseError>;

    fn listar_por_salida(&self, salida_id: i64) -> Result<Vec<SalidaRutaDocumento>, DatabaseError>;

    /// Para que `RutaService` sepa si un documento ya tiene entregas
    /// previas en otros tramos (recarga) antes de decidir si reusarlo.
    fn listar_por_documento(
        &self,
        documento_id: i64,
    ) -> Result<Vec<SalidaRutaDocumento>, DatabaseError>;

    /// Usada por `RutaCatalogoService::dar_de_baja` -- una ruta con un
    /// documento vinculado a un tramo todavía abierto no puede darse de
    /// baja, mismo criterio que `GafeteRepository`/
    /// `buscar_ingreso_activo_por_gafete`. Reemplaza a
    /// `SalidaRutaRepository::buscar_activa_por_ruta` del modelo viejo --
    /// ahora el número de ruta vive en `documentos_ruta`, no en
    /// `salidas_ruta`, así que hace falta el join de las 3 tablas.
    fn ruta_tiene_documento_en_tramo_activo(&self, ruta_id: i64) -> Result<bool, DatabaseError>;
}

pub struct SqliteSalidaRutaDocumentoRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteSalidaRutaDocumentoRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<SalidaRutaDocumento> {
    let resultado_texto: String = row.get(2)?;
    let resultado = resultado_de_texto(&resultado_texto).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(2, "resultado".to_string(), rusqlite::types::Type::Text)
    })?;

    Ok(SalidaRutaDocumento {
        salida_id: row.get(0)?,
        documento_id: row.get(1)?,
        resultado,
    })
}

const SELECT_VINCULO: &str =
    "SELECT salida_id, documento_id, resultado FROM salida_ruta_documentos";

impl SalidaRutaDocumentoRepository for SqliteSalidaRutaDocumentoRepository<'_> {
    fn vincular(
        &self,
        salida_id: i64,
        documento_id: i64,
        resultado: ResultadoSalidaRuta,
    ) -> Result<(), DatabaseError> {
        let (resultado_texto, motivo_resultado) = resultado_a_texto(resultado);
        let uuid = generar_uuid_v4();

        self.connection.execute(
            "
            INSERT INTO salida_ruta_documentos (
                salida_id, documento_id, resultado, motivo_resultado, uuid
            )
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                salida_id,
                documento_id,
                resultado_texto,
                motivo_resultado,
                uuid
            ],
        )?;

        cola_salida::encolar(self.connection, "salida_ruta_documento", &uuid, "crear")?;

        Ok(())
    }

    fn listar_por_salida(&self, salida_id: i64) -> Result<Vec<SalidaRutaDocumento>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_VINCULO} WHERE salida_id = ?1"))?;
        let vinculos = statement
            .query_map(params![salida_id], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(vinculos)
    }

    fn listar_por_documento(
        &self,
        documento_id: i64,
    ) -> Result<Vec<SalidaRutaDocumento>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_VINCULO} WHERE documento_id = ?1"))?;
        let vinculos = statement
            .query_map(params![documento_id], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(vinculos)
    }

    fn ruta_tiene_documento_en_tramo_activo(&self, ruta_id: i64) -> Result<bool, DatabaseError> {
        self.connection
            .prepare(
                "
                SELECT 1
                FROM salida_ruta_documentos srd
                JOIN documentos_ruta d ON d.id = srd.documento_id
                JOIN salidas_ruta s ON s.id = srd.salida_id
                WHERE d.ruta_id = ?1 AND s.fecha_hora_retorno IS NULL
                LIMIT 1
                ",
            )?
            .exists(params![ruta_id])
            .map_err(DatabaseError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::documento_ruta_repository::{
        DocumentoRutaRepository, SqliteDocumentoRutaRepository,
    };
    use crate::database::repositories::viaje_ruta_repository::{
        SqliteViajeRutaRepository, ViajeRutaRepository,
    };
    use crate::database::schema::initialize_database;
    use crate::models::documento_ruta::NuevoDocumentoRuta;
    use crate::models::viaje_ruta::NuevoViajeRuta;
    use chrono::{TimeZone, Utc};

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        connection
    }

    fn crear_salida(connection: &Connection, viaje_id: i64) -> i64 {
        connection
            .execute(
                "INSERT INTO salidas_ruta (
                    viaje_id, vehiculo_placa, encargado_nombre,
                    fecha_hora_salida, usuario_salida_id, usuario_salida_nombre, uuid
                ) VALUES (?1, 'C12345', 'Carlos Mendez', '2026-09-19T08:00:00Z', 1, 'Operador', ?2)",
                params![viaje_id, generar_uuid_v4()],
            )
            .unwrap();
        connection.last_insert_rowid()
    }

    #[test]
    fn vincular_y_listar_por_salida_redondea_el_viaje() {
        let connection = conexion();
        let viaje_id = SqliteViajeRutaRepository::new(&connection)
            .crear(&NuevoViajeRuta {
                vehiculo_id: None,
                vehiculo_placa: "C12345".to_string(),
                vehiculo_numero_unidad: None,
                encargado_id: None,
                encargado_nombre: "Carlos Mendez".to_string(),
                fecha_hora_creacion: Utc.with_ymd_and_hms(2026, 9, 19, 8, 0, 0).unwrap(),
                usuario_creacion_id: 1,
            })
            .unwrap();
        let salida_id = crear_salida(&connection, viaje_id);
        let documento_id = SqliteDocumentoRutaRepository::new(&connection)
            .crear(
                &NuevoDocumentoRuta {
                    numero_documento: "700101452".to_string(),
                    ruta_id: None,
                    sub_numero: 1,
                    fecha_documento: "2026-09-19".parse().unwrap(),
                },
                Utc.with_ymd_and_hms(2026, 9, 19, 8, 0, 0).unwrap(),
            )
            .unwrap();
        let repo = SqliteSalidaRutaDocumentoRepository::new(&connection);

        repo.vincular(salida_id, documento_id, ResultadoSalidaRuta::Permitido)
            .unwrap();

        let vinculos = repo.listar_por_salida(salida_id).unwrap();
        assert_eq!(vinculos.len(), 1);
        assert_eq!(vinculos[0].documento_id, documento_id);
        assert_eq!(vinculos[0].resultado, ResultadoSalidaRuta::Permitido);
    }

    #[test]
    fn un_documento_puede_vincularse_a_dos_salidas_distintas() {
        // Caso real de la recarga: el mismo documento se reparte en 2
        // tramos porque la carga no cupo en un solo viaje.
        let connection = conexion();
        let viaje_id = SqliteViajeRutaRepository::new(&connection)
            .crear(&NuevoViajeRuta {
                vehiculo_id: None,
                vehiculo_placa: "C12345".to_string(),
                vehiculo_numero_unidad: None,
                encargado_id: None,
                encargado_nombre: "Carlos Mendez".to_string(),
                fecha_hora_creacion: Utc.with_ymd_and_hms(2026, 9, 19, 8, 0, 0).unwrap(),
                usuario_creacion_id: 1,
            })
            .unwrap();
        let salida_1 = crear_salida(&connection, viaje_id);
        // Cierra la primera antes de abrir la segunda -- el índice único
        // "un vehículo no puede tener dos salidas abiertas a la vez" no
        // distingue documento repartido de cualquier otro caso.
        connection
            .execute(
                "UPDATE salidas_ruta SET fecha_hora_retorno = '2026-09-19T09:00:00Z',
                 usuario_retorno_id = 1, usuario_retorno_nombre = 'Operador' WHERE id = ?1",
                params![salida_1],
            )
            .unwrap();
        let salida_2 = crear_salida(&connection, viaje_id);
        let documento_id = SqliteDocumentoRutaRepository::new(&connection)
            .crear(
                &NuevoDocumentoRuta {
                    numero_documento: "700101452".to_string(),
                    ruta_id: None,
                    sub_numero: 1,
                    fecha_documento: "2026-09-19".parse().unwrap(),
                },
                Utc.with_ymd_and_hms(2026, 9, 19, 8, 0, 0).unwrap(),
            )
            .unwrap();
        let repo = SqliteSalidaRutaDocumentoRepository::new(&connection);

        repo.vincular(salida_1, documento_id, ResultadoSalidaRuta::Permitido)
            .unwrap();
        repo.vincular(salida_2, documento_id, ResultadoSalidaRuta::Permitido)
            .unwrap();

        assert_eq!(repo.listar_por_documento(documento_id).unwrap().len(), 2);
    }

    #[test]
    fn ruta_tiene_documento_en_tramo_activo_detecta_el_vinculo() {
        let connection = conexion();
        let ruta_id = {
            connection
                .execute(
                    "INSERT INTO rutas (numero, activo, uuid) VALUES (79, 1, ?1)",
                    params![generar_uuid_v4()],
                )
                .unwrap();
            connection.last_insert_rowid()
        };
        let viaje_id = SqliteViajeRutaRepository::new(&connection)
            .crear(&NuevoViajeRuta {
                vehiculo_id: None,
                vehiculo_placa: "C12345".to_string(),
                vehiculo_numero_unidad: None,
                encargado_id: None,
                encargado_nombre: "Carlos Mendez".to_string(),
                fecha_hora_creacion: Utc.with_ymd_and_hms(2026, 9, 19, 8, 0, 0).unwrap(),
                usuario_creacion_id: 1,
            })
            .unwrap();
        let salida_id = crear_salida(&connection, viaje_id);
        let documento_id = SqliteDocumentoRutaRepository::new(&connection)
            .crear(
                &NuevoDocumentoRuta {
                    numero_documento: "700101452".to_string(),
                    ruta_id: Some(ruta_id),
                    sub_numero: 1,
                    fecha_documento: "2026-09-19".parse().unwrap(),
                },
                Utc.with_ymd_and_hms(2026, 9, 19, 8, 0, 0).unwrap(),
            )
            .unwrap();
        let repo = SqliteSalidaRutaDocumentoRepository::new(&connection);
        repo.vincular(salida_id, documento_id, ResultadoSalidaRuta::Permitido)
            .unwrap();

        assert!(repo.ruta_tiene_documento_en_tramo_activo(ruta_id).unwrap());

        connection
            .execute(
                "UPDATE salidas_ruta SET fecha_hora_retorno = '2026-09-19T10:00:00Z',
                 usuario_retorno_id = 1, usuario_retorno_nombre = 'Operador' WHERE id = ?1",
                params![salida_id],
            )
            .unwrap();

        assert!(!repo.ruta_tiene_documento_en_tramo_activo(ruta_id).unwrap());
    }
}

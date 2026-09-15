use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, Row, named_params, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::domain::resultado_salida_ruta::ResultadoSalidaRuta;
use crate::models::salida_ruta::{
    NuevaSalidaRuta, RetornoSalidaRuta, SalidaRuta, SalidaRutaActivaResumen,
};
use crate::tiempo::{parsear_utc, serializar_utc};

pub trait SalidaRutaRepository {
    fn crear(&self, salida: &NuevaSalidaRuta) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<SalidaRuta>, DatabaseError>;

    /// Mismo motivo que `RegistroIngresoRepository::buscar_ingreso_activo`:
    /// evitar que un vehículo quede con dos salidas abiertas a la vez.
    /// Por placa (texto), no por `vehiculo_id` -- el índice único de la
    /// base (`idx_salidas_ruta_placa_activa`) tampoco depende de que haya
    /// match de catálogo.
    fn buscar_activa_por_placa(&self, placa: &str) -> Result<Option<SalidaRuta>, DatabaseError>;

    fn buscar_por_numero_documento(
        &self,
        numero_documento: &str,
    ) -> Result<Option<SalidaRuta>, DatabaseError>;

    fn registrar_retorno(
        &self,
        id: i64,
        fecha_hora_retorno: DateTime<Utc>,
        usuario_retorno_id: i64,
    ) -> Result<(), DatabaseError>;

    /// Fila aplanada para la pantalla "Rutas activas" -- análoga a
    /// `RegistroIngresoRepository`/`MovimientoVisitaRepository::listar_activos`.
    fn listar_activas(&self) -> Result<Vec<SalidaRutaActivaResumen>, DatabaseError>;
}

fn resultado_a_texto(resultado: ResultadoSalidaRuta) -> (&'static str, Option<&'static str>) {
    match resultado {
        ResultadoSalidaRuta::Permitido => ("PERMITIDO", None),
        ResultadoSalidaRuta::PermitidoConAutorizacion => (
            "PERMITIDO_CON_AUTORIZACION",
            Some("DOCUMENTO_FECHA_DISTINTA"),
        ),
    }
}

pub struct SqliteSalidaRutaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteSalidaRutaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<SalidaRuta> {
    let fecha_documento_texto: String = row.get(9)?;
    let fecha_documento =
        NaiveDate::parse_from_str(&fecha_documento_texto, "%Y-%m-%d").map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                9,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;

    let resultado_texto: String = row.get(10)?;
    let resultado = match resultado_texto.as_str() {
        "PERMITIDO" => ResultadoSalidaRuta::Permitido,
        "PERMITIDO_CON_AUTORIZACION" => ResultadoSalidaRuta::PermitidoConAutorizacion,
        _ => {
            return Err(rusqlite::Error::InvalidColumnType(
                10,
                "resultado".to_string(),
                rusqlite::types::Type::Text,
            ));
        }
    };

    let fecha_hora_salida_texto: String = row.get(11)?;
    let fecha_hora_salida = parsear_utc(&fecha_hora_salida_texto).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(11, rusqlite::types::Type::Text, Box::new(error))
    })?;

    let fecha_hora_retorno_texto: Option<String> = row.get(13)?;
    let fecha_hora_retorno = fecha_hora_retorno_texto
        .map(|fecha| {
            parsear_utc(&fecha).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    13,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;
    let usuario_retorno_id: Option<i64> = row.get(14)?;
    // `CHECK (fecha_hora_retorno IS NULL) = (usuario_retorno_id IS NULL)` en
    // el esquema (MIGRACION_36) garantiza que ambos vienen juntos o ninguno.
    let retorno = fecha_hora_retorno
        .zip(usuario_retorno_id)
        .map(|(fecha_hora, usuario_id)| RetornoSalidaRuta {
            fecha_hora,
            usuario_id,
        });

    Ok(SalidaRuta {
        id: row.get(0)?,
        vehiculo_id: row.get(1)?,
        vehiculo_placa: row.get(2)?,
        vehiculo_numero_unidad: row.get(3)?,
        encargado_id: row.get(4)?,
        encargado_nombre: row.get(5)?,
        numero_ruta: row.get(6)?,
        sub_numero: row.get(7)?,
        numero_documento: row.get(8)?,
        fecha_documento,
        resultado,
        fecha_hora_salida,
        usuario_salida_id: row.get(12)?,
        retorno,
    })
}

const SELECT_SALIDA: &str = "
    SELECT id, vehiculo_id, vehiculo_placa, vehiculo_numero_unidad,
           encargado_id, encargado_nombre, numero_ruta, sub_numero,
           numero_documento, fecha_documento, resultado,
           fecha_hora_salida, usuario_salida_id,
           fecha_hora_retorno, usuario_retorno_id
    FROM salidas_ruta
";

impl SalidaRutaRepository for SqliteSalidaRutaRepository<'_> {
    fn crear(&self, salida: &NuevaSalidaRuta) -> Result<i64, DatabaseError> {
        let fecha_documento = salida.fecha_documento.format("%Y-%m-%d").to_string();
        let fecha_hora_salida = serializar_utc(salida.fecha_hora_salida);
        let (resultado, motivo_resultado) = resultado_a_texto(salida.resultado);
        let uuid = generar_uuid_v4();

        let filas = self.connection.execute(
            "
            INSERT INTO salidas_ruta (
                vehiculo_id, vehiculo_placa, vehiculo_numero_unidad,
                encargado_id, encargado_nombre,
                numero_ruta, sub_numero, numero_documento, fecha_documento,
                resultado, motivo_resultado,
                fecha_hora_salida, usuario_salida_id, usuario_salida_nombre,
                uuid
            )
            SELECT
                :vehiculo_id, :vehiculo_placa, :vehiculo_numero_unidad,
                :encargado_id, :encargado_nombre,
                :numero_ruta, :sub_numero, :numero_documento, :fecha_documento,
                :resultado, :motivo_resultado,
                :fecha_hora_salida, :usuario_salida_id, u.nombre,
                :uuid
            FROM usuarios AS u
            WHERE u.id = :usuario_salida_id
            ",
            named_params! {
                ":vehiculo_id": salida.vehiculo_id,
                ":vehiculo_placa": salida.vehiculo_placa,
                ":vehiculo_numero_unidad": salida.vehiculo_numero_unidad,
                ":encargado_id": salida.encargado_id,
                ":encargado_nombre": salida.encargado_nombre,
                ":numero_ruta": salida.numero_ruta,
                ":sub_numero": salida.sub_numero,
                ":numero_documento": salida.numero_documento,
                ":fecha_documento": fecha_documento,
                ":resultado": resultado,
                ":motivo_resultado": motivo_resultado,
                ":fecha_hora_salida": fecha_hora_salida,
                ":usuario_salida_id": salida.usuario_salida_id,
                ":uuid": uuid,
            },
        )?;

        if filas == 0 {
            return Err(DatabaseError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "salida_ruta", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<SalidaRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_SALIDA} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(salida) => Ok(Some(salida)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_activa_por_placa(&self, placa: &str) -> Result<Option<SalidaRuta>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_SALIDA} WHERE vehiculo_placa = ?1 AND fecha_hora_retorno IS NULL
             ORDER BY fecha_hora_salida DESC LIMIT 1"
        ))?;
        match statement.query_row(params![placa], convertir_fila) {
            Ok(salida) => Ok(Some(salida)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_numero_documento(
        &self,
        numero_documento: &str,
    ) -> Result<Option<SalidaRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_SALIDA} WHERE numero_documento = ?1"))?;
        match statement.query_row(params![numero_documento], convertir_fila) {
            Ok(salida) => Ok(Some(salida)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn registrar_retorno(
        &self,
        id: i64,
        fecha_hora_retorno: DateTime<Utc>,
        usuario_retorno_id: i64,
    ) -> Result<(), DatabaseError> {
        let fecha_hora_retorno = serializar_utc(fecha_hora_retorno);

        let filas_afectadas = self.connection.execute(
            "
            UPDATE salidas_ruta
            SET
                fecha_hora_retorno = ?1,
                usuario_retorno_id = ?2,
                usuario_retorno_nombre = (SELECT nombre FROM usuarios WHERE id = ?2)
            WHERE id = ?3
              AND fecha_hora_retorno IS NULL
            ",
            params![fecha_hora_retorno, usuario_retorno_id, id],
        )?;

        if filas_afectadas == 0 {
            return Err(DatabaseError::SalidaRutaNoActiva);
        }

        let uuid: String = self.connection.query_row(
            "SELECT uuid FROM salidas_ruta WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        cola_salida::encolar(self.connection, "salida_ruta", &uuid, "cerrar")?;

        Ok(())
    }

    fn listar_activas(&self) -> Result<Vec<SalidaRutaActivaResumen>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id, vehiculo_placa, vehiculo_numero_unidad, encargado_nombre,
                numero_ruta, sub_numero, numero_documento, fecha_documento,
                resultado, fecha_hora_salida, usuario_salida_nombre
            FROM salidas_ruta
            WHERE fecha_hora_retorno IS NULL
            ORDER BY fecha_hora_salida ASC
            ",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        filas
            .into_iter()
            .map(
                |(
                    id,
                    vehiculo_placa,
                    vehiculo_numero_unidad,
                    encargado_nombre,
                    numero_ruta,
                    sub_numero,
                    numero_documento,
                    fecha_documento_texto,
                    resultado_texto,
                    fecha_hora_salida_texto,
                    usuario_salida_nombre,
                )| {
                    let fecha_documento =
                        NaiveDate::parse_from_str(&fecha_documento_texto, "%Y-%m-%d")
                            .map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))?;
                    let resultado = match resultado_texto.as_str() {
                        "PERMITIDO" => ResultadoSalidaRuta::Permitido,
                        "PERMITIDO_CON_AUTORIZACION" => {
                            ResultadoSalidaRuta::PermitidoConAutorizacion
                        }
                        otro => {
                            return Err(DatabaseError::FechaCorrupta(format!(
                                "resultado desconocido: {otro}"
                            )));
                        }
                    };
                    let fecha_hora_salida = parsear_utc(&fecha_hora_salida_texto)
                        .map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))?;
                    Ok(SalidaRutaActivaResumen {
                        id,
                        vehiculo_placa,
                        vehiculo_numero_unidad,
                        encargado_nombre,
                        numero_ruta,
                        sub_numero,
                        numero_documento,
                        fecha_documento,
                        resultado,
                        fecha_hora_salida,
                        usuario_salida_nombre,
                    })
                },
            )
            .collect()
    }
}

const ULTIMO_INSTANTE_SALIDA_RUTA_SQL: &str = "
    SELECT MAX(instante)
    FROM (
        SELECT MAX(fecha_hora_salida) AS instante
        FROM salidas_ruta
        UNION ALL
        SELECT MAX(fecha_hora_retorno) AS instante
        FROM salidas_ruta
        WHERE fecha_hora_retorno IS NOT NULL
    )";

/// Mismo criterio y misma forma que
/// `movimiento_visita_repository::ultimo_instante_movimiento_visita` --
/// `AppCore::registrar_salida_ruta`/`registrar_retorno_ruta`
/// (`application/rutas.rs`) toman el máximo entre esto y los demás
/// dominios (ingresos, visitas) para que un sitio que sólo tuvo actividad
/// de rutas (sin ingresos/visitas todavía) también quede protegido contra
/// un reloj retrocedido.
pub fn ultimo_instante_salida_ruta(
    connection: &Connection,
) -> Result<Option<DateTime<Utc>>, DatabaseError> {
    let ultima: Option<String> =
        connection.query_row(ULTIMO_INSTANTE_SALIDA_RUTA_SQL, [], |row| row.get(0))?;
    ultima
        .map(|texto| {
            parsear_utc(&texto).map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

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

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    fn nueva(placa: &str, numero_documento: &str) -> NuevaSalidaRuta {
        NuevaSalidaRuta {
            vehiculo_id: None,
            vehiculo_placa: placa.to_string(),
            vehiculo_numero_unidad: Some("22906".to_string()),
            encargado_id: None,
            encargado_nombre: "Carlos Balmaceda".to_string(),
            numero_ruta: "CRR079".to_string(),
            sub_numero: 1,
            numero_documento: numero_documento.to_string(),
            fecha_documento: fecha("2026-09-15"),
            resultado: ResultadoSalidaRuta::Permitido,
            fecha_hora_salida: Utc::now(),
            usuario_salida_id: 1,
        }
    }

    #[test]
    fn crear_y_buscar_por_id_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);

        let id = repo.crear(&nueva("C12345", "700101452")).unwrap();
        let salida = repo.buscar_por_id(id).unwrap().unwrap();

        assert_eq!(salida.vehiculo_placa, "C12345");
        assert_eq!(salida.numero_documento, "700101452");
        assert_eq!(salida.resultado, ResultadoSalidaRuta::Permitido);
        assert!(salida.retorno.is_none());
    }

    #[test]
    fn crear_sin_match_de_catalogo_igual_registra_el_snapshot() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);

        let id = repo.crear(&nueva("BPH485", "700101453")).unwrap();
        let salida = repo.buscar_por_id(id).unwrap().unwrap();

        assert!(salida.vehiculo_id.is_none());
        assert!(salida.encargado_id.is_none());
        assert_eq!(salida.vehiculo_placa, "BPH485");
    }

    #[test]
    fn numero_documento_duplicado_falla() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);
        repo.crear(&nueva("C12345", "700101452")).unwrap();

        let error = repo.crear(&nueva("C99999", "700101452")).unwrap_err();
        assert!(error.es_constraint_unique());
    }

    #[test]
    fn dos_salidas_abiertas_a_la_vez_con_la_misma_placa_falla() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);
        repo.crear(&nueva("C12345", "700101452")).unwrap();

        assert!(repo.crear(&nueva("C12345", "700101453")).is_err());
    }

    #[test]
    fn buscar_activa_por_placa_ignora_las_ya_retornadas() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);
        let id = repo.crear(&nueva("C12345", "700101452")).unwrap();

        assert!(repo.buscar_activa_por_placa("C12345").unwrap().is_some());

        repo.registrar_retorno(id, Utc::now(), 1).unwrap();

        assert!(repo.buscar_activa_por_placa("C12345").unwrap().is_none());
    }

    #[test]
    fn registrar_retorno_dos_veces_falla_la_segunda() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);
        let id = repo.crear(&nueva("C12345", "700101452")).unwrap();
        repo.registrar_retorno(id, Utc::now(), 1).unwrap();

        assert!(matches!(
            repo.registrar_retorno(id, Utc::now(), 1),
            Err(DatabaseError::SalidaRutaNoActiva)
        ));
    }

    #[test]
    fn listar_activas_omite_las_ya_retornadas() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);
        let activa_id = repo.crear(&nueva("C12345", "700101452")).unwrap();
        let cerrada_id = repo.crear(&nueva("C99999", "700101453")).unwrap();
        repo.registrar_retorno(cerrada_id, Utc::now(), 1).unwrap();

        let activas = repo.listar_activas().unwrap();

        assert_eq!(activas.len(), 1);
        assert_eq!(activas[0].id, activa_id);
        assert_eq!(activas[0].usuario_salida_nombre, "Operador");
    }

    #[test]
    fn crear_permitido_con_autorizacion_persiste_el_resultado() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);

        let id = repo
            .crear(&NuevaSalidaRuta {
                resultado: ResultadoSalidaRuta::PermitidoConAutorizacion,
                ..nueva("C12345", "700101452")
            })
            .unwrap();

        let salida = repo.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(
            salida.resultado,
            ResultadoSalidaRuta::PermitidoConAutorizacion
        );
    }

    #[test]
    fn ultimo_instante_sin_salidas_es_ninguno() {
        let connection = conexion();
        assert_eq!(ultimo_instante_salida_ruta(&connection).unwrap(), None);
    }

    #[test]
    fn ultimo_instante_toma_el_retorno_si_es_mas_nuevo_que_la_salida() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);
        let id = repo.crear(&nueva("C12345", "700101452")).unwrap();
        let salida = repo.buscar_por_id(id).unwrap().unwrap();
        let retorno = salida.fecha_hora_salida + chrono::Duration::hours(2);
        repo.registrar_retorno(id, retorno, 1).unwrap();

        assert_eq!(
            ultimo_instante_salida_ruta(&connection).unwrap(),
            Some(retorno)
        );
    }
}

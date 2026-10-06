//! Ingreso/salida "por correo" (visita autorizada por correo, comodín previo
//! al módulo de Visitas) -- mismo armazón que
//! `RegistroIngresoProveedorRepository`: snapshot puro de cédula/nombre, con
//! `motivo` en vez de empresa. Sincroniza a la nube con `'ingreso_correo'` en
//! `cola_salida` (`MIGRACION_55`), ciclo abrir/cerrar.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, named_params, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::registro_ingreso_correo::{
    NuevoRegistroIngresoCorreo, RegistroIngresoCorreo, RegistroIngresoCorreoActivoResumen,
    SalidaRegistroIngresoCorreo,
};
use crate::tiempo::{parsear_utc, serializar_utc};

pub trait RegistroIngresoCorreoRepository {
    fn crear(&self, registro: &NuevoRegistroIngresoCorreo) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<RegistroIngresoCorreo>, DatabaseError>;

    /// ¿Esta cédula tiene el acceso negado como contratista? Ver
    /// `database::queries::contratistas::cedula_con_acceso_negado`.
    fn cedula_con_acceso_negado(&self, cedula: &str) -> Result<bool, DatabaseError>;

    /// Ingreso por correo abierto (sin salida) para esta cédula.
    fn buscar_ingreso_activo(
        &self,
        cedula: &str,
    ) -> Result<Option<RegistroIngresoCorreo>, DatabaseError>;

    /// ¿El gafete de visita está en uso en este equipo? Mira los ingresos
    /// por correo y también los movimientos de visita, que usan el mismo
    /// catálogo de gafetes de visita.
    fn gafete_de_visita_en_uso(&self, gafete_numero: i64) -> Result<bool, DatabaseError>;

    fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError>;

    fn listar_activos(&self) -> Result<Vec<RegistroIngresoCorreoActivoResumen>, DatabaseError>;
}

pub struct SqliteRegistroIngresoCorreoRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteRegistroIngresoCorreoRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn fecha_de_columna(columna: usize, texto: &str) -> rusqlite::Result<DateTime<Utc>> {
    parsear_utc(texto).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            columna,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

/// Columnas en el orden de [`SELECT_REGISTRO`].
fn convertir_fila(row: &Row) -> rusqlite::Result<RegistroIngresoCorreo> {
    let fecha_hora_ingreso = fecha_de_columna(6, &row.get::<_, String>(6)?)?;
    let fecha_hora_salida = row
        .get::<_, Option<String>>(8)?
        .map(|fecha| fecha_de_columna(8, &fecha))
        .transpose()?;
    let usuario_salida_id: Option<i64> = row.get(9)?;
    let salida = fecha_hora_salida
        .zip(usuario_salida_id)
        .map(|(fecha_hora, usuario_id)| SalidaRegistroIngresoCorreo {
            fecha_hora,
            usuario_id,
        });

    Ok(RegistroIngresoCorreo {
        id: row.get(0)?,
        cedula: row.get(1)?,
        nombre: row.get(2)?,
        motivo: row.get(3)?,
        placa: row.get(4)?,
        gafete_numero: row.get(5)?,
        fecha_hora_ingreso,
        usuario_ingreso_id: row.get(7)?,
        salida,
    })
}

/// Mismo criterio que en proveedores: compara la cédula en forma única
/// (`NORMALIZAR_CEDULA`) y recorre sólo el índice parcial de los abiertos.
const FILTRO_ACTIVO_POR_CEDULA: &str = "
    WHERE NORMALIZAR_CEDULA(cedula) = NORMALIZAR_CEDULA(?1)
      AND fecha_hora_salida IS NULL
    ORDER BY fecha_hora_ingreso DESC LIMIT 1";

const SELECT_REGISTRO: &str = "
    SELECT id, cedula, nombre, motivo, placa, gafete_numero,
           fecha_hora_ingreso, usuario_ingreso_id, fecha_hora_salida, usuario_salida_id
    FROM registro_ingresos_correo
";

impl RegistroIngresoCorreoRepository for SqliteRegistroIngresoCorreoRepository<'_> {
    fn cedula_con_acceso_negado(&self, cedula: &str) -> Result<bool, DatabaseError> {
        crate::database::queries::contratistas::cedula_con_acceso_negado(self.connection, cedula)
    }

    fn crear(&self, registro: &NuevoRegistroIngresoCorreo) -> Result<i64, DatabaseError> {
        let uuid = generar_uuid_v4();
        let filas = self.connection.execute(
            "
            INSERT INTO registro_ingresos_correo (
                cedula, nombre, motivo, placa, gafete_numero,
                fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre, uuid
            )
            SELECT :cedula, :nombre, :motivo, :placa, :gafete_numero,
                   :fecha_hora_ingreso, :usuario_ingreso_id, u.nombre, :uuid
            FROM usuarios AS u
            WHERE u.id = :usuario_ingreso_id
            ",
            named_params! {
                ":cedula": registro.cedula,
                ":nombre": registro.nombre,
                ":motivo": registro.motivo,
                ":placa": registro.placa,
                ":gafete_numero": registro.gafete_numero,
                ":fecha_hora_ingreso": serializar_utc(registro.fecha_hora_ingreso),
                ":usuario_ingreso_id": registro.usuario_ingreso_id,
                ":uuid": uuid,
            },
        )?;
        if filas == 0 {
            return Err(DatabaseError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }

        // Capturado antes de encolar: `encolar` hace su propio INSERT.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "ingreso_correo", &uuid, "crear")?;
        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<RegistroIngresoCorreo>, DatabaseError> {
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
    ) -> Result<Option<RegistroIngresoCorreo>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_REGISTRO} {FILTRO_ACTIVO_POR_CEDULA}"))?;
        match statement.query_row(params![cedula], convertir_fila) {
            Ok(registro) => Ok(Some(registro)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn gafete_de_visita_en_uso(&self, gafete_numero: i64) -> Result<bool, DatabaseError> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM registro_ingresos_correo
                 WHERE gafete_numero = ?1 AND fecha_hora_salida IS NULL
             ) OR EXISTS(
                 SELECT 1 FROM movimientos_visita
                 WHERE gafete_numero = ?1 AND fecha_hora_salida IS NULL
             )",
            params![gafete_numero],
            |row| row.get(0),
        )?)
    }

    fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError> {
        let filas_afectadas = self.connection.execute(
            "
            UPDATE registro_ingresos_correo
            SET
                fecha_hora_salida = ?1,
                usuario_salida_id = ?2,
                usuario_salida_nombre = (SELECT nombre FROM usuarios WHERE id = ?2)
            WHERE id = ?3
              AND fecha_hora_salida IS NULL
            ",
            params![serializar_utc(fecha_hora_salida), usuario_salida_id, id],
        )?;
        if filas_afectadas == 0 {
            return Err(DatabaseError::RegistroCorreoNoActivo);
        }

        let uuid: String = self.connection.query_row(
            "SELECT uuid FROM registro_ingresos_correo WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        cola_salida::encolar(self.connection, "ingreso_correo", &uuid, "cerrar")?;
        Ok(())
    }

    fn listar_activos(&self) -> Result<Vec<RegistroIngresoCorreoActivoResumen>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, cedula, nombre, motivo, placa, gafete_numero, fecha_hora_ingreso,
                   usuario_ingreso_nombre
            FROM registro_ingresos_correo
            WHERE fecha_hora_salida IS NULL
            ORDER BY fecha_hora_ingreso ASC
            ",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok(RegistroIngresoCorreoActivoResumen {
                    id: row.get(0)?,
                    cedula: row.get(1)?,
                    nombre: row.get(2)?,
                    motivo: row.get(3)?,
                    placa: row.get(4)?,
                    gafete_numero: row.get(5)?,
                    fecha_hora_ingreso: fecha_de_columna(6, &row.get::<_, String>(6)?)?,
                    usuario_ingreso_nombre: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(filas)
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
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        connection
    }

    fn nuevo(cedula: &str, gafete: i64) -> NuevoRegistroIngresoCorreo {
        NuevoRegistroIngresoCorreo {
            cedula: cedula.to_string(),
            nombre: "Ana Solano".to_string(),
            motivo: "Entrevista RH".to_string(),
            placa: None,
            gafete_numero: gafete,
            fecha_hora_ingreso: Utc.with_ymd_and_hms(2026, 10, 3, 14, 0, 0).unwrap(),
            usuario_ingreso_id: 1,
        }
    }

    #[test]
    fn crear_encola_y_salida_cierra_y_encola() {
        let connection = conexion();
        let repositorio = SqliteRegistroIngresoCorreoRepository::new(&connection);

        let id = repositorio.crear(&nuevo("111111111", 5)).unwrap();
        assert!(
            repositorio
                .buscar_ingreso_activo("111111111")
                .unwrap()
                .is_some()
        );
        assert!(repositorio.gafete_de_visita_en_uso(5).unwrap());
        assert_eq!(repositorio.listar_activos().unwrap().len(), 1);

        let salida = Utc.with_ymd_and_hms(2026, 10, 3, 15, 0, 0).unwrap();
        repositorio.registrar_salida(id, salida, 1).unwrap();
        let registro = repositorio.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(registro.salida.unwrap().fecha_hora, salida);
        assert!(!repositorio.gafete_de_visita_en_uso(5).unwrap());
        assert!(repositorio.listar_activos().unwrap().is_empty());
        assert!(matches!(
            repositorio.registrar_salida(id, salida, 1),
            Err(DatabaseError::RegistroCorreoNoActivo)
        ));

        let operaciones: Vec<String> = connection
            .prepare(
                "SELECT operacion FROM cola_salida WHERE entidad = 'ingreso_correo' ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(operaciones, ["crear", "cerrar"]);
    }

    #[test]
    fn el_esquema_rechaza_motivo_vacio_y_cedula_adentro_dos_veces() {
        let connection = conexion();
        let repositorio = SqliteRegistroIngresoCorreoRepository::new(&connection);

        let mut sin_motivo = nuevo("222222222", 6);
        sin_motivo.motivo = "   ".to_string();
        assert!(repositorio.crear(&sin_motivo).is_err());

        repositorio.crear(&nuevo("333333333", 7)).unwrap();
        assert!(repositorio.crear(&nuevo("333333333", 8)).is_err());
    }
}

//! Último instante que selló el reloj de ESTE equipo en cualquiera de los
//! dominios operativos (contratistas, visitas, rutas, proveedores y
//! correo). Base de la comprobación "¿el reloj del equipo retrocedió?" de
//! proveedores y correo, que hasta ahora no la hacían (contratistas,
//! visitas y rutas sí).
//!
//! Mismo criterio que `ingresos::ultimo_instante_movimiento`: una salida
//! que dio el otro dispositivo del sitio llega por la sincronización con
//! `usuario_salida_id = NULL` y la hora del reloj de ese otro equipo. No
//! cuenta: si ese reloj va adelantado, bloquearía aquí todo registro aunque
//! el reloj propio esté bien. Las entradas siempre son propias (lo que
//! abrió el otro equipo vive en las cachés `*_remotos`).

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::database::error::DatabaseError;
use crate::database::queries::ingresos::ultimo_instante_movimiento;
use crate::database::repositories::movimiento_visita_repository::ultimo_instante_movimiento_visita;
use crate::database::repositories::salida_ruta_repository::ultimo_instante_salida_ruta;
use crate::tiempo::parsear_utc;

const ULTIMO_INSTANTE_PROVEEDOR_Y_CORREO_SQL: &str = "
    SELECT MAX(instante)
    FROM (
        SELECT MAX(fecha_hora_ingreso) AS instante FROM registro_ingresos_proveedor
        UNION ALL
        SELECT MAX(fecha_hora_salida) FROM registro_ingresos_proveedor
        WHERE usuario_salida_id IS NOT NULL
        UNION ALL
        SELECT MAX(fecha_hora_ingreso) FROM registro_ingresos_correo
        UNION ALL
        SELECT MAX(fecha_hora_salida) FROM registro_ingresos_correo
        WHERE usuario_salida_id IS NOT NULL
    )";

fn ultimo_instante_proveedor_y_correo(
    connection: &Connection,
) -> Result<Option<DateTime<Utc>>, DatabaseError> {
    let ultima: Option<String> =
        connection.query_row(ULTIMO_INSTANTE_PROVEEDOR_Y_CORREO_SQL, [], |row| row.get(0))?;
    ultima
        .map(|texto| {
            parsear_utc(&texto).map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))
        })
        .transpose()
}

/// Máximo entre los cinco dominios operativos -- `None` si este equipo
/// nunca registró ningún movimiento.
pub fn ultimo_instante_operativo(
    connection: &Connection,
) -> Result<Option<DateTime<Utc>>, DatabaseError> {
    Ok([
        ultimo_instante_movimiento(connection)?,
        ultimo_instante_movimiento_visita(connection)?,
        ultimo_instante_salida_ruta(connection)?,
        ultimo_instante_proveedor_y_correo(connection)?,
    ]
    .into_iter()
    .flatten()
    .max())
}

/// ¿El reloj del equipo (`ahora`) quedó antes del último movimiento que
/// este mismo equipo selló?
pub fn reloj_retrocedido(
    connection: &Connection,
    ahora: DateTime<Utc>,
) -> Result<bool, DatabaseError> {
    Ok(ultimo_instante_operativo(connection)?.is_some_and(|ultimo| ahora < ultimo))
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
            .execute_batch(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                    VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
                 INSERT INTO empresas_proveedor (id, nombre, activo, uuid)
                    VALUES (1, 'Maika', 1, 'uuid-empresa');",
            )
            .unwrap();
        connection
    }

    fn instante(hora: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 8, hora, 0, 0).unwrap()
    }

    #[test]
    fn sin_movimientos_no_hay_instante_ni_reloj_retrocedido() {
        let connection = conexion();

        assert_eq!(ultimo_instante_operativo(&connection).unwrap(), None);
        assert!(!reloj_retrocedido(&connection, instante(0)).unwrap());
    }

    #[test]
    fn toma_en_cuenta_la_entrada_de_un_proveedor() {
        let connection = conexion();
        connection
            .execute(
                "INSERT INTO registro_ingresos_proveedor (cedula, nombre, empresa_id,
                    empresa_nombre, gafete_numero, fecha_hora_ingreso, usuario_ingreso_id,
                    usuario_ingreso_nombre, uuid)
                 VALUES ('111111111', 'ANA', 1, 'Maika', 7, '2026-10-08T15:00:00Z', 1,
                    'Operador', 'u1')",
                [],
            )
            .unwrap();

        assert_eq!(
            ultimo_instante_operativo(&connection).unwrap(),
            Some(instante(15))
        );
        assert!(reloj_retrocedido(&connection, instante(14)).unwrap());
        assert!(!reloj_retrocedido(&connection, instante(15)).unwrap());
    }

    #[test]
    fn toma_en_cuenta_la_salida_propia_de_un_ingreso_por_correo() {
        let connection = conexion();
        connection
            .execute(
                "INSERT INTO registro_ingresos_correo (cedula, nombre, motivo, gafete_numero,
                    fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre,
                    fecha_hora_salida, usuario_salida_id, usuario_salida_nombre, uuid)
                 VALUES ('111111111', 'ANA', 'Entrevista', 4, '2026-10-08T10:00:00Z', 1,
                    'Operador', '2026-10-08T16:00:00Z', 1, 'Operador', 'u1')",
                [],
            )
            .unwrap();

        assert_eq!(
            ultimo_instante_operativo(&connection).unwrap(),
            Some(instante(16))
        );
    }

    #[test]
    fn ignora_la_salida_que_dio_el_otro_dispositivo() {
        let connection = conexion();
        connection
            .execute(
                "INSERT INTO registro_ingresos_correo (cedula, nombre, motivo, gafete_numero,
                    fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre,
                    fecha_hora_salida, usuario_salida_id, usuario_salida_nombre, uuid)
                 VALUES ('111111111', 'ANA', 'Entrevista', 4, '2026-10-08T10:00:00Z', 1,
                    'Operador', '2026-10-08T23:00:00Z', NULL, 'Otro equipo', 'u1')",
                [],
            )
            .unwrap();

        assert_eq!(
            ultimo_instante_operativo(&connection).unwrap(),
            Some(instante(10))
        );
    }
}

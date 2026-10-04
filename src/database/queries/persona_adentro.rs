//! ¿Esta cédula ya está adentro por OTRA vía? Una persona no puede estar a
//! la vez adentro como contratista, como proveedor y por correo: cada vía
//! tiene su tabla y su índice único de cédula activa, pero esos índices no
//! se ven entre sí (revisión del 2026-10-04,
//! `docs/auditorias/revision-carreras-2026-10-04.md`). Mira lo abierto en
//! este equipo y lo que el otro equipo de la unidad tiene abierto (cachés
//! `*_remotos`, las llena la sincronización). La garantía final entre
//! equipos y unidades la da el trigger de la nube
//! (`20261004150000_persona_adentro_por_una_sola_via`).

use rusqlite::{Connection, params};

use crate::database::error::DatabaseError;
use crate::models::via_ingreso::ViaIngreso;

/// Por cada vía: lo abierto en este equipo y lo abierto en el otro equipo
/// (caché), comparando la cédula en forma única (`NORMALIZAR_CEDULA`).
const CONSULTAS: [(ViaIngreso, &str); 3] = [
    (
        ViaIngreso::Contratista,
        "SELECT EXISTS(
             SELECT 1 FROM registro_ingresos
             WHERE fecha_hora_salida IS NULL
               AND NORMALIZAR_CEDULA(contratista_cedula) = NORMALIZAR_CEDULA(?1)
         ) OR EXISTS(
             SELECT 1 FROM ingresos_remotos
             WHERE NORMALIZAR_CEDULA(contratista_cedula) = NORMALIZAR_CEDULA(?1)
         )",
    ),
    (
        ViaIngreso::Proveedor,
        "SELECT EXISTS(
             SELECT 1 FROM registro_ingresos_proveedor
             WHERE fecha_hora_salida IS NULL
               AND NORMALIZAR_CEDULA(cedula) = NORMALIZAR_CEDULA(?1)
         ) OR EXISTS(
             SELECT 1 FROM ingresos_proveedor_remotos
             WHERE NORMALIZAR_CEDULA(cedula) = NORMALIZAR_CEDULA(?1)
         )",
    ),
    (
        ViaIngreso::PorCorreo,
        "SELECT EXISTS(
             SELECT 1 FROM registro_ingresos_correo
             WHERE fecha_hora_salida IS NULL
               AND NORMALIZAR_CEDULA(cedula) = NORMALIZAR_CEDULA(?1)
         ) OR EXISTS(
             SELECT 1 FROM ingresos_correo_remotos
             WHERE NORMALIZAR_CEDULA(cedula) = NORMALIZAR_CEDULA(?1)
         )",
    ),
];

/// La primera vía distinta de `propia` por la que esta cédula está adentro,
/// en este equipo o en el otro de la unidad. `None` si no está adentro por
/// ninguna otra.
pub fn adentro_por_otra_via(
    connection: &Connection,
    cedula: &str,
    propia: ViaIngreso,
) -> Result<Option<ViaIngreso>, DatabaseError> {
    for (via, consulta) in CONSULTAS {
        if via == propia {
            continue;
        }
        let adentro: bool = connection.query_row(consulta, params![cedula], |fila| fila.get(0))?;
        if adentro {
            return Ok(Some(via));
        }
    }
    Ok(None)
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

    #[test]
    fn ve_un_ingreso_por_correo_abierto_aunque_la_cedula_venga_con_guiones() {
        let connection = conexion();
        connection
            .execute(
                "INSERT INTO registro_ingresos_correo (cedula, nombre, motivo, gafete_numero,
                    fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre, uuid)
                 VALUES ('111111111', 'Ana', 'Entrevista RH', 4,
                    '2026-10-03T14:00:00Z', 1, 'Operador', 'u1')",
                [],
            )
            .unwrap();

        assert_eq!(
            adentro_por_otra_via(&connection, "1-1111-1111", ViaIngreso::Proveedor).unwrap(),
            Some(ViaIngreso::PorCorreo)
        );
        assert_eq!(
            adentro_por_otra_via(&connection, "1-1111-1111", ViaIngreso::PorCorreo).unwrap(),
            None,
            "la vía propia la revisa cada servicio con su propia regla"
        );
        assert_eq!(
            adentro_por_otra_via(&connection, "222222222", ViaIngreso::Proveedor).unwrap(),
            None
        );
    }

    #[test]
    fn ve_lo_que_tiene_abierto_el_otro_equipo_de_la_unidad() {
        let connection = conexion();
        connection
            .execute(
                "INSERT INTO ingresos_proveedor_remotos (uuid, sitio_id, cedula, nombre,
                    empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre,
                    dispositivo_entrada_id, actualizado_en)
                 VALUES ('r1', 's1', '333333333', 'Luis', 'ACME', 9,
                    '2026-10-03T14:00:00Z', 'Otro', 'd2', '2026-10-03T14:00:00Z')",
                [],
            )
            .unwrap();

        assert_eq!(
            adentro_por_otra_via(&connection, "333333333", ViaIngreso::Contratista).unwrap(),
            Some(ViaIngreso::Proveedor)
        );
    }

    #[test]
    fn un_ingreso_ya_cerrado_no_cuenta() {
        let connection = conexion();
        connection
            .execute(
                "INSERT INTO registro_ingresos_correo (cedula, nombre, motivo, gafete_numero,
                    fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre,
                    fecha_hora_salida, usuario_salida_id, usuario_salida_nombre, uuid)
                 VALUES ('111111111', 'Ana', 'Entrevista RH', 4,
                    '2026-10-03T14:00:00Z', 1, 'Operador',
                    '2026-10-03T15:00:00Z', 1, 'Operador', 'u1')",
                [],
            )
            .unwrap();

        assert_eq!(
            adentro_por_otra_via(&connection, "111111111", ViaIngreso::Contratista).unwrap(),
            None
        );
    }
}

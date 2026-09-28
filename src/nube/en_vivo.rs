//! Aviso en vivo con los datos: el trigger `cambio_nube` de Supabase manda
//! la fila completa (`registro`) de las tablas que la soportan, y el equipo
//! la guarda directo en su `SQLite` sin consultar a la nube. Antes el aviso
//! sólo decía "cambió la tabla X" y el equipo tenía que ir a buscar los
//! datos: ese viaje de ida y vuelta era la demora que se notaba.
//!
//! Usa el mismo código que la sincronización para guardar cada fila
//! (`guardar_ingreso_remoto`, `aplicar_cierre_de_ingreso_propio`), así que
//! el resultado es idéntico. La sincronización por tabla sigue corriendo
//! detrás como red de seguridad: corrige un aviso fuera de orden o perdido.
//!
//! Hoy sólo `ingresos` (la pantalla Activos). Un aviso de otra tabla, o uno
//! viejo sin `registro`, devuelve `false` y queda para la sincronización.

use rusqlite::{Connection, params};

use super::SincronizacionError;
use super::sincronizacion::{
    FilaIngresoRemoto, aplicar_cierre_de_ingreso_propio, guardar_ingreso_remoto,
};

#[derive(serde::Deserialize)]
struct AvisoCambio {
    table: String,
    operation: String,
    /// Id de la fila (también en DELETE, donde no hay `registro`).
    id: Option<String>,
    registro: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct EstadoIngreso {
    id: String,
    sitio_id: String,
    hora_salida: Option<String>,
    usuario_salida_nombre: Option<String>,
}

/// Aplica un aviso `cambio_nube` con datos. `true` si lo aplicó; `false`
/// si no es una tabla soportada o el aviso no trae la fila (queda para la
/// sincronización por tabla). Un aviso que no se puede interpretar también
/// devuelve `false`: nunca debe tumbar nada, la sincronización lo corrige.
pub fn aplicar_cambio_en_vivo(
    conexion: &Connection,
    aviso: &serde_json::Value,
) -> Result<bool, SincronizacionError> {
    let Ok(aviso) = serde_json::from_value::<AvisoCambio>(aviso.clone()) else {
        log::warn!("aviso en vivo: formato inesperado");
        return Ok(false);
    };
    if aviso.table != "ingresos" {
        return Ok(false);
    }
    if aviso.operation == "DELETE" {
        let Some(id) = aviso.id else {
            return Ok(false);
        };
        conexion.execute("DELETE FROM ingresos_remotos WHERE uuid = ?1", params![id])?;
        return Ok(true);
    }
    let Some(registro) = aviso.registro else {
        return Ok(false);
    };
    let (Ok(estado), Ok(fila)) = (
        serde_json::from_value::<EstadoIngreso>(registro.clone()),
        serde_json::from_value::<FilaIngresoRemoto>(registro),
    ) else {
        log::warn!("aviso en vivo: fila de ingreso con formato inesperado");
        return Ok(false);
    };

    let transaccion = conexion.unchecked_transaction()?;
    if let Some(hora_salida) = &estado.hora_salida {
        // Cerrado: deja de estar adentro en el otro equipo y, si era
        // propio, se registra la salida acá.
        transaccion.execute(
            "DELETE FROM ingresos_remotos WHERE uuid = ?1",
            params![estado.id],
        )?;
        aplicar_cierre_de_ingreso_propio(
            &transaccion,
            &estado.id,
            hora_salida,
            estado.usuario_salida_nombre.as_deref(),
        )?;
    } else {
        guardar_ingreso_remoto(&transaccion, &estado.sitio_id, fila)?;
    }
    transaccion.commit()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::database::schema::initialize_database;

    fn base() -> Connection {
        let conexion = Connection::open_in_memory().unwrap();
        initialize_database(&conexion).unwrap();
        conexion
    }

    fn aviso(operacion: &str, hora_salida: Option<&str>) -> serde_json::Value {
        json!({
            "table": "ingresos",
            "operation": operacion,
            "id": "u-otro",
            "sitio_id": "s1",
            "registro": {
                "id": "u-otro",
                "sitio_id": "s1",
                "contratista_nombre": "PERSONA",
                "contratista_cedula": "101110111",
                "hora_entrada": "2026-09-27T12:00:00.123+00:00",
                "usuario_entrada_nombre": "Guarda",
                "dispositivo_entrada_id": "otro-equipo",
                "empresa_nombre": "Empresa",
                "tipo_ingreso": "SWAT",
                "medio_ingreso": "CAMINANDO",
                "gafete_numero": null,
                "placa": null,
                "hora_salida": hora_salida,
                "usuario_salida_nombre": hora_salida.map(|_| "Guarda 2"),
                "columna_que_no_usamos": 1
            }
        })
    }

    fn remotos(conexion: &Connection) -> Vec<(String, String)> {
        conexion
            .prepare("SELECT uuid, hora_entrada FROM ingresos_remotos ORDER BY uuid")
            .unwrap()
            .query_map([], |fila| Ok((fila.get(0)?, fila.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn un_ingreso_del_otro_equipo_queda_adentro_al_instante() {
        let conexion = base();

        assert!(aplicar_cambio_en_vivo(&conexion, &aviso("INSERT", None)).unwrap());

        // La fecha queda en el formato único de la app, igual que al sincronizar.
        assert_eq!(
            remotos(&conexion),
            vec![("u-otro".to_string(), "2026-09-27T12:00:00Z".to_string())]
        );
    }

    #[test]
    fn el_mismo_aviso_repetido_no_duplica() {
        let conexion = base();
        aplicar_cambio_en_vivo(&conexion, &aviso("INSERT", None)).unwrap();
        aplicar_cambio_en_vivo(&conexion, &aviso("INSERT", None)).unwrap();

        assert_eq!(remotos(&conexion).len(), 1);
    }

    #[test]
    fn la_salida_lo_saca_de_adentro() {
        let conexion = base();
        aplicar_cambio_en_vivo(&conexion, &aviso("INSERT", None)).unwrap();

        let aplicado =
            aplicar_cambio_en_vivo(&conexion, &aviso("UPDATE", Some("2026-09-27T13:00:00Z")))
                .unwrap();

        assert!(aplicado);
        assert!(remotos(&conexion).is_empty());
    }

    #[test]
    fn la_salida_de_un_ingreso_propio_se_registra_aca() {
        let conexion = base();
        conexion
            .execute_batch(
                "INSERT INTO empresas(id,nombre) VALUES (1,'Empresa');
                 INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo)
                 VALUES (1,'U1','Operador','hash','OPERADOR',1);
                 INSERT INTO contratistas(
                     id,cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso
                 ) VALUES (1,'101110111','PERSONA',1,'SWAT',0,1);
                 INSERT INTO registro_ingresos (
                     contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
                     usuario_ingreso_id, contratista_cedula, contratista_nombre, empresa_nombre,
                     usuario_ingreso_nombre, es_personal_ruta, tiene_acceso, resultado_acceso,
                     reglas_version, uuid
                 ) VALUES (
                     1, 1, '2026-09-27T12:00:00Z', 'CAMINANDO', 'SWAT',
                     1, '101110111', 'PERSONA', 'Empresa',
                     'Operador', 0, 1, 'PERMITIDO', 1, 'u-otro'
                 );",
            )
            .unwrap();

        aplicar_cambio_en_vivo(&conexion, &aviso("UPDATE", Some("2026-09-27T13:00:00Z"))).unwrap();

        let salida: Option<String> = conexion
            .query_row(
                "SELECT fecha_hora_salida FROM registro_ingresos WHERE uuid = 'u-otro'",
                [],
                |fila| fila.get(0),
            )
            .unwrap();
        assert_eq!(salida.as_deref(), Some("2026-09-27T13:00:00Z"));
        assert!(remotos(&conexion).is_empty(), "lo propio no va a la caché");
    }

    #[test]
    fn un_delete_lo_saca_de_la_cache() {
        let conexion = base();
        aplicar_cambio_en_vivo(&conexion, &aviso("INSERT", None)).unwrap();

        let borrado = json!({"table": "ingresos", "operation": "DELETE", "id": "u-otro"});
        assert!(aplicar_cambio_en_vivo(&conexion, &borrado).unwrap());
        assert!(remotos(&conexion).is_empty());
    }

    #[test]
    fn otra_tabla_o_aviso_sin_datos_queda_para_la_sincronizacion() {
        let conexion = base();
        let otra_tabla = json!({"table": "empresas", "operation": "INSERT", "registro": {}});
        let sin_datos = json!({"table": "ingresos", "operation": "INSERT", "id": "x"});
        let raro = json!({"lo_que_sea": true});

        assert!(!aplicar_cambio_en_vivo(&conexion, &otra_tabla).unwrap());
        assert!(!aplicar_cambio_en_vivo(&conexion, &sin_datos).unwrap());
        assert!(!aplicar_cambio_en_vivo(&conexion, &raro).unwrap());
        assert!(remotos(&conexion).is_empty());
    }
}

//! Retención de las cachés de historial que el escritorio recibe de la
//! nube. Sin esto crecían para siempre: más disco y búsquedas y
//! sincronizaciones más lentas. La nube conserva todo; lo más viejo que el
//! límite se consulta en el panel web (decisión del dueño, 2026-09-27).
//!
//! Sólo toca las cachés espejo del sitio, nunca los registros propios del
//! equipo (que tienen su cola de salida) ni un movimiento todavía abierto.

use chrono::{DateTime, Months, Utc};
use rusqlite::{Connection, TransactionBehavior};

use crate::tiempo::serializar_utc;

/// Cuánto historial del sitio guarda el escritorio.
pub const MESES_HISTORIAL_ESCRITORIO: u32 = 24;

/// Cada caché con su columna de entrada y de salida (`NULL` = abierto).
const CACHES_HISTORIAL: [(&str, &str, &str); 4] = [
    ("historial_sitio", "hora_entrada", "hora_salida"),
    ("historial_visitas_sitio", "hora_entrada", "hora_salida"),
    (
        "historial_ingresos_proveedor_sitio",
        "hora_entrada",
        "hora_salida",
    ),
    (
        "prestamos_gafete_provisional_historial_sitio",
        "fecha_hora_entrega",
        "fecha_hora_devolucion",
    ),
];

/// Primer instante que se conserva: `meses` antes de `ahora`.
pub fn limite_retencion(ahora: DateTime<Utc>, meses: u32) -> DateTime<Utc> {
    ahora
        .checked_sub_months(Months::new(meses))
        .unwrap_or(DateTime::<Utc>::MIN_UTC)
}

/// Borra de las cachés de historial los movimientos cerrados que entraron
/// antes de `limite`. Devuelve cuántas filas borró. Las fechas se guardan
/// normalizadas con `serializar_utc`, así que se comparan como texto (y
/// usan el índice de la columna de entrada de cada tabla).
pub fn purgar_historiales_del_sitio(
    conexion: &Connection,
    limite: DateTime<Utc>,
) -> Result<u32, rusqlite::Error> {
    let limite = serializar_utc(limite);
    let transaccion =
        rusqlite::Transaction::new_unchecked(conexion, TransactionBehavior::Immediate)?;
    let mut borradas = 0;
    for (tabla, entrada, salida) in CACHES_HISTORIAL {
        borradas += transaccion.execute(
            &format!("DELETE FROM {tabla} WHERE {entrada} < ?1 AND {salida} IS NOT NULL"),
            [&limite],
        )?;
    }
    transaccion.commit()?;
    Ok(u32::try_from(borradas).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::database::schema::initialize_database;

    fn fecha(anio: i32, mes: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(anio, mes, 1, 12, 0, 0).unwrap()
    }

    fn contar(conexion: &Connection, tabla: &str) -> i64 {
        conexion
            .query_row(&format!("SELECT count(*) FROM {tabla}"), [], |fila| {
                fila.get(0)
            })
            .unwrap()
    }

    #[test]
    fn el_limite_es_24_meses_antes() {
        assert_eq!(
            limite_retencion(fecha(2026, 9), MESES_HISTORIAL_ESCRITORIO),
            fecha(2024, 9)
        );
    }

    #[test]
    fn borra_solo_lo_cerrado_y_viejo_de_cada_cache() {
        let conexion = Connection::open_in_memory().unwrap();
        initialize_database(&conexion).unwrap();
        let viejo = serializar_utc(fecha(2024, 1));
        let reciente = serializar_utc(fecha(2026, 8));
        // Por cada caché: una fila vieja cerrada (se va), una vieja
        // abierta (se queda) y una reciente cerrada (se queda).
        for (tabla, entrada, salida) in CACHES_HISTORIAL {
            let columnas: Vec<(String, bool)> = conexion
                .prepare(&format!("PRAGMA table_info({tabla})"))
                .unwrap()
                .query_map([], |fila| Ok((fila.get(1)?, fila.get::<_, i64>(3)? != 0)))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            for (uuid, hora_entrada, hora_salida) in [
                ("viejo-cerrado", &viejo, Some(&viejo)),
                ("viejo-abierto", &viejo, None),
                ("reciente-cerrado", &reciente, Some(&reciente)),
            ] {
                let mut nombres = Vec::new();
                let mut valores: Vec<rusqlite::types::Value> = Vec::new();
                for (columna, obligatoria) in &columnas {
                    let valor = if columna == "uuid" {
                        Some(uuid.to_string().into())
                    } else if columna == entrada {
                        Some(hora_entrada.clone().into())
                    } else if columna == salida {
                        Some(
                            hora_salida
                                .cloned()
                                .map_or(rusqlite::types::Value::Null, Into::into),
                        )
                    } else if *obligatoria {
                        Some(rusqlite::types::Value::Integer(1))
                    } else {
                        None
                    };
                    if let Some(valor) = valor {
                        nombres.push(columna.clone());
                        valores.push(valor);
                    }
                }
                let marcas = vec!["?"; valores.len()].join(", ");
                conexion
                    .execute(
                        &format!(
                            "INSERT INTO {tabla} ({}) VALUES ({marcas})",
                            nombres.join(", ")
                        ),
                        rusqlite::params_from_iter(valores),
                    )
                    .unwrap();
            }
        }

        let borradas =
            purgar_historiales_del_sitio(&conexion, limite_retencion(fecha(2026, 9), 24)).unwrap();

        assert_eq!(borradas, 4);
        for (tabla, _, _) in CACHES_HISTORIAL {
            assert_eq!(contar(&conexion, tabla), 2, "{tabla}");
            let queda_el_abierto: bool = conexion
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {tabla} WHERE uuid = 'viejo-abierto')"),
                    [],
                    |fila| fila.get(0),
                )
                .unwrap();
            assert!(queda_el_abierto, "{tabla}");
        }
    }
}

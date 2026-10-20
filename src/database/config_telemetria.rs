//! La última orden de telemetría que recibió este equipo (`config_telemetria`,
//! una sola fila). Se lee al abrir el núcleo y se escribe cuando llega una
//! orden nueva; la consulta de cada medición no pasa por acá sino por
//! [`crate::telemetria::config`].

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::telemetria::{ConfigTelemetria, Grupos, Orden};
use crate::tiempo::{parsear_utc, serializar_utc};

/// La orden guardada y a qué equipo iba.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdenGuardada {
    pub dispositivo_id: String,
    pub orden: Orden,
}

/// Guarda la orden si es más nueva que la guardada, o si la guardada era de
/// otro equipo (el equipo se vinculó de nuevo con otra identidad). Devuelve si
/// cambió algo.
pub fn guardar(
    connection: &Connection,
    dispositivo_id: &str,
    orden: &Orden,
    ahora: DateTime<Utc>,
) -> rusqlite::Result<bool> {
    let cambios = connection.execute(
        "
        INSERT INTO config_telemetria (id, dispositivo_id, version, grupos, vence_en, recibida_en)
        VALUES (1, ?1, ?2, ?3, ?4, ?5)
        ON CONFLICT (id) DO UPDATE SET
            dispositivo_id = excluded.dispositivo_id,
            version = excluded.version,
            grupos = excluded.grupos,
            vence_en = excluded.vence_en,
            recibida_en = excluded.recibida_en
        WHERE excluded.dispositivo_id <> config_telemetria.dispositivo_id
           OR excluded.version > config_telemetria.version
        ",
        params![
            dispositivo_id,
            orden.version,
            orden.grupos.nombres_de_la_orden().join(","),
            orden.vence_en.map(serializar_utc),
            serializar_utc(ahora),
        ],
    )?;
    Ok(cambios > 0)
}

/// La orden guardada, si hay. Una fila ilegible (que el `CHECK` no deja
/// escribir) se trata como "no hay": el equipo mide lo de siempre.
pub fn leer(connection: &Connection) -> rusqlite::Result<Option<OrdenGuardada>> {
    let fila = connection
        .query_row(
            "SELECT dispositivo_id, version, grupos, vence_en FROM config_telemetria WHERE id = 1",
            [],
            |fila| {
                Ok((
                    fila.get::<_, String>(0)?,
                    fila.get::<_, i64>(1)?,
                    fila.get::<_, String>(2)?,
                    fila.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?;
    Ok(
        fila.and_then(|(dispositivo_id, version, grupos, vence_en)| {
            let vence_en = match vence_en {
                Some(texto) => Some(parsear_utc(&texto).ok()?),
                None => None,
            };
            Some(OrdenGuardada {
                dispositivo_id,
                orden: Orden {
                    version,
                    grupos: Grupos::desde_nombres(grupos.split(',')),
                    vence_en,
                },
            })
        }),
    )
}

/// Carga en memoria la orden guardada, si iba a este equipo. Se llama al abrir
/// el núcleo, para que el equipo mida desde el primer segundo aunque no tenga
/// red. Devuelve si aplicó una orden.
pub fn cargar_en(
    connection: &Connection,
    dispositivo_id: &str,
    config: &ConfigTelemetria,
) -> rusqlite::Result<bool> {
    Ok(match leer(connection)? {
        Some(guardada) if guardada.dispositivo_id == dispositivo_id => {
            config.aplicar(&guardada.orden)
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::database::schema::initialize_database;
    use crate::telemetria::Grupo;

    fn base() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    fn hora(segundos: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(segundos, 0).unwrap()
    }

    fn orden(version: i64, nombres: &[&str], vence_en: Option<i64>) -> Orden {
        Orden {
            version,
            grupos: Grupos::desde_nombres(nombres.iter().copied()),
            vence_en: vence_en.map(hora),
        }
    }

    const AHORA: i64 = 1_800_000_000;

    #[test]
    fn sin_fila_no_hay_orden() {
        assert_eq!(leer(&base()).unwrap(), None);
    }

    #[test]
    fn guardar_y_leer_conserva_la_orden() {
        let c = base();
        let o = orden(7, &["fluidez", "recursos"], Some(AHORA + 3_600));
        assert!(guardar(&c, "equipo-1", &o, hora(AHORA)).unwrap());
        assert_eq!(
            leer(&c).unwrap(),
            Some(OrdenGuardada {
                dispositivo_id: "equipo-1".into(),
                orden: o
            })
        );
    }

    #[test]
    fn una_orden_vacia_y_sin_vencimiento_tambien_se_conserva() {
        let c = base();
        let o = orden(3, &[], None);
        assert!(guardar(&c, "equipo-1", &o, hora(AHORA)).unwrap());
        assert_eq!(leer(&c).unwrap().unwrap().orden, o);
    }

    #[test]
    fn solo_se_guarda_una_orden_mas_nueva() {
        let c = base();
        assert!(guardar(&c, "equipo-1", &orden(5, &["fluidez"], None), hora(AHORA)).unwrap());
        // Igual o más vieja: no cambia nada.
        assert!(!guardar(&c, "equipo-1", &orden(5, &["recursos"], None), hora(AHORA)).unwrap());
        assert!(!guardar(&c, "equipo-1", &orden(4, &["recursos"], None), hora(AHORA)).unwrap());
        assert_eq!(
            leer(&c).unwrap().unwrap().orden,
            orden(5, &["fluidez"], None)
        );
        // Más nueva: reemplaza.
        assert!(guardar(&c, "equipo-1", &orden(6, &["recursos"], None), hora(AHORA)).unwrap());
        assert_eq!(leer(&c).unwrap().unwrap().orden.version, 6);
    }

    #[test]
    fn una_orden_de_otro_equipo_reemplaza_aunque_su_version_sea_menor() {
        // El equipo se vinculó de nuevo con otra identidad: la versión de la
        // identidad anterior ya no cuenta.
        let c = base();
        assert!(guardar(&c, "equipo-1", &orden(900, &["fluidez"], None), hora(AHORA)).unwrap());
        assert!(guardar(&c, "equipo-2", &orden(3, &["escaneo"], None), hora(AHORA)).unwrap());
        let guardada = leer(&c).unwrap().unwrap();
        assert_eq!(guardada.dispositivo_id, "equipo-2");
        assert_eq!(guardada.orden.version, 3);
    }

    #[test]
    fn el_vencimiento_se_guarda_al_segundo_en_utc() {
        let c = base();
        let con_fraccion = Orden {
            version: 1,
            grupos: Grupos::VACIO,
            vence_en: Some(Utc.timestamp_opt(AHORA, 123_456_000).unwrap()),
        };
        assert!(guardar(&c, "equipo-1", &con_fraccion, hora(AHORA)).unwrap());
        let texto: String = c
            .query_row("SELECT vence_en FROM config_telemetria", [], |f| f.get(0))
            .unwrap();
        assert!(texto.ends_with('Z') && !texto.contains('.'), "{texto}");
    }

    #[test]
    fn un_nombre_de_grupo_desconocido_se_ignora_al_leer() {
        let c = base();
        c.execute(
            "INSERT INTO config_telemetria (id, dispositivo_id, version, grupos, vence_en, recibida_en)
             VALUES (1, 'equipo-1', 2, 'fluidez,futuro,base', NULL, '2026-10-20T12:00:00Z')",
            [],
        )
        .unwrap();
        let orden = leer(&c).unwrap().unwrap().orden;
        assert!(orden.grupos.contiene(Grupo::Fluidez));
        assert!(!orden.grupos.contiene(Grupo::Base));
        assert_eq!(orden.grupos.nombres_de_la_orden(), ["fluidez"]);
    }

    #[test]
    fn al_abrir_se_carga_la_orden_si_iba_a_este_equipo() {
        let c = base();
        guardar(&c, "equipo-1", &orden(4, &["recursos"], None), hora(AHORA)).unwrap();

        let ajeno = ConfigTelemetria::nueva();
        assert!(!cargar_en(&c, "equipo-2", &ajeno).unwrap());
        assert!(!ajeno.permite(Grupo::Recursos, AHORA));
        assert_eq!(ajeno.version(), 0);

        let propio = ConfigTelemetria::nueva();
        assert!(cargar_en(&c, "equipo-1", &propio).unwrap());
        assert!(propio.permite(Grupo::Recursos, AHORA));
        assert_eq!(propio.version(), 4);
    }

    #[test]
    fn al_abrir_una_orden_vencida_deja_al_equipo_en_lo_de_siempre() {
        let c = base();
        guardar(
            &c,
            "equipo-1",
            &orden(4, &["recursos"], Some(AHORA - 10)),
            hora(AHORA - 100),
        )
        .unwrap();
        let config = ConfigTelemetria::nueva();
        cargar_en(&c, "equipo-1", &config).unwrap();
        assert!(!config.permite(Grupo::Recursos, AHORA));
        assert!(config.permite(Grupo::Rendimiento, AHORA));
    }

    #[test]
    fn sin_orden_guardada_no_se_carga_nada() {
        let config = ConfigTelemetria::nueva();
        assert!(!cargar_en(&base(), "equipo-1", &config).unwrap());
        assert_eq!(config.version(), 0);
    }
}

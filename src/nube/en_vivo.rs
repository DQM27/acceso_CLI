//! Aviso en vivo con los datos: el trigger `cambio_nube` de Supabase manda
//! la fila completa (`registro`) de las tablas que la soportan, y el equipo
//! la guarda directo en su `SQLite` sin consultar a la nube. Antes el aviso
//! sólo decía "cambió la tabla X" y el equipo tenía que ir a buscar los
//! datos: ese viaje de ida y vuelta era la demora que se notaba.
//!
//! Usa el mismo código que la sincronización para guardar cada fila
//! (`guardar_ingreso_remoto`, `aplicar_cierre_de_ingreso_propio` y, en el
//! escritorio, `guardar_historial_en_vivo`), así que el resultado es
//! idéntico: se actualiza SÓLO la fila que cambió, también en el historial.
//! Un aviso aplicado no necesita sincronización detrás; el pulso periódico
//! sigue siendo la red de seguridad para un aviso fuera de orden o perdido.
//!
//! Tablas soportadas:
//! - `ingresos` (contratistas): Activos e Historial.
//! - `ingresos_proveedor`, `ingresos_correo` y `prestamos_gafete_provisional`
//!   (KOF): la caché del otro equipo (`*_remotos`, la que mira la
//!   verificación local de un ingreso) y el cierre de lo propio. Su
//!   historial en el escritorio sólo existe como copia de la nube, así que
//!   ahí se informa `false` para que además corra la sincronización de esa
//!   tabla; en el móvil, que no guarda historial, el aviso basta.
//!
//! Un aviso de otra tabla, o uno viejo sin `registro`, devuelve `false` y
//! queda para la sincronización.

use rusqlite::{Connection, params};

use super::SincronizacionError;
use super::orquestacion::PerfilDispositivo;
use super::sincronizacion::{
    FilaIngresoRemoto, aplicar_cierre_de_ingreso_correo_propio, aplicar_cierre_de_ingreso_propio,
    aplicar_cierre_de_ingreso_proveedor_propio, aplicar_devolucion_de_prestamo_propio,
    guardar_historial_en_vivo, guardar_ingreso_correo_remoto, guardar_ingreso_proveedor_remoto,
    guardar_ingreso_remoto, guardar_prestamo_gafete_provisional_remoto,
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
/// `perfil`: el escritorio además guarda la fila en su historial del sitio;
/// el móvil no guarda historial (ver [`PerfilDispositivo`]).
pub fn aplicar_cambio_en_vivo(
    conexion: &Connection,
    aviso: &serde_json::Value,
    perfil: PerfilDispositivo,
) -> Result<bool, SincronizacionError> {
    let Ok(aviso) = serde_json::from_value::<AvisoCambio>(aviso.clone()) else {
        log::warn!("aviso en vivo: formato inesperado");
        return Ok(false);
    };
    match aviso.table.as_str() {
        "ingresos" => aplicar_ingreso(conexion, aviso, perfil),
        "ingresos_proveedor" => aplicar_abierto_o_cerrado(conexion, aviso, perfil, &PROVEEDOR),
        "ingresos_correo" => aplicar_abierto_o_cerrado(conexion, aviso, perfil, &CORREO),
        "prestamos_gafete_provisional" => aplicar_abierto_o_cerrado(conexion, aviso, perfil, &KOF),
        _ => Ok(false),
    }
}

/// Ingreso de contratista: caché del otro equipo, cierre de lo propio y, en
/// el escritorio, su línea del historial.
fn aplicar_ingreso(
    conexion: &Connection,
    aviso: AvisoCambio,
    perfil: PerfilDispositivo,
) -> Result<bool, SincronizacionError> {
    if aviso.operation == "DELETE" {
        let Some(id) = aviso.id else {
            return Ok(false);
        };
        let transaccion = conexion.unchecked_transaction()?;
        transaccion.execute("DELETE FROM ingresos_remotos WHERE uuid = ?1", params![id])?;
        if perfil.guarda_historiales() {
            transaccion.execute("DELETE FROM historial_sitio WHERE uuid = ?1", params![id])?;
        }
        transaccion.commit()?;
        return Ok(true);
    }
    let Some(registro) = aviso.registro else {
        return Ok(false);
    };
    let (Ok(estado), Ok(fila)) = (
        serde_json::from_value::<EstadoIngreso>(registro.clone()),
        serde_json::from_value::<FilaIngresoRemoto>(registro.clone()),
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
    if perfil.guarda_historiales()
        && !guardar_historial_en_vivo(&transaccion, &estado.sitio_id, registro)?
    {
        // La fila de Activos ya quedó bien; el historial lo completa el
        // próximo pulso. Igual se informa `false` para que corra la
        // sincronización por tabla y no quede un hueco hasta entonces.
        transaccion.commit()?;
        return Ok(false);
    }
    transaccion.commit()?;
    Ok(true)
}

/// Una tabla de "abiertos" del sitio -- proveedor, correo o KOF -- que el
/// aviso en vivo aplica con el mismo código que la sincronización. Lo único
/// que cambia entre ellas son los nombres y las funciones de cada módulo.
struct TablaDeAbiertos {
    /// Caché local de lo abierto por el otro equipo.
    cache: &'static str,
    /// Columna de Supabase que, con valor, dice que se cerró.
    columna_cierre: &'static str,
    /// Quién lo cerró (el texto que queda en el registro propio).
    columna_quien_cierra: &'static str,
    /// Guarda la fila abierta en la caché. `false` si no se pudo
    /// interpretar.
    guardar: fn(&Connection, &str, serde_json::Value) -> Result<bool, SincronizacionError>,
    /// Registra acá el cierre de un registro propio que cerró otro equipo.
    cerrar_propio: fn(&Connection, &str, &str, Option<&str>) -> Result<bool, SincronizacionError>,
}

const PROVEEDOR: TablaDeAbiertos = TablaDeAbiertos {
    cache: "ingresos_proveedor_remotos",
    columna_cierre: "hora_salida",
    columna_quien_cierra: "usuario_salida_nombre",
    guardar: |conexion, sitio_id, registro| {
        guardar_si_se_puede_leer(registro, |fila| {
            guardar_ingreso_proveedor_remoto(conexion, sitio_id, fila)
        })
    },
    cerrar_propio: aplicar_cierre_de_ingreso_proveedor_propio,
};

const CORREO: TablaDeAbiertos = TablaDeAbiertos {
    cache: "ingresos_correo_remotos",
    columna_cierre: "hora_salida",
    columna_quien_cierra: "usuario_salida_nombre",
    guardar: |conexion, sitio_id, registro| {
        guardar_si_se_puede_leer(registro, |fila| {
            guardar_ingreso_correo_remoto(conexion, sitio_id, fila)
        })
    },
    cerrar_propio: aplicar_cierre_de_ingreso_correo_propio,
};

const KOF: TablaDeAbiertos = TablaDeAbiertos {
    cache: "prestamos_gafete_provisional_remotos",
    columna_cierre: "hora_devolucion",
    columna_quien_cierra: "usuario_devolucion_nombre",
    guardar: |conexion, sitio_id, registro| {
        guardar_si_se_puede_leer(registro, |fila| {
            guardar_prestamo_gafete_provisional_remoto(conexion, sitio_id, fila)
        })
    },
    cerrar_propio: aplicar_devolucion_de_prestamo_propio,
};

/// Lee la fila del aviso con el formato que espera la sincronización y la
/// guarda con su misma función. `false` si no se pudo leer.
fn guardar_si_se_puede_leer<T: serde::de::DeserializeOwned, R>(
    registro: serde_json::Value,
    guardar: impl FnOnce(T) -> Result<R, SincronizacionError>,
) -> Result<bool, SincronizacionError> {
    serde_json::from_value::<T>(registro).map_or(Ok(false), |fila| guardar(fila).map(|_| true))
}

/// Abierto: queda adentro en el otro equipo (la caché que mira la
/// verificación local). Cerrado: sale de la caché y, si era propio, se
/// registra el cierre acá. Borrado: sale de la caché.
///
/// `true` sólo si ya no hace falta sincronizar: en el escritorio el
/// historial de estas tablas es sólo la copia de la nube, así que ahí se
/// devuelve `false` aunque la caché ya haya quedado al día, para que corra
/// la sincronización de esa tabla y el historial no espere al pulso.
fn aplicar_abierto_o_cerrado(
    conexion: &Connection,
    aviso: AvisoCambio,
    perfil: PerfilDispositivo,
    tabla: &TablaDeAbiertos,
) -> Result<bool, SincronizacionError> {
    let basta_con_el_aviso = !perfil.guarda_historiales();
    let quitar_de_la_cache = format!("DELETE FROM {} WHERE uuid = ?1", tabla.cache);
    if aviso.operation == "DELETE" {
        let Some(id) = aviso.id else {
            return Ok(false);
        };
        conexion.execute(&quitar_de_la_cache, params![id])?;
        return Ok(basta_con_el_aviso);
    }
    let Some(registro) = aviso.registro else {
        return Ok(false);
    };
    let texto = |columna: &str| registro.get(columna).and_then(serde_json::Value::as_str);
    let (Some(id), Some(sitio_id)) = (texto("id"), texto("sitio_id")) else {
        log::warn!(
            "aviso en vivo: fila de {} con formato inesperado",
            tabla.cache
        );
        return Ok(false);
    };
    let (id, sitio_id) = (id.to_string(), sitio_id.to_string());
    let cierre = texto(tabla.columna_cierre).map(str::to_string);
    let quien_cierra = texto(tabla.columna_quien_cierra).map(str::to_string);

    let transaccion = conexion.unchecked_transaction()?;
    if let Some(hora_cierre) = cierre {
        transaccion.execute(&quitar_de_la_cache, params![id])?;
        (tabla.cerrar_propio)(&transaccion, &id, &hora_cierre, quien_cierra.as_deref())?;
    } else if !(tabla.guardar)(&transaccion, &sitio_id, registro)? {
        log::warn!(
            "aviso en vivo: fila de {} con formato inesperado",
            tabla.cache
        );
        return Ok(false);
    }
    transaccion.commit()?;
    Ok(basta_con_el_aviso)
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
                "resultado_acceso": "PERMITIDO",
                "updated_at": "2026-09-27T12:00:00.456789+00:00",
                "columna_que_no_usamos": 1
            }
        })
    }

    /// El escritorio: el camino más completo (Activos + historial).
    fn aplicar(
        conexion: &Connection,
        aviso: &serde_json::Value,
    ) -> Result<bool, SincronizacionError> {
        aplicar_cambio_en_vivo(conexion, aviso, PerfilDispositivo::Escritorio)
    }

    /// `(uuid, hora_salida, dispositivo_entrada_tipo)` de `historial_sitio`.
    fn historial(conexion: &Connection) -> Vec<(String, Option<String>, Option<String>)> {
        conexion
            .prepare(
                "SELECT uuid, hora_salida, dispositivo_entrada_tipo FROM historial_sitio ORDER BY uuid",
            )
            .unwrap()
            .query_map([], |fila| Ok((fila.get(0)?, fila.get(1)?, fila.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
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

        assert!(aplicar(&conexion, &aviso("INSERT", None)).unwrap());

        // La fecha queda en el formato único de la app, igual que al sincronizar.
        assert_eq!(
            remotos(&conexion),
            vec![("u-otro".to_string(), "2026-09-27T12:00:00Z".to_string())]
        );
    }

    #[test]
    fn el_mismo_aviso_repetido_no_duplica() {
        let conexion = base();
        aplicar(&conexion, &aviso("INSERT", None)).unwrap();
        aplicar(&conexion, &aviso("INSERT", None)).unwrap();

        assert_eq!(remotos(&conexion).len(), 1);
    }

    #[test]
    fn la_salida_lo_saca_de_adentro() {
        let conexion = base();
        aplicar(&conexion, &aviso("INSERT", None)).unwrap();

        let aplicado = aplicar(&conexion, &aviso("UPDATE", Some("2026-09-27T13:00:00Z"))).unwrap();

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

        aplicar(&conexion, &aviso("UPDATE", Some("2026-09-27T13:00:00Z"))).unwrap();

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
        aplicar(&conexion, &aviso("INSERT", None)).unwrap();

        let borrado = json!({"table": "ingresos", "operation": "DELETE", "id": "u-otro"});
        assert!(aplicar(&conexion, &borrado).unwrap());
        assert!(remotos(&conexion).is_empty());
    }

    #[test]
    fn otra_tabla_o_aviso_sin_datos_queda_para_la_sincronizacion() {
        let conexion = base();
        let otra_tabla = json!({"table": "empresas", "operation": "INSERT", "registro": {}});
        let sin_datos = json!({"table": "ingresos", "operation": "INSERT", "id": "x"});
        let raro = json!({"lo_que_sea": true});

        assert!(!aplicar(&conexion, &otra_tabla).unwrap());
        assert!(!aplicar(&conexion, &sin_datos).unwrap());
        assert!(!aplicar(&conexion, &raro).unwrap());
        assert!(remotos(&conexion).is_empty());
    }

    #[test]
    fn en_el_escritorio_el_aviso_escribe_solo_su_linea_del_historial() {
        let conexion = base();

        assert!(aplicar(&conexion, &aviso("INSERT", None)).unwrap());
        assert_eq!(
            historial(&conexion),
            vec![("u-otro".to_string(), None, None)]
        );

        assert!(aplicar(&conexion, &aviso("UPDATE", Some("2026-09-27T13:00:00Z"))).unwrap());
        assert_eq!(
            historial(&conexion),
            vec![(
                "u-otro".to_string(),
                Some("2026-09-27T13:00:00Z".to_string()),
                None
            )]
        );
    }

    #[test]
    fn el_aviso_no_mueve_la_marca_del_historial() {
        // La fila llega sin el tipo de dispositivo (el aviso no trae el JOIN):
        // el próximo pulso tiene que volver a traerla completa.
        let conexion = base();

        aplicar(&conexion, &aviso("INSERT", None)).unwrap();

        let marca: Option<String> = conexion
            .query_row(
                "SELECT historial_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
                [],
                |fila| fila.get(0),
            )
            .unwrap();
        assert_eq!(marca, None);
    }

    #[test]
    fn el_aviso_no_borra_el_tipo_de_dispositivo_que_ya_trajo_la_sincronizacion() {
        let conexion = base();
        aplicar(&conexion, &aviso("INSERT", None)).unwrap();
        conexion
            .execute(
                "UPDATE historial_sitio SET dispositivo_entrada_tipo = 'mobile' WHERE uuid = 'u-otro'",
                [],
            )
            .unwrap();

        aplicar(&conexion, &aviso("UPDATE", Some("2026-09-27T13:00:00Z"))).unwrap();

        assert_eq!(historial(&conexion)[0].2.as_deref(), Some("mobile"));
    }

    #[test]
    fn un_delete_tambien_lo_saca_del_historial() {
        let conexion = base();
        aplicar(&conexion, &aviso("INSERT", None)).unwrap();

        let borrado = json!({"table": "ingresos", "operation": "DELETE", "id": "u-otro"});
        assert!(aplicar(&conexion, &borrado).unwrap());
        assert!(historial(&conexion).is_empty());
    }

    #[test]
    fn el_movil_no_guarda_historial() {
        let conexion = base();

        assert!(
            aplicar_cambio_en_vivo(&conexion, &aviso("INSERT", None), PerfilDispositivo::Movil)
                .unwrap()
        );

        assert_eq!(remotos(&conexion).len(), 1);
        assert!(historial(&conexion).is_empty());
    }
    // ---- Proveedor, correo y KOF ----

    fn aviso_de(tabla: &str, operacion: &str, registro: serde_json::Value) -> serde_json::Value {
        json!({
            "table": tabla,
            "operation": operacion,
            "id": registro["id"].clone(),
            "sitio_id": "s1",
            "registro": registro,
        })
    }

    fn proveedor_del_otro_equipo(hora_salida: Option<&str>) -> serde_json::Value {
        json!({
            "id": "p-otro",
            "sitio_id": "s1",
            "dispositivo_entrada_id": "otro-equipo",
            "cedula": "112345678",
            "nombre": "JUAN PEREZ",
            "empresa_id": null,
            "empresa_nombre": "Maika",
            "placa": null,
            "gafete_numero": 7,
            "hora_entrada": "2026-10-18T12:00:00.123+00:00",
            "usuario_entrada_nombre": "Guarda",
            "hora_salida": hora_salida,
            "dispositivo_salida_id": null,
            "usuario_salida_nombre": hora_salida.map(|_| "Guarda 2"),
            "created_at": "2026-10-18T12:00:00.123+00:00",
            "updated_at": "2026-10-18T12:00:00.123+00:00"
        })
    }

    fn uuids(conexion: &Connection, tabla: &str) -> Vec<String> {
        conexion
            .prepare(&format!("SELECT uuid FROM {tabla} ORDER BY uuid"))
            .unwrap()
            .query_map([], |fila| fila.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// El gafete de un proveedor que entró por el otro equipo queda en la
    /// caché al instante: la verificación local del próximo ingreso ya lo
    /// ve sin preguntarle a la nube.
    #[test]
    fn un_proveedor_del_otro_equipo_queda_en_la_cache_al_instante() {
        let conexion = base();
        let aviso = aviso_de(
            "ingresos_proveedor",
            "INSERT",
            proveedor_del_otro_equipo(None),
        );

        // El móvil no guarda historial: el aviso basta.
        assert!(aplicar_cambio_en_vivo(&conexion, &aviso, PerfilDispositivo::Movil).unwrap());
        assert_eq!(
            uuids(&conexion, "ingresos_proveedor_remotos"),
            vec!["p-otro"]
        );
        let (gafete, hora): (i64, String) = conexion
            .query_row(
                "SELECT gafete_numero, hora_entrada FROM ingresos_proveedor_remotos",
                [],
                |fila| Ok((fila.get(0)?, fila.get(1)?)),
            )
            .unwrap();
        assert_eq!((gafete, hora.as_str()), (7, "2026-10-18T12:00:00Z"));
    }

    /// En el escritorio la caché queda igual de al día, pero se pide la
    /// sincronización de la tabla: su historial es sólo la copia de la nube.
    #[test]
    fn en_el_escritorio_la_cache_queda_al_dia_y_se_pide_sincronizar_el_historial() {
        let conexion = base();
        let aviso = aviso_de(
            "ingresos_proveedor",
            "INSERT",
            proveedor_del_otro_equipo(None),
        );

        assert!(!aplicar(&conexion, &aviso).unwrap());
        assert_eq!(
            uuids(&conexion, "ingresos_proveedor_remotos"),
            vec!["p-otro"]
        );
    }

    #[test]
    fn la_salida_del_proveedor_lo_saca_de_la_cache() {
        let conexion = base();
        let movil = PerfilDispositivo::Movil;
        let entrada = aviso_de(
            "ingresos_proveedor",
            "INSERT",
            proveedor_del_otro_equipo(None),
        );
        let salida = aviso_de(
            "ingresos_proveedor",
            "UPDATE",
            proveedor_del_otro_equipo(Some("2026-10-18T13:00:00+00:00")),
        );

        aplicar_cambio_en_vivo(&conexion, &entrada, movil).unwrap();
        assert!(aplicar_cambio_en_vivo(&conexion, &salida, movil).unwrap());

        assert!(uuids(&conexion, "ingresos_proveedor_remotos").is_empty());
    }

    /// Un ingreso por correo de ESTE equipo que el otro cerró: la salida
    /// queda registrada acá, sin esperar a la sincronización.
    #[test]
    fn la_salida_de_un_ingreso_por_correo_propio_se_registra_aca() {
        let conexion = base();
        conexion
            .execute_batch(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Guardia', 'hash', 'OPERADOR', 1);
                 INSERT INTO registro_ingresos_correo (
                     uuid, cedula, nombre, motivo, placa, gafete_numero,
                     fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre
                 ) VALUES (
                     'c-propio', '112345678', 'ANA SOLANO', 'Entrevista', NULL, 5,
                     '2026-10-18T12:00:00Z', 1, 'Guardia'
                 );",
            )
            .unwrap();
        let aviso = aviso_de(
            "ingresos_correo",
            "UPDATE",
            json!({
                "id": "c-propio",
                "sitio_id": "s1",
                "dispositivo_entrada_id": "este-equipo",
                "cedula": "112345678",
                "nombre": "ANA SOLANO",
                "motivo": "Entrevista",
                "placa": null,
                "gafete_numero": 5,
                "hora_entrada": "2026-10-18T12:00:00+00:00",
                "usuario_entrada_nombre": "Guardia",
                "hora_salida": "2026-10-18T13:30:00.5+00:00",
                "dispositivo_salida_id": "otro-equipo",
                "usuario_salida_nombre": "Guarda 2"
            }),
        );

        assert!(aplicar_cambio_en_vivo(&conexion, &aviso, PerfilDispositivo::Movil).unwrap());

        let (salida, quien): (String, String) = conexion
            .query_row(
                "SELECT fecha_hora_salida, usuario_salida_nombre FROM registro_ingresos_correo",
                [],
                |fila| Ok((fila.get(0)?, fila.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            (salida.as_str(), quien.as_str()),
            ("2026-10-18T13:30:00Z", "Guarda 2")
        );
        assert!(uuids(&conexion, "ingresos_correo_remotos").is_empty());
    }

    #[test]
    fn un_prestamo_kof_del_otro_equipo_queda_en_la_cache_y_un_delete_lo_saca() {
        let conexion = base();
        let movil = PerfilDispositivo::Movil;
        let prestamo = json!({
            "id": "k-otro",
            "sitio_id": "s1",
            "dispositivo_entrega_id": "otro-equipo",
            "encargado_id": "e1",
            "encargado_nombre": "LUIS SOTO",
            "encargado_codigo_empleado": "5040099",
            "gafete_numero": 12,
            "hora_entrega": "2026-10-18T12:00:00+00:00",
            "usuario_entrega_nombre": "Guarda",
            "hora_devolucion": null,
            "dispositivo_devolucion_id": null,
            "usuario_devolucion_nombre": null
        });

        let entrega = aviso_de("prestamos_gafete_provisional", "INSERT", prestamo);
        assert!(aplicar_cambio_en_vivo(&conexion, &entrega, movil).unwrap());
        assert_eq!(
            uuids(&conexion, "prestamos_gafete_provisional_remotos"),
            vec!["k-otro"]
        );

        let borrado =
            json!({"table": "prestamos_gafete_provisional", "operation": "DELETE", "id": "k-otro"});
        assert!(aplicar_cambio_en_vivo(&conexion, &borrado, movil).unwrap());
        assert!(uuids(&conexion, "prestamos_gafete_provisional_remotos").is_empty());
    }

    /// Un aviso viejo, de antes de la migración que agrega la fila a estas
    /// tablas, o una fila que no se puede leer: queda para la sincronización.
    #[test]
    fn sin_fila_o_con_una_fila_rara_las_otras_clases_quedan_para_la_sincronizacion() {
        let conexion = base();
        let movil = PerfilDispositivo::Movil;
        let viejo = json!({"table": "ingresos_correo", "operation": "INSERT", "id": "x"});
        let raro = aviso_de(
            "ingresos_proveedor",
            "INSERT",
            json!({"id": "x", "sitio_id": "s1"}),
        );

        assert!(!aplicar_cambio_en_vivo(&conexion, &viejo, movil).unwrap());
        assert!(!aplicar_cambio_en_vivo(&conexion, &raro, movil).unwrap());
        assert!(uuids(&conexion, "ingresos_proveedor_remotos").is_empty());
    }
}

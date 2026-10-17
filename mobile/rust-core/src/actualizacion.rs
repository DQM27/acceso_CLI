//! Reglas del actualizador de la app Android (ver
//! `docs/actualizaciones-android.md`): leer y validar el manifiesto que
//! publica cada release (`android/latest.json`) y decidir si hay una versión
//! nueva y si es obligatoria. Kotlin sólo descarga, verifica la firma e
//! instala; no decide nada de esto.

use serde_json::{Map, Value};
use url::Url;

use crate::error::{NucleoError, interno};

/// Versión del formato del manifiesto. Sube sólo si cambia el significado o
/// el nombre de un campo existente; agregar uno nuevo no la cambia.
const ESQUEMA_MANIFIESTO: u64 = 1;

/// Tope del APK: el real ronda los 50 MB. Un manifiesto que anuncie más es un
/// error de publicación, no una descarga a intentar.
const TAMANO_MAXIMO_BYTES: i64 = 300 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ManifiestoActualizacion {
    pub version_code: i64,
    pub version_name: String,
    /// Dirección HTTPS del APK, en el mismo servidor que el manifiesto.
    pub url: String,
    /// SHA-256 del APK en hexadecimal, minúsculas.
    pub sha256: String,
    pub tamano_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum NovedadActualizacion {
    AlDia,
    /// `obligatoria`: la versión instalada quedó por debajo de
    /// `version_minima_code` del manifiesto (ya no es compatible).
    Disponible {
        manifiesto: ManifiestoActualizacion,
        obligatoria: bool,
    },
}

/// Valida el manifiesto descargado de `url_manifiesto` y lo compara con el
/// `versionCode` instalado.
///
/// Rechaza un manifiesto que no se pueda usar con seguridad: el APK sólo se
/// baja por HTTPS y del mismo servidor que publicó el manifiesto, así que uno
/// alterado no puede mandar al teléfono a descargar de otro lado. Un
/// manifiesto inválido es un error de publicación: llega como
/// `NucleoError::Interno` con el detalle en el registro técnico.
#[uniffi::export]
pub fn evaluar_manifiesto_actualizacion(
    json: String,
    url_manifiesto: String,
    version_code_instalada: i64,
) -> Result<NovedadActualizacion, NucleoError> {
    let (manifiesto, version_minima_code) =
        leer_manifiesto(&json, &url_manifiesto).map_err(|motivo| NucleoError::Interno {
            mensaje: interno(format!("manifiesto de actualización inválido: {motivo}")),
        })?;
    Ok(novedad(
        manifiesto,
        version_minima_code,
        version_code_instalada,
    ))
}

/// Android tampoco instala una versión igual o menor a la instalada: sólo un
/// `versionCode` mayor es novedad.
fn novedad(
    manifiesto: ManifiestoActualizacion,
    version_minima_code: i64,
    version_code_instalada: i64,
) -> NovedadActualizacion {
    if manifiesto.version_code > version_code_instalada {
        NovedadActualizacion::Disponible {
            manifiesto,
            obligatoria: version_code_instalada < version_minima_code,
        }
    } else {
        NovedadActualizacion::AlDia
    }
}

fn leer_manifiesto(
    json: &str,
    url_manifiesto: &str,
) -> Result<(ManifiestoActualizacion, i64), String> {
    let valor: Value = serde_json::from_str(json).map_err(|e| format!("JSON inválido: {e}"))?;
    let campos = valor.as_object().ok_or("no es un objeto JSON")?;

    let esquema = campos
        .get("esquema")
        .and_then(Value::as_u64)
        .ok_or("falta esquema")?;
    if esquema != ESQUEMA_MANIFIESTO {
        return Err(format!("esquema {esquema} no soportado"));
    }
    let version_code = entero(campos, "version_code")?;
    if version_code <= 0 {
        return Err("version_code inválido".into());
    }
    let version_name = texto(campos, "version_name")?;
    if version_name.trim().is_empty() {
        return Err("version_name vacío".into());
    }
    let sha256 = texto(campos, "sha256")?;
    if sha256.len() != 64
        || !sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("sha256 inválido".into());
    }
    let tamano_bytes = entero(campos, "tamano_bytes")?;
    if !(1..=TAMANO_MAXIMO_BYTES).contains(&tamano_bytes) {
        return Err("tamano_bytes fuera de rango".into());
    }
    let version_minima_code = match campos.get("version_minima_code") {
        None | Some(Value::Null) => 0,
        Some(_) => entero(campos, "version_minima_code")?,
    };
    if !(0..=version_code).contains(&version_minima_code) {
        return Err("version_minima_code fuera de rango".into());
    }
    let url = texto(campos, "url")?;
    validar_origen(&url, url_manifiesto)?;

    Ok((
        ManifiestoActualizacion {
            version_code,
            version_name,
            url,
            sha256,
            tamano_bytes,
        },
        version_minima_code,
    ))
}

fn validar_origen(url_apk: &str, url_manifiesto: &str) -> Result<(), String> {
    let apk = url_segura(url_apk).ok_or("url del APK inválida")?;
    let origen = url_segura(url_manifiesto).ok_or("url del manifiesto inválida")?;
    if apk.scheme() != "https" {
        return Err("el APK no se sirve por HTTPS".into());
    }
    // `Url` ya normaliza el host a minúsculas.
    if apk.host_str() != origen.host_str()
        || apk.port_or_known_default() != origen.port_or_known_default()
    {
        return Err("el APK no está en el mismo servidor que el manifiesto".into());
    }
    Ok(())
}

fn url_segura(texto: &str) -> Option<Url> {
    Url::parse(texto).ok().filter(|url| {
        url.host_str().is_some() && url.username().is_empty() && url.password().is_none()
    })
}

fn entero(campos: &Map<String, Value>, nombre: &str) -> Result<i64, String> {
    campos
        .get(nombre)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("falta {nombre} o no es entero"))
}

fn texto(campos: &Map<String, Value>, nombre: &str) -> Result<String, String> {
    campos
        .get(nombre)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("falta {nombre} o no es texto"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL_MANIFIESTO: &str = "https://lt.megabrisas.com/android/latest.json";

    fn manifiesto_json(cambios: &[(&str, Value)]) -> String {
        let mut valor = serde_json::json!({
            "esquema": 1,
            "version_code": 33,
            "version_name": "1.4.11",
            "url": "https://lt.megabrisas.com/android/Lattis-mobile-v1.4.11-0123456789ab.apk",
            "sha256": "a".repeat(64),
            "tamano_bytes": 50_000_000,
            "version_minima_code": 0,
        });
        let objeto = valor.as_object_mut().expect("objeto");
        for (clave, nuevo) in cambios {
            if nuevo.is_null() {
                objeto.remove(*clave);
            } else {
                objeto.insert((*clave).to_owned(), nuevo.clone());
            }
        }
        valor.to_string()
    }

    fn evaluar(json: &str, instalada: i64) -> Result<NovedadActualizacion, NucleoError> {
        evaluar_manifiesto_actualizacion(json.to_owned(), URL_MANIFIESTO.to_owned(), instalada)
    }

    fn rechaza(cambios: &[(&str, Value)]) {
        let resultado = evaluar(&manifiesto_json(cambios), 32);
        assert!(
            matches!(resultado, Err(NucleoError::Interno { .. })),
            "{cambios:?} -> {resultado:?}"
        );
    }

    #[test]
    fn version_mayor_es_novedad_opcional() {
        let NovedadActualizacion::Disponible {
            manifiesto,
            obligatoria,
        } = evaluar(&manifiesto_json(&[]), 32).expect("válido")
        else {
            panic!("se esperaba novedad");
        };
        assert!(!obligatoria);
        assert_eq!(manifiesto.version_code, 33);
        assert_eq!(manifiesto.version_name, "1.4.11");
        assert_eq!(manifiesto.tamano_bytes, 50_000_000);
    }

    #[test]
    fn version_igual_o_menor_esta_al_dia() {
        assert_eq!(
            evaluar(&manifiesto_json(&[]), 33).expect("válido"),
            NovedadActualizacion::AlDia
        );
        assert_eq!(
            evaluar(&manifiesto_json(&[]), 40).expect("válido"),
            NovedadActualizacion::AlDia
        );
    }

    #[test]
    fn por_debajo_de_la_minima_es_obligatoria() {
        let json = manifiesto_json(&[("version_minima_code", Value::from(32))]);
        assert!(matches!(
            evaluar(&json, 31).expect("válido"),
            NovedadActualizacion::Disponible {
                obligatoria: true,
                ..
            }
        ));
        assert!(matches!(
            evaluar(&json, 32).expect("válido"),
            NovedadActualizacion::Disponible {
                obligatoria: false,
                ..
            }
        ));
    }

    #[test]
    fn version_minima_ausente_o_nula_vale_cero() {
        let json = manifiesto_json(&[("version_minima_code", Value::Null)]);
        assert!(matches!(
            evaluar(&json, 1).expect("válido"),
            NovedadActualizacion::Disponible {
                obligatoria: false,
                ..
            }
        ));
    }

    #[test]
    fn rechaza_json_que_no_es_objeto() {
        assert!(evaluar("no es json", 32).is_err());
        assert!(evaluar("[1,2]", 32).is_err());
    }

    #[test]
    fn rechaza_esquema_desconocido_o_ausente() {
        rechaza(&[("esquema", Value::from(2))]);
        rechaza(&[("esquema", Value::Null)]);
    }

    #[test]
    fn rechaza_version_code_invalido() {
        rechaza(&[("version_code", Value::from(0))]);
        rechaza(&[("version_code", Value::from("33"))]);
    }

    #[test]
    fn rechaza_version_name_vacio() {
        rechaza(&[("version_name", Value::from("  "))]);
    }

    #[test]
    fn rechaza_sha_mal_formado() {
        rechaza(&[("sha256", Value::from("a".repeat(63)))]);
        rechaza(&[("sha256", Value::from("A".repeat(64)))]);
        rechaza(&[("sha256", Value::from("g".repeat(64)))]);
    }

    #[test]
    fn rechaza_tamano_fuera_de_rango() {
        rechaza(&[("tamano_bytes", Value::from(0))]);
        rechaza(&[("tamano_bytes", Value::from(TAMANO_MAXIMO_BYTES + 1))]);
    }

    #[test]
    fn rechaza_version_minima_fuera_de_rango() {
        rechaza(&[("version_minima_code", Value::from(34))]);
        rechaza(&[("version_minima_code", Value::from(-1))]);
    }

    #[test]
    fn rechaza_apk_sin_https_o_en_otro_servidor() {
        rechaza(&[("url", Value::from("http://lt.megabrisas.com/android/x.apk"))]);
        rechaza(&[("url", Value::from("https://otro.example.com/x.apk"))]);
        rechaza(&[(
            "url",
            Value::from("https://lt.megabrisas.com.example.com/x.apk"),
        )]);
        rechaza(&[("url", Value::from("https://lt.megabrisas.com:8443/x.apk"))]);
    }

    #[test]
    fn rechaza_url_con_credenciales_o_mal_formada() {
        rechaza(&[(
            "url",
            Value::from("https://usuario@lt.megabrisas.com/x.apk"),
        )]);
        rechaza(&[("url", Value::from("no es una url"))]);
    }

    #[test]
    fn acepta_host_con_otra_capitalizacion() {
        let json = manifiesto_json(&[(
            "url",
            Value::from("https://Lt.MegaBrisas.com/android/x.apk"),
        )]);
        assert!(evaluar(&json, 32).is_ok());
    }
}

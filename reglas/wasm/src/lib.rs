//! Las reglas de `control_acceso_reglas`, expuestas a JavaScript con
//! `wasm-bindgen`. Las usan el panel web (para avisar mientras se llena el
//! formulario) y la Edge Function `admin-crear-contratista` (para decidir si
//! guarda). Es el mismo código que corre en escritorio y teléfono.
//!
//! Los tipos de ingreso viajan con los códigos de la nube (`"PRAIND"`,
//! `"IN_HOUSE"`, `"POR_CORREO"`, `"SWAT"`) y las fechas como `"AAAA-MM-DD"`.

use chrono::NaiveDate;
use control_acceso_reglas::cedula::Cedula;
use control_acceso_reglas::contratista::{self, DatosContratista, EstadoAnterior};
use control_acceso_reglas::tipo_ingreso::TipoIngreso;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

mod huella {
    include!(concat!(env!("OUT_DIR"), "/huella.rs"));
}

/// Huella de las fuentes con las que se compiló este paquete (ver
/// `build.rs`). Los tests del panel la comparan con las fuentes del repo.
#[wasm_bindgen(js_name = huellaFuentes)]
pub fn huella_fuentes() -> String {
    huella::HUELLA.to_string()
}

fn tipo(codigo: &str) -> Option<TipoIngreso> {
    TipoIngreso::from_str_sql(codigo)
}

/// Cédula en su forma única (sin guiones, espacios ni el cero del TSE), o
/// `undefined` si no es una cédula de contratista (9 a 13 dígitos).
#[wasm_bindgen(js_name = normalizarCedulaContratista)]
pub fn normalizar_cedula_contratista(texto: &str) -> Option<String> {
    Cedula::normalizar(texto)
        .ok()
        .filter(Cedula::es_nacional_o_de_extranjero)
        .map(Cedula::into_string)
}

/// ¿Este tipo (o ser personal de ruta) exige fecha de PRAIND?
#[wasm_bindgen(js_name = requierePraind)]
pub fn requiere_praind(tipo_ingreso: &str, personal_ruta: bool) -> bool {
    tipo(tipo_ingreso).is_some_and(|tipo| contratista::requiere_praind_de(tipo, personal_ruta))
}

/// ¿Este tipo exige gafete al entrar?
#[wasm_bindgen(js_name = requiereGafete)]
pub fn requiere_gafete(tipo_ingreso: &str, personal_ruta: bool) -> bool {
    tipo(tipo_ingreso).is_some_and(|tipo| contratista::requiere_gafete_de(tipo, personal_ruta))
}

/// ¿Este tipo admite la casilla "personal de ruta"?
#[wasm_bindgen(js_name = admitePersonalRuta)]
pub fn admite_personal_ruta(tipo_ingreso: &str) -> bool {
    tipo(tipo_ingreso).is_some_and(contratista::admite_personal_ruta)
}

/// Los tipos que se pueden elegir para un contratista nuevo, en el orden de
/// siempre.
#[wasm_bindgen(js_name = tiposIngresoSeleccionables)]
pub fn tipos_ingreso_seleccionables() -> Vec<String> {
    TipoIngreso::ALL
        .into_iter()
        .filter(|tipo| contratista::tipo_ingreso_seleccionable(*tipo))
        .map(|tipo| tipo.as_str_sql().to_string())
        .collect()
}

#[derive(Deserialize)]
struct EntradaContratista {
    cedula: String,
    nombre: String,
    tipo_ingreso: String,
    fecha_vencimiento_praind: Option<String>,
    #[serde(default)]
    es_personal_ruta: bool,
    #[serde(default = "verdadero")]
    tiene_acceso: bool,
}

fn verdadero() -> bool {
    true
}

#[derive(Serialize)]
struct SalidaContratista {
    cedula: String,
    nombre: String,
    tipo_ingreso: &'static str,
    fecha_vencimiento_praind: Option<String>,
    es_personal_ruta: bool,
    tiene_acceso: bool,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Resultado {
    Valido {
        ok: bool,
        contratista: SalidaContratista,
    },
    Invalido {
        ok: bool,
        codigo: &'static str,
        mensaje: String,
    },
}

/// Al editar: lo que el contratista tenía guardado (decide qué reglas se
/// vuelven a revisar, ver `validar_contratista` del crate de reglas).
#[derive(Deserialize)]
struct EntradaAnterior {
    tipo_ingreso: String,
    #[serde(default)]
    es_personal_ruta: bool,
    fecha_vencimiento_praind: Option<String>,
}

fn invalido(codigo: &'static str, mensaje: &str) -> Resultado {
    Resultado::Invalido {
        ok: false,
        codigo,
        mensaje: mensaje.to_string(),
    }
}

fn fecha(texto: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texto, "%Y-%m-%d").ok()
}

/// Valida un contratista con todas las reglas de criterio
/// (`control_acceso_reglas::contratista::validar_contratista`).
///
/// `datos`: `{ cedula, nombre, tipo_ingreso, fecha_vencimiento_praind,
/// es_personal_ruta?, tiene_acceso? }`. `hoy`: la fecha de Costa Rica,
/// `"AAAA-MM-DD"`. `anterior` (sólo al editar, si no `undefined`/`null`): lo
/// que tenía guardado, `{ tipo_ingreso, es_personal_ruta?,
/// fecha_vencimiento_praind }`; con él, a alguien con el PRAIND ya vencido se
/// le puede corregir el nombre o quitar el acceso sin cambiar la fecha.
///
/// Devuelve `{ ok: true, contratista }` con los datos normalizados, o
/// `{ ok: false, codigo, mensaje }` con el primer motivo que falla.
#[wasm_bindgen(js_name = validarContratista)]
pub fn validar_contratista(
    datos: JsValue,
    hoy: &str,
    anterior: JsValue,
) -> Result<JsValue, JsError> {
    let anterior = if anterior.is_undefined() || anterior.is_null() {
        Ok(None)
    } else {
        serde_wasm_bindgen::from_value::<EntradaAnterior>(anterior).map(Some)
    };
    let resultado = match (
        serde_wasm_bindgen::from_value::<EntradaContratista>(datos),
        anterior,
    ) {
        (Ok(entrada), Ok(anterior)) => validar(&entrada, hoy, anterior.as_ref()),
        _ => invalido("datos_invalidos", "Faltan datos del contratista"),
    };
    // `None` viaja como `null` (no `undefined`): es lo que esperan el panel y
    // la Edge Function, y lo que guarda Postgres.
    let serializador = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    resultado
        .serialize(&serializador)
        .map_err(|error| JsError::new(&error.to_string()))
}

fn validar(
    entrada: &EntradaContratista,
    hoy: &str,
    anterior: Option<&EntradaAnterior>,
) -> Resultado {
    let Some(hoy) = fecha(hoy) else {
        return invalido("fecha_invalida", "La fecha de hoy no es válida");
    };
    // Lo guardado viene de la base: un tipo o una fecha que no se entienden
    // no deberían pasar nunca, pero si pasan se valida como un alta (lo más
    // estricto) en vez de fallar.
    let anterior = anterior.and_then(|previo| {
        Some(EstadoAnterior {
            tipo_ingreso: tipo(&previo.tipo_ingreso)?,
            es_personal_ruta: previo.es_personal_ruta,
            fecha_vencimiento_praind: match previo.fecha_vencimiento_praind.as_deref() {
                None | Some("") => None,
                Some(texto) => Some(fecha(texto)?),
            },
        })
    });
    let Some(tipo_ingreso) = tipo(&entrada.tipo_ingreso) else {
        return invalido("tipo_ingreso_invalido", "El tipo de ingreso no es válido");
    };
    let fecha_vencimiento_praind = match entrada.fecha_vencimiento_praind.as_deref() {
        None | Some("") => None,
        Some(texto) => match fecha(texto) {
            Some(fecha) => Some(fecha),
            None => return invalido("fecha_invalida", "La fecha de PRAIND no es válida"),
        },
    };

    match contratista::validar_contratista(
        DatosContratista {
            cedula: &entrada.cedula,
            nombre: &entrada.nombre,
            tipo_ingreso,
            fecha_vencimiento_praind,
            es_personal_ruta: entrada.es_personal_ruta,
            tiene_acceso: entrada.tiene_acceso,
        },
        anterior.as_ref(),
        hoy,
    ) {
        Ok(valido) => Resultado::Valido {
            ok: true,
            contratista: SalidaContratista {
                cedula: valido.cedula,
                nombre: valido.nombre,
                tipo_ingreso: valido.tipo_ingreso.as_str_sql(),
                fecha_vencimiento_praind: valido
                    .fecha_vencimiento_praind
                    .map(|fecha| fecha.format("%Y-%m-%d").to_string()),
                es_personal_ruta: valido.es_personal_ruta,
                tiene_acceso: valido.tiene_acceso,
            },
        },
        Err(error) => invalido(error.codigo(), &error.mensaje()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrada(tipo: &str, praind: Option<&str>) -> EntradaContratista {
        EntradaContratista {
            cedula: "1-1111-1111".into(),
            nombre: "ana solano".into(),
            tipo_ingreso: tipo.into(),
            fecha_vencimiento_praind: praind.map(Into::into),
            es_personal_ruta: false,
            tiene_acceso: true,
        }
    }

    #[test]
    fn traduce_los_codigos_de_la_nube() {
        assert!(requiere_praind("IN_HOUSE", false));
        assert!(!requiere_praind("SWAT", false));
        assert!(!requiere_praind("DESCONOCIDO", false));
        assert_eq!(
            tipos_ingreso_seleccionables(),
            vec!["PRAIND", "IN_HOUSE", "SWAT"]
        );
        assert_eq!(
            normalizar_cedula_contratista("01-1111-1111").as_deref(),
            Some("111111111")
        );
        assert_eq!(normalizar_cedula_contratista("AB123"), None);
    }

    #[test]
    fn valida_con_las_reglas_del_nucleo() {
        assert!(matches!(
            validar(&entrada("SWAT", None), "2026-10-04", None),
            Resultado::Valido { contratista, .. } if contratista.cedula == "111111111" && contratista.nombre == "ANA SOLANO"
        ));
        assert!(matches!(
            validar(&entrada("POR_CORREO", None), "2026-10-04", None),
            Resultado::Invalido {
                codigo: "tipo_ingreso_retirado",
                ..
            }
        ));
        assert!(matches!(
            validar(&entrada("PRAIND", Some("2026-10-03")), "2026-10-04", None),
            Resultado::Invalido {
                codigo: "praind_vencido",
                ..
            }
        ));
        assert!(matches!(
            validar(&entrada("PRAIND", Some("ayer")), "2026-10-04", None),
            Resultado::Invalido {
                codigo: "fecha_invalida",
                ..
            }
        ));
    }

    #[test]
    fn al_editar_respeta_lo_que_ya_tenia_guardado() {
        let anterior = |tipo: &str, praind: Option<&str>| EntradaAnterior {
            tipo_ingreso: tipo.into(),
            es_personal_ruta: false,
            fecha_vencimiento_praind: praind.map(Into::into),
        };
        // Un contratista viejo "por correo" se puede corregir sin cambiarle el tipo...
        assert!(matches!(
            validar(
                &entrada("POR_CORREO", None),
                "2026-10-04",
                Some(&anterior("POR_CORREO", None))
            ),
            Resultado::Valido { .. }
        ));
        // ...y con el PRAIND ya vencido se le corrige el nombre sin tocar la fecha.
        assert!(matches!(
            validar(
                &entrada("PRAIND", Some("2026-10-03")),
                "2026-10-04",
                Some(&anterior("PRAIND", Some("2026-10-03")))
            ),
            Resultado::Valido { .. }
        ));
        // Pero si cambia la fecha, tiene que quedar vigente.
        assert!(matches!(
            validar(
                &entrada("PRAIND", Some("2026-10-02")),
                "2026-10-04",
                Some(&anterior("PRAIND", Some("2026-10-03")))
            ),
            Resultado::Invalido {
                codigo: "praind_vencido",
                ..
            }
        ));
    }
}

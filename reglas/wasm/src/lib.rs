//! Las reglas de `control_acceso_reglas`, expuestas a JavaScript con
//! `wasm-bindgen`. Las usan el panel web y la web de visitas (para avisar
//! mientras se llena el formulario) y la Edge Function
//! `admin-crear-contratista` (para decidir si guarda). Es el mismo código
//! que corre en escritorio y teléfono.
//!
//! Los tipos de ingreso viajan con los códigos de la nube (`"PRAIND"`,
//! `"IN_HOUSE"`, `"POR_CORREO"`, `"SWAT"`) y las fechas como `"AAAA-MM-DD"`.

use chrono::NaiveDate;
use control_acceso_reglas::cedula::Cedula;
use control_acceso_reglas::cita::{self, CitaNueva, VisitanteNuevo};
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

// ---- Nombres (todas las interfaces) ----

/// Todo nombre de persona o empresa, como se guarda: espacios de más fuera
/// y en mayúscula (`nombre::nombre_en_mayusculas`).
#[wasm_bindgen(js_name = nombreEnMayusculas)]
pub fn nombre_en_mayusculas(texto: &str) -> String {
    control_acceso_reglas::nombre::nombre_en_mayusculas(texto)
}

/// Mientras se escribe en un campo de nombre: sólo mayúscula, sin tocar los
/// espacios (`nombre::nombre_mientras_se_escribe`).
#[wasm_bindgen(js_name = nombreMientrasSeEscribe)]
pub fn nombre_mientras_se_escribe(texto: &str) -> String {
    control_acceso_reglas::nombre::nombre_mientras_se_escribe(texto)
}

// ---- Visitas (web de visitas) ----

/// Documento de un visitante en su forma única (la misma que reconoce el
/// check-in de la portería), o `undefined` si no es válido. Admite
/// pasaportes con letras.
#[wasm_bindgen(js_name = normalizarDocumentoVisitante)]
pub fn normalizar_documento_visitante(texto: &str) -> Option<String> {
    cita::documento_de_visitante(texto)
        .ok()
        .map(Cedula::into_string)
}

#[derive(Deserialize)]
struct EntradaVisitante {
    #[serde(default)]
    nombre: String,
    #[serde(default)]
    cedula: String,
    #[serde(default)]
    empresa: Option<String>,
    #[serde(default)]
    placa_vehiculo: Option<String>,
}

#[derive(Deserialize)]
struct EntradaCita {
    #[serde(default)]
    fecha_desde: String,
    #[serde(default)]
    fecha_hasta: String,
    #[serde(default)]
    hora_estimada: Option<String>,
    #[serde(default)]
    motivo: Option<String>,
    #[serde(default)]
    sitios: Vec<String>,
    #[serde(default)]
    visitantes: Vec<EntradaVisitante>,
}

#[derive(Serialize)]
struct SalidaVisitante {
    nombre: String,
    cedula: String,
    empresa: Option<String>,
    placa_vehiculo: Option<String>,
}

#[derive(Serialize)]
struct SalidaCita {
    fecha_desde: String,
    fecha_hasta: String,
    /// `"HH:MM"`.
    hora_estimada: Option<String>,
    motivo: Option<String>,
    sitios: Vec<String>,
    visitantes: Vec<SalidaVisitante>,
}

#[derive(Serialize)]
struct SalidaErrorCampo {
    campo: String,
    mensaje: String,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ResultadoCita {
    Valida {
        ok: bool,
        cita: SalidaCita,
    },
    Invalida {
        ok: bool,
        errores: Vec<SalidaErrorCampo>,
    },
}

fn invalida(campo: &str, mensaje: &str) -> ResultadoCita {
    ResultadoCita::Invalida {
        ok: false,
        errores: vec![SalidaErrorCampo {
            campo: campo.to_string(),
            mensaje: mensaje.to_string(),
        }],
    }
}

/// Valida una cita nueva con las reglas del núcleo
/// (`control_acceso_reglas::cita::validar_cita`).
///
/// `datos`: `{ fecha_desde, fecha_hasta, hora_estimada?, motivo?, sitios,
/// visitantes: [{ nombre, cedula, empresa?, placa_vehiculo? }] }`, tal cual
/// del formulario. `hoy`: la fecha de Costa Rica, `"AAAA-MM-DD"`.
///
/// Devuelve `{ ok: true, cita }` con los datos normalizados (cédula en su
/// forma única, textos recortados, vacíos como `null`), o `{ ok: false,
/// errores: [{ campo, mensaje }] }` con todos los problemas.
#[wasm_bindgen(js_name = validarCita)]
pub fn validar_cita(datos: JsValue, hoy: &str) -> Result<JsValue, JsError> {
    let resultado = match serde_wasm_bindgen::from_value::<EntradaCita>(datos) {
        Ok(entrada) => validar_cita_entrada(entrada, hoy),
        Err(_) => invalida("formulario", "Faltan datos de la visita"),
    };
    let serializador = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    resultado
        .serialize(&serializador)
        .map_err(|error| JsError::new(&error.to_string()))
}

fn validar_cita_entrada(entrada: EntradaCita, hoy: &str) -> ResultadoCita {
    let Some(hoy) = fecha(hoy) else {
        return invalida("formulario", "La fecha de hoy no es válida");
    };
    let nueva = CitaNueva {
        fecha_desde: entrada.fecha_desde,
        fecha_hasta: entrada.fecha_hasta,
        hora_estimada: entrada.hora_estimada,
        motivo: entrada.motivo,
        sitios: entrada.sitios,
        visitantes: entrada
            .visitantes
            .into_iter()
            .map(|visitante| VisitanteNuevo {
                nombre: visitante.nombre,
                cedula: visitante.cedula,
                empresa: visitante.empresa,
                placa_vehiculo: visitante.placa_vehiculo,
            })
            .collect(),
    };
    match cita::validar_cita(&nueva, hoy) {
        Ok(valida) => ResultadoCita::Valida {
            ok: true,
            cita: SalidaCita {
                fecha_desde: valida.fecha_desde.format("%Y-%m-%d").to_string(),
                fecha_hasta: valida.fecha_hasta.format("%Y-%m-%d").to_string(),
                hora_estimada: valida
                    .hora_estimada
                    .map(|hora| hora.format("%H:%M").to_string()),
                motivo: valida.motivo,
                sitios: valida.sitios,
                visitantes: valida
                    .visitantes
                    .into_iter()
                    .map(|visitante| SalidaVisitante {
                        nombre: visitante.nombre,
                        cedula: visitante.cedula,
                        empresa: visitante.empresa,
                        placa_vehiculo: visitante.placa_vehiculo,
                    })
                    .collect(),
            },
        },
        Err(errores) => ResultadoCita::Invalida {
            ok: false,
            errores: errores
                .into_iter()
                .map(|error| SalidaErrorCampo {
                    campo: error.campo,
                    mensaje: error.mensaje,
                })
                .collect(),
        },
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

    #[test]
    fn valida_una_visita_con_las_reglas_del_nucleo() {
        let entrada = |cedula_dos: &str| EntradaCita {
            fecha_desde: "2026-10-05".into(),
            fecha_hasta: "2026-10-05".into(),
            hora_estimada: Some("08:00".into()),
            motivo: Some(" ".into()),
            sitios: vec!["s1".into()],
            visitantes: vec![
                EntradaVisitante {
                    nombre: "Ana Mora".into(),
                    cedula: "01-0847-0293".into(),
                    empresa: None,
                    placa_vehiculo: None,
                },
                EntradaVisitante {
                    nombre: "Luis Rojas".into(),
                    cedula: cedula_dos.into(),
                    empresa: Some("ACME".into()),
                    placa_vehiculo: Some("bcd123".into()),
                },
            ],
        };
        assert!(matches!(
            validar_cita_entrada(entrada("204560789"), "2026-10-05"),
            ResultadoCita::Valida { cita, .. }
                if cita.visitantes[0].cedula == "108470293"
                    && cita.visitantes[1].placa_vehiculo.as_deref() == Some("BCD123")
                    && cita.motivo.is_none()
                    && cita.hora_estimada.as_deref() == Some("08:00")
        ));
        assert!(matches!(
            validar_cita_entrada(entrada("108470293"), "2026-10-05"),
            ResultadoCita::Invalida { errores, .. }
                if errores.len() == 1 && errores[0].campo == "visitantes.1.cedula"
        ));
        assert_eq!(
            normalizar_documento_visitante(" ab-123 ").as_deref(),
            Some("AB123")
        );
        assert_eq!(normalizar_documento_visitante("1-2"), None);
    }
}

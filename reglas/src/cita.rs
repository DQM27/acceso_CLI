//! Reglas para agendar una visita (cita): las fechas, los límites, quién
//! viene y en qué. Las usa la web de visitas (compiladas a WebAssembly) para
//! avisar mientras se llena el formulario, y las podrá usar el teléfono. La
//! decisión final de guardar la toma la base (`crear_cita_anfitrion`), que
//! revisa lo mismo del lado del servidor.
//!
//! La cédula de cada persona pasa por [`Cedula::normalizar`], la misma forma
//! única que usa el check-in de la portería: así una persona escrita de dos
//! maneras (`01-0847-0293` y `108470293`) se detecta repetida en el
//! formulario, y nadie agenda un documento que la portería después no
//! reconocería (más de 20 caracteres).
//!
//! A diferencia de [`crate::contratista::validar_contratista`], acá se
//! devuelven **todos** los problemas a la vez, cada uno con su campo: el
//! formulario los marca juntos.

use chrono::{NaiveDate, NaiveTime};

use crate::cedula::{Cedula, CedulaInvalida};

pub const MAX_VISITANTES: usize = 50;
pub const MAX_SITIOS: usize = 100;
pub const MAX_MOTIVO: usize = 1000;
pub const MAX_NOMBRE: usize = 150;
pub const MIN_NOMBRE: usize = 2;
pub const MAX_EMPRESA: usize = 150;
pub const MAX_PLACA: usize = 20;
/// Un documento de menos caracteres es un error de tipeo.
pub const MIN_DOCUMENTO: usize = 3;

/// Lo que llega del formulario, tal cual (sin recortar).
#[derive(Debug, Clone, Default)]
pub struct VisitanteNuevo {
    pub nombre: String,
    pub cedula: String,
    pub empresa: Option<String>,
    pub placa_vehiculo: Option<String>,
}

/// Lo que llega del formulario. Las fechas como `"AAAA-MM-DD"` y la hora
/// como `"HH:MM"` (vacía = sin hora): se validan acá.
#[derive(Debug, Clone, Default)]
pub struct CitaNueva {
    pub fecha_desde: String,
    pub fecha_hasta: String,
    pub hora_estimada: Option<String>,
    pub motivo: Option<String>,
    pub sitios: Vec<String>,
    pub visitantes: Vec<VisitanteNuevo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisitanteValido {
    /// Sin espacios de más.
    pub nombre: String,
    /// En su forma única ([`Cedula::normalizar`]).
    pub cedula: String,
    pub empresa: Option<String>,
    /// En mayúsculas.
    pub placa_vehiculo: Option<String>,
}

/// La cita lista para guardar: textos recortados, vacíos como `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitaValida {
    pub fecha_desde: NaiveDate,
    pub fecha_hasta: NaiveDate,
    pub hora_estimada: Option<NaiveTime>,
    pub motivo: Option<String>,
    pub sitios: Vec<String>,
    pub visitantes: Vec<VisitanteValido>,
}

/// Un problema y dónde está: `"fecha_desde"`, `"sitios"`,
/// `"visitantes.1.cedula"`... (el mismo camino que usa el formulario).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorCampo {
    pub campo: String,
    pub mensaje: String,
}

pub const MENSAJE_CARACTER_RARO: &str = "Ese texto tiene un carácter que no se puede guardar (por ejemplo, pegado desde otro programa). Bórrelo y escríbalo de nuevo.";

fn tiene_control(texto: &str) -> bool {
    texto.chars().any(char::is_control)
}

fn hasta(maximo: usize) -> String {
    format!("Use hasta {maximo} caracteres.")
}

/// Texto opcional: recortado, vacío = `None`; con su largo y sin caracteres
/// de control.
fn texto_opcional(
    valor: Option<&str>,
    maximo: usize,
    campo: &str,
    errores: &mut Vec<ErrorCampo>,
) -> Option<String> {
    let limpio = valor.map(str::trim).filter(|texto| !texto.is_empty())?;
    if limpio.chars().count() > maximo {
        errores.push(error(campo, &hasta(maximo)));
    } else if tiene_control(limpio) {
        errores.push(error(campo, MENSAJE_CARACTER_RARO));
    }
    Some(limpio.to_string())
}

fn error(campo: &str, mensaje: &str) -> ErrorCampo {
    ErrorCampo {
        campo: campo.to_string(),
        mensaje: mensaje.to_string(),
    }
}

fn fecha(texto: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texto.trim(), "%Y-%m-%d").ok()
}

/// La cédula de un visitante: la forma única del núcleo; admite pasaportes
/// (letras), a diferencia de contratistas.
pub fn documento_de_visitante(texto: &str) -> Result<Cedula, &'static str> {
    match Cedula::normalizar(texto) {
        Ok(cedula) if cedula.as_str().chars().count() < MIN_DOCUMENTO => {
            Err("Ingrese un documento válido.")
        }
        Ok(cedula) => Ok(cedula),
        Err(CedulaInvalida::Vacia) => Err("Ingrese la cédula o el documento."),
        Err(CedulaInvalida::CaracteresNoPermitidos) => {
            Err("Use letras y números (los espacios, puntos y guiones se quitan solos).")
        }
        Err(CedulaInvalida::DemasiadoLarga) => Err("El documento admite hasta 20 caracteres."),
    }
}

fn validar_visitante(
    indice: usize,
    visitante: &VisitanteNuevo,
    errores: &mut Vec<ErrorCampo>,
) -> Option<VisitanteValido> {
    let campo = |nombre: &str| format!("visitantes.{indice}.{nombre}");
    let antes = errores.len();

    // Como todo nombre de persona o empresa: en mayúscula.
    let nombre = crate::nombre::nombre_en_mayusculas(&visitante.nombre);
    if nombre.chars().count() < MIN_NOMBRE {
        errores.push(error(&campo("nombre"), "Ingrese el nombre completo."));
    } else if nombre.chars().count() > MAX_NOMBRE {
        errores.push(error(&campo("nombre"), &hasta(MAX_NOMBRE)));
    } else if tiene_control(&nombre) {
        errores.push(error(&campo("nombre"), MENSAJE_CARACTER_RARO));
    }

    let cedula = match documento_de_visitante(&visitante.cedula) {
        Ok(cedula) => Some(cedula.into_string()),
        Err(mensaje) => {
            errores.push(error(&campo("cedula"), mensaje));
            None
        }
    };

    let empresa = texto_opcional(
        visitante.empresa.as_deref(),
        MAX_EMPRESA,
        &campo("empresa"),
        errores,
    )
    .map(|empresa| crate::nombre::nombre_en_mayusculas(&empresa));
    let placa_vehiculo = texto_opcional(
        visitante.placa_vehiculo.as_deref(),
        MAX_PLACA,
        &campo("placa_vehiculo"),
        errores,
    )
    .map(|placa| placa.to_uppercase());

    (errores.len() == antes).then(|| VisitanteValido {
        nombre,
        cedula: cedula.unwrap_or_default(),
        empresa,
        placa_vehiculo,
    })
}

/// Todas las reglas de una cita nueva, con `hoy` = la fecha de Costa Rica.
/// Devuelve la cita normalizada o la lista de problemas (todos a la vez).
pub fn validar_cita(cita: &CitaNueva, hoy: NaiveDate) -> Result<CitaValida, Vec<ErrorCampo>> {
    let mut errores = Vec::new();

    let desde = fecha(&cita.fecha_desde);
    let hasta_fecha = fecha(&cita.fecha_hasta);
    if desde.is_none() {
        errores.push(error("fecha_desde", "Elija una fecha válida."));
    }
    if hasta_fecha.is_none() {
        errores.push(error("fecha_hasta", "Elija una fecha válida."));
    }
    if let Some(desde) = desde
        && desde < hoy
    {
        errores.push(error(
            "fecha_desde",
            "La fecha no puede estar en el pasado.",
        ));
    }
    if let (Some(desde), Some(hasta_fecha)) = (desde, hasta_fecha)
        && hasta_fecha < desde
    {
        errores.push(error(
            "fecha_hasta",
            "La fecha final debe ser igual o posterior al inicio.",
        ));
    }

    let hora_estimada = match cita.hora_estimada.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(texto) => match NaiveTime::parse_from_str(texto, "%H:%M") {
            Ok(hora) if texto.len() == 5 => Some(hora),
            _ => {
                errores.push(error("hora_estimada", "Ingrese una hora válida (HH:MM)."));
                None
            }
        },
    };

    let motivo = texto_opcional(cita.motivo.as_deref(), MAX_MOTIVO, "motivo", &mut errores);

    if cita.sitios.is_empty() {
        errores.push(error("sitios", "Elija al menos un lugar."));
    } else if cita.sitios.len() > MAX_SITIOS {
        errores.push(error("sitios", "Puede elegir hasta 100 lugares."));
    } else {
        let mut vistos = std::collections::HashSet::new();
        if !cita
            .sitios
            .iter()
            .all(|sitio| vistos.insert(sitio.as_str()))
        {
            errores.push(error("sitios", "Hay lugares repetidos."));
        }
    }

    if cita.visitantes.is_empty() {
        errores.push(error("visitantes", "Agregue al menos una persona."));
    } else if cita.visitantes.len() > MAX_VISITANTES {
        errores.push(error("visitantes", "Puede agregar hasta 50 personas."));
    }
    let mut visitantes = Vec::with_capacity(cita.visitantes.len());
    let mut documentos = std::collections::HashSet::new();
    for (indice, visitante) in cita.visitantes.iter().take(MAX_VISITANTES).enumerate() {
        // Repetida aunque esté escrita distinto: se compara la forma única.
        if let Ok(cedula) = documento_de_visitante(&visitante.cedula)
            && !documentos.insert(cedula.into_string())
        {
            errores.push(error(
                &format!("visitantes.{indice}.cedula"),
                "Esta persona ya está en la lista.",
            ));
            continue;
        }
        if let Some(valido) = validar_visitante(indice, visitante, &mut errores) {
            visitantes.push(valido);
        }
    }

    match (desde, hasta_fecha) {
        (Some(fecha_desde), Some(fecha_hasta)) if errores.is_empty() => Ok(CitaValida {
            fecha_desde,
            fecha_hasta,
            hora_estimada,
            motivo,
            sitios: cita.sitios.clone(),
            visitantes,
        }),
        _ => Err(errores),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hoy() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 5).unwrap()
    }

    fn visitante(nombre: &str, cedula: &str) -> VisitanteNuevo {
        VisitanteNuevo {
            nombre: nombre.into(),
            cedula: cedula.into(),
            ..VisitanteNuevo::default()
        }
    }

    fn cita() -> CitaNueva {
        CitaNueva {
            fecha_desde: "2026-10-05".into(),
            fecha_hasta: "2026-10-06".into(),
            hora_estimada: Some("09:30".into()),
            motivo: Some("  Revisión  ".into()),
            sitios: vec!["s1".into()],
            visitantes: vec![VisitanteNuevo {
                nombre: "  Ana   Mora ".into(),
                cedula: "01-0847-0293".into(),
                empresa: Some(" ".into()),
                placa_vehiculo: Some(" bcd123 ".into()),
            }],
        }
    }

    fn campos(errores: &[ErrorCampo]) -> Vec<&str> {
        errores.iter().map(|e| e.campo.as_str()).collect()
    }

    #[test]
    fn normaliza_textos_cedula_placa_y_nombre_en_mayuscula() {
        let valida = validar_cita(&cita(), hoy()).unwrap();
        assert_eq!(valida.motivo.as_deref(), Some("Revisión"));
        assert_eq!(valida.hora_estimada, NaiveTime::from_hms_opt(9, 30, 0));
        assert_eq!(
            valida.visitantes,
            vec![VisitanteValido {
                nombre: "ANA MORA".into(),
                cedula: "108470293".into(),
                empresa: None,
                placa_vehiculo: Some("BCD123".into()),
            }]
        );
    }

    #[test]
    fn la_empresa_del_visitante_tambien_va_en_mayuscula() {
        let mut entrada = cita();
        entrada.visitantes[0].empresa = Some(" acme  s.a. ".into());
        let valida = validar_cita(&entrada, hoy()).unwrap();
        assert_eq!(valida.visitantes[0].empresa.as_deref(), Some("ACME S.A."));
    }

    #[test]
    fn la_misma_persona_escrita_distinto_es_repetida() {
        let mut entrada = cita();
        entrada.visitantes.push(visitante("Ana Mora", "108470293"));
        let errores = validar_cita(&entrada, hoy()).unwrap_err();
        assert_eq!(campos(&errores), vec!["visitantes.1.cedula"]);
        assert_eq!(errores[0].mensaje, "Esta persona ya está en la lista.");
    }

    #[test]
    fn fechas_pasadas_o_al_reves_se_rechazan() {
        let mut entrada = cita();
        entrada.fecha_desde = "2026-10-04".into();
        entrada.fecha_hasta = "2026-10-03".into();
        let errores = validar_cita(&entrada, hoy()).unwrap_err();
        assert_eq!(campos(&errores), vec!["fecha_desde", "fecha_hasta"]);

        entrada.fecha_desde = "2026-02-30".into();
        let errores = validar_cita(&entrada, hoy()).unwrap_err();
        assert_eq!(errores[0].mensaje, "Elija una fecha válida.");
    }

    #[test]
    fn devuelve_todos_los_problemas_a_la_vez() {
        let entrada = CitaNueva {
            fecha_desde: "2026-10-05".into(),
            fecha_hasta: "2026-10-05".into(),
            hora_estimada: Some("25:99".into()),
            motivo: Some("texto\u{0}oculto".into()),
            sitios: vec![],
            visitantes: vec![visitante("A", "1-2"), visitante("Luis", &"X".repeat(21))],
        };
        let errores = validar_cita(&entrada, hoy()).unwrap_err();
        assert_eq!(
            campos(&errores),
            vec![
                "hora_estimada",
                "motivo",
                "sitios",
                "visitantes.0.nombre",
                "visitantes.0.cedula",
                "visitantes.1.cedula",
            ]
        );
        assert_eq!(
            errores[5].mensaje,
            "El documento admite hasta 20 caracteres."
        );
    }

    #[test]
    fn limites_de_personas_y_lugares() {
        let mut entrada = cita();
        entrada.visitantes = (0..51)
            .map(|i| visitante("Persona", &format!("DOC{i:03}")))
            .collect();
        entrada.sitios = vec!["s1".into(), "s1".into()];
        let errores = validar_cita(&entrada, hoy()).unwrap_err();
        assert_eq!(campos(&errores), vec!["sitios", "visitantes"]);
        assert_eq!(errores[0].mensaje, "Hay lugares repetidos.");
    }

    #[test]
    fn sin_hora_y_sin_textos_opcionales_vale() {
        let mut entrada = cita();
        entrada.hora_estimada = Some(String::new());
        entrada.motivo = None;
        entrada.visitantes = vec![visitante("Carla Vargas", "AB123456")];
        let valida = validar_cita(&entrada, hoy()).unwrap();
        assert_eq!(valida.hora_estimada, None);
        assert_eq!(valida.motivo, None);
        assert_eq!(valida.visitantes[0].cedula, "AB123456");
    }
}

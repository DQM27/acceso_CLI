//! Revisión de un gafete PRAIND leído (OCR de su imagen) contra la ficha del
//! contratista: qué coincide, qué no y en qué sentido. La comparten el
//! teléfono (que lee la imagen) y, más adelante, el escritorio y el panel.
//!
//! Criterios (pedidos del usuario 2026-11-21, con los gafetes reales de
//! producción como casos):
//! - La **cédula** distinta bloquea el guardado: casi seguro es el gafete de
//!   otra persona.
//! - **Nombre** y **empresa** distintos sólo avisan, con lo que dice cada
//!   lado. Mayúsculas, tildes y puntuación no cuentan como diferencia.
//! - **Vencimiento**, contra la ficha: si el gafete es **más nuevo** (vence
//!   después), se actualiza la ficha; si es **más viejo**, se avisa y se
//!   puede guardar igual como imagen ilustrativa. Sólo se actualiza con una
//!   lectura coherente y una fecha vigente (la regla del contratista no deja
//!   poner un PRAIND vencido).

use chrono::{Months, NaiveDate};

use crate::domain::contratista::normalizar_nombre;
use crate::texto::plegar_diacriticos;

/// Lo que el OCR sacó del gafete. Cualquier dato puede faltar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DatosGafeteLeido {
    pub cedula: Option<String>,
    pub nombre: Option<String>,
    pub empresa: Option<String>,
    pub induccion: Option<NaiveDate>,
    pub vencimiento: Option<NaiveDate>,
}

/// Lo que dice la ficha del contratista.
#[derive(Debug, Clone, Copy)]
pub struct DatosFicha<'a> {
    pub cedula: &'a str,
    pub nombre: &'a str,
    pub empresa: &'a str,
    pub vencimiento: Option<NaiveDate>,
}

/// Un dato del gafete comparado con la ficha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparacion {
    Coincide,
    /// Tal como lo dice cada lado, para mostrarlo.
    Difiere {
        gafete: String,
        ficha: String,
    },
    /// El OCR no lo encontró en la imagen.
    NoLeido,
}

/// El vencimiento según el gafete, contra hoy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VencimientoGafete {
    Vigente(NaiveDate),
    Vencido(NaiveDate),
    NoLeido,
}

/// El gafete contra la fecha de la ficha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FechaContraFicha {
    /// Vence el mismo día que dice la ficha.
    Igual,
    /// Vence después que la ficha (o la ficha no tiene fecha): es más nuevo.
    MasNuevo,
    /// Vence antes que la ficha: es un gafete viejo; sirve como imagen.
    MasViejo,
    /// No se leyó la fecha del gafete.
    SinComparar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionGafetePraind {
    pub cedula: Comparacion,
    pub nombre: Comparacion,
    pub empresa: Comparacion,
    pub vencimiento: VencimientoGafete,
    pub vencimiento_ficha: Option<NaiveDate>,
    pub contra_ficha: FechaContraFicha,
    /// La fecha que se deja en la ficha al guardar: el gafete es más nuevo
    /// que la ficha, está vigente y su lectura es coherente. Un gafete
    /// vencido nunca la propone (la regla del contratista no deja poner un
    /// PRAIND vencido).
    pub fecha_para_ficha: Option<NaiveDate>,
    /// Vencimiento a 1 o 2 años de la inducción: así son todos los gafetes
    /// revisados; si no cuadra, la fecha pudo leerse mal.
    pub lectura_coherente: bool,
    /// El nombre del gafete como lo guardaría la ficha, si difiere: el
    /// portero puede elegir corregir la ficha con él.
    pub nombre_para_ficha: Option<String>,
    /// La empresa del catálogo que corresponde a la del gafete, si difiere de
    /// la ficha y hay exactamente una. La completa el núcleo, que tiene el
    /// catálogo (`AppCore::revisar_gafete_praind`).
    pub empresa_para_ficha: Option<EmpresaPropuesta>,
}

/// Una empresa del catálogo propuesta para la ficha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmpresaPropuesta {
    pub id: i64,
    pub nombre: String,
}

impl RevisionGafetePraind {
    /// Si el gafete no se puede guardar para este contratista.
    #[must_use]
    pub fn bloquea_guardar(&self) -> bool {
        matches!(self.cedula, Comparacion::Difiere { .. })
    }
}

#[must_use]
pub fn revisar_gafete_praind(
    leido: &DatosGafeteLeido,
    ficha: DatosFicha<'_>,
    hoy: NaiveDate,
) -> RevisionGafetePraind {
    let cedula = comparar(leido.cedula.as_deref(), ficha.cedula, |a, b| {
        solo_digitos(a) == solo_digitos(b)
    });
    let nombre = comparar(leido.nombre.as_deref(), ficha.nombre, |a, b| {
        nombres_coinciden(Some(a), b)
    });
    let empresa = comparar(leido.empresa.as_deref(), ficha.empresa, empresas_coinciden);
    let vencimiento = match leido.vencimiento {
        Some(vence) if vence < hoy => VencimientoGafete::Vencido(vence),
        Some(vence) => VencimientoGafete::Vigente(vence),
        None => VencimientoGafete::NoLeido,
    };
    let lectura_coherente = match (leido.induccion, leido.vencimiento) {
        (Some(induccion), Some(vence)) => vencimiento_coherente(induccion, vence),
        _ => false,
    };
    let contra_ficha = match (leido.vencimiento, ficha.vencimiento) {
        (None, _) => FechaContraFicha::SinComparar,
        (Some(_), None) => FechaContraFicha::MasNuevo,
        (Some(gafete), Some(en_ficha)) if gafete == en_ficha => FechaContraFicha::Igual,
        (Some(gafete), Some(en_ficha)) if gafete > en_ficha => FechaContraFicha::MasNuevo,
        (Some(_), Some(_)) => FechaContraFicha::MasViejo,
    };
    let fecha_para_ficha = match vencimiento {
        VencimientoGafete::Vigente(vence)
            if lectura_coherente && contra_ficha == FechaContraFicha::MasNuevo =>
        {
            Some(vence)
        }
        _ => None,
    };
    RevisionGafetePraind {
        cedula,
        empresa,
        vencimiento,
        vencimiento_ficha: ficha.vencimiento,
        contra_ficha,
        fecha_para_ficha,
        lectura_coherente,
        nombre_para_ficha: match (&nombre, leido.nombre.as_deref()) {
            (Comparacion::Difiere { .. }, Some(en_gafete)) => normalizar_nombre(en_gafete),
            _ => None,
        },
        empresa_para_ficha: None,
        nombre,
    }
}

/// Lo que se usa del gafete para dar de alta a un contratista nuevo: los
/// datos para rellenar el formulario, ya en la forma en que se guardan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DatosAltaDesdeGafete {
    /// Sólo dígitos.
    pub cedula: Option<String>,
    /// Como lo guarda la ficha (mayúscula, sin espacios de más).
    pub nombre: Option<String>,
    /// La empresa tal como la dice el gafete.
    pub empresa: Option<String>,
    /// La del catálogo que le corresponde, si hay exactamente una. La
    /// completa el núcleo, que tiene el catálogo.
    pub empresa_catalogo: Option<EmpresaPropuesta>,
    /// Sólo con una lectura coherente (o sin fecha de inducción para
    /// contrastar): una fecha mal leída no se propone.
    pub vencimiento: Option<NaiveDate>,
    /// Si ese vencimiento ya pasó: el formulario no deja guardar.
    pub vencido: bool,
}

/// Los datos del gafete para el alta. Si el OCR no leyó el nombre, sirve el
/// de la página del QR (`nombre_en_origen`).
#[must_use]
pub fn datos_alta_desde_gafete(
    leido: &DatosGafeteLeido,
    nombre_en_origen: Option<&str>,
    hoy: NaiveDate,
) -> DatosAltaDesdeGafete {
    let vencimiento = match (leido.induccion, leido.vencimiento) {
        (Some(induccion), Some(vence)) if !vencimiento_coherente(induccion, vence) => None,
        (_, vence) => vence,
    };
    DatosAltaDesdeGafete {
        cedula: leido
            .cedula
            .as_deref()
            .map(solo_digitos)
            .filter(|cedula| !cedula.is_empty()),
        nombre: leido
            .nombre
            .as_deref()
            .and_then(normalizar_nombre)
            .or_else(|| nombre_en_origen.and_then(normalizar_nombre)),
        empresa: leido
            .empresa
            .as_deref()
            .map(str::trim)
            .filter(|empresa| !empresa.is_empty())
            .map(str::to_string),
        empresa_catalogo: None,
        vencido: vencimiento.is_some_and(|vence| vence < hoy),
        vencimiento,
    }
}

fn comparar(
    gafete: Option<&str>,
    ficha: &str,
    iguales: impl Fn(&str, &str) -> bool,
) -> Comparacion {
    match gafete.map(str::trim).filter(|g| !g.is_empty()) {
        None => Comparacion::NoLeido,
        Some(g) if iguales(g, ficha) => Comparacion::Coincide,
        Some(g) => Comparacion::Difiere {
            gafete: g.to_string(),
            ficha: ficha.trim().to_string(),
        },
    }
}

fn solo_digitos(texto: &str) -> String {
    texto.chars().filter(char::is_ascii_digit).collect()
}

/// Vencimiento a 1 o 2 años de la inducción, con un día de margen: varios
/// gafetes vencen el día antes (inducción 07/06/2026, vence 06/06/2028).
fn vencimiento_coherente(induccion: NaiveDate, vence: NaiveDate) -> bool {
    [12, 24].into_iter().any(|meses| {
        induccion
            .checked_add_months(Months::new(meses))
            .is_some_and(|esperado| (esperado - vence).num_days().abs() <= 1)
    })
}

/// Palabras sin tildes y en mayúscula. Los nombres se parten sólo en letras;
/// las empresas también conservan los números ("Grupo 3M").
fn palabras(texto: &str, con_numeros: bool) -> Vec<String> {
    plegar_diacriticos(texto)
        .to_uppercase()
        .split(|c: char| !(c.is_ascii_alphabetic() || (con_numeros && c.is_ascii_digit())))
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

/// Si el nombre del gafete (o de la página del QR) corresponde al del
/// contratista: todas las palabras del nombre más corto están en el otro
/// (puede sobrar un segundo nombre o apellido), con al menos dos en común,
/// sin tildes ni mayúsculas. Mismo criterio que el escritorio y el panel
/// (`nombresCoinciden` en TS).
#[must_use]
pub fn nombres_coinciden(en_gafete: Option<&str>, contratista: &str) -> bool {
    let Some(en_gafete) = en_gafete else {
        return false;
    };
    let a = palabras(en_gafete, false);
    let b = palabras(contratista, false);
    let (corto, largo) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let comunes = corto.iter().filter(|p| largo.contains(p)).count();
    comunes == corto.len() && comunes >= 2
}

/// Palabras que no distinguen una empresa de otra: la forma jurídica
/// ("S.A.", "SRL", "Inc") y los artículos.
const PALABRAS_DE_RELLENO: &[&str] = &[
    "S", "A", "SA", "SRL", "INC", "LTDA", "LIMITADA", "SOCIEDAD", "ANONIMA", "DE", "DEL", "LA",
    "EL", "LOS", "LAS", "Y",
];

/// Misma empresa si, sin forma jurídica ni artículos, todas las palabras de
/// la más corta están en la otra ("SODEXO" y "Sodexo S.A.";
/// "MULTISERVICIOS DE ALDAMA DE SAN RAFAEL" escrito como en el gafete).
#[must_use]
pub fn empresas_coinciden(en_gafete: &str, ficha: &str) -> bool {
    let utiles = |texto: &str| -> Vec<String> {
        palabras(texto, true)
            .into_iter()
            .filter(|p| !PALABRAS_DE_RELLENO.contains(&p.as_str()))
            .collect()
    };
    let a = utiles(en_gafete);
    let b = utiles(ficha);
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let (corto, largo) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    corto.iter().all(|p| largo.contains(p))
}

/// Las palabras que tiene sólo el gafete y sólo la ficha, en mayúscula y con
/// sus tildes, para decir **dónde** difiere ("JOHANNA" en el gafete,
/// "JOHANA" en la ficha). Al comparar no cuentan tildes ni mayúsculas, y en
/// una empresa tampoco la forma jurídica ni los artículos.
#[must_use]
pub fn palabras_que_difieren(
    gafete: &str,
    ficha: &str,
    es_empresa: bool,
) -> (Vec<String>, Vec<String>) {
    let con_clave = |texto: &str| -> Vec<(String, String)> {
        texto
            .split(|c: char| !c.is_alphanumeric())
            .filter(|p| !p.is_empty())
            .map(|p| (plegar_diacriticos(p).to_uppercase(), p.to_uppercase()))
            .filter(|(clave, _)| !es_empresa || !PALABRAS_DE_RELLENO.contains(&clave.as_str()))
            .collect()
    };
    let a = con_clave(gafete);
    let b = con_clave(ficha);
    let solo = |de: &[(String, String)], en: &[(String, String)]| -> Vec<String> {
        de.iter()
            .filter(|(clave, _)| !en.iter().any(|(otra, _)| otra == clave))
            .map(|(_, palabra)| palabra.clone())
            .collect()
    };
    (solo(&a, &b), solo(&b, &a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fecha(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").expect("fecha de prueba")
    }

    fn hoy() -> NaiveDate {
        fecha("2026-11-21")
    }

    /// El gafete de Hazel (producción, 2026-11-21): viejo, ficha con la
    /// fecha provisional.
    fn gafete_hazel() -> DatosGafeteLeido {
        DatosGafeteLeido {
            cedula: Some("305450949".into()),
            nombre: Some("Hazel Johanna Angulo Aguilar".into()),
            empresa: Some("Sodexo".into()),
            induccion: Some(fecha("2025-06-19")),
            vencimiento: Some(fecha("2026-06-19")),
        }
    }

    fn ficha_hazel(vencimiento: &str) -> DatosFicha<'static> {
        DatosFicha {
            cedula: "305450949",
            nombre: "HAZEL JOHANNA ANGULO AGUILAR",
            empresa: "SODEXO",
            vencimiento: Some(fecha(vencimiento)),
        }
    }

    #[test]
    fn un_gafete_vencido_avisa_y_no_propone_tocar_la_ficha() {
        let revision = revisar_gafete_praind(&gafete_hazel(), ficha_hazel("2027-05-30"), hoy());
        assert_eq!(revision.cedula, Comparacion::Coincide);
        assert_eq!(revision.nombre, Comparacion::Coincide);
        assert_eq!(revision.empresa, Comparacion::Coincide);
        assert_eq!(
            revision.vencimiento,
            VencimientoGafete::Vencido(fecha("2026-06-19"))
        );
        assert_eq!(revision.contra_ficha, FechaContraFicha::MasViejo);
        assert_eq!(revision.fecha_para_ficha, None);
        assert!(!revision.bloquea_guardar());
    }

    #[test]
    fn un_gafete_vigente_mas_viejo_que_la_ficha_no_la_toca() {
        // Vigente, pero la ficha ya tiene una fecha posterior.
        let leido = DatosGafeteLeido {
            induccion: Some(fecha("2026-01-19")),
            vencimiento: Some(fecha("2027-01-19")),
            ..gafete_hazel()
        };
        let revision = revisar_gafete_praind(&leido, ficha_hazel("2027-05-30"), hoy());
        assert_eq!(revision.contra_ficha, FechaContraFicha::MasViejo);
        assert_eq!(revision.fecha_para_ficha, None);
    }

    #[test]
    fn una_ficha_sin_fecha_toma_la_del_gafete() {
        let leido = DatosGafeteLeido {
            induccion: Some(fecha("2026-06-07")),
            vencimiento: Some(fecha("2028-06-06")),
            ..gafete_hazel()
        };
        let ficha = DatosFicha {
            vencimiento: None,
            ..ficha_hazel("2027-05-30")
        };
        let revision = revisar_gafete_praind(&leido, ficha, hoy());
        assert_eq!(revision.contra_ficha, FechaContraFicha::MasNuevo);
        assert_eq!(revision.fecha_para_ficha, Some(fecha("2028-06-06")));
    }

    #[test]
    fn un_gafete_mas_nuevo_que_la_ficha_la_actualiza() {
        let leido = DatosGafeteLeido {
            induccion: Some(fecha("2026-06-07")),
            vencimiento: Some(fecha("2028-06-06")),
            ..gafete_hazel()
        };
        let revision = revisar_gafete_praind(&leido, ficha_hazel("2027-05-30"), hoy());
        assert!(revision.lectura_coherente);
        assert_eq!(
            revision.vencimiento,
            VencimientoGafete::Vigente(fecha("2028-06-06"))
        );
        assert_eq!(revision.contra_ficha, FechaContraFicha::MasNuevo);
        assert_eq!(revision.fecha_para_ficha, Some(fecha("2028-06-06")));

        let igual = revisar_gafete_praind(&leido, ficha_hazel("2028-06-06"), hoy());
        assert_eq!(igual.contra_ficha, FechaContraFicha::Igual);
        assert_eq!(igual.fecha_para_ficha, None);
    }

    #[test]
    fn una_lectura_incoherente_no_toca_la_ficha() {
        // Vence 8 meses después de la inducción: un dígito mal leído.
        let leido = DatosGafeteLeido {
            induccion: Some(fecha("2026-06-07")),
            vencimiento: Some(fecha("2027-02-06")),
            ..gafete_hazel()
        };
        let revision = revisar_gafete_praind(&leido, ficha_hazel("2027-05-30"), hoy());
        assert!(!revision.lectura_coherente);
        assert_eq!(revision.fecha_para_ficha, None);

        // Sin la fecha de inducción tampoco se sabe si cuadra.
        let sin_induccion = DatosGafeteLeido {
            induccion: None,
            vencimiento: Some(fecha("2028-06-06")),
            ..gafete_hazel()
        };
        let revision = revisar_gafete_praind(&sin_induccion, ficha_hazel("2027-05-30"), hoy());
        assert_eq!(revision.fecha_para_ficha, None);
    }

    #[test]
    fn coherente_a_uno_o_dos_anios_con_un_dia_de_margen() {
        assert!(vencimiento_coherente(
            fecha("2025-06-19"),
            fecha("2026-06-19")
        ));
        assert!(vencimiento_coherente(
            fecha("2025-04-04"),
            fecha("2027-04-04")
        ));
        assert!(vencimiento_coherente(
            fecha("2026-04-02"),
            fecha("2028-04-01")
        ));
        assert!(!vencimiento_coherente(
            fecha("2025-06-19"),
            fecha("2026-12-31")
        ));
        assert!(!vencimiento_coherente(
            fecha("2025-06-19"),
            fecha("2028-06-19")
        ));
    }

    #[test]
    fn la_cedula_distinta_bloquea() {
        let leido = DatosGafeteLeido {
            cedula: Some("No. 1 1 6 1 5 0 1 0 9".into()),
            ..gafete_hazel()
        };
        let revision = revisar_gafete_praind(&leido, ficha_hazel("2027-05-30"), hoy());
        assert_eq!(
            revision.cedula,
            Comparacion::Difiere {
                gafete: "No. 1 1 6 1 5 0 1 0 9".into(),
                ficha: "305450949".into(),
            }
        );
        assert!(revision.bloquea_guardar());
    }

    #[test]
    fn lo_que_no_se_leyo_no_bloquea() {
        let revision = revisar_gafete_praind(
            &DatosGafeteLeido::default(),
            ficha_hazel("2027-05-30"),
            hoy(),
        );
        assert_eq!(revision.cedula, Comparacion::NoLeido);
        assert_eq!(revision.nombre, Comparacion::NoLeido);
        assert_eq!(revision.vencimiento, VencimientoGafete::NoLeido);
        assert!(!revision.bloquea_guardar());
    }

    #[test]
    fn empresas_iguales_sin_forma_juridica_ni_tildes() {
        assert!(empresas_coinciden("Sodexo", "SODEXO"));
        assert!(empresas_coinciden("Expenic Ing S.A", "EXPENIC ING S.A."));
        assert!(empresas_coinciden(
            "Integracom de Centroamérica S A",
            "INTEGRACOM DE CENTROAMÉRICA S.A."
        ));
        assert!(empresas_coinciden(
            "Ciasa Money Systems, S.A.",
            "CIASA MONEY SYSTEMS, S.A."
        ));
        assert!(empresas_coinciden("Broomday", "BROOMDAY"));
        assert!(!empresas_coinciden(
            "Sodexo",
            "MULTISERVICIOS DE ALDAMA DE SAN RAFAEL"
        ));
        assert!(!empresas_coinciden("S.A.", "SODEXO"));
    }

    #[test]
    fn palabras_que_difieren_sin_contar_tildes_mayusculas_ni_relleno() {
        assert_eq!(
            palabras_que_difieren(
                "Hazel Johanna Angulo Aguilar",
                "HAZEL JOHANA ANGULO AGUILAR",
                false
            ),
            (vec!["JOHANNA".to_string()], vec!["JOHANA".to_string()])
        );
        assert_eq!(
            palabras_que_difieren("Rafael Carranza Nuñez", "RAFAEL CARRANZA NUNEZ", false),
            (vec![], vec![])
        );
        assert_eq!(
            palabras_que_difieren("Expenic Ing S.A", "ALDAMA S.A.", true),
            (
                vec!["EXPENIC".to_string(), "ING".to_string()],
                vec!["ALDAMA".to_string()]
            )
        );
    }

    #[test]
    fn nombres_con_un_apellido_de_mas_coinciden() {
        assert!(nombres_coinciden(
            Some("Rafael Carranza Nuñez"),
            "RAFAEL CARRANZA NUÑEZ"
        ));
        assert!(nombres_coinciden(
            Some("Johnny Morales Solano"),
            "JOHNNY MORALES"
        ));
        assert!(!nombres_coinciden(Some("Johnny"), "JOHNNY MORALES"));
        assert!(!nombres_coinciden(Some("Ana Perez"), "JOHNNY MORALES"));
        assert!(!nombres_coinciden(None, "JOHNNY MORALES"));
    }

    #[test]
    fn alta_desde_gafete_deja_los_datos_como_los_guarda_la_ficha() {
        let leido = DatosGafeteLeido {
            cedula: Some("3-0545-0949".into()),
            induccion: Some(fecha("2026-06-19")),
            vencimiento: Some(fecha("2027-06-19")),
            ..gafete_hazel()
        };
        let datos = datos_alta_desde_gafete(&leido, None, hoy());
        assert_eq!(datos.cedula.as_deref(), Some("305450949"));
        assert_eq!(datos.nombre.as_deref(), Some("HAZEL JOHANNA ANGULO AGUILAR"));
        assert_eq!(datos.empresa.as_deref(), Some("Sodexo"));
        assert_eq!(datos.vencimiento, Some(fecha("2027-06-19")));
        assert!(!datos.vencido);
    }

    #[test]
    fn alta_desde_gafete_vencido_lo_marca() {
        let datos = datos_alta_desde_gafete(&gafete_hazel(), None, hoy());
        assert_eq!(datos.vencimiento, Some(fecha("2026-06-19")));
        assert!(datos.vencido);
    }

    #[test]
    fn alta_desde_gafete_no_propone_una_fecha_mal_leida() {
        let leido = DatosGafeteLeido {
            vencimiento: Some(fecha("2027-01-19")),
            ..gafete_hazel()
        };
        assert_eq!(datos_alta_desde_gafete(&leido, None, hoy()).vencimiento, None);
        // Sin fecha de inducción no hay con qué contrastar: se propone.
        let sin_induccion = DatosGafeteLeido {
            induccion: None,
            vencimiento: Some(fecha("2027-01-19")),
            ..gafete_hazel()
        };
        assert_eq!(
            datos_alta_desde_gafete(&sin_induccion, None, hoy()).vencimiento,
            Some(fecha("2027-01-19"))
        );
    }

    #[test]
    fn alta_desde_gafete_sin_nombre_leido_usa_el_de_la_pagina() {
        let leido = DatosGafeteLeido {
            nombre: None,
            ..gafete_hazel()
        };
        let datos = datos_alta_desde_gafete(&leido, Some("Hazel Johanna Angulo Aguilar"), hoy());
        assert_eq!(datos.nombre.as_deref(), Some("HAZEL JOHANNA ANGULO AGUILAR"));
    }
}

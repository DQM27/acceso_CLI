//! Parseo puro de la línea de comandos de la interfaz `--comandos`.
//!
//! Sin IO, sin ratatui, sin SQLite: toma el texto crudo del input y devuelve
//! una [`Entrada`] estructurada. Todo lo inválido (comando desconocido, gafete
//! no numérico, medio no reconocido) se representa en el valor devuelto — el
//! parser nunca entra en pánico ni descarta información sin avisar.
//!
//! Sintaxis: `/comando texto libre CLAVE:valor CLAVE:valor`, sin distinguir
//! mayúsculas y con orden de parámetros libre. Claves reconocidas: `G:` (número
//! de gafete) y `M:` (medio de ingreso: `caminando` o `vehiculo`, admite
//! cualquier prefijo no ambiguo). El resto de los tokens forman la consulta de
//! búsqueda libre.
//!
//! Decisiones de diseño:
//! - No existe un comando `/buscar`: el texto sin `/` inicial YA es la
//!   búsqueda (la acción más común después de ingresos, y la única sin
//!   efectos laterales) — un comando aparte para lo mismo sería redundante.
//! - Una clave con valor vacío mientras se teclea (`G:` sin número todavía) se
//!   trata como parámetro ausente, no como error — el parseo debe funcionar con
//!   entrada parcial en cada pulsación de tecla.
//! - Un token `clave:valor` con clave desconocida (p. ej. `X:3`) se conserva
//!   como texto de la consulta, no como error.

use crate::models::medio_ingreso::MedioIngreso;

/// Comandos reconocidos tras el `/` inicial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comando {
    Ingreso,
    Salida,
    Activos,
    Nuevo,
    Editar,
    Ayuda,
    CerrarSesion,
}

impl Comando {
    /// Los 7 comandos en el orden en que se presentan en la ayuda.
    pub const TODOS: [Self; 7] = [
        Self::Ingreso,
        Self::Salida,
        Self::Activos,
        Self::Nuevo,
        Self::Editar,
        Self::Ayuda,
        Self::CerrarSesion,
    ];

    pub fn nombre(self) -> &'static str {
        match self {
            Self::Ingreso => "ingreso",
            Self::Salida => "salida",
            Self::Activos => "activos",
            Self::Nuevo => "nuevo",
            Self::Editar => "editar",
            Self::Ayuda => "ayuda",
            Self::CerrarSesion => "cerrarsesion",
        }
    }

    /// Reconoce el nombre largo y el alias de una letra, sin distinguir
    /// mayúsculas. Los alias sólo existen aquí: la ayuda y el autocompletado
    /// muestran siempre el nombre largo.
    pub fn desde_texto(texto: &str) -> Option<Self> {
        match texto.to_lowercase().as_str() {
            "ingreso" | "i" => Some(Self::Ingreso),
            "salida" | "s" => Some(Self::Salida),
            "activos" | "a" => Some(Self::Activos),
            "nuevo" | "n" => Some(Self::Nuevo),
            "editar" | "e" => Some(Self::Editar),
            "ayuda" => Some(Self::Ayuda),
            "cerrarsesion" | "cs" => Some(Self::CerrarSesion),
            _ => None,
        }
    }
}

/// Resultado de interpretar un parámetro `G:valor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GafeteParse {
    Valido(i64),
    /// El valor no era un número entero — se conserva el texto original para
    /// mostrarlo en el mensaje de error.
    Invalido(String),
}

/// Resultado de interpretar un parámetro `M:valor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MedioParse {
    Valido(MedioIngreso),
    Invalido(String),
}

/// El input ya interpretado, listo para que el resolver lo cruce con `AppCore`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entrada {
    /// Input vacío o sólo espacios.
    Inicio,
    /// Texto sin `/` inicial: es la búsqueda de contratistas (la acción más
    /// común, no necesita comando propio).
    BusquedaLibre { consulta: String },
    /// Comando reconocido con sus argumentos (posiblemente vacíos mientras se
    /// teclea: `/ingreso car` ya parsea a `Ingreso` con consulta "car").
    Comando {
        comando: Comando,
        consulta: String,
        gafete: Option<GafeteParse>,
        medio: Option<MedioParse>,
    },
    /// `/xyz ...`: se conserva el nombre para el mensaje "comando no
    /// reconocido" con sugerencia de `/ayuda`.
    Desconocido { nombre: String },
}

/// Parsea el texto crudo del input. Función pura: mismo texto, mismo resultado.
pub fn parsear(texto: &str) -> Entrada {
    let recortado = texto.trim();
    if recortado.is_empty() {
        return Entrada::Inicio;
    }

    let mut tokens = recortado.split_whitespace();
    let primero = tokens.next().unwrap_or_default();
    let Some(nombre) = primero.strip_prefix('/') else {
        // Texto libre sin comando: se conserva el texto completo (con sus
        // espacios internos) como consulta de búsqueda.
        return Entrada::BusquedaLibre {
            consulta: recortado.to_string(),
        };
    };

    // "/" a secas no es un error: el operador apenas empezó a escribir un
    // comando — las sugerencias ya le muestran los disponibles.
    if nombre.is_empty() {
        return Entrada::Inicio;
    }

    let Some(comando) = Comando::desde_texto(nombre) else {
        return Entrada::Desconocido {
            nombre: nombre.to_string(),
        };
    };

    let mut consulta: Vec<&str> = Vec::new();
    let mut gafete: Option<GafeteParse> = None;
    let mut medio: Option<MedioParse> = None;

    for token in tokens {
        match clasificar_token(token) {
            Clasificacion::Gafete(valor) => gafete = Some(valor),
            Clasificacion::Medio(valor) => medio = Some(valor),
            Clasificacion::Ignorado => {}
            Clasificacion::Texto => consulta.push(token),
        }
    }

    Entrada::Comando {
        comando,
        consulta: consulta.join(" "),
        gafete,
        medio,
    }
}

/// Cómo se interpreta un token que sigue al comando.
enum Clasificacion {
    Gafete(GafeteParse),
    Medio(MedioParse),
    /// Clave conocida sin valor todavía (`G:` a medio escribir): no aporta ni
    /// a los parámetros ni a la consulta.
    Ignorado,
    /// No es un parámetro reconocido: pasa al texto de la consulta tal cual.
    Texto,
}

/// Reconoce `G:valor` / `M:valor` (cualquier casing, clave de una letra). Un
/// token `clave:valor` con clave desconocida se clasifica como texto libre, no
/// como error.
fn clasificar_token(token: &str) -> Clasificacion {
    let Some((clave, valor)) = token.split_once(':') else {
        return Clasificacion::Texto;
    };
    if clave.len() != 1 {
        return Clasificacion::Texto;
    }
    match clave.to_lowercase().as_str() {
        "g" if valor.is_empty() => Clasificacion::Ignorado,
        "m" if valor.is_empty() => Clasificacion::Ignorado,
        "g" => Clasificacion::Gafete(parsear_gafete(valor)),
        "m" => Clasificacion::Medio(parsear_medio(valor)),
        _ => Clasificacion::Texto,
    }
}

fn parsear_gafete(valor: &str) -> GafeteParse {
    match valor.parse::<i64>() {
        Ok(numero) if numero > 0 => GafeteParse::Valido(numero),
        _ => GafeteParse::Invalido(valor.to_string()),
    }
}

/// Admite cualquier prefijo no ambiguo de `caminando`/`vehiculo` (`M:v`,
/// `M:VEHI`, ...), sin distinguir mayúsculas.
fn parsear_medio(valor: &str) -> MedioParse {
    let minusculas = valor.to_lowercase();
    if "caminando".starts_with(&minusculas) {
        MedioParse::Valido(MedioIngreso::Caminando)
    } else if "vehiculo".starts_with(&minusculas) {
        MedioParse::Valido(MedioIngreso::Vehiculo)
    } else {
        MedioParse::Invalido(valor.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comando(entrada: Entrada) -> (Comando, String, Option<GafeteParse>, Option<MedioParse>) {
        match entrada {
            Entrada::Comando {
                comando,
                consulta,
                gafete,
                medio,
            } => (comando, consulta, gafete, medio),
            otra => panic!("se esperaba Entrada::Comando, llegó {otra:?}"),
        }
    }

    // ── Los 7 casos del enunciado ────────────────────────────────────────

    #[test]
    fn ingreso_con_gafete() {
        let (cmd, consulta, gafete, medio) = comando(parsear("/ingreso Carlos G:27"));
        assert_eq!(cmd, Comando::Ingreso);
        assert_eq!(consulta, "Carlos");
        assert_eq!(gafete, Some(GafeteParse::Valido(27)));
        assert_eq!(medio, None);
    }

    #[test]
    fn ingreso_con_gafete_y_medio() {
        let (cmd, consulta, gafete, medio) = comando(parsear("/ingreso Carlos G:27 M:vehiculo"));
        assert_eq!(cmd, Comando::Ingreso);
        assert_eq!(consulta, "Carlos");
        assert_eq!(gafete, Some(GafeteParse::Valido(27)));
        assert_eq!(medio, Some(MedioParse::Valido(MedioIngreso::Vehiculo)));
    }

    #[test]
    fn ingreso_por_cedula() {
        let (cmd, consulta, gafete, _) = comando(parsear("/ingreso 119430546 G:12"));
        assert_eq!(cmd, Comando::Ingreso);
        assert_eq!(consulta, "119430546");
        assert_eq!(gafete, Some(GafeteParse::Valido(12)));
    }

    #[test]
    fn salida_por_nombre() {
        let (cmd, consulta, gafete, _) = comando(parsear("/salida Carlos"));
        assert_eq!(cmd, Comando::Salida);
        assert_eq!(consulta, "Carlos");
        assert_eq!(gafete, None);
    }

    #[test]
    fn salida_por_gafete() {
        let (cmd, consulta, gafete, _) = comando(parsear("/salida G:27"));
        assert_eq!(cmd, Comando::Salida);
        assert_eq!(consulta, "");
        assert_eq!(gafete, Some(GafeteParse::Valido(27)));
    }

    #[test]
    fn activos_sin_argumentos() {
        let (cmd, consulta, gafete, medio) = comando(parsear("/activos"));
        assert_eq!(cmd, Comando::Activos);
        assert_eq!(consulta, "");
        assert_eq!(gafete, None);
        assert_eq!(medio, None);
    }

    #[test]
    fn cerrarsesion_sin_argumentos() {
        let (cmd, consulta, gafete, medio) = comando(parsear("/cerrarsesion"));
        assert_eq!(cmd, Comando::CerrarSesion);
        assert_eq!(consulta, "");
        assert_eq!(gafete, None);
        assert_eq!(medio, None);
    }

    #[test]
    fn nuevo_sin_argumentos() {
        let (cmd, consulta, _, _) = comando(parsear("/nuevo"));
        assert_eq!(cmd, Comando::Nuevo);
        assert_eq!(consulta, "");
    }

    #[test]
    fn editar_conserva_la_consulta() {
        let (cmd, consulta, _, _) = comando(parsear("/editar carlos"));
        assert_eq!(cmd, Comando::Editar);
        assert_eq!(consulta, "carlos");
    }

    // ── Equivalencias de casing y orden ──────────────────────────────────

    #[test]
    fn mayusculas_no_cambian_el_parseo() {
        assert_eq!(
            parsear("/INGRESO Carlos G:27 M:VEHICULO"),
            parsear("/ingreso Carlos g:27 m:vehiculo"),
        );
    }

    #[test]
    fn el_orden_de_los_parametros_es_libre() {
        assert_eq!(
            parsear("/ingreso Carlos G:27 M:vehiculo"),
            parsear("/ingreso Carlos m:vehiculo g:27"),
        );
        // Y los parámetros pueden ir antes del texto.
        assert_eq!(
            parsear("/ingreso Carlos G:27"),
            parsear("/ingreso G:27 Carlos"),
        );
    }

    #[test]
    fn medio_admite_prefijos_no_ambiguos() {
        let (_, _, _, medio) = comando(parsear("/ingreso Ana M:v"));
        assert_eq!(medio, Some(MedioParse::Valido(MedioIngreso::Vehiculo)));
        let (_, _, _, medio) = comando(parsear("/ingreso Ana M:cam"));
        assert_eq!(medio, Some(MedioParse::Valido(MedioIngreso::Caminando)));
    }

    // ── Errores representados en el valor, nunca pánico ─────────────────

    #[test]
    fn gafete_no_numerico_es_error_de_parseo() {
        let (_, _, gafete, _) = comando(parsear("/ingreso Carlos G:abc"));
        assert_eq!(gafete, Some(GafeteParse::Invalido("abc".to_string())));
    }

    #[test]
    fn gafete_cero_o_negativo_es_invalido() {
        let (_, _, gafete, _) = comando(parsear("/ingreso Carlos G:0"));
        assert_eq!(gafete, Some(GafeteParse::Invalido("0".to_string())));
        let (_, _, gafete, _) = comando(parsear("/ingreso Carlos G:-3"));
        assert_eq!(gafete, Some(GafeteParse::Invalido("-3".to_string())));
    }

    #[test]
    fn medio_desconocido_es_error_de_parseo() {
        let (_, _, _, medio) = comando(parsear("/ingreso Carlos M:avion"));
        assert_eq!(medio, Some(MedioParse::Invalido("avion".to_string())));
    }

    #[test]
    fn comando_desconocido_conserva_el_nombre() {
        assert_eq!(
            parsear("/xyz algo"),
            Entrada::Desconocido {
                nombre: "xyz".to_string()
            }
        );
    }

    // ── Entrada parcial y límites ────────────────────────────────────────

    #[test]
    fn entrada_parcial_mientras_se_teclea() {
        let (cmd, consulta, gafete, medio) = comando(parsear("/ingreso car"));
        assert_eq!(cmd, Comando::Ingreso);
        assert_eq!(consulta, "car");
        assert_eq!(gafete, None);
        assert_eq!(medio, None);

        // `G:` sin valor todavía: parámetro ausente, no error.
        let (_, consulta, gafete, _) = comando(parsear("/ingreso car G:"));
        assert_eq!(consulta, "car");
        assert_eq!(gafete, None);
    }

    #[test]
    fn barra_sola_no_es_error() {
        assert_eq!(parsear("/"), Entrada::Inicio);
        assert_eq!(parsear("/ "), Entrada::Inicio);
    }

    #[test]
    fn cadena_vacia_o_espacios_es_inicio() {
        assert_eq!(parsear(""), Entrada::Inicio);
        assert_eq!(parsear("   "), Entrada::Inicio);
    }

    #[test]
    fn texto_sin_barra_es_busqueda_libre() {
        assert_eq!(
            parsear("Carlos Pérez"),
            Entrada::BusquedaLibre {
                consulta: "Carlos Pérez".to_string()
            }
        );
    }

    #[test]
    fn aliases_cortos() {
        assert_eq!(comando(parsear("/i Ana")).0, Comando::Ingreso);
        assert_eq!(comando(parsear("/s Ana")).0, Comando::Salida);
        assert_eq!(comando(parsear("/a")).0, Comando::Activos);
        assert_eq!(comando(parsear("/n")).0, Comando::Nuevo);
        assert_eq!(comando(parsear("/e Ana")).0, Comando::Editar);
        assert_eq!(comando(parsear("/cs")).0, Comando::CerrarSesion);
    }

    #[test]
    fn clave_desconocida_pasa_como_texto_libre() {
        let (_, consulta, gafete, medio) = comando(parsear("/ingreso Carlos X:3"));
        assert_eq!(consulta, "Carlos X:3");
        assert_eq!(gafete, None);
        assert_eq!(medio, None);
    }

    #[test]
    fn consulta_con_varias_palabras() {
        assert_eq!(
            parsear("maria de los angeles"),
            Entrada::BusquedaLibre {
                consulta: "maria de los angeles".to_string()
            }
        );
    }
}

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
//! Segunda forma de llegar a un comando "de ítem" (`ingreso`/`salida`/
//! `editar`): un modificador `--letra`/`--palabra` sobre texto libre, p. ej.
//! `Ana --e` o `Ana --i G:27` — produce exactamente la misma [`Entrada`] que
//! `/editar Ana` o `/ingreso Ana G:27`. Ver §5.1 y DEC-018/DEC-021 de
//! `docs/lenguaje-visual-mutaciones.md`. Los comandos globales (`nuevo`,
//! `activos`, `ayuda`, `cerrarsesion`) no tienen esta segunda forma: no
//! actúan sobre un resultado de búsqueda, así que sólo existen como
//! `/comando`.
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
//! - Los parámetros `G:`/`M:` sólo se interpretan cuando hay una acción
//!   seleccionada (`/comando` o `--modificador`): una búsqueda libre sin
//!   acción nunca pierde texto por parecerse a un parámetro.

use crate::models::medio_ingreso::MedioIngreso;

/// Comandos reconocidos tras el `/` inicial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comando {
    Ingreso,
    Salida,
    Gafete,
    Activos,
    Nuevo,
    Editar,
    Historial,
    Auditoria,
    Ayuda,
    Clave,
    Clasico,
    CerrarSesion,
}

impl Comando {
    /// Los 12 comandos en el orden en que se presentan en la ayuda.
    pub const TODOS: [Self; 12] = [
        Self::Ingreso,
        Self::Salida,
        Self::Gafete,
        Self::Activos,
        Self::Nuevo,
        Self::Editar,
        Self::Historial,
        Self::Auditoria,
        Self::Ayuda,
        Self::Clave,
        Self::Clasico,
        Self::CerrarSesion,
    ];

    pub fn nombre(self) -> &'static str {
        match self {
            Self::Ingreso => "ingreso",
            Self::Salida => "salida",
            Self::Gafete => "gafete",
            Self::Activos => "activos",
            Self::Nuevo => "nuevo",
            Self::Editar => "editar",
            Self::Historial => "historial",
            Self::Auditoria => "auditoria",
            Self::Ayuda => "ayuda",
            Self::Clave => "clave",
            Self::Clasico => "clasico",
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
            "gafete" | "g" => Some(Self::Gafete),
            "activos" | "a" => Some(Self::Activos),
            "nuevo" | "n" => Some(Self::Nuevo),
            "editar" | "e" => Some(Self::Editar),
            "historial" | "h" => Some(Self::Historial),
            "auditoria" => Some(Self::Auditoria),
            "ayuda" => Some(Self::Ayuda),
            "clave" => Some(Self::Clave),
            "clasico" => Some(Self::Clasico),
            "cerrarsesion" | "cs" => Some(Self::CerrarSesion),
            _ => None,
        }
    }

    /// Comandos que actúan sobre un resultado de búsqueda ya encontrado —
    /// los únicos que, además de `/comando`, aceptan un modificador
    /// `--letra`/`--palabra` sobre texto libre (DEC-021).
    pub fn es_de_item(self) -> bool {
        matches!(self, Self::Ingreso | Self::Salida | Self::Editar)
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
        return parsear_busqueda_libre(recortado);
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

/// Texto sin `/` inicial: por defecto es búsqueda libre y se conserva
/// exactamente como se escribió (§5.1). Si contiene un modificador de acción
/// `--letra`/`--palabra` de un comando "de ítem" (`es_de_item`), en cambio
/// produce la misma [`Entrada::Comando`] que `/comando` generaría — los
/// parámetros `G:`/`M:` sólo se interpretan en ese caso, nunca sobre una
/// búsqueda sin acción seleccionada.
fn parsear_busqueda_libre(recortado: &str) -> Entrada {
    let comando = recortado.split_whitespace().find_map(|token| {
        token
            .strip_prefix("--")
            .and_then(Comando::desde_texto)
            .filter(|c| c.es_de_item())
    });

    let Some(comando) = comando else {
        // Texto libre sin acción: se conserva el texto completo (con sus
        // espacios internos) tal cual, sin interpretar nada.
        return Entrada::BusquedaLibre {
            consulta: recortado.to_string(),
        };
    };

    let mut consulta: Vec<&str> = Vec::new();
    let mut gafete: Option<GafeteParse> = None;
    let mut medio: Option<MedioParse> = None;

    for token in recortado.split_whitespace() {
        let es_el_modificador = token
            .strip_prefix("--")
            .and_then(Comando::desde_texto)
            .is_some_and(|c| c == comando);
        if es_el_modificador {
            continue;
        }
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
    fn gafete_conserva_una_lista_con_comas() {
        let (cmd, consulta, _, _) = comando(parsear("/gafete 2, 25, 85, 11"));
        assert_eq!(cmd, Comando::Gafete);
        assert_eq!(consulta, "2, 25, 85, 11");
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
        assert_eq!(comando(parsear("/g 27")).0, Comando::Gafete);
        assert_eq!(comando(parsear("/a")).0, Comando::Activos);
        assert_eq!(comando(parsear("/n")).0, Comando::Nuevo);
        assert_eq!(comando(parsear("/e Ana")).0, Comando::Editar);
        assert_eq!(comando(parsear("/h")).0, Comando::Historial);
        assert_eq!(comando(parsear("/cs")).0, Comando::CerrarSesion);
    }

    #[test]
    fn historial_sin_argumentos() {
        let (cmd, consulta, _, _) = comando(parsear("/historial"));
        assert_eq!(cmd, Comando::Historial);
        assert_eq!(consulta, "");
    }

    #[test]
    fn clave_desconocida_pasa_como_texto_libre() {
        let (_, consulta, gafete, medio) = comando(parsear("/ingreso Carlos X:3"));
        assert_eq!(consulta, "Carlos X:3");
        assert_eq!(gafete, None);
        assert_eq!(medio, None);
    }

    // ── Modificador de acción `--letra`/`--palabra` (§5.1) ──────────────

    #[test]
    fn modificador_letra_equivale_al_comando_lider() {
        assert_eq!(comando(parsear("Ana --e")), comando(parsear("/editar Ana")),);
    }

    #[test]
    fn modificador_palabra_completa_tambien_funciona() {
        let (cmd, consulta, _, _) = comando(parsear("Ana --editar"));
        assert_eq!(cmd, Comando::Editar);
        assert_eq!(consulta, "Ana");
    }

    #[test]
    fn modificador_ingreso_admite_parametros() {
        let (cmd, consulta, gafete, medio) = comando(parsear("Ana --i G:27 M:vehiculo"));
        assert_eq!(cmd, Comando::Ingreso);
        assert_eq!(consulta, "Ana");
        assert_eq!(gafete, Some(GafeteParse::Valido(27)));
        assert_eq!(medio, Some(MedioParse::Valido(MedioIngreso::Vehiculo)));
    }

    #[test]
    fn modificador_no_distingue_mayusculas_ni_posicion() {
        let (cmd, consulta, ..) = comando(parsear("--S Ana"));
        assert_eq!(cmd, Comando::Salida);
        assert_eq!(consulta, "Ana");
    }

    #[test]
    fn modificador_de_comando_global_no_es_valido_queda_como_texto() {
        // "nuevo"/"activos"/etc. no actúan sobre un resultado de búsqueda
        // (DEC-021): el texto se conserva tal cual, sin interpretar el "--".
        assert_eq!(
            parsear("Ana --n"),
            Entrada::BusquedaLibre {
                consulta: "Ana --n".to_string()
            }
        );
        assert_eq!(
            parsear("Ana --activos"),
            Entrada::BusquedaLibre {
                consulta: "Ana --activos".to_string()
            }
        );
    }

    #[test]
    fn modificador_desconocido_queda_como_texto() {
        assert_eq!(
            parsear("Ana --xyz"),
            Entrada::BusquedaLibre {
                consulta: "Ana --xyz".to_string()
            }
        );
    }

    #[test]
    fn dos_modificadores_solo_el_primero_se_interpreta() {
        // El segundo, ya sin sentido, degrada a texto libre en vez de
        // generar un error o pelear con el primero.
        let (cmd, consulta, ..) = comando(parsear("Ana --e --i"));
        assert_eq!(cmd, Comando::Editar);
        assert_eq!(consulta, "Ana --i");
    }

    #[test]
    fn busqueda_libre_sin_modificador_no_interpreta_clave_valor() {
        // Sin acción seleccionada, "G:27" es texto de búsqueda tal cual, no
        // un parámetro — evita perder texto de una búsqueda que sólo se
        // parece a un parámetro.
        assert_eq!(
            parsear("Ana G:27"),
            Entrada::BusquedaLibre {
                consulta: "Ana G:27".to_string()
            }
        );
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

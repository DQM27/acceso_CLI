//! Motor genérico de búsqueda "clave:valor" (estilo GitHub/Gmail) de
//! `--comandos` — primer y único consumidor: Historial (DEC-022). El
//! análisis léxico en sí (comillas, escapes, negación `-clave:valor`,
//! separar claves de texto libre) no es nuestro: lo hace el crate
//! `query-parser` (github.com/staktrace/query-parser), ya probado contra
//! esos casos raros (comillas sin cerrar, escapes anidados, guion suelto)
//! que un parser casero tiende a pasar por alto. Lo único que agregamos
//! encima es la noción de "lista" (`clave:a,b,c`), que `query-parser` no
//! modela de forma nativa — sólo tiene `TermValue::Simple(String)` — y un
//! par de helpers para reconstruir el texto original cuando el resolutor no
//! reconoce una clave y hay que devolverla a texto libre en vez de
//! aplicarla a medias.
//!
//! Es una reescritura deliberada de `src/tui/ui_kit/query_lang.rs`, no una
//! importación: `src/tui/` no se toca ni se comparte (DEC-002/DEC-014). El
//! crate ya era una dependencia directa (`Cargo.toml`), así que esto no
//! agrega nada nuevo al árbol de dependencias.
//!
//! Quien use este módulo escribe su propio resolutor sobre
//! `analizar(texto).terms` (ver `historial.rs::aplicar_clave`) — este
//! módulo no sabe qué claves existen ni qué significan.
//!
//! Nota sobre listas: `query-parser` corta un término en el primer espacio
//! en blanco (no tiene noción de listas), así que `clave:a,b,c` es un solo
//! término pero `clave:a, b, c` son tres — una clave suelta `b` y `c` que
//! probablemente no se reconozcan y caigan a texto libre. Para una lista con
//! espacios hay que citarla: `clave:"a, b, c"`.

pub use query_parser::{Query, Term, TermValue};

pub fn analizar(texto: &str) -> Query {
    query_parser::parse(texto)
}

/// `TermValue` sólo tiene la variante `Simple` en `query-parser 0.2.x`
/// (fijado en `Cargo.toml`), así que el patrón es irrefutable hoy — pero
/// deja de compilar (no de fallar en silencio) si una versión futura agrega
/// una variante nueva. Único punto donde se hace este supuesto.
fn valor_simple(term: &Term) -> &str {
    let TermValue::Simple(valor) = &term.value;
    valor
}

/// Valores de una clave, separados por coma (`clave:a,b,c`) y sin espacios
/// sobrantes. Para una clave sin lista (`clave:valor`) devuelve un solo
/// elemento.
pub fn valores(term: &Term) -> Vec<String> {
    valor_simple(term)
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Texto de un término sin clave (texto libre), con el signo de negación si
/// lo tenía, listo para unirse con otros términos libres.
pub fn texto_libre(term: &Term) -> String {
    let valor = valor_simple(term);
    if term.negated {
        format!("-{valor}")
    } else {
        valor.to_owned()
    }
}

/// Reconstruye `clave:valor` (con negación) tal como se volvería a escribir,
/// para cuando no se reconoce la clave, o la combinación negada/lista que
/// trae, y hay que devolverla a texto libre en vez de aplicarla a medias.
pub fn reconstruir_clave(term: &Term) -> String {
    let valor = valor_simple(term);
    let clave = term.key.as_deref().unwrap_or_default();
    let prefijo = if term.negated { "-" } else { "" };
    format!("{prefijo}{clave}:{valor}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolucionConsulta {
    /// Texto que finalmente recibirá la búsqueda libre, incluidos los
    /// términos estructurados que no pudieron interpretarse.
    pub texto_libre: String,
    /// Términos `clave:valor` que se degradaron a texto libre.
    pub no_reconocidos: Vec<String>,
}

/// Separa términos libres de términos con clave, mutando `filtro` vía
/// `aplicar_clave` para cada uno con clave — lo no reconocido (o rechazado
/// por `aplicar_clave`, que devuelve `false` para eso) se reconstruye como
/// texto libre en vez de aplicarse a medias.
pub fn resolver_terminos<F>(
    texto: &str,
    filtro: &mut F,
    mut aplicar_clave: impl FnMut(&mut F, &Term) -> bool,
) -> ResolucionConsulta {
    let mut libres = Vec::new();
    let mut no_reconocidos = Vec::new();
    for term in analizar(texto).terms {
        if term.key.is_none() {
            libres.push(texto_libre(&term));
            continue;
        }
        if !aplicar_clave(filtro, &term) {
            let reconstruido = reconstruir_clave(&term);
            no_reconocidos.push(reconstruido.clone());
            libres.push(reconstruido);
        }
    }
    ResolucionConsulta {
        texto_libre: libres.join(" "),
        no_reconocidos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separa_clave_valor_de_texto_libre_conservando_el_orden() {
        let query = analizar("Ana tipo:praind Solano");
        assert_eq!(query.terms.len(), 3);
        assert_eq!(query.terms[0].key, None);
        assert_eq!(texto_libre(&query.terms[0]), "Ana");
        assert_eq!(query.terms[1].key.as_deref(), Some("tipo"));
        assert_eq!(valores(&query.terms[1]), vec!["praind"]);
        assert_eq!(query.terms[2].key, None);
        assert_eq!(texto_libre(&query.terms[2]), "Solano");
    }

    #[test]
    fn respeta_comillas_como_un_solo_valor() {
        let query = analizar("empresa:\"Brisas del Oeste\"");
        assert_eq!(valores(&query.terms[0]), vec!["Brisas del Oeste"]);
    }

    #[test]
    fn reconoce_negacion_con_prefijo_guion() {
        let query = analizar("-estado:cerrados");
        assert!(query.terms[0].negated);
        assert_eq!(query.terms[0].key.as_deref(), Some("estado"));
        assert_eq!(valores(&query.terms[0]), vec!["cerrados"]);
    }

    #[test]
    fn reconoce_listas_separadas_por_coma_sin_espacios() {
        let query = analizar("tipo:praind,swat,inhouse");
        assert_eq!(valores(&query.terms[0]), vec!["praind", "swat", "inhouse"]);
    }

    #[test]
    fn reconstruir_clave_regenera_el_token_negado_y_con_lista() {
        let query = analizar("-tipo:praind,swat");
        assert_eq!(reconstruir_clave(&query.terms[0]), "-tipo:praind,swat");
    }

    #[test]
    fn resolucion_identifica_claves_que_caen_a_texto_libre() {
        let mut filtro = Vec::<String>::new();
        let resolucion =
            resolver_terminos("Ana tipo:praind clave:invalida", &mut filtro, |f, term| {
                if term.key.as_deref() == Some("tipo") {
                    f.extend(valores(term));
                    true
                } else {
                    false
                }
            });

        assert_eq!(filtro, ["praind"]);
        assert_eq!(resolucion.texto_libre, "Ana clave:invalida");
        assert_eq!(resolucion.no_reconocidos, ["clave:invalida"]);
    }
}

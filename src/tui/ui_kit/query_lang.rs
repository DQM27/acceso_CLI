//! Motor genérico de búsqueda "clave:valor" (estilo GitHub/Gmail), compartido
//! entre pantallas. El análisis léxico en sí (comillas, escapes, negación
//! `-clave:valor`, separar claves de texto libre) no es nuestro: lo hace el
//! crate `query-parser` (github.com/staktrace/query-parser), ya probado
//! contra esos casos raros (comillas sin cerrar, escapes anidados, guion
//! suelto) que un parser casero tiende a pasar por alto. Lo único que
//! agregamos encima es la noción de "lista" (`clave:a,b,c`), que
//! `query-parser` no modela de forma nativa — sólo tiene
//! `TermValue::Simple(String)` — y un par de helpers para reconstruir el
//! texto original cuando el resolutor de una pantalla no reconoce una clave
//! y hay que devolverla a texto libre en vez de aplicarla a medias.
//!
//! Cada pantalla que quiera esta sintaxis escribe su propio resolutor sobre
//! `analizar(texto).terms` (ver `historial/filtros.rs::aplicar_clave` como
//! referencia) — este módulo no sabe qué claves existen ni qué significan.
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

/// Valores de una clave, separados por coma (`clave:a,b,c`) y sin espacios
/// sobrantes. Para una clave sin lista (`clave:valor`) devuelve un solo
/// elemento.
pub fn valores(term: &Term) -> Vec<String> {
    let TermValue::Simple(valor) = &term.value;
    valor
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Texto de un término sin clave (texto libre), con el signo de negación si
/// lo tenía, listo para unirse con otros términos libres.
pub fn texto_libre(term: &Term) -> String {
    let TermValue::Simple(valor) = &term.value;
    if term.negated {
        format!("-{valor}")
    } else {
        valor.clone()
    }
}

/// Reconstruye `clave:valor` (con negación) tal como se volvería a escribir,
/// para cuando un resolutor no reconoce la clave, o la combinación
/// negada/lista que trae, y hay que devolverla a texto libre en vez de
/// aplicarla a medias.
pub fn reconstruir_clave(term: &Term) -> String {
    let TermValue::Simple(valor) = &term.value;
    let clave = term.key.as_deref().unwrap_or_default();
    let prefijo = if term.negated { "-" } else { "" };
    format!("{prefijo}{clave}:{valor}")
}

/// Bucle común a toda pantalla con búsqueda `clave:valor`: separa términos
/// libres de términos con clave, mutando `filtro` vía `aplicar_clave` para
/// cada uno con clave — lo no reconocido (o rechazado por `aplicar_clave`,
/// que devuelve `false` para eso) se reconstruye como texto libre en vez de
/// aplicarse a medias. Devuelve el texto libre acumulado, unido con espacios.
/// Antes este mismo bucle estaba copiado igual en 3 pantallas
/// (`activos`, `contratistas`, `historial`); cada una sólo aporta su propio
/// resolutor de claves — este módulo sigue sin saber qué claves existen.
pub fn resolver_terminos<F>(
    texto: &str,
    filtro: &mut F,
    mut aplicar_clave: impl FnMut(&mut F, &Term) -> bool,
) -> String {
    let mut libres = Vec::new();
    for term in analizar(texto).terms {
        if term.key.is_none() {
            libres.push(texto_libre(&term));
            continue;
        }
        if !aplicar_clave(filtro, &term) {
            libres.push(reconstruir_clave(&term));
        }
    }
    libres.join(" ")
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
        // query-parser corta un término en el primer espacio en blanco (no
        // sabe de listas), así que una lista sólo es un término si no lleva
        // espacios sueltos: "a,b,c" sí, "a, b, c" se parte en tres términos.
        let query = analizar("tipo:praind,swat,inhouse");
        assert_eq!(valores(&query.terms[0]), vec!["praind", "swat", "inhouse"]);
    }

    #[test]
    fn una_lista_entre_comillas_admite_espacios_despues_de_la_coma() {
        let query = analizar("tipo:\"praind, swat\"");
        assert_eq!(valores(&query.terms[0]), vec!["praind", "swat"]);
    }

    #[test]
    fn reconstruir_clave_regenera_el_token_negado_y_con_lista() {
        let query = analizar("-tipo:praind,swat");
        assert_eq!(reconstruir_clave(&query.terms[0]), "-tipo:praind,swat");
    }
}

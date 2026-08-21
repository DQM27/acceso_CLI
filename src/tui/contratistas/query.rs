//! Lenguaje de búsqueda `clave:valor` de la pantalla Contratistas.

use chrono::NaiveDate;

use crate::{
    database::queries::{
        Igualdad,
        contratistas::{FiltroContratistas, FiltroPraind},
    },
    models::{empresa::Empresa, tipo_ingreso::TipoIngreso},
    tui::ui_kit::{Term, plegar_diacriticos, resolver_terminos, valores},
};

/// Interpreta `texto` con la sintaxis `clave:valor` de `ui_kit::query_lang`
/// (`empresa:`, `tipo:`, `praind:vence|vencido|sin` — todas con negación
/// `-clave:valor` —, `ruta:si|no`, `acceso:si|no`). Lo no reconocido, y lo
/// que no calza con lo que una clave admite, se deja como texto libre para
/// nombre/cédula.
pub(super) fn parsear_consulta(
    texto: &str,
    empresas: &[Empresa],
    hoy: NaiveDate,
) -> (FiltroContratistas, String) {
    let mut filtro = FiltroContratistas::default();
    let libres = resolver_terminos(texto, &mut filtro, |f, term| {
        aplicar_clave(f, term, empresas, hoy)
    });
    (filtro, libres)
}

pub(super) fn aplicar_clave(
    f: &mut FiltroContratistas,
    term: &Term,
    empresas: &[Empresa],
    hoy: NaiveDate,
) -> bool {
    let clave = term.key.as_deref().unwrap_or_default().to_lowercase();
    let valores = valores(term);
    match clave.as_str() {
        "empresa" if valores.len() == 1 => {
            let buscado = plegar_diacriticos(&valores[0].to_lowercase());
            match empresas
                .iter()
                .find(|e| plegar_diacriticos(&e.nombre.to_lowercase()).contains(&buscado))
            {
                Some(e) => {
                    f.empresa_id = Some(if term.negated {
                        Igualdad::Excluye(e.id)
                    } else {
                        Igualdad::Incluye(e.id)
                    });
                    true
                }
                None => false,
            }
        }
        "tipo" => {
            let Some(reconocidos) = valores
                .iter()
                .map(|v| TipoIngreso::from_str_filtro(v))
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            if reconocidos.is_empty() {
                return false;
            }
            f.tipos_incluidos = Some(if term.negated {
                TipoIngreso::ALL
                    .into_iter()
                    .filter(|t| !reconocidos.contains(t))
                    .collect()
            } else {
                reconocidos
            });
            true
        }
        "praind" if valores.len() == 1 => match valores[0].to_lowercase().as_str() {
            "vence" | "proximo" | "próximo" => {
                f.praind = Some(FiltroPraind::ProximoAVencer { hoy });
                f.praind_negado = term.negated;
                true
            }
            "vencido" => {
                f.praind = Some(FiltroPraind::Vencido { hoy });
                f.praind_negado = term.negated;
                true
            }
            "sin" | "sinfecha" => {
                f.praind = Some(FiltroPraind::SinFecha);
                f.praind_negado = term.negated;
                true
            }
            _ => false,
        },
        "ruta" if valores.len() == 1 => match bool_desde_texto(&valores[0]) {
            Some(b) => {
                f.personal_ruta = Some(b != term.negated);
                true
            }
            None => false,
        },
        "acceso" if valores.len() == 1 => match bool_desde_texto(&valores[0]) {
            Some(b) => {
                f.tiene_acceso = Some(b != term.negated);
                true
            }
            None => false,
        },
        _ => false,
    }
}

fn bool_desde_texto(v: &str) -> Option<bool> {
    match v.to_lowercase().as_str() {
        "si" | "sí" | "yes" | "true" => Some(true),
        "no" | "false" => Some(false),
        _ => None,
    }
}

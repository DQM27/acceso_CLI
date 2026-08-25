//! Resultados de búsqueda por texto libre: contratistas (con columnas F4 y
//! paginación exacta), empresas y usuarios (listas simples, paginación
//! aproximada) — las tres superficies que alimenta `resolver::pagina_*`.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::comandos::columnas::{Columna, ColumnaBusqueda, SelectorColumnas};
use crate::comandos::resolver::{es_comodin_todos, MIN_CONSULTA};

use super::estilos::{acento, estilo_seleccion, muted};
use super::tabla::{anchos_columnas, columnas_visibles, fila_columnas};
use super::util::{rol_texto, si_no, tipo_texto};

fn ancho_fijo_busqueda(columna: ColumnaBusqueda) -> Option<usize> {
    match columna {
        ColumnaBusqueda::Cedula => Some(14),
        ColumnaBusqueda::Tipo => Some(12),
        ColumnaBusqueda::Praind => Some(12),
        ColumnaBusqueda::Ruta => Some(6),
        ColumnaBusqueda::Acceso => Some(8),
        ColumnaBusqueda::Estado => Some(8),
        ColumnaBusqueda::Nombre | ColumnaBusqueda::Empresa => None,
    }
}

/// Tope propio por columna flexible: nombre de persona (Nombre) suele
/// necesitar bastante más que nombre de empresa (Empresa, en la práctica casi
/// siempre corto) — antes ambas repartían el mismo `ANCHO_FLEXIBLE_MAXIMO` en
/// partes iguales, dejando a Empresa con espacio de sobra sin usar mientras
/// Nombre truncaba con "…" (reportado en runtime real). Sólo se invoca sobre
/// columnas flexibles (`ancho_fijo_busqueda` devuelve `None`), así que el
/// resto de variantes no importa acá.
fn ancho_maximo_busqueda(columna: ColumnaBusqueda) -> usize {
    match columna {
        ColumnaBusqueda::Empresa => 22,
        _ => 40,
    }
}

fn valor_busqueda(
    item: &crate::database::queries::contratistas::ContratistaResumen,
    columna: ColumnaBusqueda,
) -> String {
    match columna {
        ColumnaBusqueda::Cedula => item.cedula.clone(),
        ColumnaBusqueda::Nombre => item.nombre.clone(),
        ColumnaBusqueda::Empresa => item.empresa_nombre.clone(),
        ColumnaBusqueda::Tipo => tipo_texto(item.tipo_ingreso).to_string(),
        ColumnaBusqueda::Praind => item
            .fecha_vencimiento_praind
            .map(|fecha| fecha.format("%d/%m/%Y").to_string())
            .unwrap_or_else(|| "—".to_string()),
        ColumnaBusqueda::Ruta => si_no(item.es_personal_ruta).to_string(),
        ColumnaBusqueda::Acceso => si_no(item.tiene_acceso).to_string(),
        ColumnaBusqueda::Estado => {
            if item.tiene_ingreso_activo {
                "DENTRO".to_string()
            } else {
                "FUERA".to_string()
            }
        }
    }
}

/// Pie de paginación con conteo exacto (contratistas: `buscar_contratistas`
/// ya devuelve `total` sin límite) — "X-Y de Z" más PageUp/PageDown según
/// corresponda. `None` en la única página que además es la primera: nada
/// que paginar, no hace falta el pie.
fn linea_paginacion_exacta(offset: usize, cantidad: usize, total: usize) -> Option<Line<'static>> {
    if offset == 0 && cantidad >= total {
        return None;
    }
    let desde = offset + 1;
    let hasta = offset + cantidad;
    let mas = if hasta < total { " · PageDown más" } else { "" };
    let atras = if offset > 0 { " · PageUp atrás" } else { "" };
    Some(Line::from(Span::styled(
        format!("{desde}-{hasta} de {total}{mas}{atras}"),
        muted(),
    )))
}

/// Mismo pie que `linea_paginacion_exacta`, sin el "de Z": empresas/usuarios
/// no tienen un conteo total (`hay_mas` viene del truco del elemento de más,
/// ver `resolver::pagina_empresas`), así que no se puede prometer una cifra
/// exacta — sólo el rango cargado y si hay para adelante/atrás.
fn linea_paginacion_aproximada(
    offset: usize,
    cantidad: usize,
    hay_mas: bool,
) -> Option<Line<'static>> {
    if offset == 0 && !hay_mas {
        return None;
    }
    let desde = offset + 1;
    let hasta = offset + cantidad;
    let mas = if hay_mas { " · PageDown más" } else { "" };
    let atras = if offset > 0 { " · PageUp atrás" } else { "" };
    Some(Line::from(Span::styled(
        format!("{desde}-{hasta}{mas}{atras}"),
        muted(),
    )))
}

/// Lista simple, sin columnas ni F4: `EmpresaResumen`/`UsuarioResumen`
/// tienen pocos campos (3 y 4) y esta pantalla es de paso — elegir con ↑↓ y
/// entrar al formulario de edición — no un reporte que valga la pena poder
/// reconfigurar (DEC-052).
pub(super) fn lineas_coincidencias_empresas(
    consulta: &str,
    items: &[crate::database::queries::empresas::EmpresaResumen],
    seleccion: usize,
    offset: usize,
    hay_mas: bool,
) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled("EDITAR EMPRESA", muted())),
        Line::from(""),
    ];
    if !es_comodin_todos(consulta) && consulta.chars().count() < MIN_CONSULTA {
        lineas.push(Line::from(Span::styled(
            format!("Escriba al menos {MIN_CONSULTA} letras para buscar, o \"*\" para ver todas"),
            muted(),
        )));
        return lineas;
    }
    if items.is_empty() {
        lineas.push(Line::from(Span::styled(
            format!("Sin empresas para \"{consulta}\""),
            muted(),
        )));
        return lineas;
    }
    for (indice, empresa) in items.iter().enumerate() {
        let marcador = if indice == seleccion { "› " } else { "  " };
        let estado = if empresa.activo { "" } else { " (inactiva)" };
        let texto = format!("{marcador}{}{estado}", empresa.nombre);
        lineas.push(if indice == seleccion {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else {
            Line::from(texto)
        });
    }
    if let Some(linea) = linea_paginacion_aproximada(offset, items.len(), hay_mas) {
        lineas.push(Line::from(""));
        lineas.push(linea);
    }
    lineas
}

pub(super) fn lineas_coincidencias_usuarios(
    consulta: &str,
    items: &[crate::database::queries::usuarios::UsuarioResumen],
    seleccion: usize,
    offset: usize,
    hay_mas: bool,
) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled("EDITAR USUARIO", muted())),
        Line::from(""),
    ];
    if !es_comodin_todos(consulta) && consulta.chars().count() < MIN_CONSULTA {
        lineas.push(Line::from(Span::styled(
            format!("Escriba al menos {MIN_CONSULTA} letras para buscar, o \"*\" para ver todos"),
            muted(),
        )));
        return lineas;
    }
    if items.is_empty() {
        lineas.push(Line::from(Span::styled(
            format!("Sin usuarios para \"{consulta}\""),
            muted(),
        )));
        return lineas;
    }
    for (indice, usuario) in items.iter().enumerate() {
        let marcador = if indice == seleccion { "› " } else { "  " };
        let estado = if usuario.activo { "" } else { " (inactivo)" };
        let texto = format!(
            "{marcador}{} — {} — {}{estado}",
            usuario.cedula,
            usuario.nombre,
            rol_texto(usuario.rol)
        );
        lineas.push(if indice == seleccion {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else {
            Line::from(texto)
        });
    }
    if let Some(linea) = linea_paginacion_aproximada(offset, items.len(), hay_mas) {
        lineas.push(Line::from(""));
        lineas.push(linea);
    }
    lineas
}

pub(super) fn lineas_coincidencias(
    consulta: &str,
    items: &[crate::database::queries::contratistas::ContratistaResumen],
    seleccion: usize,
    offset: usize,
    total: usize,
    ancho: u16,
    columnas: &SelectorColumnas<ColumnaBusqueda>,
) -> Vec<Line<'static>> {
    if !es_comodin_todos(consulta) && consulta.chars().count() < MIN_CONSULTA {
        return vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Escriba al menos {MIN_CONSULTA} letras para buscar, o \"*\" para ver todos"),
                muted(),
            )),
        ];
    }
    if items.is_empty() {
        return vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Sin coincidencias para \"{consulta}\""),
                muted(),
            )),
        ];
    }
    // Mismas 7 columnas que la tabla de contratistas de la TUI clásica
    // (cédula/nombre/empresa/tipo/praind/ruta/acceso) — sólo se listan las
    // que estén visibles (F4, ColumnaBusqueda).
    let anchos = anchos_columnas(
        ancho,
        columnas_visibles(columnas),
        ancho_fijo_busqueda,
        ancho_maximo_busqueda,
    );
    let mut lineas = vec![
        Line::from(Span::styled(
            format!(
                "  {}",
                fila_columnas(&anchos, |_| false, |c| c.etiqueta().to_uppercase())
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("─".repeat(ancho as usize), muted())),
    ];
    // `Estado` (DENTRO/FUERA) se separa del resto en su propio `Span` para
    // llevar color propio (mismo criterio que `lineas_ficha`: DENTRO en
    // acento, FUERA atenuado). La fila se arma completa con un solo
    // `fila_columnas(&anchos, ...)` — igual que antes de esta columna — y
    // recién después se corta el string resultante en el punto donde
    // empieza Estado; partir directamente el slice `anchos` en dos (como en
    // un primer intento) hacía que la penúltima columna (Acceso) perdiera su
    // padding, por quedar marcada como "la última" de su propio segmento
    // (`fila_columnas` no rellena la última celda) — acá sigue siendo la
    // penúltima de la fila real, así que conserva su ancho fijo de siempre.
    let ancho_prefijo_estado: usize = anchos
        .iter()
        .filter(|(c, _)| *c != ColumnaBusqueda::Estado)
        .map(|(_, ancho)| *ancho)
        .sum();
    let hay_estado = anchos.iter().any(|(c, _)| *c == ColumnaBusqueda::Estado);
    for (indice, item) in items.iter().enumerate() {
        let marcador = if indice == seleccion { "› " } else { "  " };
        let seleccionado = indice == seleccion;
        let texto_completo = format!(
            "{marcador}{}",
            fila_columnas(&anchos, |_| false, |c| valor_busqueda(item, c))
        );
        if !hay_estado {
            lineas.push(if seleccionado {
                Line::from(Span::styled(texto_completo, estilo_seleccion()))
            } else {
                Line::from(texto_completo)
            });
            continue;
        }
        // Posición en CARACTERES (no bytes: `marcador` trae "›", 3 bytes en
        // UTF-8 pero 1 solo carácter) del corte, resuelta después a un
        // índice de byte válido con `char_indices` — cortar por bytes a
        // secas partiría un carácter multi-byte del nombre a la mitad y
        // haría panic.
        let corte_caracteres = marcador.chars().count() + ancho_prefijo_estado;
        let corte_bytes = texto_completo
            .char_indices()
            .nth(corte_caracteres)
            .map(|(indice_byte, _)| indice_byte)
            .unwrap_or(texto_completo.len());
        let (resto, estado) = texto_completo.split_at(corte_bytes);
        let estilo_resto = if seleccionado {
            estilo_seleccion()
        } else {
            Style::default()
        };
        let color_estado = if item.tiene_ingreso_activo {
            acento()
        } else {
            muted()
        };
        let estilo_estado = if seleccionado {
            color_estado.add_modifier(Modifier::REVERSED)
        } else {
            color_estado
        };
        lineas.push(Line::from(vec![
            Span::styled(resto.to_string(), estilo_resto),
            Span::styled(estado.to_string(), estilo_estado),
        ]));
    }
    if let Some(linea) = linea_paginacion_exacta(offset, items.len(), total) {
        lineas.push(Line::from(""));
        lineas.push(linea);
    }
    lineas
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texto_linea(linea: &Line<'static>) -> String {
        linea.spans.iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn paginacion_exacta_sin_pie_en_la_unica_pagina() {
        assert!(linea_paginacion_exacta(0, 5, 5).is_none());
    }

    #[test]
    fn paginacion_exacta_muestra_rango_y_pagedown_si_falta() {
        let linea = linea_paginacion_exacta(0, 9, 23).unwrap();
        assert_eq!(texto_linea(&linea), "1-9 de 23 · PageDown más");
    }

    #[test]
    fn paginacion_exacta_muestra_pageup_en_la_ultima_pagina() {
        let linea = linea_paginacion_exacta(18, 5, 23).unwrap();
        assert_eq!(texto_linea(&linea), "19-23 de 23 · PageUp atrás");
    }

    #[test]
    fn paginacion_aproximada_sin_pie_en_la_primera_pagina_sin_mas() {
        assert!(linea_paginacion_aproximada(0, 3, false).is_none());
    }

    #[test]
    fn paginacion_aproximada_sin_total_muestra_solo_el_rango() {
        let linea = linea_paginacion_aproximada(0, 9, true).unwrap();
        assert_eq!(texto_linea(&linea), "1-9 · PageDown más");
        let linea = linea_paginacion_aproximada(9, 4, false).unwrap();
        assert_eq!(texto_linea(&linea), "10-13 · PageUp atrás");
    }
}

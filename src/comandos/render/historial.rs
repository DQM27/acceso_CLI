//! Superficie enclavada de Historial (§5.2/DEC-023/024): resumen del filtro
//! vigente, tabla de movimientos con columnas F4, y la pantalla de
//! exportación a XLSX (F5).

use chrono::Utc;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::comandos::columnas::{Columna, ColumnaHistorial, SelectorColumnas};
use crate::comandos::historial::HistorialState;
use crate::tiempo::a_costa_rica;

use super::estilos::{advertencia, estilo_fundido, estilo_seleccion, muted, FADE_ACENTO, FADE_MUTED};
use super::tabla::{anchos_columnas, columnas_visibles, fila_columnas};
use super::util::tipo_texto;

fn ancho_fijo_historial(columna: ColumnaHistorial) -> Option<usize> {
    match columna {
        // 18 = "25/08/2026 15:44" (16 caracteres) + los 2 de separación —
        // antes eran 13, pensados para el formato sin año ("25/08 15:44").
        ColumnaHistorial::Ingreso | ColumnaHistorial::Salida => Some(18),
        ColumnaHistorial::Tipo => Some(12),
        ColumnaHistorial::Gafete => Some(8),
        ColumnaHistorial::Nombre | ColumnaHistorial::Empresa | ColumnaHistorial::Usuario => None,
    }
}

/// Mismo criterio que `busqueda.rs::ancho_maximo_busqueda`/
/// `activos.rs::ancho_maximo_activos`: Nombre (persona) más ancho que
/// Empresa (nombre de empresa, casi siempre corto); Usuario ("Da ingreso")
/// con un tope intermedio.
fn ancho_maximo_historial(columna: ColumnaHistorial) -> usize {
    match columna {
        ColumnaHistorial::Empresa => 22,
        ColumnaHistorial::Usuario => 26,
        _ => 40,
    }
}

fn derecha_historial(columna: ColumnaHistorial) -> bool {
    matches!(columna, ColumnaHistorial::Gafete)
}

/// Mismo formato `%d/%m/%Y %H:%M` que ya usa el resto de la app para fecha +
/// hora (`tui::activos::render`, `tui::auditoria::render`,
/// `tui::configuracion::state`) — acá mostraba sólo `%d/%m %H:%M`, sin año,
/// inconsistente con el resto y ambiguo en un historial que sí puede cruzar
/// años (reportado en runtime real).
fn fecha_hora_corta(instante: chrono::DateTime<Utc>) -> String {
    a_costa_rica(instante).format("%d/%m/%Y %H:%M").to_string()
}

/// `FiltroHistorial::hasta` es el límite exclusivo (inicio del día
/// siguiente al último incluido) — para mostrarlo como la fecha "hasta" que
/// el operador espera ver, se resta un día antes de formatear.
fn fecha_hasta_visual(hasta: chrono::DateTime<Utc>) -> String {
    a_costa_rica(hasta - chrono::Duration::days(1))
        .format("%d/%m/%Y")
        .to_string()
}

fn valor_historial(
    m: &crate::database::queries::ingresos::MovimientoIngresoResumen,
    columna: ColumnaHistorial,
) -> String {
    match columna {
        ColumnaHistorial::Ingreso => fecha_hora_corta(m.fecha_hora_ingreso),
        ColumnaHistorial::Nombre => m.contratista_nombre.clone(),
        ColumnaHistorial::Empresa => m.empresa_nombre.clone(),
        ColumnaHistorial::Tipo => tipo_texto(m.tipo_ingreso).to_string(),
        ColumnaHistorial::Gafete => m
            .gafete_numero
            .map(|numero| numero.to_string())
            .unwrap_or_else(|| "—".to_string()),
        ColumnaHistorial::Salida => m
            .fecha_hora_salida
            .map(fecha_hora_corta)
            .unwrap_or_else(|| "— activo".to_string()),
        ColumnaHistorial::Usuario => m.usuario_ingreso_nombre.clone(),
    }
}

/// Resume el filtro vigente en una línea ("empresa: Brisas · tipo: PRAIND
/// o SWAT · ⚠ sin interpretar: clave:x"), igual criterio que la etiqueta de
/// búsqueda de la TUI clásica — para que el operador vea qué se aplicó de
/// verdad sin tener que releer lo que tecleó.
fn resumen_filtro_historial(historial: &HistorialState) -> String {
    let f = &historial.filtro;
    let mut partes = Vec::new();
    partes.push(format!(
        "{} – {}",
        a_costa_rica(f.desde).format("%d/%m/%Y"),
        fecha_hasta_visual(f.hasta)
    ));
    if let Some(empresa_id) = &f.empresa_id {
        let nombre = historial
            .empresas
            .iter()
            .find(|e| e.id == *empresa_id.valor())
            .map_or("?", |e| e.nombre.as_str());
        let signo = if matches!(empresa_id, crate::database::queries::Igualdad::Excluye(_)) {
            "≠"
        } else {
            ""
        };
        partes.push(format!("empresa: {signo}{nombre}"));
    }
    if let Some(tipos) = &f.tipos_incluidos {
        partes.push(format!(
            "tipo: {}",
            tipos
                .iter()
                .map(|t| tipo_texto(*t))
                .collect::<Vec<_>>()
                .join(" o ")
        ));
    }
    if f.estado != crate::database::queries::ingresos::EstadoMovimiento::Todos {
        let texto = match f.estado {
            crate::database::queries::ingresos::EstadoMovimiento::Activos => "Activos",
            crate::database::queries::ingresos::EstadoMovimiento::Cerrados => "Cerrados",
            crate::database::queries::ingresos::EstadoMovimiento::Todos => unreachable!(),
        };
        partes.push(format!("estado: {texto}"));
    }
    if let Some(gafete) = &f.gafete_numero {
        let signo = if matches!(gafete, crate::database::queries::Igualdad::Excluye(_)) {
            "≠"
        } else {
            ""
        };
        partes.push(format!("gafete: {signo}{}", gafete.valor()));
    }
    if let Some(usuario) = &f.usuario_ingreso {
        let signo = if f.usuario_ingreso_negado { "≠" } else { "" };
        partes.push(format!("ingreso: {signo}{usuario}"));
    }
    if let Some(usuario) = &f.usuario_salida {
        let signo = if f.usuario_salida_negado { "≠" } else { "" };
        partes.push(format!("salida: {signo}{usuario}"));
    }
    if let Some(texto) = &f.texto_persona {
        partes.push(format!("\"{texto}\""));
    }
    partes.join(" · ")
}

/// Opacidades vigentes de la Surface de Historial (Fase 5).
pub(super) struct OpacidadesHistorial {
    /// Encabezado del resultado aplicado (funde al aparecer o al cambiar de
    /// página/consulta).
    pub(super) resultado: f32,
    /// Pantalla de exportación (`F5`).
    pub(super) exportar: f32,
}

pub(super) fn lineas_historial(
    historial: &HistorialState,
    texto_input: &str,
    ancho: u16,
    columnas: &SelectorColumnas<ColumnaHistorial>,
    opacidades: &OpacidadesHistorial,
) -> (Vec<Line<'static>>, Option<usize>) {
    if historial.exportacion_destino.is_some() {
        let total = historial.resultado.as_ref().map_or(0, |r| r.total);
        return (
            vec![
                Line::from(Span::styled(
                    "EXPORTAR HISTORIAL",
                    estilo_fundido(FADE_MUTED, opacidades.exportar, Modifier::empty()),
                )),
                Line::from(""),
                Line::from(format!(
                    "Se exportarán los {total} movimientos del filtro vigente a un archivo XLSX."
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Enter para exportar · Esc para cancelar",
                    estilo_fundido(FADE_ACENTO, opacidades.exportar, Modifier::empty()),
                )),
            ],
            None,
        );
    }
    let Some(resultado) = &historial.resultado else {
        // Editando: todavía no se aplicó ninguna consulta (o se volvió a
        // editar con Esc) — sin filtrado en vivo, DEC-024.
        let rango = format!(
            "Rango actual: {} – {}",
            a_costa_rica(historial.filtro.desde).format("%d/%m/%Y"),
            fecha_hasta_visual(historial.filtro.hasta)
        );
        return (
            vec![
                Line::from(Span::styled("HISTORIAL", muted())),
                Line::from(""),
                Line::from(Span::styled(rango, muted())),
                Line::from(Span::styled(
                    "empresa: · tipo: · estado: · gafete: · ingreso: · salida: · desde: · hasta:",
                    muted(),
                )),
                Line::from(Span::styled(
                    "Ejemplo: empresa:brisas tipo:praind,swat desde:01/08/2026 -salida:ana",
                    muted(),
                )),
                Line::from(""),
                Line::from(if texto_input.is_empty() {
                    Span::styled(
                        "Enter aplica el rango del mes actual sin más filtro",
                        muted(),
                    )
                } else {
                    Span::raw(texto_input.to_string())
                }),
            ],
            None,
        );
    };

    let mut lineas = vec![Line::from(Span::styled(
        resumen_filtro_historial(historial),
        estilo_fundido(FADE_MUTED, opacidades.resultado, Modifier::empty()),
    ))];
    if !historial.no_reconocidos.is_empty() {
        lineas.push(Line::from(Span::styled(
            format!("⚠ sin interpretar: {}", historial.no_reconocidos.join(", ")),
            advertencia(),
        )));
    }
    lineas.push(Line::from(""));

    if resultado.items.is_empty() {
        lineas.push(Line::from(Span::styled(
            "Sin movimientos para este filtro",
            muted(),
        )));
        return (lineas, None);
    }

    let anchos = anchos_columnas(
        ancho,
        columnas_visibles(columnas),
        ancho_fijo_historial,
        ancho_maximo_historial,
    );
    lineas.push(Line::from(Span::styled(
        format!(
            "  {}",
            fila_columnas(&anchos, derecha_historial, |c| c.etiqueta().to_uppercase())
        ),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lineas.push(Line::from(Span::styled(
        "─".repeat(ancho as usize),
        muted(),
    )));
    // El preámbulo hasta acá ya varía (resumen del filtro + aviso opcional
    // de no-reconocidos + encabezado + divisor) — a diferencia de
    // `FILAS_ANTES_DE_ITEMS`, acá conviene tomar el largo real en vez de
    // otra constante que se desincronizaría fácil.
    let seleccionada = lineas.len() + historial.seleccion;
    for (indice, item) in resultado.items.iter().enumerate() {
        let marcador = if indice == historial.seleccion {
            "› "
        } else {
            "  "
        };
        let texto = format!(
            "{marcador}{}",
            fila_columnas(&anchos, derecha_historial, |c| valor_historial(item, c))
        );
        lineas.push(if indice == historial.seleccion {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else {
            Line::from(texto)
        });
    }
    lineas.push(Line::from(""));
    let desde = historial.filtro.offset + 1;
    let hasta = historial.filtro.offset + resultado.items.len();
    lineas.push(Line::from(Span::styled(
        format!(
            "{desde}–{hasta} de {} · PageUp/PageDown para más",
            resultado.total
        ),
        muted(),
    )));
    (lineas, Some(seleccionada))
}

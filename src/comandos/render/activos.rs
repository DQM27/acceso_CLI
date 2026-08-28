//! Todo lo que gira en torno a ingresos activos: la tabla de `/activos`, la
//! búsqueda filtrada, las tarjetas de confirmación de ingreso/salida, la
//! ficha de un contratista y el modo enclavado de salida por gafete.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::comandos::breakpoint::Breakpoint;
use crate::comandos::columnas::{Columna, ColumnaActivos, SelectorColumnas};
use crate::comandos::estado::ContextState;
use crate::comandos::salida_gafete::SalidaGafeteState;
use crate::domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};
use crate::services::registro_ingreso_service::IngresoActivoResumen;

use super::estilos::{acento, estilo_error, estilo_seleccion, exito, muted};
use super::tabla::{anchos_columnas, columnas_visibles, fila_columnas};
use super::util::{cantidad_personas, duracion_texto, hora_cr, medio_texto, tipo_texto};

fn ancho_fijo_activos(columna: ColumnaActivos) -> Option<usize> {
    match columna {
        ColumnaActivos::Cedula => Some(14),
        ColumnaActivos::Tipo => Some(12),
        ColumnaActivos::Hora => Some(7),
        ColumnaActivos::Gafete => Some(8),
        ColumnaActivos::Medio => Some(11),
        ColumnaActivos::Nombre | ColumnaActivos::Empresa | ColumnaActivos::Usuario => None,
    }
}

/// Mismo criterio que `busqueda.rs::ancho_maximo_busqueda`: Nombre (persona
/// que entra) necesita bastante más que Empresa (nombre de empresa, casi
/// siempre corto); Usuario ("Da ingreso", el nombre de quien opera) es
/// persona también, pero el mismo operador se repite fila tras fila y rara
/// vez hace falta verlo tan ancho como el contratista — se le da un tope
/// intermedio, no el mínimo de Empresa.
fn ancho_maximo_activos(columna: ColumnaActivos) -> usize {
    match columna {
        ColumnaActivos::Empresa => 22,
        ColumnaActivos::Usuario => 26,
        _ => 40,
    }
}

/// Sólo Gafete se alinea a la derecha: es la única columna numérica que
/// queda pegada a otra (Hora) — alinearla a la derecha la separa de sus
/// dígitos en vez de leerse como un solo número (ver captura real).
fn derecha_activos(columna: ColumnaActivos) -> bool {
    matches!(columna, ColumnaActivos::Gafete)
}

fn valor_activos(item: &IngresoActivoResumen, columna: ColumnaActivos) -> String {
    match columna {
        ColumnaActivos::Cedula => item.cedula.clone(),
        ColumnaActivos::Nombre => item.contratista_nombre.clone(),
        ColumnaActivos::Empresa => item.empresa_nombre.clone(),
        ColumnaActivos::Tipo => tipo_texto(item.tipo_ingreso).to_string(),
        ColumnaActivos::Hora => hora_cr(item.fecha_hora_ingreso),
        ColumnaActivos::Gafete => item
            .gafete_numero
            .map(|numero| numero.to_string())
            .unwrap_or_else(|| "S/G".to_string()),
        ColumnaActivos::Medio => medio_texto(item.medio_ingreso).to_string(),
        ColumnaActivos::Usuario => item.usuario_ingreso_nombre.clone(),
    }
}

pub(super) fn lineas_coincidencias_activos(
    descripcion: &str,
    items: &[IngresoActivoResumen],
    seleccion: usize,
    ancho: u16,
    columnas: &SelectorColumnas<ColumnaActivos>,
) -> Vec<Line<'static>> {
    if items.is_empty() {
        let mensaje = if descripcion.is_empty() {
            "Escriba un nombre o G:<número> del gafete…".to_string()
        } else {
            format!("No hay ingreso activo para {descripcion}")
        };
        return vec![Line::from(""), Line::from(Span::styled(mensaje, muted()))];
    }
    // Mismas 8 columnas que `/activos` — es la misma fuente de datos
    // (ingresos activos), sólo que filtrada por la búsqueda. Sólo se listan
    // las columnas visibles (F4, ColumnaActivos).
    let anchos = anchos_columnas(
        ancho,
        columnas_visibles(columnas),
        ancho_fijo_activos,
        ancho_maximo_activos,
    );
    let mut lineas = vec![
        Line::from(Span::styled(
            format!(
                "  {}",
                fila_columnas(&anchos, derecha_activos, |c| c.etiqueta().to_uppercase())
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("─".repeat(ancho as usize), muted())),
    ];
    for (indice, item) in items.iter().enumerate() {
        let marcador = if indice == seleccion { "› " } else { "  " };
        let texto = format!(
            "{marcador}{}",
            fila_columnas(&anchos, derecha_activos, |c| valor_activos(item, c))
        );
        lineas.push(if indice == seleccion {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else {
            Line::from(texto)
        });
    }
    lineas
}

fn chequeo(simbolo: &str, estilo: Style, texto: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{simbolo} "), estilo),
        Span::raw(texto),
    ])
}

fn motivo_denegacion_texto(motivo: &MotivoDenegacion) -> String {
    match motivo {
        MotivoDenegacion::SinAcceso => "Sin acceso autorizado".into(),
        MotivoDenegacion::PraindVencido => "PRAIND vencido".into(),
        MotivoDenegacion::PraindNoRegistrado => "PRAIND sin fecha registrada".into(),
        MotivoDenegacion::EmpresaInactiva => "La empresa está inactiva".into(),
    }
}

/// Tarjeta de validación previa al ingreso: un símbolo por chequeo, y al pie
/// la acción disponible (registrar sólo si todo está en ✓/⚠).
pub(super) fn lineas_resumen_ingreso(contexto: &ContextState) -> Vec<Line<'static>> {
    let ContextState::ResumenIngreso {
        preparacion,
        gafete,
        medio,
        gafete_ocupante,
    } = contexto
    else {
        return Vec::new();
    };

    let mut lineas = vec![
        Line::from(Span::styled("INGRESO", muted())),
        Line::from(vec![
            Span::styled("Cédula:  ", muted()),
            Span::raw(preparacion.cedula.clone()),
        ]),
        Line::from(vec![
            Span::styled("Nombre:  ", muted()),
            Span::styled(
                preparacion.nombre.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Empresa: ", muted()),
            Span::raw(preparacion.empresa_nombre.clone()),
        ]),
        Line::from(vec![
            Span::styled("Tipo:    ", muted()),
            Span::raw(tipo_texto(preparacion.tipo_ingreso)),
        ]),
        Line::from(""),
    ];

    lineas.push(match &preparacion.resultado_acceso {
        ResultadoAcceso::Permitido => chequeo("✓", exito(), "Acceso autorizado".into()),
        ResultadoAcceso::PermitidoConAdvertencia => chequeo(
            "⚠",
            super::estilos::advertencia(),
            "Acceso autorizado — PRAIND próximo a vencer".into(),
        ),
        ResultadoAcceso::Denegado(motivo) => {
            chequeo("✗", estilo_error(), motivo_denegacion_texto(motivo))
        }
    });

    lineas.push(if preparacion.tiene_ingreso_activo {
        chequeo("✗", estilo_error(), "Ya tiene un ingreso activo".into())
    } else {
        chequeo("✓", exito(), "Sin ingreso activo".into())
    });

    if preparacion.requiere_gafete {
        lineas.push(match (gafete, gafete_ocupante) {
            (None, _) => chequeo(
                "✗",
                estilo_error(),
                "Gafete requerido: indique con G:<número>".into(),
            ),
            (Some(_), Some(ocupante)) => chequeo(
                "✗",
                estilo_error(),
                format!(
                    "Gafete {} ocupado por {}",
                    gafete.unwrap_or_default(),
                    ocupante.contratista_nombre
                ),
            ),
            (Some(numero), None) => chequeo("✓", exito(), format!("Gafete {numero} disponible")),
        });
    } else {
        lineas.push(chequeo("—", muted(), "No requiere gafete".into()));
    }

    lineas.push(Line::from(format!("   Medio: {}", medio_texto(*medio))));
    lineas.push(Line::from(""));
    lineas.push(if contexto.ingreso_confirmable() {
        Line::from(Span::styled(
            "ENTER para registrar ingreso · Esc para cancelar",
            acento(),
        ))
    } else {
        Line::from(Span::styled(
            "No se puede registrar: revise los ✗ · Esc para cancelar",
            muted(),
        ))
    });
    lineas
}

/// Superficie del modo enclavado de salida por gafete (DEC-057): una fila
/// por número reconocido en `estado.texto`, con el nombre ya resuelto en
/// vivo — esa vista previa es la confirmación (no hay una segunda
/// pantalla): lo que se ve acá es exactamente a quién se le va a registrar
/// la salida al presionar Enter.
pub(super) fn lineas_salida_gafete(estado: &SalidaGafeteState) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled("SALIDA POR GAFETE", muted())),
        Line::from(""),
    ];
    if estado.coincidencias.is_empty() {
        lineas.push(Line::from(Span::styled(
            "Escriba uno o varios números de gafete, separados por coma…",
            muted(),
        )));
        return lineas;
    }
    for (numero, item) in &estado.coincidencias {
        let linea = match item {
            Some(item) => Line::from(vec![
                Span::styled(format!("  {numero:<5}"), acento()),
                Span::styled(
                    item.contratista_nombre.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" — {}", item.empresa_nombre), muted()),
            ]),
            None => Line::from(vec![
                Span::styled(format!("  {numero:<5}"), estilo_error()),
                Span::styled("sin ingreso activo con ese gafete", estilo_error()),
            ]),
        };
        lineas.push(linea);
    }
    lineas
}

pub(super) fn lineas_resumen_salida(activo: &IngresoActivoResumen) -> Vec<Line<'static>> {
    let gafete = activo
        .gafete_numero
        .map(|numero| numero.to_string())
        .unwrap_or_else(|| "S/G".to_string());
    vec![
        Line::from(Span::styled("SALIDA", muted())),
        Line::from(vec![
            Span::styled("Cédula:  ", muted()),
            Span::raw(activo.cedula.clone()),
        ]),
        Line::from(vec![
            Span::styled("Nombre:  ", muted()),
            Span::styled(
                activo.contratista_nombre.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Empresa: ", muted()),
            Span::raw(activo.empresa_nombre.clone()),
        ]),
        Line::from(vec![
            Span::styled("Tipo:    ", muted()),
            Span::raw(tipo_texto(activo.tipo_ingreso)),
        ]),
        Line::from(vec![Span::styled("Gafete:  ", muted()), Span::raw(gafete)]),
        Line::from(""),
        Line::from(format!(
            "Ingresó {} · lleva {} dentro",
            hora_cr(activo.fecha_hora_ingreso),
            duracion_texto(activo.fecha_hora_ingreso)
        )),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para registrar salida · Esc para cancelar",
            acento(),
        )),
    ]
}

pub(super) fn lineas_tabla_activos(
    items: &[IngresoActivoResumen],
    total: usize,
    seleccion: usize,
    ancho: u16,
    columnas: &SelectorColumnas<ColumnaActivos>,
) -> Vec<Line<'static>> {
    // Terminal angosta (Breakpoint::Compact): Empresa se apaga sola además
    // de lo que haya elegido el operador con F4 — mismo umbral que ya tenía
    // esta tabla. Mismas 8 columnas que la tabla de arriba (ColumnaActivos).
    // Navegable con ↑↓ desde DEC-056 — mismo marcador "› " que
    // `lineas_coincidencias_activos`, con el que comparte fuente de datos.
    let angosto = Breakpoint::desde_ancho(ancho) == Breakpoint::Compact;
    let visibles =
        columnas_visibles(columnas).filter(|c| !(angosto && *c == ColumnaActivos::Empresa));
    let anchos = anchos_columnas(ancho, visibles, ancho_fijo_activos, ancho_maximo_activos);

    let mut lineas = vec![
        Line::from(Span::styled(
            format!(
                "  {}",
                fila_columnas(&anchos, derecha_activos, |c| c.etiqueta().to_uppercase())
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("─".repeat(ancho as usize), muted())),
    ];
    for (indice, item) in items.iter().enumerate() {
        let marcador = if indice == seleccion { "› " } else { "  " };
        let texto = format!(
            "{marcador}{}",
            fila_columnas(&anchos, derecha_activos, |c| valor_activos(item, c))
        );
        lineas.push(if indice == seleccion {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else {
            Line::from(texto)
        });
    }
    if items.is_empty() {
        lineas.push(Line::from(Span::styled(
            "Nadie dentro ahora mismo",
            muted(),
        )));
    }
    lineas.push(Line::from(""));
    lineas.push(Line::from(Span::styled(
        format!("{} dentro", cantidad_personas(total)),
        muted(),
    )));
    lineas
}

pub(super) fn lineas_ficha(
    resumen: &crate::database::queries::contratistas::ContratistaResumen,
) -> Vec<Line<'static>> {
    let praind = resumen
        .fecha_vencimiento_praind
        .map(|fecha| format!("vence {}", fecha.format("%d/%m/%Y")))
        .unwrap_or_else(|| "sin fecha registrada".to_string());
    let acceso = if resumen.tiene_acceso {
        chequeo("✓", exito(), "Acceso autorizado".into())
    } else {
        chequeo("✗", estilo_error(), "Sin acceso autorizado".into())
    };
    // Misma idea visual que la tabla de /activos: encabezado en negrita y una
    // fila de datos, en el orden cédula → empresa → tipo → estado.
    let (estado_texto, estado_estilo) = if resumen.tiene_ingreso_activo {
        ("DENTRO", acento())
    } else {
        ("FUERA", muted())
    };
    vec![
        Line::from(Span::styled(
            resumen.nombre.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{:<14}{:<24}{:<12}ESTADO", "CÉDULA", "EMPRESA", "TIPO"),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("─".repeat(56), muted())),
        Line::from(vec![
            Span::raw(format!(
                "{:<14}",
                super::util::recortar(&resumen.cedula, 13)
            )),
            Span::raw(format!(
                "{:<24}",
                super::util::recortar(&resumen.empresa_nombre, 23)
            )),
            Span::raw(format!("{:<12}", tipo_texto(resumen.tipo_ingreso))),
            Span::styled(estado_texto, estado_estilo),
        ]),
        Line::from(""),
        Line::from(format!("PRAIND: {praind}")),
        acceso,
    ]
}

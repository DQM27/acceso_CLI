//! Render adaptativo puro: `&AppState` → frame, sin tocar `AppCore` ni el
//! input. El área contextual cambia de contenido según el [`ContextState`];
//! debajo, una línea de feedback/sugerencias y el prompt siempre visible.
//!
//! Estilo sobrio: sin cajas anidadas ni bordes decorativos. La jerarquía sale
//! del espacio, la alineación y los símbolos ✓ ⚠ ✗ — el color es apoyo, nunca
//! el único canal (una terminal sin color sigue transmitiendo lo mismo).

use chrono::{DateTime, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::tipo_ingreso::TipoIngreso;
use crate::services::registro_ingreso_service::IngresoActivoResumen;
use crate::tiempo::a_costa_rica;

use super::estado::{AppState, ContextState, Fase, NivelFeedback};
use super::parser::Comando;
use super::resolver::MIN_CONSULTA;

/// Mínimos razonables: por debajo de esto no cabe ni la tarjeta más simple —
/// se muestra un aviso en vez de romper el prompt.
const ANCHO_MINIMO: u16 = 40;
const ALTO_MINIMO: u16 = 10;

/// Ancho a partir del cual la tabla de activos muestra también la empresa.
const ANCHO_TABLA_COMPLETA: u16 = 64;

fn exito() -> Style {
    Style::default().fg(Color::Green)
}
fn advertencia() -> Style {
    Style::default().fg(Color::Yellow)
}
fn estilo_error() -> Style {
    Style::default().fg(Color::Red)
}
fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}
fn acento() -> Style {
    Style::default().fg(Color::Cyan)
}
fn estilo_seleccion() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

pub fn render(frame: &mut Frame, app: &AppState) {
    let area = frame.area();
    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        frame.render_widget(
            Paragraph::new(format!(
                "Terminal demasiado pequeña (mínimo {ANCHO_MINIMO}x{ALTO_MINIMO})"
            )),
            area,
        );
        return;
    }

    let paleta = paleta_comandos(app);
    let filas_comandos = paleta.as_ref().map_or(0, |comandos| comandos.len() as u16);
    // Con paleta, el input y la lista viven en un único recuadro (2 bordes +
    // input + divisor + N filas); sin paleta, el recuadro del input solo.
    // El cap deja al menos 3 filas para el área de contexto arriba.
    let cap = area.height.saturating_sub(3);
    let alto_bloque_prompt = match &paleta {
        Some(_) => (4 + filas_comandos).min(cap.max(4)),
        None => 3,
    };
    let alto_pista = if paleta.is_some() { 0 } else { 1 };

    let [area_contexto, area_prompt, area_pista] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(alto_bloque_prompt),
        Constraint::Length(alto_pista),
    ])
    .areas(area);

    let lineas = match &app.fase {
        Fase::Operando { .. } => lineas_contexto(&app.contexto, area_contexto.width),
        fase => lineas_login(fase),
    };
    frame.render_widget(Paragraph::new(lineas), area_contexto);

    render_prompt(frame, area_prompt, app, paleta.as_deref());
    if paleta.is_none() {
        render_pista(frame, area_pista, app);
    }
}

/// Comandos a mostrar en el desplegable bajo el input: sólo mientras se
/// teclea el nombre del comando (`/`, `/in`, …) — antes del primer espacio.
/// En cuanto hay un espacio (ya se eligió comando y se sigue con argumentos)
/// el desplegable desaparece y vuelve la línea de pistas normal.
fn paleta_comandos(app: &AppState) -> Option<Vec<Comando>> {
    if !matches!(app.fase, Fase::Operando { .. }) {
        return None;
    }
    let texto = app.input.value();
    if !texto.starts_with('/') || texto.contains(' ') {
        return None;
    }
    let prefijo = texto[1..].to_lowercase();
    let coincidentes: Vec<Comando> = Comando::TODOS
        .into_iter()
        .filter(|comando| comando.nombre().starts_with(&prefijo))
        .collect();
    (!coincidentes.is_empty()).then_some(coincidentes)
}

/// Línea debajo del recuadro del input (estilo CLI moderna): el feedback
/// transitorio tiene prioridad; sin feedback, las sugerencias del
/// autocompletado contextual, truncadas al ancho disponible.
fn render_pista(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Some(feedback) = app.feedback_vigente() {
        let (simbolo, estilo) = match feedback.nivel {
            NivelFeedback::Exito => ("✓", exito()),
            NivelFeedback::Advertencia => ("⚠", advertencia()),
            NivelFeedback::Error => ("✗", estilo_error()),
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{simbolo} "), estilo),
                Span::styled(&feedback.texto, estilo),
            ])),
            area,
        );
        return;
    }
    if !app.sugerencias.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                app.sugerencias.join("   "),
                muted(),
            ))),
            area,
        );
    }
}

/// El prompt nunca desaparece: vive dentro de un recuadro de línea fina
/// (bordes redondeados) y cambia de etiqueta según la fase (cédula, contraseña
/// enmascarada, o el `>` de comandos), siempre con el cursor visible.
///
/// Con `paleta` en `Some`, el desplegable de comandos se dibuja **dentro del
/// mismo marco** que el input, separado por un divisor real (`├──┤`) en vez
/// de un segundo recuadro pegado al primero — así no quedan dos juegos de
/// esquinas redondeadas encontrándose a mitad de una línea vertical, que es
/// lo que se veía "cortado".
fn render_prompt(frame: &mut Frame, area: Rect, app: &AppState, paleta: Option<&[Comando]>) {
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(muted());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);

    let fila_input = Rect::new(interior.x, interior.y, interior.width, 1);
    render_prompt_linea(frame, fila_input, app);

    let Some(comandos) = paleta else { return };
    if interior.height < 2 {
        return;
    }

    // El divisor se dibuja sobre el ancho completo del recuadro exterior
    // (no del interior) para que "├"/"┤" caigan exactamente sobre las
    // líneas verticales del marco y se empalmen con ellas.
    let fila_divisor = Rect::new(area.x, interior.y + 1, area.width, 1);
    let divisor = format!("├{}┤", "─".repeat((area.width as usize).saturating_sub(2)));
    frame.render_widget(Paragraph::new(divisor).style(muted()), fila_divisor);

    let filas_disponibles = interior.height.saturating_sub(2);
    let fila_comandos = Rect::new(
        interior.x,
        interior.y + 2,
        interior.width,
        filas_disponibles,
    );
    let lineas: Vec<Line> = comandos
        .iter()
        .map(|comando| {
            Line::from(vec![
                Span::styled(format!("/{:<8}", comando.nombre()), acento()),
                Span::styled(descripcion_comando(*comando), muted()),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), fila_comandos);
}

/// Sólo la línea de texto del input (etiqueta + valor + cursor), sin marco —
/// el marco lo pone `render_prompt`, que reutiliza esto tanto con paleta
/// como sin ella.
fn render_prompt_linea(frame: &mut Frame, area: Rect, app: &AppState) {
    let (etiqueta, valor, cursor_visible, cursor_chars) = match &app.fase {
        Fase::LoginCedula { .. } => (
            "cédula › ",
            app.input.value().to_string(),
            true,
            app.input.visual_cursor(),
        ),
        Fase::LoginPassword { .. } => {
            let largo = app.input.value().chars().count();
            ("contraseña › ", "•".repeat(largo), true, largo)
        }
        Fase::Verificando => ("verificando › ", String::new(), false, 0),
        Fase::Operando { .. } => (
            "> ",
            app.input.value().to_string(),
            true,
            app.input.visual_cursor(),
        ),
    };

    let ancho_etiqueta = etiqueta.chars().count() as u16;
    let viewport = area.width.saturating_sub(ancho_etiqueta + 1) as usize;
    let scroll = if cursor_visible {
        app.input.visual_scroll(viewport)
    } else {
        0
    };
    let visible: String = valor.chars().skip(scroll).take(viewport).collect();

    let linea = Line::from(vec![Span::styled(etiqueta, acento()), Span::raw(visible)]);
    frame.render_widget(Paragraph::new(linea), area);

    if cursor_visible {
        let columna = cursor_chars.saturating_sub(scroll).min(viewport) as u16;
        frame.set_cursor_position((area.x + ancho_etiqueta + columna, area.y));
    }
}

// ── Login ────────────────────────────────────────────────────────────────

fn lineas_login(fase: &Fase) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled(
            "BRISAS CLI",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    match fase {
        Fase::LoginCedula { error } => {
            lineas.push(Line::from(
                "Escriba su cédula y presione Enter para iniciar sesión.",
            ));
            if let Some(mensaje) = error {
                lineas.push(Line::from(""));
                lineas.push(Line::from(Span::styled(
                    format!("✗ {mensaje}"),
                    estilo_error(),
                )));
            }
        }
        Fase::LoginPassword { cedula, error } => {
            lineas.push(Line::from(format!("Cédula: {cedula}")));
            lineas.push(Line::from(
                "Escriba su contraseña y presione Enter. Esc vuelve a la cédula.",
            ));
            if let Some(mensaje) = error {
                lineas.push(Line::from(""));
                lineas.push(Line::from(Span::styled(
                    format!("✗ {mensaje}"),
                    estilo_error(),
                )));
            }
        }
        Fase::Verificando => {
            lineas.push(Line::from("Verificando…"));
        }
        Fase::Operando { .. } => {}
    }
    lineas
}

// ── Contexto operativo ───────────────────────────────────────────────────

fn lineas_contexto(contexto: &ContextState, ancho: u16) -> Vec<Line<'static>> {
    match contexto {
        ContextState::Inicio { total_dentro } => lineas_inicio(*total_dentro),
        ContextState::Coincidencias {
            consulta,
            items,
            seleccion,
        } => lineas_coincidencias(consulta, items, *seleccion, ancho),
        ContextState::CoincidenciasActivos {
            descripcion,
            items,
            seleccion,
        } => lineas_coincidencias_activos(descripcion, items, *seleccion, ancho),
        ContextState::ResumenIngreso { .. } => lineas_resumen_ingreso(contexto),
        ContextState::ResumenSalida { activo } => lineas_resumen_salida(activo),
        ContextState::TablaActivos { items, total } => lineas_tabla_activos(items, *total, ancho),
        ContextState::FichaContratista { resumen } => lineas_ficha(resumen),
        ContextState::ConfirmarCerrarSesion => lineas_cerrar_sesion(),
        ContextState::Ayuda => lineas_ayuda(),
        ContextState::MensajeError { mensaje } => vec![
            Line::from(""),
            Line::from(Span::styled(format!("✗ {mensaje}"), estilo_error())),
        ],
    }
}

fn lineas_inicio(total_dentro: usize) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "BRISAS CLI",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "{} actualmente dentro",
            cantidad_personas(total_dentro)
        )),
    ]
}

fn cantidad_personas(total: usize) -> String {
    if total == 1 {
        "1 persona".to_string()
    } else {
        format!("{total} personas")
    }
}

fn descripcion_comando(comando: Comando) -> &'static str {
    match comando {
        Comando::Ingreso => "registrar ingreso — /ingreso <nombre> G:<n> M:<medio>",
        Comando::Salida => "registrar salida — /salida <nombre> o /salida G:<n>",
        Comando::Activos => "quién está dentro ahora",
        Comando::Ayuda => "sintaxis completa y ejemplos",
        Comando::CerrarSesion => "cerrar sesión y volver al login",
    }
}

/// Reparte el espacio libre entre NOMBRE y EMPRESA una vez descontadas las
/// columnas de ancho fijo de cada tabla (marcador, cédula/gafete, tipo/hora)
/// — así el nombre deja de truncarse apenas la terminal tiene espacio, igual
/// que ya hacía `/activos` con su propio ancho completo/reducido.
fn anchos_nombre_empresa(ancho: u16, fijo: u16) -> (usize, usize) {
    let disponible = ancho.saturating_sub(fijo).max(30) as usize;
    let nombre = (disponible * 55 / 100).max(18);
    let empresa = disponible.saturating_sub(nombre).max(14);
    (nombre, empresa)
}

fn lineas_coincidencias(
    consulta: &str,
    items: &[crate::database::queries::contratistas::ContratistaResumen],
    seleccion: usize,
    ancho: u16,
) -> Vec<Line<'static>> {
    if consulta.chars().count() < MIN_CONSULTA {
        return vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Escriba al menos {MIN_CONSULTA} letras para buscar…"),
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
    // Mismo orden de columnas que la tabla de contratistas de la TUI clásica
    // (cédula → nombre → empresa → tipo) y mismo estilo de encabezado que
    // `/activos`, para que todas las listas se lean igual.
    const CEDULA: usize = 14;
    let (nombre_ancho, empresa_ancho) = anchos_nombre_empresa(ancho, 2 + CEDULA as u16 + 10);
    let mut lineas = vec![
        Line::from(Span::styled(
            format!(
                "  {:<CEDULA$}{:<nombre_ancho$}{:<empresa_ancho$}TIPO",
                "CÉDULA", "NOMBRE", "EMPRESA"
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("─".repeat(ancho as usize), muted())),
    ];
    for (indice, item) in items.iter().enumerate() {
        let marcador = if indice == seleccion { "▸ " } else { "  " };
        let texto = format!(
            "{marcador}{:<CEDULA$}{:<nombre_ancho$}{:<empresa_ancho$}{}",
            recortar(&item.cedula, CEDULA - 1),
            recortar(&item.nombre, nombre_ancho.saturating_sub(1)),
            recortar(&item.empresa_nombre, empresa_ancho.saturating_sub(1)),
            tipo_texto(item.tipo_ingreso)
        );
        lineas.push(if indice == seleccion {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else {
            Line::from(texto)
        });
    }
    lineas
}

fn lineas_coincidencias_activos(
    descripcion: &str,
    items: &[IngresoActivoResumen],
    seleccion: usize,
    ancho: u16,
) -> Vec<Line<'static>> {
    if items.is_empty() {
        let mensaje = if descripcion.is_empty() {
            "Escriba un nombre o G:<número> del gafete…".to_string()
        } else {
            format!("No hay ingreso activo para {descripcion}")
        };
        return vec![Line::from(""), Line::from(Span::styled(mensaje, muted()))];
    }
    // Mismo encabezado y orden de columnas que `/activos` — es la misma
    // fuente de datos (ingresos activos), sólo que filtrada por la búsqueda.
    const GAFETE: usize = 8;
    let (nombre_ancho, empresa_ancho) = anchos_nombre_empresa(ancho, 2 + GAFETE as u16 + 6);
    let mut lineas = vec![
        Line::from(Span::styled(
            format!(
                "  {:<GAFETE$}{:<nombre_ancho$}{:<empresa_ancho$}INGRESO",
                "GAFETE", "NOMBRE", "EMPRESA"
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("─".repeat(ancho as usize), muted())),
    ];
    for (indice, item) in items.iter().enumerate() {
        let marcador = if indice == seleccion { "▸ " } else { "  " };
        let gafete = item
            .gafete_numero
            .map(|numero| numero.to_string())
            .unwrap_or_else(|| "—".into());
        let texto = format!(
            "{marcador}{:<GAFETE$}{:<nombre_ancho$}{:<empresa_ancho$}{}",
            recortar(&gafete, GAFETE - 1),
            recortar(&item.contratista_nombre, nombre_ancho.saturating_sub(1)),
            recortar(&item.empresa_nombre, empresa_ancho.saturating_sub(1)),
            hora_cr(item.fecha_hora_ingreso)
        );
        lineas.push(if indice == seleccion {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else {
            Line::from(texto)
        });
    }
    lineas
}

/// Tarjeta de validación previa al ingreso: un símbolo por chequeo, y al pie
/// la acción disponible (registrar sólo si todo está en ✓/⚠).
fn lineas_resumen_ingreso(contexto: &ContextState) -> Vec<Line<'static>> {
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
            advertencia(),
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

fn lineas_resumen_salida(activo: &IngresoActivoResumen) -> Vec<Line<'static>> {
    let gafete = activo
        .gafete_numero
        .map(|numero| numero.to_string())
        .unwrap_or_else(|| "Sin gafete".to_string());
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
        Line::from(vec![
            Span::styled("Gafete:  ", muted()),
            Span::raw(gafete),
        ]),
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

fn lineas_tabla_activos(
    items: &[IngresoActivoResumen],
    total: usize,
    ancho: u16,
) -> Vec<Line<'static>> {
    let completa = ancho >= ANCHO_TABLA_COMPLETA;
    let encabezado = if completa {
        format!(
            "{:<8}{:<30}{:<24}{}",
            "GAFETE", "NOMBRE", "EMPRESA", "INGRESO"
        )
    } else {
        format!("{:<8}{:<24}{}", "GAFETE", "NOMBRE", "INGRESO")
    };
    let mut lineas = vec![
        Line::from(Span::styled(
            encabezado,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("─".repeat(ancho as usize), muted())),
    ];
    for item in items {
        let gafete = item
            .gafete_numero
            .map(|numero| numero.to_string())
            .unwrap_or_else(|| "—".into());
        let hora = hora_cr(item.fecha_hora_ingreso);
        lineas.push(Line::from(if completa {
            format!(
                "{:<8}{:<30}{:<24}{}",
                recortar(&gafete, 8),
                recortar(&item.contratista_nombre, 29),
                recortar(&item.empresa_nombre, 23),
                hora
            )
        } else {
            format!(
                "{:<8}{:<24}{}",
                recortar(&gafete, 8),
                recortar(&item.contratista_nombre, 23),
                hora
            )
        }));
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

fn lineas_ficha(
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
            Span::raw(format!("{:<14}", recortar(&resumen.cedula, 13))),
            Span::raw(format!("{:<24}", recortar(&resumen.empresa_nombre, 23))),
            Span::raw(format!("{:<12}", tipo_texto(resumen.tipo_ingreso))),
            Span::styled(estado_texto, estado_estilo),
        ]),
        Line::from(""),
        Line::from(format!("PRAIND: {praind}")),
        acceso,
    ]
}

fn lineas_cerrar_sesion() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("CERRAR SESIÓN", muted())),
        Line::from(""),
        Line::from("La sesión actual se cerrará y volverá a la pantalla de autenticación."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para cerrar sesión · Esc para cancelar",
            acento(),
        )),
    ]
}

fn lineas_ayuda() -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled(
            "AYUDA — sintaxis de comandos",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    let ejemplos: [(&str, &str); 8] = [
        ("/ingreso <nombre> G:<n> M:<medio>", "registrar un ingreso"),
        ("/ingreso 119430546 G:12", "también por cédula"),
        ("/salida <nombre>", "registrar salida por nombre"),
        ("/salida G:27", "registrar salida por gafete"),
        ("/activos", "tabla de personas dentro"),
        ("/cerrarsesion", "cerrar sesión y volver al login"),
        ("/ayuda", "esta ayuda"),
        ("texto sin /", "búsqueda de contratistas"),
    ];
    for (sintaxis, descripcion) in ejemplos {
        lineas.push(Line::from(vec![
            Span::styled(format!("{sintaxis:<36}"), acento()),
            Span::styled(descripcion, muted()),
        ]));
    }
    lineas.push(Line::from(""));
    lineas.push(Line::from(Span::styled(
        "Claves: G: gafete · M: caminando|vehiculo (por defecto caminando) · alias: /i /s /a /cs",
        muted(),
    )));
    lineas.push(Line::from(Span::styled(
        "Tab completa comandos, gafetes libres y medios · Esc limpia · Ctrl+C sale",
        muted(),
    )));
    lineas
}

// ── Utilidades de texto ──────────────────────────────────────────────────

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

pub fn tipo_texto(tipo: TipoIngreso) -> &'static str {
    match tipo {
        TipoIngreso::Praind => "PRAIND",
        TipoIngreso::InHouse => "IN HOUSE",
        TipoIngreso::PorCorreo => "POR CORREO",
        TipoIngreso::Swat => "SWAT",
    }
}

pub fn medio_texto(medio: MedioIngreso) -> &'static str {
    match medio {
        MedioIngreso::Caminando => "CAMINANDO",
        MedioIngreso::Vehiculo => "VEHICULO",
    }
}

pub fn hora_cr(instante: DateTime<Utc>) -> String {
    a_costa_rica(instante).format("%H:%M").to_string()
}

/// "2 h 15 min" / "45 min" — duración desde `desde` hasta ahora. Un instante
/// futuro (reloj inconsistente) se reporta como "0 min", nunca negativo.
pub fn duracion_texto(desde: DateTime<Utc>) -> String {
    let minutos = (Utc::now() - desde).num_minutes().max(0);
    let horas = minutos / 60;
    if horas > 0 {
        format!("{horas} h {:02} min", minutos % 60)
    } else {
        format!("{minutos} min")
    }
}

/// Trunca a `ancho` columnas añadiendo "…" cuando recorta — por caracteres,
/// no por bytes, para no romper UTF-8.
fn recortar(texto: &str, ancho: usize) -> String {
    if texto.chars().count() <= ancho {
        return texto.to_string();
    }
    let mut recortado: String = texto.chars().take(ancho.saturating_sub(1)).collect();
    recortado.push('…');
    recortado
}

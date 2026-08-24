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
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use tui_big_text::{BigText, PixelSize};

use crate::domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::tipo_ingreso::TipoIngreso;
use crate::services::registro_ingreso_service::IngresoActivoResumen;
use crate::tiempo::a_costa_rica;

use super::columnas::{
    Columna, ColumnaActivos, ColumnaBusqueda, ColumnaHistorial, SelectorColumnas,
};
use super::estado::{
    AppState, ContextState, EdicionColumnas, Fase, NivelFeedback, ObjetivoColumnas,
};
use super::formulario::{
    Campo, FormularioContratista, MAX_VISIBLES_EMPRESAS, ModoFormulario, Subfase,
};
use super::historial::HistorialState;
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

/// Gramática visual compartida por toda la app (ver
/// `docs/lenguaje-visual-mutaciones.md`): el glifo nunca depende del color
/// para transmitir significado — el color sólo refuerza.
///
/// ```text
/// ●  procesando / sistema activo
/// ›  esperando entrada / foco
/// ✓  completado
/// !  advertencia
/// ×  falló / error / rechazo
/// ```
fn glifo_feedback(nivel: NivelFeedback) -> (&'static str, Style) {
    match nivel {
        NivelFeedback::Exito => ("✓", exito()),
        NivelFeedback::Advertencia => ("!", advertencia()),
        NivelFeedback::Error => ("×", estilo_error()),
    }
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

    // El login vive en una composición propia, sin cajas ni el prompt de
    // línea de comandos — no comparte layout con la interfaz operativa.
    if !matches!(app.fase, Fase::Operando { .. }) {
        render_login(frame, area, app);
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

    let lineas = if let Some(formulario) = &app.formulario {
        lineas_formulario(formulario, app.input.value())
    } else if let Some(edicion) = &app.edicion_columnas {
        lineas_selector_columnas(app, *edicion)
    } else if let Some(historial) = &app.historial {
        lineas_historial(
            historial,
            app.input.value(),
            area_contexto.width,
            &app.columnas_historial,
        )
    } else {
        lineas_contexto(
            &app.contexto,
            area_contexto.width,
            &app.columnas_busqueda,
            &app.columnas_activos,
        )
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
    if !matches!(app.fase, Fase::Operando { .. }) || app.formulario.is_some() {
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
        let (simbolo, estilo) = glifo_feedback(feedback.nivel);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{simbolo} "), estilo),
                Span::styled(&feedback.texto, estilo),
            ])),
            area,
        );
        return;
    }
    if app.edicion_columnas.is_some() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "↑↓ columna · Space marcar/desmarcar · Esc cerrar",
                muted(),
            ))),
            area,
        );
        return;
    }
    if let Some(historial) = &app.historial {
        let pista = if historial.resultado.is_some() {
            "↑↓ moverse · PageUp/PageDown más resultados · F4 columnas · Esc editar filtro"
        } else {
            "escriba clave:valor · Enter aplica · Esc cierra Historial"
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(pista, muted()))),
            area,
        );
        return;
    }
    // Con el formulario abierto, la pista describe las teclas de la sub-fase
    // (las sugerencias del autocompletado no aplican: el input edita campos).
    if let Some(formulario) = &app.formulario {
        let pista = match formulario.subfase {
            Subfase::Editando => {
                "↑↓ campo · Space/←/→ cambiar valor · Enter guardar · Esc cancelar"
            }
            Subfase::EligiendoEmpresa { .. } => {
                "escriba para filtrar · ↑↓ elegir · Enter aceptar · Esc volver"
            }
            Subfase::Resumen => "Enter guardar · Esc volver a editar",
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(pista, muted()))),
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
/// como sin ella. Sólo se llama en fase `Operando`: el login tiene su propia
/// composición sin caja (`render_login`), nunca pasa por acá.
fn render_prompt_linea(frame: &mut Frame, area: Rect, app: &AppState) {
    let (etiqueta, valor, cursor_visible, cursor_chars) = if let Some(historial) = &app.historial {
        // Editando: el input escribe el filtro clave:valor. Mostrando
        // resultado (Enter ya aplicó, DEC-024): el texto queda congelado
        // hasta Esc, sin cursor — no se edita mientras se navega.
        let editable = historial.resultado.is_none();
        (
            "historial › ".to_string(),
            app.input.value().to_string(),
            editable,
            if editable {
                app.input.visual_cursor()
            } else {
                0
            },
        )
    } else {
        match &app.formulario {
            // Con el formulario abierto el input edita el campo activo (o
            // filtra empresas en el selector): la etiqueta lo anuncia y el
            // cursor sólo aparece cuando hay algo que teclear.
            Some(formulario) => {
                let etiqueta = match formulario.subfase {
                    Subfase::EligiendoEmpresa { .. } => "empresa › ".to_string(),
                    Subfase::Resumen => "confirmar › ".to_string(),
                    Subfase::Editando => match formulario.campo {
                        Campo::Cedula => "cédula › ".to_string(),
                        Campo::Nombre => "nombre › ".to_string(),
                        Campo::FechaPraind => "fecha praind › ".to_string(),
                        campo => format!("{} › ", campo.etiqueta().to_lowercase()),
                    },
                };
                let editable = matches!(formulario.subfase, Subfase::EligiendoEmpresa { .. })
                    || formulario.campo.es_texto();
                (
                    etiqueta,
                    app.input.value().to_string(),
                    editable,
                    if editable {
                        app.input.visual_cursor()
                    } else {
                        0
                    },
                )
            }
            None => (
                "> ".to_string(),
                app.input.value().to_string(),
                true,
                app.input.visual_cursor(),
            ),
        }
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
//
// Escena propia, sin cajas ni bordes: identidad, foco y aviso apoyados sólo
// en espaciado, alineación y la gramática de glifos (● › ✓ ! ×). El cursor
// es un "_" con estilo, nunca el bloque parpadeante del terminal — por eso
// esta función jamás llama a `frame.set_cursor_position`.

/// Alto en filas del título grande (`PixelSize::Quadrant`: 4 filas, 4
/// columnas por carácter — suficiente para distinguirse sin ocupar media
/// pantalla ni depender de glifos de bloque más finos, que no todos los
/// terminales dibujan igual).
const ALTO_TITULO_GRANDE: u16 = 4;

/// Paleta propia del login en RGB explícito (no los `Color` con nombre del
/// resto del archivo): un fundido necesita interpolar componentes, y sólo
/// `Color::Rgb` los tiene. Fondo asumido oscuro — es la base de todo el tema
/// actual (ver `muted()`/`acento()`), no una novedad de esta escena.
const FADE_FONDO: (u8, u8, u8) = (10, 10, 12);
const FADE_ACENTO: (u8, u8, u8) = (86, 200, 214);
const FADE_MUTED: (u8, u8, u8) = (120, 120, 130);
const FADE_TEXTO: (u8, u8, u8) = (225, 225, 230);
const FADE_EXITO: (u8, u8, u8) = (94, 201, 133);
const FADE_ADVERTENCIA: (u8, u8, u8) = (214, 181, 92);
const FADE_ERROR: (u8, u8, u8) = (214, 92, 92);

fn interpolar_color(desde: (u8, u8, u8), hasta: (u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let mezclar = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color::Rgb(
        mezclar(desde.0, hasta.0),
        mezclar(desde.1, hasta.1),
        mezclar(desde.2, hasta.2),
    )
}

/// Estilo que funde desde `FADE_FONDO` hacia `color` según `opacidad`
/// (0.0 = invisible, fundido con el fondo; 1.0 = color final). Con
/// `opacidad` en 1.0 (elemento ya resuelto, o `VisualQuality::Off`) el color
/// resultante coincide exactamente con el color final, sin diferencia visual.
fn estilo_fundido(color: (u8, u8, u8), opacidad: f32, modificador: Modifier) -> Style {
    Style::default()
        .fg(interpolar_color(FADE_FONDO, color, opacidad))
        .add_modifier(modificador)
}

fn render_login(frame: &mut Frame, area: Rect, app: &AppState) {
    let opacidad_titulo = app.presentacion.opacidad("titulo");
    let opacidad_prompt = app.presentacion.opacidad("prompt");
    let opacidad_aviso = app.presentacion.opacidad("feedback");

    // Sólo el nombre de la app ("Brisas CLI") usa el tratamiento grande: es
    // la marca. La identidad del operador, que ocupa la misma ranura visual
    // después, es deliberadamente más chica y de otro color — es la sesión
    // de una persona, no compite en jerarquía con la marca.
    let titulo_grande = matches!(app.fase, Fase::LoginCedula);
    let titulo_alto = if titulo_grande { ALTO_TITULO_GRANDE } else { 1 };
    // Aire entre bloques: dos filas, no una — con una sola el conjunto se
    // veía apretado y perdido en el resto de la pantalla vacía.
    const AIRE: u16 = 2;
    // título + aire + prompt + aire + aviso.
    let alto_total = (titulo_alto + 2 * AIRE + 2).min(area.height);
    // El bloque arranca un poco antes de la mitad de la pantalla — arriba
    // del centro, no exactamente centrado ni pegado al borde — dejando aire
    // debajo para que la escena respire.
    let y = area.y + area.height.saturating_sub(alto_total) / 3;

    let area_titulo = Rect::new(area.x, y, area.width, titulo_alto.min(area.height));
    if titulo_grande {
        let titulo = BigText::builder()
            .pixel_size(PixelSize::Quadrant)
            .centered()
            .style(estilo_fundido(FADE_ACENTO, opacidad_titulo, Modifier::BOLD))
            .lines(vec![Line::from(super::NOMBRE_APP.to_uppercase())])
            .build();
        frame.render_widget(titulo, area_titulo);
    } else {
        frame.render_widget(
            Paragraph::new(linea_identidad_operador(&app.fase, opacidad_titulo))
                .alignment(Alignment::Center),
            area_titulo,
        );
    }

    let y_prompt = y + titulo_alto + AIRE;
    if alto_total > titulo_alto + AIRE {
        match etiqueta_prompt(&app.fase) {
            // Espaciado entre letras, igual que el nombre del operador: sin
            // eso el prompt se leía diminuto al lado del título/identidad, y
            // el aviso transitorio (una oración normal, sin espaciar) pesaba
            // visualmente más que el propio input — justo al revés de la
            // jerarquía que debería tener.
            //
            // El punto de anclaje horizontal se calcula centrando la
            // ETIQUETA ya espaciada (largo fijo), nunca el valor tecleado:
            // así coincide con el centro del título (mismo cálculo, mismo
            // `area.width`) y no se recalcula tecla a tecla — el `›` se
            // queda quieto y el texto crece hacia la derecha.
            Some(etiqueta) => {
                let etiqueta_espaciada = espaciar_texto(etiqueta);
                let vacio = app.input.value().is_empty();
                let valor_espaciado = espaciar_texto(&valor_prompt(&app.fase, app));
                let linea = linea_prompt(
                    &etiqueta_espaciada,
                    &valor_espaciado,
                    vacio,
                    opacidad_prompt,
                );
                let ancho_centrado =
                    (2 + etiqueta_espaciada.chars().count() as u16).min(area.width);
                let x_prompt = area.x + area.width.saturating_sub(ancho_centrado) / 2;
                let ancho_render = area.width.saturating_sub(x_prompt - area.x);
                frame.render_widget(
                    Paragraph::new(linea),
                    Rect::new(x_prompt, y_prompt, ancho_render, 1),
                );
            }
            // Verificando: no crece con tecleo, se centra como el título.
            None => {
                frame.render_widget(
                    Paragraph::new(linea_verificando(opacidad_prompt)).alignment(Alignment::Center),
                    Rect::new(area.x, y_prompt, area.width, 1),
                );
            }
        }
    }

    let y_aviso = y_prompt + AIRE;
    if alto_total > titulo_alto + AIRE + 1 {
        frame.render_widget(
            Paragraph::new(linea_aviso_login(app, opacidad_aviso)).alignment(Alignment::Center),
            Rect::new(area.x, y_aviso, area.width, 1),
        );
    }
}

/// El nombre del operador ocupa la misma ranura visual que "Brisas CLI",
/// pero con menos peso a propósito: texto normal (no bloques grandes) y un
/// color neutro en vez del acento de la marca.
fn linea_identidad_operador(fase: &Fase, opacidad: f32) -> Line<'static> {
    let nombre = match fase {
        Fase::LoginPassword { nombre, .. } | Fase::Verificando { nombre } => nombre.as_str(),
        Fase::LoginCedula | Fase::Operando { .. } => "",
    };
    // Mayúscula + espaciado entre letras le dan más presencia sin usar el
    // mismo tratamiento de bloques que la marca — sigue siendo, a
    // propósito, un peso visual menor que el título grande.
    Line::from(Span::styled(
        espaciar_texto(&nombre.to_uppercase()),
        estilo_fundido(FADE_TEXTO, opacidad, Modifier::BOLD),
    ))
}

/// Inserta un espacio entre cada carácter (y espacio triple donde ya había
/// uno) para que el texto ocupe más ancho visual sin cambiar de tamaño de
/// fuente — la terminal no tiene tamaños de fuente, sólo columnas.
fn espaciar_texto(texto: &str) -> String {
    let mut espaciado = String::new();
    for caracter in texto.chars() {
        if caracter == ' ' {
            espaciado.push_str("   ");
        } else {
            espaciado.push(caracter);
            espaciado.push(' ');
        }
    }
    espaciado.trim_end().to_string()
}

fn linea_verificando(opacidad: f32) -> Line<'static> {
    // Trabajo real (Argon2 en un hilo aparte), no una animación decorativa:
    // el glifo ● es el mismo que en el resto de la app para "sistema activo".
    Line::from(Span::styled(
        "● Verificando",
        estilo_fundido(FADE_MUTED, opacidad, Modifier::empty()),
    ))
}

/// Etiqueta de marcador de posición del prompt según la fase — largo fijo,
/// usado tanto para el texto mostrado cuando no se ha tecleado nada como
/// para calcular el punto de anclaje horizontal. `None` en `Verificando`:
/// ahí no hay nada que escribir.
fn etiqueta_prompt(fase: &Fase) -> Option<&'static str> {
    match fase {
        Fase::LoginCedula => Some("Identificación"),
        Fase::LoginPassword { .. } => Some("Contraseña"),
        Fase::Verificando { .. } | Fase::Operando { .. } => None,
    }
}

fn valor_prompt(fase: &Fase, app: &AppState) -> String {
    match fase {
        Fase::LoginCedula => app.input.value().to_string(),
        Fase::LoginPassword { .. } => "•".repeat(app.input.value().chars().count()),
        Fase::Verificando { .. } | Fase::Operando { .. } => String::new(),
    }
}

/// `vacio`: sin nada tecleado se muestra la etiqueta como pista (el foco `›`
/// ya está puesto, no hace falta cursor todavía); en cuanto hay texto, la
/// etiqueta se retira y el valor ocupa su lugar con el cursor `_` al final —
/// la etiqueta se simplifica en el propio valor, no coexisten. La aparición
/// (fade-in) es por transición de fase, nunca por tecla: escribir no anima.
fn linea_prompt(etiqueta: &str, valor_mostrado: &str, vacio: bool, opacidad: f32) -> Line<'static> {
    if vacio {
        Line::from(vec![
            Span::styled(
                "› ",
                estilo_fundido(FADE_ACENTO, opacidad, Modifier::empty()),
            ),
            Span::styled(
                etiqueta.to_string(),
                estilo_fundido(FADE_MUTED, opacidad, Modifier::empty()),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "› ",
                estilo_fundido(FADE_ACENTO, opacidad, Modifier::empty()),
            ),
            Span::styled(
                valor_mostrado.to_string(),
                estilo_fundido(FADE_TEXTO, opacidad, Modifier::empty()),
            ),
            Span::styled(
                "_",
                estilo_fundido(FADE_ACENTO, opacidad, Modifier::empty()),
            ),
        ])
    }
}

/// Símbolo de la gramática compartida (`✓ ! ×`) con color propio en RGB —
/// distinto de `glifo_feedback` (que usa `Color` con nombre) porque acá hace
/// falta poder fundirlo con `estilo_fundido`.
fn color_nivel_login(nivel: NivelFeedback) -> (u8, u8, u8) {
    match nivel {
        NivelFeedback::Exito => FADE_EXITO,
        NivelFeedback::Advertencia => FADE_ADVERTENCIA,
        NivelFeedback::Error => FADE_ERROR,
    }
}

fn linea_aviso_login(app: &AppState, opacidad: f32) -> Line<'static> {
    match app.feedback_vigente() {
        Some(feedback) => {
            let (simbolo, _) = glifo_feedback(feedback.nivel);
            let estilo = estilo_fundido(
                color_nivel_login(feedback.nivel),
                opacidad,
                Modifier::empty(),
            );
            Line::from(vec![
                Span::styled(format!("{simbolo} "), estilo),
                Span::styled(feedback.texto.clone(), estilo),
            ])
        }
        None => Line::from(""),
    }
}

// ── Contexto operativo ───────────────────────────────────────────────────

fn lineas_contexto(
    contexto: &ContextState,
    ancho: u16,
    columnas_busqueda: &SelectorColumnas<ColumnaBusqueda>,
    columnas_activos: &SelectorColumnas<ColumnaActivos>,
) -> Vec<Line<'static>> {
    match contexto {
        ContextState::Inicio { total_dentro } => lineas_inicio(*total_dentro),
        ContextState::Coincidencias {
            consulta,
            items,
            seleccion,
        } => lineas_coincidencias(consulta, items, *seleccion, ancho, columnas_busqueda),
        ContextState::CoincidenciasActivos {
            descripcion,
            items,
            seleccion,
        } => lineas_coincidencias_activos(descripcion, items, *seleccion, ancho, columnas_activos),
        ContextState::ResumenIngreso { .. } => lineas_resumen_ingreso(contexto),
        ContextState::ResumenSalida { activo } => lineas_resumen_salida(activo),
        ContextState::TablaActivos { items, total } => {
            lineas_tabla_activos(items, *total, ancho, columnas_activos)
        }
        ContextState::FichaContratista { resumen } => lineas_ficha(resumen),
        ContextState::ConfirmarCerrarSesion => lineas_cerrar_sesion(),
        ContextState::NuevoContratista => lineas_nuevo_contratista(),
        ContextState::AbrirHistorial => lineas_abrir_historial(),
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
        Comando::Nuevo => "dar de alta un contratista",
        Comando::Editar => "editar un contratista — /editar <nombre>",
        Comando::Historial => "explorar movimientos — filtro empresa/tipo/fecha…",
        Comando::Ayuda => "sintaxis completa y ejemplos",
        Comando::CerrarSesion => "cerrar sesión y volver al login",
    }
}

fn columnas_visibles<C: Columna>(columnas: &SelectorColumnas<C>) -> impl Iterator<Item = C> + '_ {
    columnas.iter().filter(|(_, v)| *v).map(|(c, _)| c)
}

/// Tope al ancho de una columna flexible (Nombre/Empresa/Usuario): sin este
/// tope, en una terminal ancha se comen todo el espacio sobrante y dejan un
/// hueco enorme antes de las columnas fijas — que es lo que se veía "feo".
/// Un nombre de persona o empresa rara vez necesita más que esto; el que se
/// pase igual se trunca con "…", no se pierde información silenciosamente.
const ANCHO_FLEXIBLE_MAXIMO: usize = 28;

/// Ancho de cada columna visible: la fija (`Some`) se conserva tal cual, la
/// flexible (`None`, típicamente Nombre/Empresa) se reparte en partes
/// iguales el espacio que sobra, con tope (`ANCHO_FLEXIBLE_MAXIMO`) — así
/// una tabla con 3 columnas visibles aprovecha el ancho que dejó libre la 4ª
/// que se ocultó con F4, sin desbordarse en una terminal ancha.
fn anchos_columnas<C: Columna>(
    ancho_total: u16,
    visibles: impl Iterator<Item = C>,
    ancho_fijo: impl Fn(C) -> Option<usize>,
) -> Vec<(C, usize)> {
    let visibles: Vec<C> = visibles.collect();
    let fijo_total: u16 = visibles
        .iter()
        .filter_map(|c| ancho_fijo(*c))
        .map(|a| a as u16)
        .sum();
    let n_flex = visibles
        .iter()
        .filter(|c| ancho_fijo(**c).is_none())
        .count();
    let flex_ancho = if n_flex == 0 {
        0
    } else {
        let disponible = ancho_total
            .saturating_sub(fijo_total + 2)
            .max(12 * n_flex as u16);
        (disponible / n_flex as u16)
            .max(12)
            .min(ANCHO_FLEXIBLE_MAXIMO as u16) as usize
    };
    visibles
        .into_iter()
        .map(|c| (c, ancho_fijo(c).unwrap_or(flex_ancho)))
        .collect()
}

/// Concatena celdas ya resueltas a `(ancho, columna)`, salvo la última (que
/// crece sin relleno hasta el borde) — como sólo se listan las visibles,
/// "la última" cambia sola según cuál quede más a la derecha. Cada columna
/// reserva 2 espacios de separación (no 1): con columnas numéricas
/// consecutivas (p. ej. Hora/Gafete en `/activos`) un solo espacio las hacía
/// leerse como un número pegado. `derecha` alinea a la derecha las columnas
/// numéricas (Gafete) — separa visualmente sus dígitos de los de la columna
/// anterior en vez de quedar pegados contra el borde izquierdo.
fn fila_columnas<C: Columna>(
    anchos: &[(C, usize)],
    derecha: impl Fn(C) -> bool,
    valor: impl Fn(C) -> String,
) -> String {
    let ultimo = anchos.len().saturating_sub(1);
    anchos
        .iter()
        .enumerate()
        .map(|(indice, (columna, ancho))| {
            let texto = valor(*columna);
            if indice == ultimo {
                return texto;
            }
            let contenido = ancho.saturating_sub(2);
            let recortado = recortar(&texto, contenido);
            if derecha(*columna) {
                format!("{recortado:>contenido$}  ")
            } else {
                format!("{recortado:<contenido$}  ")
            }
        })
        .collect()
}

fn ancho_fijo_busqueda(columna: ColumnaBusqueda) -> Option<usize> {
    match columna {
        ColumnaBusqueda::Cedula => Some(14),
        ColumnaBusqueda::Tipo => Some(12),
        ColumnaBusqueda::Praind => Some(12),
        ColumnaBusqueda::Ruta => Some(6),
        ColumnaBusqueda::Acceso => Some(8),
        ColumnaBusqueda::Nombre | ColumnaBusqueda::Empresa => None,
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
    }
}

fn lineas_coincidencias(
    consulta: &str,
    items: &[crate::database::queries::contratistas::ContratistaResumen],
    seleccion: usize,
    ancho: u16,
    columnas: &SelectorColumnas<ColumnaBusqueda>,
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
    // Mismas 7 columnas que la tabla de contratistas de la TUI clásica
    // (cédula/nombre/empresa/tipo/praind/ruta/acceso) — sólo se listan las
    // que estén visibles (F4, ColumnaBusqueda).
    let anchos = anchos_columnas(ancho, columnas_visibles(columnas), ancho_fijo_busqueda);
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
    for (indice, item) in items.iter().enumerate() {
        let marcador = if indice == seleccion { "› " } else { "  " };
        let texto = format!(
            "{marcador}{}",
            fila_columnas(&anchos, |_| false, |c| valor_busqueda(item, c))
        );
        lineas.push(if indice == seleccion {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else {
            Line::from(texto)
        });
    }
    lineas
}

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
            .unwrap_or_else(|| "—".to_string()),
        ColumnaActivos::Medio => medio_texto(item.medio_ingreso).to_string(),
        ColumnaActivos::Usuario => item.usuario_ingreso_nombre.clone(),
    }
}

fn lineas_coincidencias_activos(
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
    let anchos = anchos_columnas(ancho, columnas_visibles(columnas), ancho_fijo_activos);
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

fn lineas_tabla_activos(
    items: &[IngresoActivoResumen],
    total: usize,
    ancho: u16,
    columnas: &SelectorColumnas<ColumnaActivos>,
) -> Vec<Line<'static>> {
    // Terminal angosta: Empresa se apaga sola además de lo que haya elegido
    // el operador con F4 — mismo umbral que ya tenía esta tabla. Mismas 8
    // columnas que la tabla de arriba (ColumnaActivos), sin marcador de
    // selección: esta vista no navega ítem por ítem.
    let angosto = ancho < ANCHO_TABLA_COMPLETA;
    let visibles =
        columnas_visibles(columnas).filter(|c| !(angosto && *c == ColumnaActivos::Empresa));
    let anchos = anchos_columnas(ancho, visibles, ancho_fijo_activos);

    let mut lineas = vec![
        Line::from(Span::styled(
            fila_columnas(&anchos, derecha_activos, |c| c.etiqueta().to_uppercase()),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("─".repeat(ancho as usize), muted())),
    ];
    for item in items {
        lineas.push(Line::from(fila_columnas(&anchos, derecha_activos, |c| {
            valor_activos(item, c)
        })));
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

fn lineas_nuevo_contratista() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("NUEVO CONTRATISTA", muted())),
        Line::from(""),
        Line::from("Se abrirá el formulario de alta: cédula, nombre, empresa, tipo,"),
        Line::from("fecha PRAIND, personal de ruta y acceso."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para abrir el formulario · Esc para cancelar",
            acento(),
        )),
    ]
}

fn lineas_abrir_historial() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("HISTORIAL", muted())),
        Line::from(""),
        Line::from("Se abrirá el explorador de movimientos: filtro clave:valor"),
        Line::from("(empresa/tipo/estado/gafete/ingreso/salida/desde/hasta)."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para abrir · Esc para cancelar",
            acento(),
        )),
    ]
}

// ── Selector de columnas (F4) ────────────────────────────────────────────

/// Segunda Surface enclavada (§5.2, junto al formulario): `[✓]`/`[ ]` con
/// `›` en la activa — mismo vocabulario de foco que el resto de la app.
fn lineas_selector_columnas(app: &AppState, edicion: EdicionColumnas) -> Vec<Line<'static>> {
    let titulo = match edicion.objetivo {
        ObjetivoColumnas::Busqueda => "COLUMNAS — resultados de búsqueda",
        ObjetivoColumnas::Activos => "COLUMNAS — activos",
        ObjetivoColumnas::Historial => "COLUMNAS — historial",
    };
    let filas: Vec<(&'static str, bool)> = match edicion.objetivo {
        ObjetivoColumnas::Busqueda => app
            .columnas_busqueda
            .iter()
            .map(|(c, v)| (c.etiqueta(), v))
            .collect(),
        ObjetivoColumnas::Activos => app
            .columnas_activos
            .iter()
            .map(|(c, v)| (c.etiqueta(), v))
            .collect(),
        ObjetivoColumnas::Historial => app
            .columnas_historial
            .iter()
            .map(|(c, v)| (c.etiqueta(), v))
            .collect(),
    };

    let mut lineas = vec![Line::from(Span::styled(titulo, muted())), Line::from("")];
    for (indice, (etiqueta, visible)) in filas.into_iter().enumerate() {
        let activo = indice == edicion.seleccion;
        let marcador = if activo { "› " } else { "  " };
        let casillero = if visible { "[✓] " } else { "[ ] " };
        let texto = format!("{marcador}{casillero}{etiqueta}");
        lineas.push(if activo {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else if visible {
            Line::from(texto)
        } else {
            Line::from(Span::styled(texto, muted()))
        });
    }
    lineas
}

// ── Historial (Surface enclavada, §5.2/DEC-023/024) ──────────────────────

fn ancho_fijo_historial(columna: ColumnaHistorial) -> Option<usize> {
    match columna {
        ColumnaHistorial::Ingreso | ColumnaHistorial::Salida => Some(13),
        ColumnaHistorial::Tipo => Some(12),
        ColumnaHistorial::Gafete => Some(8),
        ColumnaHistorial::Nombre | ColumnaHistorial::Empresa | ColumnaHistorial::Usuario => None,
    }
}

fn derecha_historial(columna: ColumnaHistorial) -> bool {
    matches!(columna, ColumnaHistorial::Gafete)
}

fn fecha_hora_corta(instante: chrono::DateTime<Utc>) -> String {
    a_costa_rica(instante).format("%d/%m %H:%M").to_string()
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

fn lineas_historial(
    historial: &HistorialState,
    texto_input: &str,
    ancho: u16,
    columnas: &SelectorColumnas<ColumnaHistorial>,
) -> Vec<Line<'static>> {
    let Some(resultado) = &historial.resultado else {
        // Editando: todavía no se aplicó ninguna consulta (o se volvió a
        // editar con Esc) — sin filtrado en vivo, DEC-024.
        let rango = format!(
            "Rango actual: {} – {}",
            a_costa_rica(historial.filtro.desde).format("%d/%m/%Y"),
            fecha_hasta_visual(historial.filtro.hasta)
        );
        return vec![
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
        ];
    };

    let mut lineas = vec![Line::from(Span::styled(
        resumen_filtro_historial(historial),
        muted(),
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
        return lineas;
    }

    let anchos = anchos_columnas(ancho, columnas_visibles(columnas), ancho_fijo_historial);
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
    lineas
}

// ── Formulario de contratista ────────────────────────────────────────────

fn lineas_formulario(
    formulario: &FormularioContratista,
    consulta_empresa: &str,
) -> Vec<Line<'static>> {
    match formulario.subfase {
        Subfase::Resumen => lineas_resumen_formulario(formulario),
        _ => lineas_campos_formulario(formulario, consulta_empresa),
    }
}

fn lineas_campos_formulario(
    formulario: &FormularioContratista,
    consulta_empresa: &str,
) -> Vec<Line<'static>> {
    let titulo = match formulario.modo {
        ModoFormulario::Nuevo => "NUEVO CONTRATISTA".to_string(),
        ModoFormulario::Editar { .. } => format!("EDITAR CONTRATISTA — {}", formulario.nombre),
    };
    let mut lineas = vec![Line::from(Span::styled(titulo, muted())), Line::from("")];

    for campo in Campo::ORDEN {
        lineas.push(linea_campo(formulario, campo));
        // El desplegable de empresa muta justo debajo de su propio campo, no
        // al final del formulario — mismo criterio que la TUI clásica
        // (`restricciones.insert(indice + 1, ...)`), para que la mutación se
        // vea anclada a lo que la originó en vez de aparecer desconectada
        // más abajo (DEC-025).
        if campo == Campo::Empresa
            && let Subfase::EligiendoEmpresa { seleccion } = formulario.subfase
        {
            lineas.extend(lineas_selector_empresa(
                formulario,
                consulta_empresa,
                seleccion,
            ));
        }
    }
    lineas
}

fn lineas_selector_empresa(
    formulario: &FormularioContratista,
    consulta_empresa: &str,
    seleccion: usize,
) -> Vec<Line<'static>> {
    let filtradas = formulario.empresas_filtradas(consulta_empresa);
    if filtradas.is_empty() {
        return vec![Line::from(Span::styled(
            format!("    Sin empresas para \"{consulta_empresa}\""),
            muted(),
        ))];
    }
    filtradas
        .iter()
        .take(MAX_VISIBLES_EMPRESAS)
        .enumerate()
        .map(|(indice, empresa)| {
            let marcador = if indice == seleccion {
                "  › "
            } else {
                "    "
            };
            let texto = format!("{marcador}{}", empresa.nombre);
            if indice == seleccion {
                Line::from(Span::styled(texto, estilo_seleccion()))
            } else {
                Line::from(texto)
            }
        })
        .collect()
}

/// Una línea por campo: `›` en el activo, bloqueados apagados con su motivo,
/// `✓`/`×` de estado en los campos que admiten quedar vacíos o inválidos
/// (`Campo::admite_estado`) — mismo vocabulario de glifos que el resto de la
/// app (`glifo_feedback`, §5), no un marcador propio del formulario.
fn linea_campo(formulario: &FormularioContratista, campo: Campo) -> Line<'static> {
    let activo = formulario.campo == campo
        && matches!(
            formulario.subfase,
            Subfase::Editando | Subfase::EligiendoEmpresa { .. }
        );
    let habilitado = formulario.campo_habilitado(campo);
    let marcador = if activo { "› " } else { "  " };
    let etiqueta = format!("{:<16}", campo.etiqueta());

    if !habilitado {
        return Line::from(Span::styled(
            format!(
                "{marcador}{etiqueta}{} (sin permiso)",
                valor_campo(formulario, campo)
            ),
            muted(),
        ));
    }

    let mut spans = vec![Span::styled(
        format!("{marcador}{etiqueta}"),
        if activo { acento() } else { Style::default() },
    )];
    let valor = valor_campo(formulario, campo);
    let valor_presente = !valor.is_empty();
    let valor_mostrado = if valor.is_empty() {
        match campo {
            Campo::FechaPraind => "DD/MM/AAAA".to_string(),
            Campo::Empresa => "Space para elegir…".to_string(),
            _ => String::new(),
        }
    } else {
        valor
    };
    let estilo_valor = if valor_mostrado.is_empty() {
        muted()
    } else if activo && !campo.es_texto() {
        acento()
    } else if campo == Campo::FechaPraind && valor_mostrado == "DD/MM/AAAA"
        || campo == Campo::Empresa && valor_mostrado == "Space para elegir…"
    {
        muted()
    } else {
        Style::default()
    };
    // Los valores que se alternan se muestran entre guiones cuando están
    // activos, sugiriendo el Space/←/→.
    let valor_final = if activo && matches!(campo, Campo::Tipo | Campo::Ruta | Campo::Acceso) {
        format!("‹ {valor_mostrado} ›")
    } else {
        valor_mostrado
    };
    spans.push(Span::styled(valor_final, estilo_valor));
    if let Some(mensaje) = formulario.error_de(campo) {
        spans.push(Span::styled(format!("  × {mensaje}"), estilo_error()));
    } else if campo.admite_estado() && valor_presente {
        spans.push(Span::styled("  ✓", exito()));
    }
    Line::from(spans)
}

fn valor_campo(formulario: &FormularioContratista, campo: Campo) -> String {
    match campo {
        Campo::Cedula => formulario.cedula.clone(),
        Campo::Nombre => formulario.nombre.clone(),
        Campo::Empresa => formulario
            .empresa
            .as_ref()
            .map(|(_, nombre)| nombre.clone())
            .unwrap_or_default(),
        Campo::Tipo => tipo_texto(formulario.tipo).to_string(),
        Campo::FechaPraind => formulario.fecha_praind.clone(),
        Campo::Ruta => si_no(formulario.es_personal_ruta).to_string(),
        Campo::Acceso => si_no(formulario.tiene_acceso).to_string(),
    }
}

fn si_no(valor: bool) -> &'static str {
    if valor { "Sí" } else { "No" }
}

fn lineas_resumen_formulario(formulario: &FormularioContratista) -> Vec<Line<'static>> {
    let fecha = if formulario.requiere_praind() {
        formulario.fecha_praind.clone()
    } else {
        "no aplica".to_string()
    };
    let empresa = formulario
        .empresa
        .as_ref()
        .map(|(_, nombre)| nombre.clone())
        .unwrap_or_default();
    let filas = [
        ("Cédula", formulario.cedula.clone()),
        ("Nombre", formulario.nombre.clone()),
        ("Empresa", empresa),
        ("Tipo", tipo_texto(formulario.tipo).to_string()),
        ("Fecha PRAIND", fecha),
        (
            "Personal de ruta",
            si_no(formulario.es_personal_ruta).to_string(),
        ),
        ("Acceso", si_no(formulario.tiene_acceso).to_string()),
    ];
    let mut lineas = vec![
        Line::from(Span::styled("REVISAR Y CONFIRMAR", muted())),
        Line::from(""),
    ];
    for (etiqueta, valor) in filas {
        lineas.push(Line::from(vec![
            Span::styled(format!("{etiqueta:<18}"), muted()),
            Span::raw(valor),
        ]));
    }
    lineas.push(Line::from(""));
    lineas.push(Line::from(Span::styled(
        "ENTER para guardar · Esc para volver a editar",
        acento(),
    )));
    lineas
}

fn lineas_ayuda() -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled(
            "AYUDA — sintaxis de comandos",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    let ejemplos: [(&str, &str); 13] = [
        ("/ingreso <nombre> G:<n> M:<medio>", "registrar un ingreso"),
        ("/ingreso 119430546 G:12", "también por cédula"),
        ("/salida <nombre>", "registrar salida por nombre"),
        ("/salida G:27", "registrar salida por gafete"),
        ("/activos", "tabla de personas dentro"),
        ("/nuevo", "dar de alta un contratista"),
        ("/editar <nombre>", "editar un contratista"),
        ("/historial", "explorar movimientos (alias /h)"),
        ("/cerrarsesion", "cerrar sesión y volver al login"),
        ("/ayuda", "esta ayuda"),
        ("texto sin /", "búsqueda de contratistas"),
        (
            "<nombre> --i G:<n> M:<medio>",
            "atajo: mismo resultado que /ingreso, /salida o /editar",
        ),
        (
            "empresa:x tipo:a,b -salida:ana",
            "dentro de /historial: filtro clave:valor, negable con -",
        ),
    ];
    for (sintaxis, descripcion) in ejemplos {
        lineas.push(Line::from(vec![
            Span::styled(format!("{sintaxis:<36}"), acento()),
            Span::styled(descripcion, muted()),
        ]));
    }
    lineas.push(Line::from(""));
    lineas.push(Line::from(Span::styled(
        "Claves: G: gafete · M: caminando|vehiculo (por defecto caminando) · alias: /i /s /a /n /e /h /cs",
        muted(),
    )));
    lineas.push(Line::from(Span::styled(
        "Modificador sobre una búsqueda: --i/--ingreso, --s/--salida, --e/--editar (sólo éstos tres)",
        muted(),
    )));
    lineas.push(Line::from(Span::styled(
        "F4 sobre una tabla de resultados: elegir qué columnas mostrar",
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

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
use crate::models::usuario::RolUsuario;
use crate::services::registro_ingreso_service::IngresoActivoResumen;
use crate::tiempo::a_costa_rica;

use super::breakpoint::Breakpoint;
use super::columnas::{
    Columna, ColumnaActivos, ColumnaBusqueda, ColumnaHistorial, SelectorColumnas,
};
use super::estado::{
    AppState, ContextState, EdicionColumnas, Fase, NivelFeedback, ObjetivoColumnas, SurfaceActiva,
};
use super::formulario::{
    Campo, FormularioContratista, MAX_VISIBLES_EMPRESAS, ModoFormulario, Subfase,
};
use super::formulario_empresa::{FormularioEmpresa, ModoFormularioEmpresa};
use super::formulario_usuario::{
    CampoUsuario, FormularioUsuario, ModoFormularioUsuario, SubfaseUsuario,
};
use super::historial::HistorialState;
use super::parser::Comando;
use super::resolver::MIN_CONSULTA;
use super::salida_gafete::SalidaGafeteState;

/// Mínimos razonables: por debajo de esto no cabe ni la tarjeta más simple —
/// se muestra un aviso en vez de romper el prompt.
const ANCHO_MINIMO: u16 = 40;
const ALTO_MINIMO: u16 = 10;

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

    let paleta = app.paleta_comandos();
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
        let opacidades = OpacidadesFormulario {
            campo: app.presentacion.opacidad("form_campo"),
            resumen: app.presentacion.opacidad("form_resumen"),
            error: app.presentacion.opacidad("form_error"),
        };
        lineas_formulario(formulario, app.input.value(), &opacidades)
    } else if let Some(fe) = &app.formulario_empresa {
        lineas_formulario_empresa(fe)
    } else if let Some(fu) = &app.formulario_usuario {
        lineas_formulario_usuario(fu)
    } else if let Some(edicion) = &app.edicion_columnas {
        lineas_selector_columnas(app, *edicion)
    } else if let Some(historial) = &app.historial {
        let opacidades = OpacidadesHistorial {
            resultado: app.presentacion.opacidad("historial_resultado"),
            exportar: app.presentacion.opacidad("historial_exportar"),
        };
        lineas_historial(
            historial,
            app.input.value(),
            area_contexto.width,
            &app.columnas_historial,
            &opacidades,
        )
    } else if let Some(sg) = &app.salida_gafete {
        lineas_salida_gafete(sg)
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
        let pista = if historial.exportacion_destino.is_some() {
            "escriba la ruta del XLSX · Enter exporta · Esc cancela"
        } else if historial.resultado.is_some() {
            "↑↓ moverse · PageUp/PageDown más · F4 columnas · F5 exportar · Esc editar filtro"
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
    if app.formulario_empresa.is_some() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Enter guardar · Esc cancelar",
                muted(),
            ))),
            area,
        );
        return;
    }
    if let Some(fu) = &app.formulario_usuario {
        let pista = match fu.subfase {
            SubfaseUsuario::Editando => {
                "↑↓ campo · Space/←/→ cambiar rol · Enter guardar · Esc cancelar"
            }
            SubfaseUsuario::Resumen => "Enter guardar · Esc volver a editar",
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(pista, muted()))),
            area,
        );
        return;
    }
    if app.salida_gafete.is_some() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "número(s) de gafete, separados por coma · Enter confirma salida · Esc cierra",
                muted(),
            ))),
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
    // Borde en acento cuando el teclado está enclavado en una Surface
    // (§5.2) — misma señal visual en los cuatro casos (formulario de alta,
    // formulario de empresa/usuario, columnas, Historial), para que se note
    // de un vistazo que Esc hace falta para volver a los comandos, sin
    // tener que leer la pista de abajo.
    let enclavado = !matches!(app.surface_activa(), SurfaceActiva::Ninguna);
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if enclavado { acento() } else { muted() });
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
    // Fila resaltada = la que Tab/Enter completarían (`app.seleccion_paleta`,
    // movida con ↑↓ — ver `operando.rs`). Se acota por si la lista se achicó
    // al seguir escribiendo y el índice quedó desactualizado.
    let seleccionada = app
        .seleccion_paleta
        .min(comandos.len().saturating_sub(1));
    let lineas: Vec<Line> = comandos
        .iter()
        .enumerate()
        .map(|(indice, comando)| {
            let marcador = if indice == seleccionada { "› " } else { "  " };
            if indice == seleccionada {
                Line::from(Span::styled(
                    format!("{marcador}/{:<8}{}", comando.nombre(), descripcion_comando(*comando)),
                    estilo_seleccion(),
                ))
            } else {
                Line::from(vec![
                    Span::styled(format!("{marcador}/{:<8}", comando.nombre()), acento()),
                    Span::styled(descripcion_comando(*comando), muted()),
                ])
            }
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), fila_comandos);
}

/// Sólo la línea de texto del input (etiqueta + valor + cursor), sin marco —
/// el marco lo pone `render_prompt`, que reutiliza esto tanto con paleta
/// como sin ella. Sólo se llama en fase `Operando`: el login tiene su propia
/// composición sin caja (`render_login`), nunca pasa por acá.
fn render_prompt_linea(frame: &mut Frame, area: Rect, app: &AppState) {
    let (etiqueta, valor, cursor_visible, cursor_chars) = if let Some(destino) = app
        .historial
        .as_ref()
        .and_then(|h| h.exportacion_destino.as_ref())
    {
        // F5: el input edita la ruta de exportación — propio, no
        // `app.input`, para no pisar el filtro congelado detrás.
        (
            "destino xlsx › ".to_string(),
            destino.value().to_string(),
            true,
            destino.visual_cursor(),
        )
    } else if let Some(historial) = &app.historial {
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
    } else if app.formulario_empresa.is_some() {
        (
            "empresa › ".to_string(),
            app.input.value().to_string(),
            true,
            app.input.visual_cursor(),
        )
    } else if let Some(fu) = &app.formulario_usuario {
        let etiqueta = match fu.subfase {
            SubfaseUsuario::Resumen => "confirmar › ".to_string(),
            SubfaseUsuario::Editando => format!("{} › ", fu.campo.etiqueta().to_lowercase()),
        };
        let editable = matches!(fu.subfase, SubfaseUsuario::Editando) && fu.campo.es_texto();
        // Password/Confirmar se enmascaran también acá — no sólo en el
        // área de contenido — nunca se ve la contraseña en texto plano.
        let valor = if editable && fu.campo.es_secreto() {
            "•".repeat(app.input.value().chars().count())
        } else {
            app.input.value().to_string()
        };
        (
            etiqueta,
            valor,
            editable,
            if editable {
                app.input.visual_cursor()
            } else {
                0
            },
        )
    } else if app.salida_gafete.is_some() {
        (
            "gafete › ".to_string(),
            app.input.value().to_string(),
            true,
            app.input.visual_cursor(),
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
    // El scroll se calcula sobre el `Input` que de verdad gobierna el
    // cursor en este frame — con la exportación abierta es
    // `exportacion_destino`, no `app.input` (que sigue congelado detrás).
    let scroll = match (
        cursor_visible,
        app.historial
            .as_ref()
            .and_then(|h| h.exportacion_destino.as_ref()),
    ) {
        (false, _) => 0,
        (true, Some(destino)) => destino.visual_scroll(viewport),
        (true, None) => app.input.visual_scroll(viewport),
    };
    let visible: String = valor.chars().skip(scroll).take(viewport).collect();

    let mut spans = vec![Span::styled(etiqueta, acento())];
    if cursor_visible {
        // Cursor propio (celda resaltada), nunca el bloque real del
        // terminal — mismo criterio que ya usa el login (`linea_prompt`) y
        // por la misma razón: el cursor real de cada emulador de terminal
        // parpadea y se reposiciona con su propio timing, fuera de nuestro
        // control, y se veía aparecer/desaparecer de forma inconsistente.
        // A diferencia del login (que sólo escribe al final), acá el
        // cursor puede estar a mitad del texto (←/→/Home/End de
        // `tui_input`), así que se resalta el carácter bajo el cursor en
        // vez de insertar un "_" que correría el resto del texto.
        let columna = cursor_chars.saturating_sub(scroll).min(viewport);
        let (antes, resto) = visible.split_at(
            visible
                .char_indices()
                .nth(columna)
                .map_or(visible.len(), |(i, _)| i),
        );
        let mut caracteres = resto.chars();
        let bajo_cursor = caracteres
            .next()
            .map(String::from)
            .unwrap_or_else(|| " ".to_string());
        let despues: String = caracteres.collect();
        spans.push(Span::raw(antes.to_string()));
        spans.push(Span::styled(
            bajo_cursor,
            Style::default().add_modifier(Modifier::REVERSED),
        ));
        spans.push(Span::raw(despues));
    } else {
        spans.push(Span::raw(visible));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
                // El valor NO se espacia — a diferencia de la etiqueta (un
                // rótulo fijo, decorativo), esto es lo que el operador acaba
                // de teclear: separar sus dígitos o el enmascarado de la
                // contraseña sólo lo hace más difícil de releer.
                let valor = valor_prompt(&app.fase, app);
                let linea = linea_prompt(&etiqueta_espaciada, &valor, vacio, opacidad_prompt);
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
    // Mayúscula (sin espaciado entre letras — se sentía impostado en un
    // nombre real, a diferencia de la etiqueta fija del prompt) le da más
    // presencia sin usar el mismo tratamiento de bloques que la marca —
    // sigue siendo, a propósito, un peso visual menor que el título grande.
    Line::from(Span::styled(
        nombre.to_uppercase(),
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
        ContextState::CoincidenciasEmpresas {
            consulta,
            items,
            seleccion,
        } => lineas_coincidencias_empresas(consulta, items, *seleccion),
        ContextState::CoincidenciasUsuarios {
            consulta,
            items,
            seleccion,
        } => lineas_coincidencias_usuarios(consulta, items, *seleccion),
        ContextState::ResumenIngreso { .. } => lineas_resumen_ingreso(contexto),
        ContextState::ResumenSalida { activo } => lineas_resumen_salida(activo),
        ContextState::TablaActivos {
            items,
            total,
            seleccion,
        } => lineas_tabla_activos(items, *total, *seleccion, ancho, columnas_activos),
        ContextState::FichaContratista { resumen } => lineas_ficha(resumen),
        ContextState::ConfirmarCerrarSesion => lineas_cerrar_sesion(),
        ContextState::NuevoContratista => lineas_nuevo_contratista(),
        ContextState::NuevoEmpresa => lineas_nuevo_empresa(),
        ContextState::NuevoUsuario => lineas_nuevo_usuario(),
        ContextState::AbrirHistorial => lineas_abrir_historial(),
        ContextState::AbrirSalidaGafete { texto } => lineas_abrir_salida_gafete(texto),
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
        Comando::Gafete => "salida rápida por gafete — uno o varios, separados por coma",
        Comando::Activos => "quién está dentro ahora",
        Comando::Nuevo => "dar de alta — contratista (default), empresa o usuario",
        Comando::Editar => "editar — contratista (default), empresa o usuario",
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

/// Lista simple, sin columnas ni F4: `EmpresaResumen`/`UsuarioResumen`
/// tienen pocos campos (3 y 4) y esta pantalla es de paso — elegir con ↑↓ y
/// entrar al formulario de edición — no un reporte que valga la pena poder
/// reconfigurar (DEC-052).
fn lineas_coincidencias_empresas(
    consulta: &str,
    items: &[crate::database::queries::empresas::EmpresaResumen],
    seleccion: usize,
) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled("EDITAR EMPRESA", muted())),
        Line::from(""),
    ];
    if consulta.chars().count() < MIN_CONSULTA {
        lineas.push(Line::from(Span::styled(
            format!("Escriba al menos {MIN_CONSULTA} letras para buscar…"),
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
    lineas
}

fn lineas_coincidencias_usuarios(
    consulta: &str,
    items: &[crate::database::queries::usuarios::UsuarioResumen],
    seleccion: usize,
) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled("EDITAR USUARIO", muted())),
        Line::from(""),
    ];
    if consulta.chars().count() < MIN_CONSULTA {
        lineas.push(Line::from(Span::styled(
            format!("Escriba al menos {MIN_CONSULTA} letras para buscar…"),
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
    lineas
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

/// Superficie del modo enclavado de salida por gafete (DEC-057): una fila
/// por número reconocido en `estado.texto`, con el nombre ya resuelto en
/// vivo — esa vista previa es la confirmación (no hay una segunda
/// pantalla): lo que se ve acá es exactamente a quién se le va a registrar
/// la salida al presionar Enter.
fn lineas_salida_gafete(estado: &SalidaGafeteState) -> Vec<Line<'static>> {
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
    let anchos = anchos_columnas(ancho, visibles, ancho_fijo_activos);

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

fn lineas_nuevo_empresa() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("NUEVA EMPRESA", muted())),
        Line::from(""),
        Line::from("Se abrirá el alta de empresa: sólo el nombre."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para abrir el formulario · Esc para cancelar",
            acento(),
        )),
    ]
}

fn lineas_nuevo_usuario() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("NUEVO USUARIO", muted())),
        Line::from(""),
        Line::from("Se abrirá el formulario de alta: cédula, nombre, rol y contraseña."),
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

/// `texto` es lo que ya se escribió después de `/gafete` — si no está
/// vacío, Enter lo procesa de una vez (DEC-057), así que la tarjeta lo
/// anticipa en vez de mostrar el mismo texto genérico siempre.
fn lineas_abrir_salida_gafete(texto: &str) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled("SALIDA POR GAFETE", muted())),
        Line::from(""),
        Line::from("Se abrirá el modo de salida rápida: sólo números de gafete,"),
        Line::from("uno o varios separados por coma. Queda abierto para el siguiente."),
        Line::from(""),
    ];
    if texto.trim().is_empty() {
        lineas.push(Line::from(Span::styled(
            "ENTER para abrir · Esc para cancelar",
            acento(),
        )));
    } else {
        lineas.push(Line::from(Span::styled(
            format!("ENTER registra la salida de: {}", texto.trim()),
            acento(),
        )));
    }
    lineas
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

/// Opacidades vigentes de la Surface de Historial (Fase 5).
struct OpacidadesHistorial {
    /// Encabezado del resultado aplicado (funde al aparecer o al cambiar de
    /// página/consulta).
    resultado: f32,
    /// Pantalla de exportación (`F5`).
    exportar: f32,
}

fn lineas_historial(
    historial: &HistorialState,
    texto_input: &str,
    ancho: u16,
    columnas: &SelectorColumnas<ColumnaHistorial>,
    opacidades: &OpacidadesHistorial,
) -> Vec<Line<'static>> {
    if historial.exportacion_destino.is_some() {
        let total = historial.resultado.as_ref().map_or(0, |r| r.total);
        return vec![
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
        ];
    }
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

/// Opacidades vigentes de la Surface del formulario (Fase 5) — una por
/// elemento que puede fundir, ya resueltas por `render()` desde
/// `app.presentacion` antes de bajar a estas funciones puras.
struct OpacidadesFormulario {
    /// Campo activo (marcador `›` + etiqueta) o fila resaltada del
    /// selector de empresa.
    campo: f32,
    /// Tarjeta "REVISAR Y CONFIRMAR".
    resumen: f32,
    /// Glifos `×` de error, todos juntos.
    error: f32,
}

fn lineas_formulario(
    formulario: &FormularioContratista,
    consulta_empresa: &str,
    opacidades: &OpacidadesFormulario,
) -> Vec<Line<'static>> {
    match formulario.subfase {
        Subfase::Resumen => lineas_resumen_formulario(formulario, opacidades.resumen),
        _ => lineas_campos_formulario(formulario, consulta_empresa, opacidades),
    }
}

fn lineas_campos_formulario(
    formulario: &FormularioContratista,
    consulta_empresa: &str,
    opacidades: &OpacidadesFormulario,
) -> Vec<Line<'static>> {
    let titulo = match formulario.modo {
        ModoFormulario::Nuevo => "NUEVO CONTRATISTA".to_string(),
        ModoFormulario::Editar { .. } => format!("EDITAR CONTRATISTA — {}", formulario.nombre),
    };
    let mut lineas = vec![Line::from(Span::styled(titulo, muted())), Line::from("")];

    for campo in Campo::ORDEN {
        lineas.push(linea_campo(formulario, campo, opacidades));
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

/// Una línea por campo: un solo glifo a la izquierda resume el estado —
/// `›` en edición (el campo activo ahora mismo), `×` con error, `✓`
/// completo, o nada si todavía no aplica ninguno — mismo vocabulario que el
/// resto de la app (`glifo_feedback`, §5), un slot en vez de dos (antes el
/// foco vivía a la izquierda y la validez a la derecha, por separado, sin
/// necesidad: son estados del mismo lugar, nunca simultáneos). `›` gana
/// mientras el campo está activo — es la información más útil en ese
/// momento — y `×`/`✓` aparecen recién al alejarse.
fn linea_campo(
    formulario: &FormularioContratista,
    campo: Campo,
    opacidades: &OpacidadesFormulario,
) -> Line<'static> {
    let activo = formulario.campo == campo
        && matches!(
            formulario.subfase,
            Subfase::Editando | Subfase::EligiendoEmpresa { .. }
        );
    let habilitado = formulario.campo_habilitado(campo);
    let etiqueta = format!("{:<16}", campo.etiqueta());
    let valor = valor_campo(formulario, campo);
    let valor_presente = !valor.is_empty();
    // El campo activo funde su acento en vez de aparecer a color pleno de
    // golpe — mismo mecanismo que el título del login (Fase 5), sobre el
    // mismo `estilo_fundido`.
    let estilo_activo = || estilo_fundido(FADE_ACENTO, opacidades.campo, Modifier::empty());

    let (glifo, estilo_glifo) = if activo {
        ("›", estilo_activo())
    } else if formulario.error_de(campo).is_some() {
        (
            "×",
            estilo_fundido(FADE_ERROR, opacidades.error, Modifier::empty()),
        )
    } else if campo.admite_estado() && valor_presente {
        ("✓", exito())
    } else {
        (" ", Style::default())
    };
    let marcador = format!("{glifo} ");

    if !habilitado {
        return Line::from(Span::styled(
            format!("{marcador}{etiqueta}{valor} (sin permiso)"),
            muted(),
        ));
    }

    let mut spans = vec![
        Span::styled(marcador, estilo_glifo),
        Span::styled(
            etiqueta,
            if activo {
                estilo_activo()
            } else {
                Style::default()
            },
        ),
    ];
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
        estilo_activo()
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
    // El glifo de error ya vive a la izquierda (`×`, junto al foco) — acá
    // sólo queda el texto del motivo, sin repetirlo.
    if let Some(mensaje) = formulario.error_de(campo) {
        spans.push(Span::styled(
            format!("  {mensaje}"),
            estilo_fundido(FADE_ERROR, opacidades.error, Modifier::empty()),
        ));
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

fn lineas_resumen_formulario(
    formulario: &FormularioContratista,
    opacidad: f32,
) -> Vec<Line<'static>> {
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
    // El título y la acción fundan al aparecer — mismo mecanismo que el
    // login (Fase 5): sólo esos dos elementos, no la tarjeta entera, igual
    // criterio minimalista que ya usaba la escena de login (título/prompt/
    // aviso, no cada línea).
    let mut lineas = vec![
        Line::from(Span::styled(
            "REVISAR Y CONFIRMAR",
            estilo_fundido(FADE_MUTED, opacidad, Modifier::empty()),
        )),
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
        estilo_fundido(FADE_ACENTO, opacidad, Modifier::empty()),
    )));
    lineas
}

// ── Formulario de empresa (un solo campo, sin Resumen) ───────────────────

fn lineas_formulario_empresa(form: &FormularioEmpresa) -> Vec<Line<'static>> {
    let (glifo, estilo_glifo) = if form.error.is_some() {
        ("× ", estilo_error())
    } else {
        ("› ", acento())
    };
    let titulo = match form.modo {
        ModoFormularioEmpresa::Nuevo => "NUEVA EMPRESA".to_string(),
        ModoFormularioEmpresa::Editar { .. } => "EDITAR EMPRESA".to_string(),
    };
    let mut lineas = vec![
        Line::from(Span::styled(titulo, muted())),
        Line::from(""),
        Line::from(vec![
            Span::styled(glifo, estilo_glifo),
            Span::styled(format!("{:<10}", "Nombre"), acento()),
            Span::raw(form.nombre.clone()),
        ]),
    ];
    if let Some(mensaje) = &form.error {
        lineas.push(Line::from(Span::styled(
            format!("  {mensaje}"),
            estilo_error(),
        )));
    }
    lineas
}

// ── Formulario de usuario ─────────────────────────────────────────────────

fn rol_texto(rol: RolUsuario) -> &'static str {
    match rol {
        RolUsuario::Root => "ROOT",
        RolUsuario::Administrador => "ADMINISTRADOR",
        RolUsuario::Operador => "OPERADOR",
    }
}

fn lineas_formulario_usuario(form: &FormularioUsuario) -> Vec<Line<'static>> {
    match form.subfase {
        SubfaseUsuario::Resumen => lineas_resumen_usuario(form),
        SubfaseUsuario::Editando => lineas_campos_usuario(form),
    }
}

fn lineas_campos_usuario(form: &FormularioUsuario) -> Vec<Line<'static>> {
    let titulo = match form.modo {
        ModoFormularioUsuario::Nuevo => "NUEVO USUARIO".to_string(),
        ModoFormularioUsuario::Editar { .. } => format!("EDITAR USUARIO — {}", form.nombre),
    };
    let mut lineas = vec![Line::from(Span::styled(titulo, muted())), Line::from("")];
    for campo in CampoUsuario::ORDEN {
        lineas.push(linea_campo_usuario(form, campo));
    }
    lineas
}

/// Mismo glifo unificado a la izquierda que el formulario de contratista
/// (DEC-042): `›` en edición, `×` con error, `✓` completo. Rol no admite
/// "completo" — siempre tiene un valor, un check ahí no aportaría nada.
fn linea_campo_usuario(form: &FormularioUsuario, campo: CampoUsuario) -> Line<'static> {
    let activo = form.campo == campo;
    let etiqueta = format!("{:<16}", campo.etiqueta());
    let completo = match campo {
        CampoUsuario::Cedula => !form.cedula.is_empty(),
        CampoUsuario::Nombre => !form.nombre.is_empty(),
        CampoUsuario::Password => !form.password.is_empty(),
        CampoUsuario::ConfirmarPassword => !form.confirmar_password.is_empty(),
        CampoUsuario::Rol => false,
    };
    let (glifo, estilo_glifo) = if activo {
        ("› ", acento())
    } else if form.error_de(campo).is_some() {
        ("× ", estilo_error())
    } else if completo {
        ("✓ ", exito())
    } else {
        ("  ", Style::default())
    };

    let mut spans = vec![
        Span::styled(glifo, estilo_glifo),
        Span::styled(etiqueta, if activo { acento() } else { Style::default() }),
    ];
    let valor_mostrado = match campo {
        CampoUsuario::Cedula => form.cedula.clone(),
        CampoUsuario::Nombre => form.nombre.clone(),
        CampoUsuario::Rol => rol_texto(form.rol).to_string(),
        CampoUsuario::Password => "•".repeat(form.password.chars().count()),
        CampoUsuario::ConfirmarPassword => "•".repeat(form.confirmar_password.chars().count()),
    };
    let es_rol_activo = activo && campo == CampoUsuario::Rol;
    let valor_final = if es_rol_activo {
        format!("‹ {valor_mostrado} ›")
    } else {
        valor_mostrado
    };
    spans.push(Span::styled(
        valor_final,
        if es_rol_activo {
            acento()
        } else {
            Style::default()
        },
    ));
    if let Some(mensaje) = form.error_de(campo) {
        spans.push(Span::styled(format!("  {mensaje}"), estilo_error()));
    } else if campo == CampoUsuario::Password
        && form.password.is_empty()
        && matches!(form.modo, ModoFormularioUsuario::Editar { .. })
    {
        spans.push(Span::styled(
            "  (dejar en blanco para no cambiarla)",
            muted(),
        ));
    }
    Line::from(spans)
}

fn lineas_resumen_usuario(form: &FormularioUsuario) -> Vec<Line<'static>> {
    // Nunca se muestra la contraseña, ni en el resumen — sólo si se
    // definió/cambió. En edición, dejarla en blanco significa "sin
    // cambios" (DEC-053), distinto de "(definida)" para no sugerir que se
    // está por sobrescribir algo que en realidad se conserva.
    let texto_password = if !form.password.is_empty() {
        "(definida)"
    } else if matches!(form.modo, ModoFormularioUsuario::Editar { .. }) {
        "(sin cambios)"
    } else {
        "(definida)"
    };
    let filas = [
        ("Cédula", form.cedula.clone()),
        ("Nombre", form.nombre.clone()),
        ("Rol", rol_texto(form.rol).to_string()),
        ("Contraseña", texto_password.to_string()),
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
    let ejemplos: [(&str, &str); 18] = [
        ("/ingreso <nombre> G:<n> M:<medio>", "registrar un ingreso"),
        ("/ingreso 119430546 G:12", "también por cédula"),
        ("/salida <nombre>", "registrar salida por nombre"),
        ("/salida G:27", "registrar salida por gafete"),
        (
            "/gafete 2, 25, 85",
            "salida rápida de uno o varios gafetes (alias /g)",
        ),
        ("/activos", "tabla de personas dentro, ↑↓ Enter da salida"),
        ("/nuevo", "dar de alta un contratista (default)"),
        ("/nuevo empresa", "dar de alta una empresa (alias /n em)"),
        (
            "/nuevo usuario",
            "dar de alta un usuario (alias /n u, requiere permiso)",
        ),
        ("/editar <nombre>", "editar un contratista (default)"),
        (
            "/editar empresa <nombre>",
            "editar una empresa (alias /e em)",
        ),
        (
            "/editar usuario <cédula|nombre>",
            "editar un usuario (alias /e u, requiere permiso)",
        ),
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
        "G:/M: admiten un solo valor, sin lista ni negación — listas (a,b,c) y -clave sólo existen dentro de /historial",
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
        "F5 con resultados de /historial: exportar el filtro completo a XLSX",
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

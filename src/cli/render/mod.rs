//! Render adaptativo puro: `&AppState` → frame, sin tocar `AppCore` ni el
//! input. El área contextual cambia de contenido según el [`ContextState`];
//! debajo, una línea de feedback/sugerencias y el prompt siempre visible.
//!
//! Estilo sobrio: sin cajas anidadas ni bordes decorativos. La jerarquía sale
//! del espacio, la alineación y los símbolos ✓ ⚠ ✗ — el color es apoyo, nunca
//! el único canal (una terminal sin color sigue transmitiendo lo mismo).
//!
//! Cada Surface tiene su propio módulo de render (mismo corte que los
//! controladores en `mod.rs`): [`login`], [`prompt`] (línea de input +
//! paleta de comandos), [`contexto`] (el despachador del área central),
//! [`busqueda`]/[`activos`]/[`historial`] (las tablas), [`formulario`]/
//! [`formulario_empresa`]/[`formulario_usuario`], [`columnas_selector`]
//! (F4) y [`ayuda`]. [`estilos`] y [`util`] son las primitivas compartidas
//! por todos — este archivo sólo arma el layout y despacha.

mod activos;
mod auditoria;
mod ayuda;
mod busqueda;
mod columnas_selector;
mod contexto;
mod estilos;
mod formulario;
mod formulario_empresa;
mod formulario_password;
mod formulario_usuario;
mod historial;
mod login;
mod prompt;
mod tabla;
mod util;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::cli::estado::{AppState, Fase, PERIODO_BLINK_MS};
use crate::cli::formulario::Subfase;
use crate::cli::formulario_password::SubfasePassword;
use crate::cli::formulario_usuario::SubfaseUsuario;

use contexto::scroll_hacia_seleccion;
use estilos::{glifo_feedback, muted};
use formulario::OpacidadesFormulario;
use historial::OpacidadesHistorial;
use login::render_login;

/// Mínimos razonables: por debajo de esto no cabe ni la tarjeta más simple —
/// se muestra un aviso en vez de romper el prompt.
const ANCHO_MINIMO: u16 = 40;
const ALTO_MINIMO: u16 = 10;

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
    let filas_comandos = paleta.as_ref().map_or(0, |comandos| {
        u16::try_from(comandos.len()).unwrap_or(u16::MAX)
    });
    // Sin borde en ningún caso (ver `prompt::render_prompt`): sin paleta,
    // sólo la fila del input; con paleta, input + divisor + N filas.
    // El cap deja al menos 3 filas para el área de contexto arriba.
    let cap = area.height.saturating_sub(3);
    let alto_bloque_prompt = match &paleta {
        Some(_) => (2 + filas_comandos).min(cap.max(2)),
        None => 1,
    };
    // Siempre 1, con o sin paleta — si desapareciera al abrir la paleta, esa
    // fila liberada se la comía el bloque de arriba, corriendo 1 fila hacia
    // abajo el resto del prompt justo al escribir `/`. Reservarla siempre
    // mantiene esa esquina del layout completamente fija.
    //
    // Va arriba del input (no abajo, como antes): probando el layout previo
    // en runtime, el input quedaba encajonado entre la paginación de la
    // lista (arriba) y esta fila (abajo), las dos en gris, sin leerse como
    // "la línea activa" — se sentía flotando en el medio. Con la pista
    // arriba, el input vuelve a ser la última línea de la pantalla, sin nada
    // debajo, como en cualquier CLI.
    let alto_pista = 1;

    let [area_contexto, area_pista, area_prompt] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(alto_pista),
        Constraint::Length(alto_bloque_prompt),
    ])
    .areas(area);

    let (lineas, seleccionada) = if let Some(formulario) = &app.formulario {
        let opacidades = OpacidadesFormulario {
            campo: app.presentacion.opacidad("form_campo"),
            resumen: app.presentacion.opacidad("form_resumen"),
            error: app.presentacion.opacidad("form_error"),
        };
        (
            formulario::lineas_formulario(formulario, app.input.value(), &opacidades),
            None,
        )
    } else if let Some(fe) = &app.formulario_empresa {
        (formulario_empresa::lineas_formulario_empresa(fe), None)
    } else if let Some(fu) = &app.formulario_usuario {
        (formulario_usuario::lineas_formulario_usuario(fu), None)
    } else if let Some(fp) = &app.formulario_password {
        (formulario_password::lineas_formulario_password(fp), None)
    } else if let Some(edicion) = &app.edicion_columnas {
        (
            columnas_selector::lineas_selector_columnas(app, *edicion),
            None,
        )
    } else if let Some(historial) = &app.historial {
        let opacidades = OpacidadesHistorial {
            resultado: app.presentacion.opacidad("historial_resultado"),
            exportar: app.presentacion.opacidad("historial_exportar"),
        };
        historial::lineas_historial(
            historial,
            app.input.value(),
            area_contexto.width,
            &app.columnas_historial,
            &opacidades,
        )
    } else if let Some(sg) = &app.salida_gafete {
        (activos::lineas_salida_gafete(sg), None)
    } else {
        let (lineas, seleccionada) = contexto::lineas_contexto(
            &app.contexto,
            area_contexto.width,
            &app.columnas_busqueda,
            &app.columnas_activos,
        );
        (
            estilos::atenuar(lineas, app.presentacion.opacidad("area_contexto")),
            seleccionada,
        )
    };
    // Mantiene la fila resaltada visible en listas más largas que el área
    // disponible (Coincidencias/Historial ya pueden traer hasta 50 filas) —
    // sin esto, ↓ seguía moviendo la selección más allá de lo que se
    // alcanzaba a dibujar, sin ninguna señal de que se había ido de la
    // pantalla.
    let scroll_y = scroll_hacia_seleccion(seleccionada, area_contexto.height);
    frame.render_widget(Paragraph::new(lineas).scroll((scroll_y, 0)), area_contexto);

    render_pista(frame, area_pista, app);
    prompt::render_prompt(frame, area_prompt, app, paleta.as_deref());
}

/// Comandos a mostrar en el desplegable bajo el input: sólo mientras se
/// teclea el nombre del comando (`/`, `/in`, …) — antes del primer espacio.
/// En cuanto hay un espacio (ya se eligió comando y se sigue con argumentos)
/// el desplegable desaparece y vuelve la línea de pistas normal.
/// Línea debajo del recuadro del input (estilo CLI moderna): el feedback
/// transitorio tiene prioridad; sin feedback, las sugerencias del
/// autocompletado contextual, truncadas al ancho disponible.
/// Contenido "de la izquierda" de la línea de pista — feedback, la ayuda de
/// teclas de la Surface abierta, o las sugerencias del autocompletado, en ese
/// orden de precedencia. `None` cuando no hay nada que decir (input vacío,
/// sin Surface, sin feedback vigente) — la identidad de la derecha (ver
/// `render_pista`) sigue mostrándose igual, esta función sólo decide la
/// mitad izquierda.
fn contenido_pista(app: &AppState) -> Option<Line<'static>> {
    if let Some(feedback) = app.feedback_vigente() {
        let (simbolo, estilo) = glifo_feedback(feedback.nivel);
        return Some(Line::from(vec![
            Span::styled(format!("{simbolo} "), estilo),
            Span::styled(feedback.texto.clone(), estilo),
        ]));
    }
    if app.edicion_columnas.is_some() {
        return Some(Line::from(Span::styled(
            "↑↓ columna · Space marcar/desmarcar · Esc cerrar",
            muted(),
        )));
    }
    if let Some(historial) = &app.historial {
        let pista = if historial.exportando {
            "exportando… espere"
        } else if historial.exportacion_destino.is_some() {
            "escriba la ruta del XLSX · Enter exporta · Esc cancela"
        } else if historial.resultado.is_some() {
            "↑↓ moverse · PageUp/PageDown más · F4 columnas · F5 exportar · Esc editar filtro"
        } else {
            "escriba clave:valor · Enter aplica · Esc cierra Historial"
        };
        return Some(Line::from(Span::styled(pista, muted())));
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
        return Some(Line::from(Span::styled(pista, muted())));
    }
    if app.formulario_empresa.is_some() {
        return Some(Line::from(Span::styled(
            "Enter guardar · Esc cancelar",
            muted(),
        )));
    }
    if let Some(fu) = &app.formulario_usuario {
        let pista = match fu.subfase {
            SubfaseUsuario::Editando => {
                "↑↓ campo · Space/←/→ cambiar rol · Enter guardar · Esc cancelar"
            }
            SubfaseUsuario::Resumen => "Enter guardar · Esc volver a editar",
        };
        return Some(Line::from(Span::styled(pista, muted())));
    }
    if let Some(fp) = &app.formulario_password {
        let pista = match fp.subfase {
            SubfasePassword::VerificandoActual => "Enter verificar · Esc cancelar",
            SubfasePassword::Cambiando => "↑↓/Tab campo · Enter guardar · Esc volver a la actual",
        };
        return Some(Line::from(Span::styled(pista, muted())));
    }
    if app.salida_gafete.is_some() {
        return Some(Line::from(Span::styled(
            "número(s) de gafete, separados por coma · Enter confirma salida · Esc cierra",
            muted(),
        )));
    }
    if !app.sugerencias.is_empty() {
        return Some(Line::from(Span::styled(
            app.sugerencias.join("   "),
            muted(),
        )));
    }
    None
}

/// Línea de pista, ahora arriba del input (no abajo — probando layout, ver
/// el comentario de `alto_pista` en `render()`): el contenido contextual
/// (feedback, ayuda de la Surface, sugerencias), o nada si no hay nada que
/// decir. La identidad del operador que vivía acá se sacó por ahora — a
/// definir dónde va (no era el problema, pero compartía fila con esto).
fn render_pista(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Some(linea) = contenido_pista(app) {
        frame.render_widget(Paragraph::new(linea), area);
    }
}

/// `instante_inicio` es fijo por sesión: el parpadeo sigue un reloj propio
/// (ver `PERIODO_BLINK_MS`), nunca se reinicia al cambiar de Surface o al
/// tipear.
pub(super) fn blink_on(app: &AppState) -> bool {
    (app.instante_inicio.elapsed().as_millis() / PERIODO_BLINK_MS as u128).is_multiple_of(2)
}

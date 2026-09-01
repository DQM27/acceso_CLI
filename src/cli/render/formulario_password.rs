//! Render de la Surface de "cambiar mi contraseña" (`/clave`) — dos
//! pantallas, una por subfase: un solo campo enmascarado mientras se
//! verifica la actual, y dos (Nueva/Confirmar) una vez superado ese gate.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::cli::formulario_password::{CampoPassword, FormularioPassword, SubfasePassword};

use super::estilos::{acento, estilo_error, muted};

pub(super) fn lineas_formulario_password(form: &FormularioPassword) -> Vec<Line<'static>> {
    match form.subfase {
        SubfasePassword::VerificandoActual => lineas_verificando_actual(form),
        SubfasePassword::Cambiando => lineas_cambiando(form),
    }
}

fn lineas_verificando_actual(form: &FormularioPassword) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled("CAMBIAR CONTRASEÑA", muted())),
        Line::from(""),
        Line::from(vec![
            Span::styled("› ", acento()),
            Span::styled(format!("{:<16}", "Actual"), acento()),
            Span::raw("•".repeat(form.actual.chars().count())),
        ]),
    ];
    if let Some(mensaje) = &form.error {
        lineas.push(Line::from(""));
        lineas.push(Line::from(Span::styled(mensaje.clone(), estilo_error())));
    }
    lineas
}

fn lineas_cambiando(form: &FormularioPassword) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled("CAMBIAR CONTRASEÑA", muted())),
        Line::from(""),
    ];
    lineas.push(linea_campo(form, CampoPassword::Nueva));
    lineas.push(linea_campo(form, CampoPassword::Confirmar));
    if let Some(mensaje) = &form.error {
        lineas.push(Line::from(""));
        lineas.push(Line::from(Span::styled(mensaje.clone(), estilo_error())));
    }
    lineas
}

/// Mismo glifo unificado que el resto de formularios (DEC-042): `›` en
/// edición, `×` con error en ese campo — Nueva/Confirmar nunca muestran `✓`
/// (no hay "completo" útil para una contraseña, igual criterio que Rol en
/// `formulario_usuario.rs`).
fn linea_campo(form: &FormularioPassword, campo: CampoPassword) -> Line<'static> {
    let activo = form.campo == campo;
    let etiqueta = match campo {
        CampoPassword::Nueva => "Nueva",
        CampoPassword::Confirmar => "Confirmar",
    };
    let valor = match campo {
        CampoPassword::Nueva => &form.nueva,
        CampoPassword::Confirmar => &form.confirmar,
    };
    let (glifo, estilo_glifo) = if activo {
        ("› ", acento())
    } else if form.error.is_some() {
        ("× ", estilo_error())
    } else {
        ("  ", Style::default())
    };
    Line::from(vec![
        Span::styled(glifo, estilo_glifo),
        Span::styled(
            format!("{etiqueta:<16}"),
            if activo { acento() } else { Style::default() },
        ),
        Span::raw("•".repeat(valor.chars().count())),
    ])
}

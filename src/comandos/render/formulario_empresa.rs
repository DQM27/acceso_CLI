//! Formulario de empresa: un solo campo (Nombre), sin subfase de Resumen.

use ratatui::text::{Line, Span};

use crate::comandos::formulario_empresa::{FormularioEmpresa, ModoFormularioEmpresa};

use super::estilos::{acento, estilo_error, muted};

pub(super) fn lineas_formulario_empresa(form: &FormularioEmpresa) -> Vec<Line<'static>> {
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

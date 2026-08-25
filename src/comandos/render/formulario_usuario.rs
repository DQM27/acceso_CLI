//! Formulario de usuario: campos con el mismo glifo unificado que el de
//! contratista, más el selector de Rol (alternado con Space/←/→).

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::comandos::formulario_usuario::{
    CampoUsuario, FormularioUsuario, ModoFormularioUsuario, SubfaseUsuario,
};

use super::estilos::{acento, estilo_error, exito, muted};
use super::util::rol_texto;

pub(super) fn lineas_formulario_usuario(form: &FormularioUsuario) -> Vec<Line<'static>> {
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

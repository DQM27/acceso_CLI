//! Formulario de alta/edición de contratista: campos con glifo unificado de
//! estado (`›`/`×`/`✓`), el desplegable de selección de empresa anclado
//! bajo su propio campo, y la tarjeta "REVISAR Y CONFIRMAR" final.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::cli::formulario::{
    Campo, FormularioContratista, MAX_VISIBLES_EMPRESAS, ModoFormulario, Subfase,
};

use super::estilos::{estilo_fundido, exito, fade_acento, fade_error, muted};
use super::util::si_no;

/// Opacidades vigentes de la Surface del formulario (Fase 5) — una por
/// elemento que puede fundir, ya resueltas por `render()` desde
/// `app.presentacion` antes de bajar a estas funciones puras.
pub(super) struct OpacidadesFormulario {
    /// Campo activo (marcador `›` + etiqueta) o fila resaltada del
    /// selector de empresa.
    pub(super) campo: f32,
    /// Tarjeta "REVISAR Y CONFIRMAR".
    pub(super) resumen: f32,
    /// Glifos `×` de error, todos juntos.
    pub(super) error: f32,
}

pub(super) fn lineas_formulario(
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
                Line::from(Span::styled(texto, super::estilos::estilo_seleccion()))
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
    let estilo_activo = || estilo_fundido(fade_acento(), opacidades.campo, Modifier::empty());

    let (glifo, estilo_glifo) = if activo {
        ("›", estilo_activo())
    } else if formulario.error_de(campo).is_some() {
        (
            "×",
            estilo_fundido(fade_error(), opacidades.error, Modifier::empty()),
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
            estilo_fundido(fade_error(), opacidades.error, Modifier::empty()),
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
        Campo::Tipo => super::util::tipo_texto(formulario.tipo).to_string(),
        Campo::FechaPraind => formulario.fecha_praind.clone(),
        Campo::Ruta => si_no(formulario.es_personal_ruta).to_string(),
        Campo::Acceso => si_no(formulario.tiene_acceso).to_string(),
    }
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
        ("Tipo", super::util::tipo_texto(formulario.tipo).to_string()),
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
            estilo_fundido(super::estilos::fade_muted(), opacidad, Modifier::empty()),
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
        estilo_fundido(fade_acento(), opacidad, Modifier::empty()),
    )));
    lineas
}

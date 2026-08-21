use chrono::NaiveDate;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    database::queries::contratistas::ContratistaResumen,
    services::autenticacion_service::UsuarioSesion,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        ChoiceFieldOptions, CommandHint, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, ScreenShell,
        StatusKind, TextInput, Theme, detail_line, identidad_sesion, master_detail_areas,
        render_choice_field, render_separator, render_terminal_too_small,
    },
};

const COMANDOS_NORMAL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("PGUP/PGDN", "Página"),
    CommandHint::new("ENTER", "Editar"),
    CommandHint::new("N", "Nuevo"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("F4", "Columnas"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_NORMAL_FILTRADO: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("PGUP/PGDN", "Página"),
    CommandHint::new("ENTER", "Editar"),
    CommandHint::new("N", "Nuevo"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("F4", "Columnas"),
    CommandHint::new("ESC", "Limpiar filtro"),
];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Aplicar"),
    CommandHint::new("ESC", "Limpiar"),
];
const COMANDOS_FORMULARIO: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓/TAB", "Navegar"),
    CommandHint::new("SPACE", "Cambiar"),
    CommandHint::new("ENTER", "Guardar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_DESPLEGABLE: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Aceptar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_COLUMNAS: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("SPACE", "Mostrar/Ocultar"),
    CommandHint::new("ESC", "Cerrar"),
];
const ANCHO_ETIQUETA_FORMULARIO: usize = 20;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &ContratistasState,
    sesion: &UsuarioSesion,
    theme: Theme,
) {
    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        render_terminal_too_small(
            frame,
            area,
            MIN_TERMINAL_WIDTH,
            MIN_TERMINAL_HEIGHT,
            "ESC volver",
            theme,
        );
        return;
    }

    let hora = hora_actual_texto();
    let contexto = identidad_sesion(sesion);
    let (estado_texto, estado_tipo) = estado_shell(state);
    let comandos = match &state.modo {
        ModoContratistas::Normal if !state.filtro.is_empty() => COMANDOS_NORMAL_FILTRADO,
        ModoContratistas::Normal => COMANDOS_NORMAL,
        ModoContratistas::Busqueda { .. } => COMANDOS_BUSQUEDA,
        ModoContratistas::Formulario(f) if f.desplegable.is_some() => COMANDOS_DESPLEGABLE,
        ModoContratistas::Formulario(_) => COMANDOS_FORMULARIO,
        ModoContratistas::Columnas { .. } => COMANDOS_COLUMNAS,
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "CONTRATISTAS",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        authenticated: true,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: Some("Claves: empresa, tipo, praind, ruta, acceso"),
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);
}

fn estado_shell(state: &ContratistasState) -> (String, StatusKind) {
    if let ModoContratistas::Formulario(f) = &state.modo
        && let Some(error) = &f.error
    {
        return (format!("✕ {error}"), StatusKind::Error);
    }
    if let Some(mensaje) = &state.mensaje {
        let tipo = if mensaje.starts_with('✓') {
            StatusKind::Success
        } else {
            StatusKind::Error
        };
        return (mensaje.clone(), tipo);
    }
    (String::new(), StatusKind::Normal)
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &ContratistasState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let enfocado_busqueda = matches!(state.modo, ModoContratistas::Busqueda { .. });
    let (pagina, total_paginas) = state.pagina();
    let conteo = if total_paginas > 1 {
        format!(
            "{} DE {} RESULTADOS · página {pagina}/{total_paginas}",
            state.registros.len(),
            state.total()
        )
    } else {
        format!("{} RESULTADOS", state.total())
    };
    let etiqueta_busqueda = format!("BUSCAR · {conteo} · {}", state.resumen_consulta());
    let area_busqueda = render_campo(
        frame,
        filas[0],
        &etiqueta_busqueda,
        &state.filtro,
        enfocado_busqueda,
        theme,
    );
    if let ModoContratistas::Busqueda { texto } = &state.modo {
        posicionar_cursor_campo(frame, area_busqueda, texto);
    }

    let areas = master_detail_areas(filas[1], 60, altura_panel(state));
    render_separator(frame, areas.separator, areas.orientation, theme);
    let (area_tabla, area_panel) = (areas.master, areas.detail);
    render_tabla(frame, area_tabla, state, theme);
    render_panel(frame, area_panel, state, theme);
}

fn altura_fila(_: CampoFormulario, _: &FormularioContratista) -> u16 {
    1
}

fn altura_panel(state: &ContratistasState) -> u16 {
    match &state.modo {
        ModoContratistas::Formulario(f) => {
            let mut total: u16 = CampoFormulario::TODOS
                .iter()
                .map(|c| altura_fila(*c, f))
                .sum();
            if let Some((tipo, _)) = f.desplegable {
                total += match tipo {
                    Desplegable::Empresa => state.empresas.len().max(1) as u16,
                    Desplegable::Tipo => tipos().len() as u16,
                };
            }
            total + 1
        }
        ModoContratistas::Columnas { .. } => state.columnas.len() as u16 + 1,
        _ => 9,
    }
}

/// Campo editable en la misma línea y con la misma alineación que Empresa y
/// Tipo. El rectángulo devuelto comienza exactamente donde inicia el valor,
/// para posicionar allí el cursor real del `TextInput`.
fn render_campo(
    frame: &mut Frame,
    area: Rect,
    etiqueta: &str,
    valor: &str,
    activo: bool,
    theme: Theme,
) -> Rect {
    render_choice_field(
        frame,
        area,
        etiqueta,
        valor,
        activo,
        theme,
        ChoiceFieldOptions::plain(ANCHO_ETIQUETA_FORMULARIO),
    );
    let desplazamiento = ANCHO_ETIQUETA_FORMULARIO as u16 + 3;
    Rect::new(
        area.x.saturating_add(desplazamiento),
        area.y,
        area.width.saturating_sub(desplazamiento),
        1,
    )
}

fn render_opcion(
    frame: &mut Frame,
    area: Rect,
    etiqueta: &str,
    valor: &str,
    activo: bool,
    theme: Theme,
) {
    render_choice_field(
        frame,
        area,
        etiqueta,
        valor,
        activo,
        theme,
        ChoiceFieldOptions::plain(ANCHO_ETIQUETA_FORMULARIO),
    );
}

fn posicionar_cursor(frame: &mut Frame, area: Rect, contenido: &str) {
    let ancho_visible = Line::from(contenido).width() as u16;
    let x = area.x.saturating_add(ancho_visible.min(area.width));
    frame.set_cursor_position((x, area.y));
}

/// Igual que `posicionar_cursor` pero en la posición real del cursor dentro
/// del campo, no siempre al final — es lo que permite mover el cursor con
/// las flechas para corregir algo sin borrar todo el texto.
fn posicionar_cursor_campo(frame: &mut Frame, area: Rect, campo: &TextInput) {
    let antes_del_cursor: String = campo.value().chars().take(campo.cursor()).collect();
    posicionar_cursor(frame, area, &antes_del_cursor);
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &ContratistasState, theme: Theme) {
    let columnas: Vec<_> = state
        .columnas
        .iter()
        .filter_map(|(c, v)| v.then_some(*c))
        .collect();
    let capacidad = area.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.registros.len());
    let filas = state
        .registros
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, c)| {
            let seleccionado = state.seleccion == Some(inicio + visible);
            let estilo_fila = if seleccionado {
                theme.selected()
            } else {
                theme.base()
            };
            let celdas = columnas.iter().enumerate().map(|(indice, col)| {
                let valor = valor(c, *col);
                let valor = if indice == 0 {
                    format!("{} {valor}", if seleccionado { ">" } else { " " })
                } else {
                    valor
                };
                Cell::from(valor).style(if seleccionado {
                    estilo_fila
                } else {
                    estilo(c, *col, state.hoy, theme)
                })
            });
            Row::new(celdas).style(estilo_fila)
        });
    let encabezado = Row::new(columnas.iter().map(|c| c.titulo()))
        .style(theme.muted())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(
            filas,
            columnas.iter().map(|c| c.constraint()).collect::<Vec<_>>(),
        )
        .header(encabezado)
        .column_spacing(1),
        area,
    );
    if state.registros.is_empty() {
        frame.render_widget(
            Paragraph::new(if state.filtro.is_empty() {
                "Sin contratistas registrados"
            } else {
                "No hay contratistas que coincidan con la búsqueda."
            })
            .style(theme.warning())
            .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
    }
}

fn valor(c: &ContratistaResumen, col: Columna) -> String {
    match col {
        Columna::Cedula => c.cedula.clone(),
        Columna::Nombre => c.nombre.clone(),
        Columna::Empresa => c.empresa_nombre.clone(),
        Columna::Tipo => texto_tipo(c.tipo_ingreso).into(),
        Columna::Praind => c
            .fecha_vencimiento_praind
            .map(|f| f.format("%d/%m/%Y").to_string())
            .unwrap_or_else(|| "--".into()),
        Columna::Ruta => si_no(c.es_personal_ruta).into(),
        Columna::Acceso => si_no(c.tiene_acceso).into(),
    }
}

fn si_no(v: bool) -> &'static str {
    if v { "SÍ" } else { "NO" }
}

fn estilo(c: &ContratistaResumen, col: Columna, hoy: NaiveDate, theme: Theme) -> Style {
    match col {
        Columna::Acceso if c.tiene_acceso => theme.success(),
        Columna::Acceso => theme.danger(),
        Columna::Praind => estilo_fecha(c.fecha_vencimiento_praind, hoy, theme),
        _ => theme.base(),
    }
}

fn estilo_fecha(fecha: Option<NaiveDate>, hoy: NaiveDate, theme: Theme) -> Style {
    let Some(fecha) = fecha else {
        return theme.muted();
    };
    let dias = (fecha - hoy).num_days();
    if dias < 0 {
        theme.danger()
    } else if dias <= 30 {
        theme.warning()
    } else {
        theme.success()
    }
}

/// Dibuja el panel lateral según el modo y, cuando corresponde, posiciona
/// el cursor sobre el campo de texto enfocado.
fn render_panel(frame: &mut Frame, area: Rect, state: &ContratistasState, theme: Theme) {
    match &state.modo {
        ModoContratistas::Formulario(f) => render_formulario(frame, area, state, f, theme),
        ModoContratistas::Columnas { seleccion } => {
            render_columnas(frame, area, state, *seleccion, theme)
        }
        ModoContratistas::Normal | ModoContratistas::Busqueda { .. } => {
            match state.seleccionado() {
                Some(c) => render_detalle(frame, area, c, theme),
                None => frame.render_widget(
                    Paragraph::new("No hay un registro seleccionado.").style(theme.muted()),
                    area,
                ),
            }
        }
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, c: &ContratistaResumen, theme: Theme) {
    let lineas = vec![
        Line::from(c.nombre.as_str()).style(theme.title()),
        detail_line("Cédula", c.cedula.clone(), theme),
        Line::from(""),
        detail_line("Empresa", c.empresa_nombre.clone(), theme),
        detail_line("Tipo", texto_tipo(c.tipo_ingreso), theme),
        detail_line(
            "PRAIND",
            c.fecha_vencimiento_praind
                .map(|f| f.format("%d/%m/%Y").to_string())
                .unwrap_or_else(|| "No requerida".into()),
            theme,
        ),
        detail_line("Personal de ruta", si_no(c.es_personal_ruta), theme),
        detail_line(
            "Acceso",
            if c.tiene_acceso {
                "Permitido"
            } else {
                "Denegado"
            },
            theme,
        ),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_formulario(
    frame: &mut Frame,
    area: Rect,
    state: &ContratistasState,
    f: &FormularioContratista,
    theme: Theme,
) {
    let campos = CampoFormulario::TODOS;
    let mut restricciones: Vec<Constraint> = campos
        .iter()
        .map(|c| Constraint::Length(altura_fila(*c, f)))
        .collect();
    if let Some((tipo_desplegable, _)) = f.desplegable {
        let objetivo = match tipo_desplegable {
            Desplegable::Empresa => CampoFormulario::Empresa,
            Desplegable::Tipo => CampoFormulario::Tipo,
        };
        let indice = campos.iter().position(|c| *c == objetivo).unwrap_or(0);
        let alto = match tipo_desplegable {
            Desplegable::Empresa => state.empresas.len().max(1) as u16,
            Desplegable::Tipo => tipos().len() as u16,
        };
        restricciones.insert(indice + 1, Constraint::Length(alto));
    }
    restricciones.push(Constraint::Length(1));
    let filas = Layout::vertical(restricciones).split(area);

    let mut fila = 0usize;
    for (indice, campo) in campos.iter().enumerate() {
        let enfocado = f.campo == indice;
        match campo {
            CampoFormulario::Cedula => {
                if matches!(f.modo, ModoFormulario::Crear) {
                    let r = render_campo(
                        frame,
                        filas[fila],
                        "CÉDULA",
                        f.cedula.value(),
                        enfocado,
                        theme,
                    );
                    if enfocado {
                        posicionar_cursor_campo(frame, r, &f.cedula);
                    }
                } else {
                    render_opcion(
                        frame,
                        filas[fila],
                        "CÉDULA",
                        &format!("{} (no editable)", f.cedula.value()),
                        false,
                        theme,
                    );
                }
            }
            CampoFormulario::Nombre => {
                let r = render_campo(
                    frame,
                    filas[fila],
                    "NOMBRE",
                    f.nombre.value(),
                    enfocado,
                    theme,
                );
                if enfocado {
                    posicionar_cursor_campo(frame, r, &f.nombre);
                }
            }
            CampoFormulario::Empresa => {
                let valor = state
                    .empresas
                    .get(f.empresa)
                    .map_or("Sin empresas", |e| e.nombre.as_str());
                render_opcion(frame, filas[fila], "EMPRESA", valor, enfocado, theme);
                if let Some((Desplegable::Empresa, resaltado)) = f.desplegable {
                    fila += 1;
                    render_lista_desplegable(
                        frame,
                        filas[fila],
                        state.empresas.iter().map(|e| e.nombre.as_str()),
                        resaltado,
                        theme,
                    );
                }
            }
            CampoFormulario::Tipo => {
                render_opcion(
                    frame,
                    filas[fila],
                    "TIPO DE INGRESO",
                    texto_tipo(f.tipo),
                    enfocado,
                    theme,
                );
                if let Some((Desplegable::Tipo, resaltado)) = f.desplegable {
                    fila += 1;
                    render_lista_desplegable(
                        frame,
                        filas[fila],
                        tipos().iter().map(|t| texto_tipo(*t)),
                        resaltado,
                        theme,
                    );
                }
            }
            CampoFormulario::FechaPraind => {
                if f.requiere_praind() {
                    let r = render_campo(
                        frame,
                        filas[fila],
                        "FECHA PRAIND",
                        f.fecha_praind.value(),
                        enfocado,
                        theme,
                    );
                    if enfocado {
                        posicionar_cursor_campo(frame, r, &f.fecha_praind);
                    }
                } else {
                    render_opcion(
                        frame,
                        filas[fila],
                        "FECHA PRAIND",
                        "No requerida",
                        false,
                        theme,
                    );
                }
            }
            CampoFormulario::Ruta => {
                render_opcion(
                    frame,
                    filas[fila],
                    "PERSONAL DE RUTA",
                    si_no(f.personal_ruta),
                    enfocado,
                    theme,
                );
            }
            CampoFormulario::Acceso => {
                render_opcion(
                    frame,
                    filas[fila],
                    "TIENE ACCESO",
                    si_no(f.tiene_acceso),
                    enfocado,
                    theme,
                );
            }
        }
        fila += 1;
    }
    frame.render_widget(
        Paragraph::new(f.error.as_deref().unwrap_or_default()).style(theme.danger()),
        filas[fila],
    );
}

fn render_lista_desplegable<'a>(
    frame: &mut Frame,
    area: Rect,
    opciones: impl Iterator<Item = &'a str>,
    resaltado: usize,
    theme: Theme,
) {
    let lineas: Vec<Line<'_>> = opciones
        .enumerate()
        .map(|(indice, opcion)| {
            let seleccionado = indice == resaltado;
            let marcador = if seleccionado { "  >" } else { "   " };
            Line::from(format!("{marcador} {opcion}")).style(if seleccionado {
                theme.selected()
            } else {
                theme.muted()
            })
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_columnas(
    frame: &mut Frame,
    area: Rect,
    state: &ContratistasState,
    seleccion: usize,
    theme: Theme,
) {
    let lineas: Vec<Line<'_>> = state
        .columnas
        .iter()
        .enumerate()
        .map(|(indice, (c, visible))| {
            let activo = indice == seleccion;
            let estilo = if activo { theme.accent() } else { theme.base() };
            Line::from(format!(
                "{} [{}] {}",
                if activo { ">" } else { " " },
                if *visible { "x" } else { " " },
                c.titulo()
            ))
            .style(estilo)
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), area);
}

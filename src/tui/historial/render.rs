use super::*;
use crate::{
    database::queries::ingresos::MovimientoIngresoResumen,
    tiempo::{a_costa_rica, hora_actual_texto},
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, ThemePreset, render_terminal_too_small,
    },
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 22;
const ANCHO_PANEL_LATERAL: u16 = 100;

const COMANDOS_NORMAL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("PGUP/PGDN", "Página"),
    CommandHint::new("ENTER", "Detalle"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("F", "Filtros"),
    CommandHint::new("C", "Columnas"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Cerrar"),
    CommandHint::new("ESC", "Limpiar"),
];
const COMANDOS_DETALLE: &[CommandHint<'static>] = &[CommandHint::new("ESC", "Cerrar")];
const COMANDOS_FILTROS: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Editar/Seleccionar"),
    CommandHint::new("A", "Aplicar"),
    CommandHint::new("L", "Limpiar"),
    CommandHint::new("ESC", "Cerrar"),
];
const COMANDOS_FILTRO_EDITANDO: &[CommandHint<'static>] =
    &[CommandHint::new("ENTER/ESC", "Terminar")];
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

pub fn render(frame: &mut Frame, area: Rect, state: &HistorialState) {
    let theme = ThemePreset::Brisas.theme();

    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let contexto = format!("Usuario: {}", state.usuario_nombre);
    let (estado_texto_linea, estado_tipo) = estado_shell(state);
    let comandos = match &state.modo {
        ModoHistorial::Normal => COMANDOS_NORMAL,
        ModoHistorial::Busqueda { .. } => COMANDOS_BUSQUEDA,
        ModoHistorial::Detalle { .. } => COMANDOS_DETALLE,
        ModoHistorial::Filtros { editando: true, .. } => COMANDOS_FILTRO_EDITANDO,
        ModoHistorial::Filtros { .. } => COMANDOS_FILTROS,
        ModoHistorial::Desplegable { .. } => COMANDOS_DESPLEGABLE,
        ModoHistorial::Columnas { .. } => COMANDOS_COLUMNAS,
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "HISTORIAL",
        context: &contexto,
        clock: &hora,
        status: &estado_texto_linea,
        status_kind: estado_tipo,
        commands: comandos,
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);
}

fn estado_shell(state: &HistorialState) -> (String, StatusKind) {
    if let Some(m) = &state.mensaje {
        return (m.clone(), StatusKind::Error);
    }
    (String::new(), StatusKind::Normal)
}

fn etiqueta_busqueda(state: &HistorialState) -> String {
    let f = &state.filtro_aplicado;
    let (pagina, paginas) = state.pagina();
    let mut resumen = format!(
        "{} · PÁG {pagina}/{paginas} · {}–{}",
        state.total, f.desde, f.hasta
    );
    if let Some(id) = f.empresa_id {
        resumen.push_str(&format!(" · {}", empresa_texto(Some(id), &state.empresas)));
    }
    if f.tipo.is_some() {
        resumen.push_str(&format!(" · {}", tipo_texto(f.tipo)));
    }
    if f.estado != EstadoMovimiento::Todos {
        resumen.push_str(&format!(" · {}", estado_texto(f.estado)));
    }
    format!("BUSCAR · CLAVE:VALOR O TEXTO · {resumen}")
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &HistorialState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let enfocado_busqueda = matches!(state.modo, ModoHistorial::Busqueda { .. });
    let texto_busqueda = match &state.modo {
        ModoHistorial::Busqueda { texto } => texto.as_str(),
        _ => state.busqueda.as_str(),
    };
    let etiqueta = etiqueta_busqueda(state);
    let area_busqueda = render_campo(frame, filas[0], &etiqueta, texto_busqueda, enfocado_busqueda, theme);

    let (area_tabla, area_panel) = if area.width >= ANCHO_PANEL_LATERAL {
        let columnas = Layout::horizontal([
            Constraint::Percentage(63),
            Constraint::Length(1),
            Constraint::Percentage(36),
        ])
        .split(filas[1]);
        render_separador_vertical(frame, columnas[1], theme);
        (columnas[0], columnas[2])
    } else {
        let filas_apiladas = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(15.min(filas[1].height.saturating_sub(5))),
        ])
        .split(filas[1]);
        render_separador_horizontal(frame, filas_apiladas[1], theme);
        (filas_apiladas[0], filas_apiladas[2])
    };
    render_tabla(frame, area_tabla, state, theme);
    let area_edicion = render_panel(frame, area_panel, state, theme);

    if enfocado_busqueda {
        let ancho_visible = Line::from(texto_busqueda).width() as u16;
        let x = area_busqueda
            .x
            .saturating_add(ancho_visible.min(area_busqueda.width));
        frame.set_cursor_position((x, area_busqueda.y));
    } else if let Some((area_campo, valor)) = area_edicion {
        let ancho_visible = Line::from(valor.as_str()).width() as u16;
        let x = area_campo
            .x
            .saturating_add(ancho_visible.min(area_campo.width));
        frame.set_cursor_position((x, area_campo.y));
    }
}

fn render_separador_vertical(frame: &mut Frame, area: Rect, theme: Theme) {
    let lineas: Vec<Line<'static>> = (0..area.height).map(|_| Line::from("│")).collect();
    frame.render_widget(Paragraph::new(lineas).style(theme.border()), area);
}

fn render_separador_horizontal(frame: &mut Frame, area: Rect, theme: Theme) {
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(theme.border()),
        area,
    );
}

/// Misma silueta con foco o sin él (etiqueta, valor, línea); sólo cambian
/// color y peso.
fn render_campo(
    frame: &mut Frame,
    area: Rect,
    etiqueta: &str,
    valor: &str,
    activo: bool,
    theme: Theme,
) -> Rect {
    let estilo_etiqueta = if activo { theme.accent() } else { theme.muted() };
    let estilo_linea = if activo { theme.accent() } else { theme.border() };
    let valor_y = area.y.saturating_add(1);
    let linea_y = area.y.saturating_add(2);

    frame.render_widget(
        Paragraph::new(etiqueta).style(estilo_etiqueta),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(valor).style(theme.base())),
        Rect::new(area.x, valor_y, area.width, 1),
    );
    if area.height > 2 {
        frame.render_widget(
            Paragraph::new("─".repeat(area.width as usize)).style(estilo_linea),
            Rect::new(area.x, linea_y, area.width, 1),
        );
    }

    Rect::new(area.x, valor_y, area.width, 1)
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &HistorialState, theme: Theme) {
    let columnas: Vec<_> = state
        .columnas
        .iter()
        .filter_map(|(c, visible)| visible.then_some(*c))
        .collect();
    let anchos: Vec<_> = columnas.iter().map(|c| c.constraint()).collect();
    let capacidad = area.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.registros.len());
    let filas = state
        .registros
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, r)| {
            let seleccionada = state.seleccion == Some(inicio + visible);
            Row::new(columnas.iter().map(|c| Cell::from(valor(r, *c)))).style(if seleccionada {
                theme.selected()
            } else {
                theme.base()
            })
        });
    let encabezado = Row::new(columnas.iter().map(|c| c.titulo()))
        .style(theme.muted())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(filas, anchos).header(encabezado).column_spacing(1),
        area,
    );
    if state.registros.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin registros para los filtros seleccionados")
                .style(theme.warning())
                .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
    }
}

pub(super) fn valor(r: &MovimientoIngresoResumen, c: ColumnaHistorial) -> String {
    match c {
        ColumnaHistorial::Fecha => a_costa_rica(r.fecha_hora_ingreso)
            .format("%d/%m/%y")
            .to_string(),
        ColumnaHistorial::Cedula => r.cedula.clone(),
        ColumnaHistorial::Nombre => r.contratista_nombre.clone(),
        ColumnaHistorial::Empresa => r.empresa_nombre.clone(),
        ColumnaHistorial::Tipo => tipo_texto(Some(r.tipo_ingreso)).into(),
        ColumnaHistorial::Entrada => a_costa_rica(r.fecha_hora_ingreso)
            .format("%H:%M")
            .to_string(),
        ColumnaHistorial::Salida => r.fecha_hora_salida.map_or_else(
            || "--".into(),
            |f| a_costa_rica(f).format("%H:%M").to_string(),
        ),
        ColumnaHistorial::Gafete => r
            .gafete_numero
            .map_or_else(|| "S/G".into(), |g| g.to_string()),
        ColumnaHistorial::Medio => texto_medio(r.medio_ingreso).into(),
        ColumnaHistorial::UsuarioIngreso => r.usuario_ingreso_nombre.clone(),
        ColumnaHistorial::UsuarioSalida => r
            .usuario_salida_nombre
            .clone()
            .unwrap_or_else(|| "--".into()),
    }
}

fn texto_medio(m: crate::models::medio_ingreso::MedioIngreso) -> &'static str {
    match m {
        crate::models::medio_ingreso::MedioIngreso::Caminando => "Caminando",
        crate::models::medio_ingreso::MedioIngreso::Vehiculo => "Vehículo",
    }
}

/// Devuelve, cuando aplica, el área y el contenido del campo de filtro en
/// edición para que el llamador posicione el cursor.
fn render_panel(
    frame: &mut Frame,
    area: Rect,
    state: &HistorialState,
    theme: Theme,
) -> Option<(Rect, String)> {
    match &state.modo {
        ModoHistorial::Normal | ModoHistorial::Busqueda { .. } => {
            frame.render_widget(
                Paragraph::new("Seleccione ENTER para ver el detalle.").style(theme.muted()),
                area,
            );
            None
        }
        ModoHistorial::Detalle { id } => {
            if let Some(r) = state.registro(*id) {
                render_detalle(frame, area, r, theme);
            }
            None
        }
        ModoHistorial::Filtros {
            seleccion,
            editando,
        } => render_filtros(frame, area, state, *seleccion, *editando, None, theme),
        ModoHistorial::Desplegable {
            campo,
            seleccion_filtro,
            opcion,
        } => render_filtros(
            frame,
            area,
            state,
            *seleccion_filtro,
            false,
            Some((*campo, *opcion)),
            theme,
        ),
        ModoHistorial::Columnas { seleccion } => {
            render_columnas(frame, area, state, *seleccion, theme);
            None
        }
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, r: &MovimientoIngresoResumen, theme: Theme) {
    let salida = r.fecha_hora_salida.map_or_else(
        || "--".into(),
        |f| a_costa_rica(f).format("%d/%m/%Y %H:%M").to_string(),
    );
    let lineas = vec![
        Line::from(r.contratista_nombre.clone()).style(theme.title()),
        Line::from(format!("{} · {}", r.cedula, r.empresa_nombre)).style(theme.base()),
        Line::from(""),
        Line::from(format!("Tipo               {}", tipo_texto(Some(r.tipo_ingreso)))).style(theme.base()),
        Line::from(format!("Medio              {}", texto_medio(r.medio_ingreso))).style(theme.base()),
        Line::from(format!(
            "Entrada            {}",
            a_costa_rica(r.fecha_hora_ingreso).format("%d/%m/%Y %H:%M")
        ))
        .style(theme.base()),
        Line::from(format!("Usuario ingreso    {}", r.usuario_ingreso_nombre)).style(theme.base()),
        Line::from(format!("Evaluación         {}", texto_evaluacion(r))).style(theme.base()),
        Line::from(format!(
            "Reglas             {}",
            if r.reglas_version == 0 {
                "Registro migrado".to_owned()
            } else {
                format!("Versión {}", r.reglas_version)
            }
        ))
        .style(theme.muted()),
        Line::from(format!("Salida             {salida}")).style(theme.base()),
        Line::from(format!(
            "Usuario salida     {}",
            r.usuario_salida_nombre.as_deref().unwrap_or("--")
        ))
        .style(theme.base()),
        Line::from(format!(
            "Gafete             {}",
            r.gafete_numero.map_or_else(|| "S/G".into(), |g| g.to_string())
        ))
        .style(theme.base()),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}

fn texto_evaluacion(r: &MovimientoIngresoResumen) -> &'static str {
    match r.resultado_acceso {
        crate::models::registro_ingreso::ResultadoIngresoRegistrado::Permitido => "Permitido",
        crate::models::registro_ingreso::ResultadoIngresoRegistrado::PermitidoConAdvertencia => {
            "Permitido con advertencia PRAIND"
        }
        crate::models::registro_ingreso::ResultadoIngresoRegistrado::Migrado => {
            "Datos reconstruidos durante migración"
        }
    }
}

fn etiqueta_campo(c: CampoFiltro) -> &'static str {
    match c {
        CampoFiltro::Desde => "DESDE",
        CampoFiltro::Hasta => "HASTA",
        CampoFiltro::NombreCedula => "NOMBRE/CÉDULA",
        CampoFiltro::Empresa => "EMPRESA",
        CampoFiltro::Tipo => "TIPO",
        CampoFiltro::Gafete => "GAFETE",
        CampoFiltro::Estado => "ESTADO",
    }
}

fn es_campo_texto(c: CampoFiltro) -> bool {
    matches!(
        c,
        CampoFiltro::Desde | CampoFiltro::Hasta | CampoFiltro::NombreCedula | CampoFiltro::Gafete
    )
}

fn texto_filtro_edicion(f: &FiltrosHistorial, c: CampoFiltro) -> &str {
    match c {
        CampoFiltro::Desde => &f.desde,
        CampoFiltro::Hasta => &f.hasta,
        CampoFiltro::NombreCedula => &f.nombre_cedula,
        CampoFiltro::Gafete => &f.gafete,
        _ => "",
    }
}

fn valor_texto_campo(state: &HistorialState, c: CampoFiltro) -> String {
    let f = &state.filtro_edicion;
    match c {
        CampoFiltro::Empresa => empresa_texto(f.empresa_id, &state.empresas),
        CampoFiltro::Tipo => tipo_texto(f.tipo).to_owned(),
        CampoFiltro::Estado => estado_texto(f.estado).to_owned(),
        _ => String::new(),
    }
}

fn opciones_texto(state: &HistorialState, c: CampoFiltro) -> Vec<String> {
    match c {
        CampoFiltro::Empresa => std::iter::once("Todas".to_owned())
            .chain(state.empresas.iter().map(|e| e.nombre.clone()))
            .collect(),
        CampoFiltro::Tipo => ["Todos", "PRAIND", "IN HOUSE", "POR CORREO", "SWAT"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        CampoFiltro::Estado => ["Todos", "Activos", "Cerrados"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        _ => vec![],
    }
}

fn render_opcion(frame: &mut Frame, area: Rect, etiqueta: &str, valor: &str, activo: bool, theme: Theme) {
    let marcador = if activo { ">" } else { " " };
    let estilo = if activo { theme.selected() } else { theme.base() };
    frame.render_widget(
        Paragraph::new(format!("{marcador} {etiqueta:<15}{valor}")).style(estilo),
        area,
    );
}

fn render_lista_desplegable(
    frame: &mut Frame,
    area: Rect,
    opciones: &[String],
    resaltado: usize,
    theme: Theme,
) {
    let lineas: Vec<Line<'static>> = opciones
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let marcador = if i == resaltado { "  >" } else { "   " };
            Line::from(format!("{marcador} {o}")).style(if i == resaltado {
                theme.selected()
            } else {
                theme.muted()
            })
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), area);
}

/// Devuelve, cuando aplica, el área y el valor del campo en edición para
/// que el llamador posicione el cursor.
fn render_filtros(
    frame: &mut Frame,
    area: Rect,
    state: &HistorialState,
    seleccion: usize,
    editando: bool,
    desplegable: Option<(CampoFiltro, usize)>,
    theme: Theme,
) -> Option<(Rect, String)> {
    let f = &state.filtro_edicion;
    let mut y = area.y;
    let fondo = area.y.saturating_add(area.height);
    let mut cursor = None;
    for (i, campo) in CampoFiltro::TODOS.into_iter().enumerate() {
        if y >= fondo {
            break;
        }
        let seleccionado = i == seleccion;
        if es_campo_texto(campo) {
            let alto = 3.min(fondo.saturating_sub(y));
            let area_fila = Rect::new(area.x, y, area.width, alto);
            let valor = texto_filtro_edicion(f, campo);
            let area_valor = render_campo(
                frame,
                area_fila,
                etiqueta_campo(campo),
                valor,
                seleccionado,
                theme,
            );
            if seleccionado && editando {
                cursor = Some((area_valor, valor.to_owned()));
            }
            y = y.saturating_add(alto);
        } else {
            let area_fila = Rect::new(area.x, y, area.width, 1);
            render_opcion(
                frame,
                area_fila,
                etiqueta_campo(campo),
                &valor_texto_campo(state, campo),
                seleccionado,
                theme,
            );
            y = y.saturating_add(1);
            if let Some((_, opcion)) = desplegable.filter(|(c, _)| *c == campo) {
                let opciones = opciones_texto(state, campo);
                let alto = (opciones.len() as u16).min(fondo.saturating_sub(y));
                let area_lista = Rect::new(area.x, y, area.width, alto);
                render_lista_desplegable(frame, area_lista, &opciones, opcion, theme);
                y = y.saturating_add(alto);
            }
        }
    }
    cursor
}

fn render_columnas(frame: &mut Frame, area: Rect, state: &HistorialState, seleccion: usize, theme: Theme) {
    let lineas: Vec<Line<'static>> = state
        .columnas
        .iter()
        .enumerate()
        .map(|(i, (c, v))| {
            let marcador = if i == seleccion { ">" } else { " " };
            let caja = if *v { "x" } else { " " };
            Line::from(format!("{marcador} [{caja}] {}", c.titulo())).style(if i == seleccion {
                theme.selected()
            } else {
                theme.base()
            })
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), area);
}

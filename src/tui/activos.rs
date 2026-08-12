use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table},
};

use super::{mock, mock::IngresoActivoMock, theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Columna {
    Cedula,
    Nombre,
    Empresa,
    Tipo,
    Hora,
    Gafete,
    Medio,
    Usuario,
}

impl Columna {
    const TODAS: [Self; 8] = [
        Self::Cedula,
        Self::Nombre,
        Self::Empresa,
        Self::Tipo,
        Self::Hora,
        Self::Gafete,
        Self::Medio,
        Self::Usuario,
    ];

    fn titulo(self) -> &'static str {
        match self {
            Self::Cedula => "CÉDULA",
            Self::Nombre => "NOMBRE",
            Self::Empresa => "EMPRESA",
            Self::Tipo => "TIPO",
            Self::Hora => "HORA",
            Self::Gafete => "GAFETE",
            Self::Medio => "MEDIO",
            Self::Usuario => "USUARIO INGRESO",
        }
    }

    fn constraint(self) -> Constraint {
        match self {
            Self::Nombre | Self::Empresa => Constraint::Fill(3),
            Self::Cedula | Self::Tipo | Self::Usuario => Constraint::Fill(2),
            Self::Medio => Constraint::Fill(1),
            Self::Hora => Constraint::Length(7),
            Self::Gafete => Constraint::Length(8),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SalidaGafete {
    Capturando {
        numero: String,
        error: Option<String>,
    },
    Encontrado {
        id: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModoActivos {
    Normal,
    Busqueda { texto: String },
    Detalle { id: u64 },
    ConfirmarSalida { id: u64 },
    SalidaPorGafete(SalidaGafete),
    Columnas { seleccion: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionActivos {
    Ninguna,
    Volver,
}

#[derive(Debug)]
pub struct ActivosState {
    registros: Vec<IngresoActivoMock>,
    seleccion: Option<usize>,
    modo: ModoActivos,
    columnas: Vec<(Columna, bool)>,
    mensaje: Option<String>,
    filtro: String,
}

impl Default for ActivosState {
    fn default() -> Self {
        Self {
            registros: mock::ingresos_activos(),
            seleccion: Some(0),
            modo: ModoActivos::Normal,
            columnas: Columna::TODAS
                .into_iter()
                .map(|columna| {
                    (
                        columna,
                        !matches!(columna, Columna::Medio | Columna::Usuario),
                    )
                })
                .collect(),
            mensaje: None,
            filtro: String::new(),
        }
    }
}

impl ActivosState {
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionActivos {
        match self.modo.clone() {
            ModoActivos::Normal => self.handle_normal(key),
            ModoActivos::Busqueda { .. } => self.handle_busqueda(key),
            ModoActivos::Detalle { id } => self.handle_detalle(key, id),
            ModoActivos::ConfirmarSalida { id } => self.handle_confirmacion(key, id),
            ModoActivos::SalidaPorGafete(estado) => self.handle_gafete(key, estado),
            ModoActivos::Columnas { seleccion } => self.handle_columnas(key, seleccion),
        }
    }

    pub fn cantidad(&self) -> usize {
        self.registros.len()
    }

    pub fn modo(&self) -> &ModoActivos {
        &self.modo
    }

    pub fn indices_filtrados(&self) -> Vec<usize> {
        let texto = self.filtro.trim().to_lowercase();
        self.registros
            .iter()
            .enumerate()
            .filter(|(_, registro)| {
                texto.is_empty()
                    || registro.nombre.to_lowercase().contains(&texto)
                    || registro.cedula.to_lowercase().contains(&texto)
                    || registro.empresa.to_lowercase().contains(&texto)
                    || registro
                        .gafete
                        .is_some_and(|gafete| gafete.to_string().contains(&texto))
            })
            .map(|(indice, _)| indice)
            .collect()
    }

    pub fn inicio_visible(&self, capacidad: usize) -> usize {
        if capacidad == 0 {
            return 0;
        }
        self.seleccion.unwrap_or(0).saturating_sub(capacidad - 1)
    }

    fn handle_normal(&mut self, key: KeyEvent) -> AccionActivos {
        self.mensaje = None;
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id_seleccionado() {
                    self.modo = ModoActivos::Detalle { id };
                }
            }
            KeyCode::Char('s' | 'S') => self.abrir_confirmacion(),
            KeyCode::F(2) => {
                self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                    numero: String::new(),
                    error: None,
                });
            }
            KeyCode::Char('/') => {
                self.modo = ModoActivos::Busqueda {
                    texto: self.filtro.clone(),
                };
                self.seleccion = (!self.registros.is_empty()).then_some(0);
            }
            KeyCode::Char('c' | 'C') | KeyCode::F(6) => {
                self.modo = ModoActivos::Columnas { seleccion: 0 }
            }
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                self.seleccion = (!self.registros.is_empty()).then_some(0);
            }
            KeyCode::Esc => return AccionActivos::Volver,
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn handle_busqueda(&mut self, key: KeyEvent) -> AccionActivos {
        match key.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.modo = ModoActivos::Normal;
                self.seleccion = (!self.registros.is_empty()).then_some(0);
            }
            KeyCode::Enter => self.modo = ModoActivos::Normal,
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Backspace => {
                if let ModoActivos::Busqueda { texto } = &mut self.modo {
                    texto.pop();
                    self.filtro = texto.clone();
                }
                self.ajustar_seleccion_filtro();
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoActivos::Busqueda { texto } = &mut self.modo {
                    texto.push(character);
                    self.filtro = texto.clone();
                }
                self.ajustar_seleccion_filtro();
            }
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn handle_detalle(&mut self, key: KeyEvent, id: u64) -> AccionActivos {
        match key.code {
            KeyCode::Char('s' | 'S') => self.modo = ModoActivos::ConfirmarSalida { id },
            KeyCode::Esc => self.modo = ModoActivos::Normal,
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn handle_confirmacion(&mut self, key: KeyEvent, id: u64) -> AccionActivos {
        match key.code {
            KeyCode::Char('y' | 'Y') => self.eliminar(id),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => self.modo = ModoActivos::Normal,
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn handle_gafete(&mut self, key: KeyEvent, estado: SalidaGafete) -> AccionActivos {
        match estado {
            SalidaGafete::Capturando { mut numero, .. } => match key.code {
                KeyCode::Esc => self.modo = ModoActivos::Normal,
                KeyCode::Backspace => {
                    numero.pop();
                    self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                        numero,
                        error: None,
                    });
                }
                KeyCode::Char(character) if character.is_ascii_digit() => {
                    numero.push(character);
                    self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                        numero,
                        error: None,
                    });
                }
                KeyCode::Enter => {
                    let gafete = numero.parse::<u32>().ok();
                    if let Some(registro) = self.registros.iter().find(|r| r.gafete == gafete) {
                        self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Encontrado {
                            id: registro.id,
                        });
                    } else {
                        let error = format!(
                            "El gafete {} no está asignado a ningún ingreso activo",
                            if numero.is_empty() { "—" } else { &numero }
                        );
                        self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                            numero,
                            error: Some(error),
                        });
                    }
                }
                _ => {}
            },
            SalidaGafete::Encontrado { id } => match key.code {
                KeyCode::Char('y' | 'Y') => self.eliminar(id),
                KeyCode::Char('n' | 'N') | KeyCode::Esc => self.modo = ModoActivos::Normal,
                _ => {}
            },
        }
        AccionActivos::Ninguna
    }

    fn handle_columnas(&mut self, key: KeyEvent, seleccion: usize) -> AccionActivos {
        let ultimo = self.columnas.len() - 1;
        match key.code {
            KeyCode::Up => {
                self.modo = ModoActivos::Columnas {
                    seleccion: seleccion.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.modo = ModoActivos::Columnas {
                    seleccion: (seleccion + 1).min(ultimo),
                }
            }
            KeyCode::Char(' ') => {
                let visibles = self.columnas.iter().filter(|(_, visible)| *visible).count();
                if self.columnas[seleccion].1 && visibles == 1 {
                    self.mensaje = Some("Debe conservar al menos una columna".to_owned());
                } else {
                    self.columnas[seleccion].1 = !self.columnas[seleccion].1;
                    self.mensaje = None;
                }
            }
            KeyCode::Esc => self.modo = ModoActivos::Normal,
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn mover(&mut self, delta: isize) {
        let indices = self.indices_filtrados();
        if indices.is_empty() {
            self.seleccion = None;
            return;
        }
        let actual = self.seleccion.unwrap_or(0);
        self.seleccion = Some(if delta < 0 {
            actual.saturating_sub(1)
        } else {
            (actual + 1).min(indices.len() - 1)
        });
    }

    fn ajustar_seleccion_filtro(&mut self) {
        let cantidad = self.indices_filtrados().len();
        self.seleccion = if cantidad == 0 {
            None
        } else {
            Some(self.seleccion.unwrap_or(0).min(cantidad - 1))
        };
    }

    fn id_seleccionado(&self) -> Option<u64> {
        let indice = *self.indices_filtrados().get(self.seleccion?)?;
        Some(self.registros[indice].id)
    }

    fn abrir_confirmacion(&mut self) {
        if let Some(id) = self.id_seleccionado() {
            self.modo = ModoActivos::ConfirmarSalida { id };
        }
    }

    fn eliminar(&mut self, id: u64) {
        let Some(indice) = self.registros.iter().position(|registro| registro.id == id) else {
            self.modo = ModoActivos::Normal;
            return;
        };
        let eliminado = self.registros.remove(indice);
        self.mensaje = Some(match eliminado.gafete {
            Some(gafete) => format!(
                "✓ Salida registrada — {} — Gafete {:02} liberado",
                eliminado.nombre, gafete
            ),
            None => format!("✓ Salida registrada — {}", eliminado.nombre),
        });
        self.modo = ModoActivos::Normal;
        self.seleccion = if self.registros.is_empty() {
            None
        } else {
            Some(self.seleccion.unwrap_or(0).min(self.registros.len() - 1))
        };
    }

    fn registro(&self, id: u64) -> Option<&IngresoActivoMock> {
        self.registros.iter().find(|registro| registro.id == id)
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &ActivosState) {
    frame.render_widget(Block::default().style(theme::texto_normal()), area);
    if area.width < 60 || area.height < 22 {
        super::login::render(frame, area, &super::login::LoginState::default());
        return;
    }
    let contenido = Rect::new(
        area.x.saturating_add(2),
        area.y,
        area.width.saturating_sub(4),
        area.height,
    );
    let zonas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(contenido);
    render_cabecera(frame, zonas[0]);
    render_estado(frame, zonas[1], state);
    render_tabla(frame, zonas[2], state);
    render_pie(frame, zonas[3], state);
    render_modo(frame, contenido, state);
}

fn render_cabecera(frame: &mut Frame, area: Rect) {
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let columnas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(interior);
    let centro = |area: Rect| Rect::new(area.x, area.y + area.height / 2, area.width, 1);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(" B R I S A S   C L I").style(theme::titulo()),
            Line::from(" CONTROL DE ACCESO").style(theme::texto_secundario()),
        ]),
        Rect::new(
            columnas[0].x,
            columnas[0].y + columnas[0].height.saturating_sub(2) / 2,
            columnas[0].width,
            2,
        ),
    );
    frame.render_widget(
        Paragraph::new("INGRESOS ACTIVOS")
            .style(theme::foco())
            .alignment(Alignment::Center),
        centro(columnas[1]),
    );
    frame.render_widget(
        Paragraph::new("Usuario: Quintana ")
            .style(theme::texto_normal())
            .alignment(Alignment::Right),
        centro(columnas[2]),
    );
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &ActivosState) {
    let columnas: Vec<_> = state
        .columnas
        .iter()
        .filter_map(|(columna, visible)| visible.then_some(*columna))
        .collect();
    let marco = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::borde());
    let interior = marco.inner(area);
    frame.render_widget(marco, area);
    let anchos: Vec<_> = columnas
        .iter()
        .map(|columna| columna.constraint())
        .collect();
    let indices = state.indices_filtrados();
    let capacidad = interior.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(indices.len());
    let filas = indices.iter().skip(inicio).take(capacidad).enumerate().map(
        |(visible, indice_registro)| {
            let registro = &state.registros[*indice_registro];
            let seleccionado = state.seleccion == Some(inicio + visible);
            let celdas = columnas.iter().map(|columna| {
                let estilo = if seleccionado {
                    theme::seleccionado()
                } else if *columna == Columna::Tipo && registro.advertencia.is_some() {
                    theme::advertencia()
                } else {
                    theme::texto_normal()
                };
                Cell::from(valor_columna(registro, *columna)).style(estilo)
            });
            Row::new(celdas).style(if seleccionado {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            })
        },
    );
    let encabezado = Row::new(columnas.iter().map(|columna| columna.titulo()))
        .style(theme::foco())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(filas, anchos)
            .header(encabezado)
            .column_spacing(1),
        interior,
    );
    if indices.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay ingresos activos que coincidan con la búsqueda.")
                .style(theme::advertencia())
                .alignment(Alignment::Center),
            interior,
        );
    }
}

fn valor_columna(registro: &IngresoActivoMock, columna: Columna) -> String {
    match columna {
        Columna::Cedula => registro.cedula.clone(),
        Columna::Nombre => registro.nombre.clone(),
        Columna::Empresa => registro.empresa.clone(),
        Columna::Tipo => format!(
            "{}{}",
            registro.tipo,
            if registro.advertencia.is_some() {
                " !"
            } else {
                ""
            }
        ),
        Columna::Hora => registro.hora_ingreso.clone(),
        Columna::Gafete => registro
            .gafete
            .map_or_else(|| "S/G".to_owned(), |gafete| format!("{gafete:02}")),
        Columna::Medio => registro.medio.clone(),
        Columna::Usuario => registro.usuario_ingreso.clone(),
    }
}

fn render_estado(frame: &mut Frame, area: Rect, state: &ActivosState) {
    let linea = match &state.modo {
        ModoActivos::Busqueda { texto } => Line::from(vec![
            Span::styled("BUSCAR ACTIVOS: ", theme::foco()),
            Span::styled(format!("{texto}_"), theme::texto_normal()),
        ]),
        _ if !state.filtro.is_empty() => Line::from(vec![
            Span::styled("FILTRO ACTIVO: ", theme::foco()),
            Span::styled(&state.filtro, theme::texto_normal()),
            Span::styled(
                format!("    {} resultados    ", state.indices_filtrados().len()),
                theme::texto_secundario(),
            ),
            Span::styled("ESC ", theme::ayuda_tecla()),
            Span::styled("Limpiar", theme::texto_normal()),
        ]),
        _ => {
            let texto = state.mensaje.clone().unwrap_or_default();
            let estilo = if texto.starts_with('✓') {
                theme::exito()
            } else {
                theme::foco()
            };
            Line::from(texto).style(estilo)
        }
    };
    frame.render_widget(Paragraph::new(linea).alignment(Alignment::Center), area);
}

fn render_pie(frame: &mut Frame, area: Rect, state: &ActivosState) {
    let hora = Local::now().format("%H:%M:%S").to_string();
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let columnas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(40),
            Constraint::Min(20),
            Constraint::Length(12),
        ])
        .split(interior);
    let posicion = texto_posicion(state);
    frame.render_widget(
        Paragraph::new(format!(
            " {} ingresos activos │ Registro {posicion}",
            state.cantidad()
        ))
        .style(theme::texto_normal()),
        columnas[0],
    );
    let ayuda = if area.width >= 105 {
        "↑↓ Seleccionar │ ENTER Detalle │ S Salida │ F2 Gafete │ / Buscar │ C Columnas │ ESC Volver"
    } else {
        "↑↓ Mover │ ENTER Ver │ S Salida │ F2 Gafete │ / Buscar │ C Columnas"
    };
    frame.render_widget(
        Paragraph::new(ayuda)
            .style(theme::foco())
            .alignment(Alignment::Center),
        columnas[1],
    );
    frame.render_widget(
        Paragraph::new(hora)
            .style(theme::advertencia())
            .alignment(Alignment::Right)
            .block(Block::default().padding(ratatui::widgets::Padding::right(1))),
        columnas[2],
    );
}

fn render_modo(frame: &mut Frame, area: Rect, state: &ActivosState) {
    match &state.modo {
        ModoActivos::Detalle { id } => {
            if let Some(registro) = state.registro(*id) {
                render_detalle(frame, area, registro);
            }
        }
        ModoActivos::ConfirmarSalida { id } => {
            if let Some(registro) = state.registro(*id) {
                render_confirmacion(frame, area, registro);
            }
        }
        ModoActivos::SalidaPorGafete(estado) => render_gafete(frame, area, state, estado),
        ModoActivos::Columnas { seleccion } => render_columnas(frame, area, state, *seleccion),
        ModoActivos::Normal | ModoActivos::Busqueda { .. } => {}
    }
}

fn texto_posicion(state: &ActivosState) -> String {
    state.seleccion.map_or_else(
        || "—/—".to_owned(),
        |indice| format!("{}/{}", indice + 1, state.indices_filtrados().len()),
    )
}

fn overlay(frame: &mut Frame, area: Rect, ancho: u16, alto: u16, titulo: &str, lineas: Vec<Line>) {
    let area = centrar(area, ancho.min(area.width.saturating_sub(4)), alto);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lineas).style(theme::texto_normal()).block(
            Block::default()
                .title(format!(" {titulo} "))
                .title_style(theme::foco())
                .borders(Borders::ALL)
                .border_style(theme::foco())
                .padding(ratatui::widgets::Padding::uniform(1)),
        ),
        area,
    );
}

fn render_confirmacion(frame: &mut Frame, area: Rect, registro: &IngresoActivoMock) {
    overlay(
        frame,
        area,
        56,
        11,
        "REGISTRAR SALIDA",
        vec![
            Line::from(registro.nombre.clone()).style(theme::titulo()),
            Line::from(registro.empresa.clone()),
            Line::from(format!(
                "Gafete: {}",
                valor_columna(registro, Columna::Gafete)
            )),
            Line::from(""),
            Line::from("¿Desea registrar la salida?"),
            Line::from(""),
            Line::from("Y Sí        N No        ESC Cancelar").style(theme::foco()),
        ],
    );
}

fn render_detalle(frame: &mut Frame, area: Rect, registro: &IngresoActivoMock) {
    overlay(
        frame,
        area,
        62,
        16,
        "DETALLE",
        vec![
            Line::from(registro.nombre.clone()).style(theme::titulo()),
            Line::from(registro.cedula.clone()).style(theme::texto_secundario()),
            Line::from(""),
            Line::from(format!("Empresa          {}", registro.empresa)),
            Line::from(format!("Tipo             {}", registro.tipo)),
            Line::from(format!("Medio            {}", registro.medio)),
            Line::from(format!(
                "Ingreso          12/08/2026 {}",
                registro.hora_ingreso
            )),
            Line::from(format!(
                "Gafete           {}",
                valor_columna(registro, Columna::Gafete)
            )),
            Line::from(format!("Registrado por   {}", registro.usuario_ingreso)),
            Line::from(registro.advertencia.clone().unwrap_or_default())
                .style(theme::advertencia()),
            Line::from(""),
            Line::from("S Salida                         ESC Cerrar").style(theme::foco()),
        ],
    );
}

fn render_gafete(frame: &mut Frame, area: Rect, state: &ActivosState, estado: &SalidaGafete) {
    let lineas = match estado {
        SalidaGafete::Capturando { numero, error } => vec![
            Line::from(format!("Gafete: {numero}_")).style(theme::foco()),
            Line::from(""),
            Line::from(error.clone().unwrap_or_default()).style(theme::error()),
            Line::from(""),
            Line::from("ENTER Buscar                  ESC Cancelar").style(theme::foco()),
        ],
        SalidaGafete::Encontrado { id } => {
            let Some(registro) = state.registro(*id) else {
                return;
            };
            vec![
                Line::from(registro.nombre.clone()).style(theme::titulo()),
                Line::from(registro.empresa.clone()),
                Line::from(format!(
                    "Gafete: {}",
                    valor_columna(registro, Columna::Gafete)
                )),
                Line::from(format!("Ingreso: {}", registro.hora_ingreso)),
                Line::from(""),
                Line::from("¿Registrar salida?"),
                Line::from("Y Sí        N No        ESC Cancelar").style(theme::foco()),
            ]
        }
    };
    overlay(frame, area, 58, 11, "SALIDA POR GAFETE", lineas);
}

fn render_columnas(frame: &mut Frame, area: Rect, state: &ActivosState, seleccion: usize) {
    let mut lineas = Vec::new();
    for (indice, (columna, visible)) in state.columnas.iter().enumerate() {
        lineas.push(
            Line::from(format!(
                "{} [{}] {}",
                if indice == seleccion { ">" } else { " " },
                if *visible { "x" } else { " " },
                columna.titulo()
            ))
            .style(if indice == seleccion {
                theme::foco()
            } else {
                theme::texto_normal()
            }),
        );
    }
    lineas.push(Line::from(""));
    if let Some(mensaje) = &state.mensaje {
        lineas.push(Line::from(mensaje.clone()).style(theme::advertencia()));
    }
    lineas.push(Line::from("↑↓ mover  SPACE mostrar/ocultar  ESC cerrar").style(theme::foco()));
    overlay(frame, area, 48, 15, "COLUMNAS", lineas);
}

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto.min(area.height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tecla(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn escribir(state: &mut ActivosState, texto: &str) {
        for caracter in texto.chars() {
            state.handle_key(tecla(KeyCode::Char(caracter)));
        }
    }

    fn buscar(texto: &str) -> ActivosState {
        let mut state = ActivosState::default();
        state.handle_key(tecla(KeyCode::Char('/')));
        escribir(&mut state, texto);
        state
    }

    #[test]
    fn seleccion_se_mueve_y_respeta_limites() {
        let mut state = ActivosState::default();
        state.handle_key(tecla(KeyCode::Down));
        assert_eq!(state.seleccion, Some(1));
        state.handle_key(tecla(KeyCode::Up));
        state.handle_key(tecla(KeyCode::Up));
        assert_eq!(state.seleccion, Some(0));
        for _ in 0..100 {
            state.handle_key(tecla(KeyCode::Down));
        }
        assert_eq!(state.seleccion, Some(79));
    }

    #[test]
    fn scroll_logico_sigue_a_la_seleccion() {
        let mut state = ActivosState::default();
        for _ in 0..8 {
            state.handle_key(tecla(KeyCode::Down));
        }
        assert_eq!(state.inicio_visible(5), 4);
    }

    #[test]
    fn busca_por_nombre_cedula_empresa_y_gafete() {
        for (consulta, nombre) in [
            ("carlos", "Carlos Rojas"),
            ("310220488", "Carlos Rojas"),
            ("electromecánicos", "Marco Antonio Hernández"),
            ("47", "Laura Villalobos"),
        ] {
            let state = buscar(consulta);
            let indices = state.indices_filtrados();
            assert!(
                indices
                    .iter()
                    .any(|indice| state.registros[*indice].nombre == nombre)
            );
        }
    }

    #[test]
    fn busqueda_sin_resultados_y_escape_limpia_filtro() {
        let mut state = buscar("nadie-existe");
        assert!(state.indices_filtrados().is_empty());
        assert_eq!(state.seleccion, None);
        state.handle_key(tecla(KeyCode::Esc));
        assert_eq!(state.indices_filtrados().len(), 80);
        assert_eq!(state.seleccion, Some(0));
    }

    #[test]
    fn enter_conserva_filtro_y_devuelve_foco_a_tabla() {
        let mut state = buscar("carlos");
        state.handle_key(tecla(KeyCode::Enter));
        assert_eq!(state.modo, ModoActivos::Normal);
        assert_eq!(state.indices_filtrados().len(), 4);
    }

    #[test]
    fn escape_limpia_primero_el_filtro_y_despues_vuelve() {
        let mut state = buscar("carlos");
        state.handle_key(tecla(KeyCode::Enter));

        assert_eq!(
            state.handle_key(tecla(KeyCode::Esc)),
            AccionActivos::Ninguna
        );
        assert!(state.filtro.is_empty());
        assert_eq!(state.indices_filtrados().len(), 80);

        assert_eq!(state.handle_key(tecla(KeyCode::Esc)), AccionActivos::Volver);
    }

    #[test]
    fn confirmacion_se_abre_y_n_o_escape_cancelan() {
        for cancelar in [KeyCode::Char('n'), KeyCode::Esc] {
            let mut state = ActivosState::default();
            state.handle_key(tecla(KeyCode::Char('s')));
            assert!(matches!(state.modo, ModoActivos::ConfirmarSalida { .. }));
            state.handle_key(tecla(cancelar));
            assert_eq!(state.modo, ModoActivos::Normal);
            assert_eq!(state.cantidad(), 80);
        }
    }

    #[test]
    fn confirmar_salida_elimina_disminuye_contador_y_conserva_posicion() {
        let mut state = ActivosState::default();
        for _ in 0..7 {
            state.handle_key(tecla(KeyCode::Down));
        }
        let id = state.id_seleccionado().unwrap();
        state.handle_key(tecla(KeyCode::Char('s')));
        state.handle_key(tecla(KeyCode::Char('y')));
        assert_eq!(state.cantidad(), 79);
        assert!(!state.registros.iter().any(|registro| registro.id == id));
        assert_eq!(state.seleccion, Some(7));
    }

    #[test]
    fn salida_sin_gafete_no_inventa_liberacion() {
        let mut state = ActivosState::default();
        state.handle_key(tecla(KeyCode::Down));
        state.handle_key(tecla(KeyCode::Char('s')));
        state.handle_key(tecla(KeyCode::Char('y')));
        assert!(!state.mensaje.as_deref().unwrap().contains("liberado"));
    }

    #[test]
    fn f2_encuentra_gafete_y_salida_elimina_registro_correcto() {
        let mut state = ActivosState::default();
        state.handle_key(tecla(KeyCode::F(2)));
        escribir(&mut state, "8");
        state.handle_key(tecla(KeyCode::Enter));
        assert_eq!(
            state.modo,
            ModoActivos::SalidaPorGafete(SalidaGafete::Encontrado { id: 3 })
        );
        state.handle_key(tecla(KeyCode::Char('y')));
        assert_eq!(state.cantidad(), 79);
        assert!(
            !state
                .registros
                .iter()
                .any(|registro| registro.gafete == Some(8))
        );
        state.handle_key(tecla(KeyCode::F(2)));
        escribir(&mut state, "8");
        state.handle_key(tecla(KeyCode::Enter));
        assert!(matches!(
            &state.modo,
            ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { error: Some(_), .. })
        ));
    }

    #[test]
    fn f2_reporta_gafete_inexistente() {
        let mut state = ActivosState::default();
        state.handle_key(tecla(KeyCode::F(2)));
        escribir(&mut state, "999");
        state.handle_key(tecla(KeyCode::Enter));
        assert!(matches!(
            &state.modo,
            ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { error: Some(_), .. })
        ));
    }

    #[test]
    fn detalle_se_abre_cierra_y_puede_iniciar_salida() {
        let mut state = ActivosState::default();
        state.handle_key(tecla(KeyCode::Enter));
        assert!(matches!(state.modo, ModoActivos::Detalle { id: 1 }));
        state.handle_key(tecla(KeyCode::Esc));
        assert_eq!(state.modo, ModoActivos::Normal);
        state.handle_key(tecla(KeyCode::Enter));
        state.handle_key(tecla(KeyCode::Char('s')));
        assert!(matches!(state.modo, ModoActivos::ConfirmarSalida { id: 1 }));
    }

    #[test]
    fn columnas_se_abren_cambian_y_no_permiten_ocultar_todas() {
        let mut state = ActivosState::default();
        state.handle_key(tecla(KeyCode::Char('c')));
        assert!(matches!(state.modo, ModoActivos::Columnas { .. }));
        state.handle_key(tecla(KeyCode::Char(' ')));
        assert!(!state.columnas[0].1);
        for indice in 1..state.columnas.len() {
            state.columnas[indice].1 = false;
        }
        state.columnas[0].1 = true;
        state.modo = ModoActivos::Columnas { seleccion: 0 };
        state.handle_key(tecla(KeyCode::Char(' ')));
        assert!(state.columnas[0].1);
        assert!(state.mensaje.is_some());
        state.handle_key(tecla(KeyCode::Esc));
        assert_eq!(state.modo, ModoActivos::Normal);
    }

    #[test]
    fn advertencia_no_altera_seleccion() {
        let mut state = ActivosState::default();
        for _ in 0..3 {
            state.handle_key(tecla(KeyCode::Down));
        }
        assert!(state.registros[3].advertencia.is_some());
        assert_eq!(state.id_seleccionado(), Some(4));
    }
}

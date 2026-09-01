use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

use super::*;
use crate::{
    models::usuario::RolUsuario, services::autenticacion_service::UsuarioSesion,
    tui::ui_kit::ThemePreset,
};

/// Vuelca el buffer a texto plano más los tramos de estilo (color de
/// frente/fondo, negrita, etc.) que cambian a lo largo de cada fila —
/// snapshots de sólo texto no detectan una regresión que deja el contenido
/// igual pero pierde una señal de color (foco, severidad). Se omiten los
/// tramos de relleno sin texto y sin color propio para no inflar el
/// snapshot con fondo vacío.
fn volcar_buffer(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut texto = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            texto.push_str(buffer[(x, y)].symbol());
        }
        texto.push('\n');
    }

    let mut estilos = String::new();
    for y in area.top()..area.bottom() {
        let mut x = area.left();
        while x < area.right() {
            let celda = &buffer[(x, y)];
            let (fg, bg, modificador) = (celda.fg, celda.bg, celda.modifier);
            let inicio = x;
            let mut fin = x + 1;
            while fin < area.right() {
                let siguiente = &buffer[(fin, y)];
                if siguiente.fg != fg || siguiente.bg != bg || siguiente.modifier != modificador {
                    break;
                }
                fin += 1;
            }
            let tramo: String = (inicio..fin).map(|cx| buffer[(cx, y)].symbol()).collect();
            let relleno_sin_estilo =
                tramo.trim().is_empty() && fg == ratatui::style::Color::Reset && bg == fg;
            if !relleno_sin_estilo {
                estilos.push_str(&format!(
                    "{tramo:?} fg={fg:?} bg={bg:?} mod={modificador:?}\n"
                ));
            }
            x = fin;
        }
    }

    format!(
        "=== texto ===\n{}\n=== estilos ===\n{}",
        enmascarar_hora(&texto),
        enmascarar_hora(&estilos)
    )
}

/// Reemplaza cualquier `HH:MM` literal por un marcador fijo — el reloj de
/// `ScreenShell` (`hora_actual_texto()`) muestra la hora real del sistema,
/// así que sin esto el snapshot cambiaría solo con el reloj y el test
/// fallaría en cualquier corrida futura sin que nada visual haya cambiado.
fn enmascarar_hora(texto: &str) -> String {
    let chars: Vec<char> = texto.chars().collect();
    let mut resultado = String::with_capacity(texto.len());
    let mut i = 0;
    while i < chars.len() {
        let es_hora = i + 5 <= chars.len()
            && chars[i].is_ascii_digit()
            && chars[i + 1].is_ascii_digit()
            && chars[i + 2] == ':'
            && chars[i + 3].is_ascii_digit()
            && chars[i + 4].is_ascii_digit();
        if es_hora {
            resultado.push_str("··:··");
            i += 5;
        } else {
            resultado.push(chars[i]);
            i += 1;
        }
    }
    resultado
}

#[derive(Debug, Clone, Copy)]
enum Screen {
    ConfiguracionInicial,
    Login,
    Menu,
    Activos,
    Historial,
    Contratistas,
    Empresas,
    Usuarios,
    CambioPassword,
    Auditoria,
    Respaldos,
    NuevoIngreso,
}

impl Screen {
    const ALL: [Self; 12] = [
        Self::ConfiguracionInicial,
        Self::Login,
        Self::Menu,
        Self::Activos,
        Self::Historial,
        Self::Contratistas,
        Self::Empresas,
        Self::Usuarios,
        Self::CambioPassword,
        Self::Auditoria,
        Self::Respaldos,
        Self::NuevoIngreso,
    ];

    const fn title(self) -> &'static str {
        match self {
            Self::ConfiguracionInicial => "CONFIGURACIÓN INICIAL",
            Self::Login => "CONTROL DE ACCESO",
            Self::Menu => "MENÚ PRINCIPAL",
            Self::Activos => "INGRESOS ACTIVOS",
            Self::Historial => "HISTORIAL",
            Self::Contratistas => "CONTRATISTAS",
            Self::Empresas => "EMPRESAS",
            Self::Usuarios => "USUARIOS",
            Self::CambioPassword => "CAMBIAR MI CONTRASEÑA",
            Self::Auditoria => "AUDITORÍA",
            Self::Respaldos => "RESPALDOS",
            Self::NuevoIngreso => "NUEVO INGRESO",
        }
    }

    const fn min_height(self) -> u16 {
        match self {
            Self::ConfiguracionInicial => 26,
            Self::Respaldos => 20,
            _ => 22,
        }
    }

    const fn min_width(self) -> u16 {
        match self {
            Self::Auditoria => 80,
            _ => 60,
        }
    }

    /// Tecla de la pestaña que identifica esta pantalla en la barra
    /// (`OpcionMenu::desde_atajo`), o `None` para las 3 pantallas sin
    /// pestañas (ConfiguraciónInicial/Login/Menú). Las pantallas con
    /// pestañas ya no repiten su título en el encabezado — la pestaña
    /// resaltada es la única identificación visible.
    const fn tecla_pestana(self) -> Option<&'static str> {
        match self {
            Self::NuevoIngreso => Some("1"),
            Self::Activos => Some("2"),
            Self::Historial => Some("3"),
            Self::Contratistas => Some("4"),
            Self::Empresas => Some("5"),
            Self::Usuarios => Some("6"),
            Self::Auditoria => Some("7"),
            Self::Respaldos => Some("8"),
            Self::CambioPassword => Some("9"),
            Self::ConfiguracionInicial | Self::Login | Self::Menu => None,
        }
    }
}

#[test]
fn todas_las_pantallas_renderizan_la_matriz_de_tamanos_y_temas() {
    let sizes = [(60, 22), (80, 24), (99, 30), (100, 30), (140, 40)];
    let themes = [
        ThemePreset::Classic,
        ThemePreset::Brisas,
        ThemePreset::Negro,
    ];
    let session = UsuarioSesion {
        id: 1,
        cedula: "1-1111-1111".into(),
        nombre: "Operador de prueba".into(),
        rol: RolUsuario::Root,
    };

    for screen in Screen::ALL {
        for (width, height) in sizes {
            for preset in themes {
                let mut terminal =
                    Terminal::new(TestBackend::new(width, height)).expect("backend de prueba");
                terminal
                    .draw(|frame| {
                        let area = frame.area();
                        let theme = preset.theme();
                        match screen {
                            Screen::ConfiguracionInicial => configuracion_inicial::render(
                                frame,
                                area,
                                &configuracion_inicial::ConfiguracionInicialState::default(),
                                theme,
                            ),
                            Screen::Login => {
                                login::render(frame, area, &login::LoginState::default(), theme)
                            }
                            Screen::Menu => menu_principal::render(
                                frame,
                                area,
                                &menu_principal::MenuPrincipalState::default(),
                                &session,
                                theme,
                            ),
                            Screen::Activos => activos::render(
                                frame,
                                area,
                                &activos::ActivosState::default(),
                                &session,
                                theme,
                            ),
                            Screen::Historial => historial::render(
                                frame,
                                area,
                                &historial::HistorialState::default(),
                                &session,
                                theme,
                            ),
                            Screen::Contratistas => contratistas::render(
                                frame,
                                area,
                                &contratistas::ContratistasState::default(),
                                &session,
                                theme,
                            ),
                            Screen::Empresas => empresas::render(
                                frame,
                                area,
                                &empresas::EmpresasState::default(),
                                &session,
                                theme,
                            ),
                            Screen::Usuarios => usuarios::render(
                                frame,
                                area,
                                &usuarios::UsuariosState::default(),
                                &session,
                                theme,
                            ),
                            Screen::CambioPassword => cambio_password::render(
                                frame,
                                area,
                                &cambio_password::CambioPasswordState::default(),
                                &session,
                                theme,
                            ),
                            Screen::Auditoria => auditoria::render(
                                frame,
                                area,
                                &auditoria::AuditoriaState::default(),
                                &session,
                                theme,
                            ),
                            Screen::Respaldos => configuracion::render(
                                frame,
                                area,
                                &configuracion::ConfiguracionState::default(),
                                &session,
                                theme,
                            ),
                            Screen::NuevoIngreso => nuevo_ingreso::render(
                                frame,
                                area,
                                &nuevo_ingreso::NuevoIngresoState::default(),
                                &session,
                                theme,
                            ),
                        }
                    })
                    .unwrap_or_else(|error| {
                        panic!("falló {screen:?} en {width}×{height} ({preset:?}): {error}")
                    });

                let buffer = terminal.backend().buffer();
                let text: String = buffer
                    .content
                    .iter()
                    .map(ratatui::buffer::Cell::symbol)
                    .collect();
                assert!(!text.trim().is_empty(), "buffer vacío para {screen:?}");
                assert!(!text.contains('�'), "glifo inválido para {screen:?}");
                if width < screen.min_width() || height < screen.min_height() {
                    assert!(text.contains("TERMINAL DEMASIADO PEQUEÑA"));
                } else if preset == ThemePreset::Negro
                    && let Some(tecla) = screen.tecla_pestana()
                {
                    // Sólo Negro navega por pestañas; ahí reemplazan al
                    // título (ver `Screen::tecla_pestana`). Classic/Brisas
                    // siguen mostrando el título de siempre, chequeado abajo.
                    assert!(
                        text.contains(tecla),
                        "falta la pestaña de {screen:?} en {width}×{height} ({preset:?})"
                    );
                } else {
                    assert!(
                        text.contains(screen.title()),
                        "falta título de {screen:?} en {width}×{height}"
                    );
                }

                // Snapshot aprobado (texto + tramos de estilo) — detecta
                // cualquier cambio visual, no sólo que la pantalla no
                // truene o que el título siga presente.
                let volcado = volcar_buffer(buffer);
                let nombre = format!("{screen:?}_{width}x{height}_{preset:?}");
                insta::assert_snapshot!(nombre, volcado);
            }
        }
    }
}

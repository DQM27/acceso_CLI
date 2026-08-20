use ratatui::{Terminal, backend::TestBackend};

use super::*;
use crate::{
    models::usuario::RolUsuario, services::autenticacion_service::UsuarioSesion,
    tui::ui_kit::ThemePreset,
};

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
    Configuracion,
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
        Self::Configuracion,
        Self::NuevoIngreso,
    ];

    const fn title(self) -> &'static str {
        match self {
            Self::ConfiguracionInicial => "CONFIGURACIÓN INICIAL",
            Self::Login => "INICIO DE SESIÓN",
            Self::Menu => "MENÚ PRINCIPAL",
            Self::Activos => "INGRESOS ACTIVOS",
            Self::Historial => "HISTORIAL",
            Self::Contratistas => "CONTRATISTAS",
            Self::Empresas => "EMPRESAS",
            Self::Usuarios => "USUARIOS",
            Self::CambioPassword => "CAMBIAR MI CONTRASEÑA",
            Self::Auditoria => "AUDITORÍA",
            Self::Configuracion => "CONFIGURACIÓN",
            Self::NuevoIngreso => "NUEVO INGRESO",
        }
    }

    const fn min_height(self) -> u16 {
        match self {
            Self::ConfiguracionInicial => 26,
            Self::Configuracion => 20,
            _ => 22,
        }
    }

    const fn min_width(self) -> u16 {
        match self {
            Self::Auditoria => 80,
            _ => 60,
        }
    }
}

#[test]
fn todas_las_pantallas_renderizan_la_matriz_de_tamanos_y_temas() {
    let sizes = [(60, 22), (80, 24), (99, 30), (100, 30), (140, 40)];
    let themes = [ThemePreset::Classic, ThemePreset::Brisas];
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
                            Screen::Configuracion => configuracion::render(
                                frame,
                                area,
                                &configuracion::ConfiguracionState::default(),
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

                let text: String = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect();
                assert!(!text.trim().is_empty(), "buffer vacío para {screen:?}");
                assert!(!text.contains('�'), "glifo inválido para {screen:?}");
                if width < screen.min_width() || height < screen.min_height() {
                    assert!(text.contains("TERMINAL DEMASIADO PEQUEÑA"));
                } else {
                    assert!(
                        text.contains(screen.title()),
                        "falta título de {screen:?} en {width}×{height}"
                    );
                }
            }
        }
    }
}

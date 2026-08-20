use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{Terminal, backend::Backend};

use crate::application::AppCore;
use crate::models::usuario::RolUsuario;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{AutenticacionError, PasswordError, UsuarioServiceError};
use crate::services::usuario_service::CrearRootInicialInput;

use super::{
    activos::{self, AccionActivos, ActivosState},
    configuracion::{self, AccionAjustes, AccionRespaldos, ConfiguracionState},
    configuracion_inicial::{self, AccionConfiguracion, ConfiguracionInicialState, SolicitudRoot},
    contratistas::{self, AccionContratistas, ContratistasState},
    empresas::{self, AccionEmpresas, EmpresasState},
    historial::{self, AccionHistorial, HistorialState},
    login::{self, AccionLogin, LoginState},
    menu_principal::{self, AccionMenu, MenuPrincipalState, OpcionMenu},
    nuevo_ingreso::{self, AccionNuevoIngreso, NuevoIngresoState},
    salida_rapida::{self, AccionSalidaRapida, SalidaRapidaState},
    ui_kit::{StandardCommand, ThemePreset, standard_command},
    usuarios::{self, AccionUsuarios, UsuariosState},
};

const EVENT_POLL: Duration = Duration::from_millis(50);

fn mensaje_empresa(error: crate::services::error::EmpresaServiceError) -> String {
    match error {
        crate::services::error::EmpresaServiceError::NombreDuplicado => {
            "Ya existe una empresa con ese nombre".into()
        }
        crate::services::error::EmpresaServiceError::NombreEmpresaVacio => {
            "El nombre es obligatorio".into()
        }
        crate::services::error::EmpresaServiceError::EmpresaNoEncontrada => {
            "La empresa ya no existe".into()
        }
        crate::services::error::EmpresaServiceError::Database(_) => {
            "No se pudo guardar la empresa".into()
        }
    }
}

fn mensaje_contratista(error: crate::services::error::ContratistaServiceError) -> String {
    use crate::services::error::ContratistaServiceError::*;
    match error {
        ContratistaNoEncontrado => "El contratista ya no existe".into(),
        EmpresaNoEncontrada => "La empresa seleccionada ya no existe".into(),
        CedulaVacia => "La cédula es obligatoria".into(),
        NombreVacio => "El nombre es obligatorio".into(),
        PraindRequerido => "Fecha PRAIND requerida".into(),
        CedulaDuplicada => "Ya existe un contratista con esa cédula".into(),
        Database(_) => "No se pudo guardar el contratista".into(),
    }
}

fn mensaje_usuario(error: UsuarioServiceError) -> String {
    match error {
        UsuarioServiceError::UsuarioNoEncontrado => "El usuario ya no existe".into(),
        UsuarioServiceError::CedulaVacia => "La cédula es obligatoria".into(),
        UsuarioServiceError::NombreVacio => "El nombre es obligatorio".into(),
        UsuarioServiceError::PasswordDemasiadoCorto => {
            "La contraseña debe tener al menos 8 caracteres".into()
        }
        UsuarioServiceError::CedulaDuplicada => "Ya existe un usuario con esa cédula".into(),
        UsuarioServiceError::UltimoRootActivo => {
            "Debe existir al menos un usuario ROOT activo".into()
        }
        _ => "No se pudo guardar el usuario".into(),
    }
}
fn mensaje_salida(error: crate::services::error::RegistroIngresoServiceError) -> String {
    use crate::services::error::RegistroIngresoServiceError::*;
    match error {
        RegistroNoActivo => "El ingreso ya no está activo".into(),
        SalidaAnteriorAIngreso => "La salida no puede ser anterior al ingreso".into(),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        _ => "No se pudo registrar la salida".into(),
    }
}
fn mensaje_ingreso(error: crate::services::error::RegistroIngresoServiceError) -> String {
    use crate::{
        domain::resultado_acceso::MotivoDenegacion, services::error::RegistroIngresoServiceError::*,
    };
    match error {
        ContratistaNoEncontrado => "El contratista ya no existe".into(),
        IngresoActivo => "El contratista ya tiene un ingreso activo".into(),
        GafeteRequerido => "El gafete es requerido".into(),
        GafeteOcupado => "El gafete ya está en uso".into(),
        AccesoDenegado(MotivoDenegacion::SinAcceso) => "No tiene acceso autorizado".into(),
        AccesoDenegado(MotivoDenegacion::PraindVencido) => "PRAIND vencido o requerido".into(),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        _ => "No se pudo registrar el ingreso".into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vista {
    ConfiguracionInicial,
    Login,
    MenuPrincipal,
    IngresosActivos,
    Historial,
    Contratistas,
    Empresas,
    Usuarios,
    Configuracion,
    NuevoIngreso,
}

/// Cómo terminó el bucle principal: cierre normal, o una restauración de
/// respaldo confirmada que exige que `main.rs` cierre la conexión SQLite,
/// aplique el reemplazo de archivo y vuelva a arrancar la TUI desde cero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SalidaApp {
    Cerrar,
    Restaurar { candidata: std::path::PathBuf },
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use rusqlite::Connection;

    use super::*;
    use crate::database::schema::initialize_database;

    fn escribir(state: &mut ConfiguracionInicialState, texto: &str) {
        for caracter in texto.chars() {
            state.handle_key(KeyEvent::new(KeyCode::Char(caracter), KeyModifiers::NONE));
        }
    }

    #[test]
    fn configuracion_exitosa_transiciona_a_login_sin_autenticar() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let core = AppCore::new(connection);
        let mut app = App::new(true, None);
        escribir(&mut app.configuracion_inicial, "ROOT1");
        app.configuracion_inicial
            .handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        escribir(&mut app.configuracion_inicial, "Root Inicial");
        app.configuracion_inicial
            .handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        escribir(&mut app.configuracion_inicial, "password1");
        app.configuracion_inicial
            .handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        escribir(&mut app.configuracion_inicial, "password1");
        app.configuracion_inicial
            .handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));

        app.procesar_configuracion_pendiente(&core);

        // El hash de Argon2 corre en un hilo aparte; se espera el resultado real en
        // vez de asumir que ya terminó, igual que el resto de los flujos con hilo.
        for _ in 0..200 {
            app.recibir_root_inicial_si_lista(&core);
            if app.vista == Vista::Login {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        assert_eq!(app.vista, Vista::Login);
        assert!(app.sesion().is_none());
        assert!(core.autenticar("ROOT1", "password1").is_ok());
    }

    fn sesion(nombre: &str) -> UsuarioSesion {
        UsuarioSesion {
            id: 1,
            cedula: "1".into(),
            nombre: nombre.into(),
            rol: crate::models::usuario::RolUsuario::Root,
        }
    }

    fn tecla(codigo: KeyCode) -> KeyEvent {
        KeyEvent::new(codigo, KeyModifiers::NONE)
    }

    #[test]
    fn f7_cicla_el_tema_sin_importar_la_vista_activa() {
        let mut app = App {
            vista: Vista::Login,
            ..App::default()
        };
        assert_eq!(app.tema, ThemePreset::Brisas);

        app.procesar_tecla_vista(tecla(KeyCode::F(7)));
        assert_eq!(app.tema, ThemePreset::Classic);

        app.procesar_tecla_vista(tecla(KeyCode::F(7)));
        assert_eq!(app.tema, ThemePreset::Brisas);
    }

    #[test]
    fn ctrl_c_marca_salida_pero_ctrl_alt_c_no() {
        let mut app = App::default();
        app.procesar_tecla_vista(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.salir);

        let mut app = App::default();
        app.procesar_tecla_vista(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ));
        assert!(!app.salir);
    }

    #[test]
    fn menu_abre_todas_las_pantallas_y_preserva_seleccion_al_volver() {
        let casos = [
            ('1', Vista::NuevoIngreso),
            ('2', Vista::IngresosActivos),
            ('3', Vista::Historial),
            ('4', Vista::Contratistas),
            ('5', Vista::Empresas),
            ('6', Vista::Usuarios),
        ];
        for (atajo, vista) in casos {
            let mut app = App {
                vista: Vista::MenuPrincipal,
                sesion: Some(sesion("Daniel")),
                ..App::default()
            };
            app.procesar_accion_menu(tecla(KeyCode::Char(atajo)));
            assert_eq!(app.vista, vista);
            let seleccion = app.menu.seleccion;
            app.procesar_tecla_vista(tecla(KeyCode::Esc));
            assert_eq!(app.vista, Vista::MenuPrincipal);
            assert_eq!(app.menu.seleccion, seleccion);
        }
    }

    #[test]
    fn logout_limpia_sesion_login_y_conserva_estado_mock() {
        let mut app = App {
            vista: Vista::MenuPrincipal,
            sesion: Some(sesion("Usuario A")),
            ..App::default()
        };
        let activos = app.activos.cantidad();
        app.procesar_accion_menu(tecla(KeyCode::Char('L')));
        app.procesar_accion_menu(tecla(KeyCode::Enter));
        assert_eq!(app.vista, Vista::Login);
        assert!(app.sesion.is_none());
        assert_eq!(app.login.password_enmascarado(), "");
        assert_eq!(app.activos.cantidad(), activos);
        app.sesion = Some(sesion("Usuario B"));
        app.menu.nueva_sesion();
        app.vista = Vista::MenuPrincipal;
        assert_eq!(app.sesion().unwrap().nombre, "Usuario B");
    }

    #[test]
    fn salida_confirmada_marca_cierre_normal() {
        let mut app = App {
            vista: Vista::MenuPrincipal,
            sesion: Some(sesion("Daniel")),
            ..App::default()
        };
        app.procesar_accion_menu(tecla(KeyCode::Char('Q')));
        assert!(!app.salir);
        app.procesar_accion_menu(tecla(KeyCode::Enter));
        assert!(app.salir);
    }

    #[test]
    fn atajos_provisionales_ya_no_navegan() {
        let mut app = App::default();
        assert_eq!(
            app.activos.handle_key(tecla(KeyCode::Char('B'))),
            AccionActivos::Ninguna
        );
        assert_eq!(
            app.activos.handle_key(tecla(KeyCode::Char('H'))),
            AccionActivos::Ninguna
        );
        assert_eq!(
            app.activos.handle_key(tecla(KeyCode::Char('N'))),
            AccionActivos::Ninguna
        );
        assert!(matches!(
            app.contratistas.handle_key(tecla(KeyCode::Char('P'))),
            AccionContratistas::Ninguna
        ));
        assert_eq!(
            app.empresas.handle_key(tecla(KeyCode::Char('U'))),
            AccionEmpresas::Ninguna
        );
    }

    #[test]
    fn escape_raiz_regresa_al_menu_y_estados_internos_se_cierran_primero() {
        for vista in [
            Vista::IngresosActivos,
            Vista::Contratistas,
            Vista::Empresas,
            Vista::Usuarios,
        ] {
            let mut app = App {
                vista,
                sesion: Some(sesion("Daniel")),
                ..App::default()
            };
            if vista == Vista::Empresas {
                app.empresas.completar_busqueda(
                    Ok(vec![crate::database::queries::empresas::EmpresaResumen {
                        id: 1,
                        nombre: "Empresa".into(),
                        contratistas: 0,
                        activo: true,
                    }]),
                    None,
                );
            }
            if vista == Vista::IngresosActivos {
                app.activos.completar_busqueda(
                    Ok(
                        crate::services::registro_ingreso_service::ListaIngresosActivosResumen {
                            items: vec![
                                crate::services::registro_ingreso_service::IngresoActivoResumen {
                                    registro_id: 1,
                                    contratista_id: 1,
                                    cedula: "1".into(),
                                    contratista_nombre: "Persona".into(),
                                    empresa_nombre: "Empresa".into(),
                                    tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::Swat,
                                    medio_ingreso:
                                        crate::models::medio_ingreso::MedioIngreso::Caminando,
                                    fecha_hora_ingreso: crate::tiempo::local_costa_rica_a_utc(
                                        chrono::NaiveDate::from_ymd_opt(2026, 8, 12)
                                            .unwrap()
                                            .and_hms_opt(8, 0, 0)
                                            .unwrap(),
                                    )
                                    .unwrap(),
                                    gafete_numero: None,
                                    usuario_ingreso_nombre: "Ana".into(),
                                    resultado_acceso:
                                        crate::domain::resultado_acceso::ResultadoAcceso::Permitido,
                                },
                            ],
                            total: 1,
                        },
                    ),
                    None,
                );
            }
            if vista == Vista::Contratistas {
                app.contratistas.completar_busqueda(
                    Ok(crate::database::queries::contratistas::PaginaContratistas {
                        items: vec![crate::database::queries::contratistas::ContratistaResumen {
                            id: 1,
                            empresa_id: 1,
                            cedula: "1".into(),
                            nombre: "Contratista".into(),
                            empresa_nombre: "Empresa".into(),
                            tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::Swat,
                            fecha_vencimiento_praind: None,
                            es_personal_ruta: false,
                            tiene_acceso: true,
                        }],
                        total: 1,
                    }),
                    None,
                );
            }
            if vista == Vista::Usuarios {
                app.usuarios.completar_busqueda(
                    Ok(vec![crate::database::queries::usuarios::UsuarioResumen {
                        id: 1,
                        cedula: "1".into(),
                        nombre: "Daniel".into(),
                        rol: RolUsuario::Root,
                        activo: true,
                    }]),
                    None,
                );
            }
            app.procesar_tecla_vista(tecla(KeyCode::Enter));
            app.procesar_tecla_vista(tecla(KeyCode::Esc));
            assert_eq!(
                app.vista, vista,
                "el estado interno de {vista:?} debe cerrarse primero"
            );
            app.procesar_tecla_vista(tecla(KeyCode::Esc));
            assert_eq!(app.vista, Vista::MenuPrincipal);
        }

        // Historial ya no tiene un modo Detalle que cerrar primero: el panel
        // lateral refleja la selección en vivo, así que un solo ESC alcanza
        // para volver al menú.
        let mut app = App {
            vista: Vista::Historial,
            sesion: Some(sesion("Daniel")),
            ..App::default()
        };
        app.procesar_tecla_vista(tecla(KeyCode::Enter));
        app.procesar_tecla_vista(tecla(KeyCode::Esc));
        assert_eq!(app.vista, Vista::MenuPrincipal);

        let mut app = App {
            vista: Vista::NuevoIngreso,
            sesion: Some(sesion("Daniel")),
            ..App::default()
        };
        app.nuevo_ingreso.completar_busqueda(Ok(
            crate::database::queries::contratistas::PaginaContratistas {
                items: vec![crate::database::queries::contratistas::ContratistaResumen {
                    id: 1,
                    empresa_id: 1,
                    cedula: "1".into(),
                    nombre: "Persona".into(),
                    empresa_nombre: "Empresa".into(),
                    tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::Swat,
                    fecha_vencimiento_praind: None,
                    es_personal_ruta: false,
                    tiene_acceso: true,
                }],
                total: 1,
            },
        ));
        app.procesar_tecla_vista(tecla(KeyCode::Enter));
        app.nuevo_ingreso.completar_preparacion(Ok(
            crate::services::registro_ingreso_service::PreparacionIngreso {
                contratista_id: 1,
                cedula: "1".into(),
                nombre: "Persona".into(),
                empresa_nombre: "Empresa".into(),
                tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::Swat,
                resultado_acceso: crate::domain::resultado_acceso::ResultadoAcceso::Permitido,
                requiere_gafete: false,
                tiene_ingreso_activo: false,
            },
        ));
        app.procesar_tecla_vista(tecla(KeyCode::Esc));
        assert_eq!(app.vista, Vista::NuevoIngreso);
        app.procesar_tecla_vista(tecla(KeyCode::Esc));
        assert_eq!(app.vista, Vista::MenuPrincipal);
    }

    #[test]
    fn login_exitoso_inicia_menu_con_nuevo_ingreso_seleccionado() {
        let mut app = App::default();
        app.menu.seleccion = OpcionMenu::Usuarios;
        app.iniciar_sesion(sesion("Daniel Quintana"));
        assert_eq!(app.vista, Vista::MenuPrincipal);
        assert_eq!(app.menu.seleccion, OpcionMenu::NuevoIngreso);
        assert_eq!(app.sesion().unwrap().nombre, "Daniel Quintana");
    }

    #[test]
    fn empresas_tui_crea_busca_edita_y_recarga_desde_appcore_real() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let core = AppCore::new(connection);
        let mut app = App {
            vista: Vista::MenuPrincipal,
            sesion: Some(sesion("Daniel")),
            ..App::default()
        };
        app.procesar_accion_menu_con_core(tecla(KeyCode::Char('5')), Some(&core));
        assert_eq!(app.vista, Vista::Empresas);
        assert_eq!(app.empresas.cantidad(), 0);

        app.procesar_tecla_vista_con_core(tecla(KeyCode::Char('N')), Some(&core));
        for c in "Empresa Real".chars() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core));
        }
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Enter), Some(&core));
        assert_eq!(
            app.empresas.empresa_seleccionada().unwrap().nombre,
            "Empresa Real"
        );
        assert_eq!(app.empresas.empresa_seleccionada().unwrap().contratistas, 0);

        app.procesar_tecla_vista_con_core(tecla(KeyCode::Enter), Some(&core));
        for _ in 0.."Empresa Real".len() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Backspace), Some(&core));
        }
        for c in "Empresa Renombrada".chars() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core));
        }
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Enter), Some(&core));
        assert_eq!(
            app.empresas.empresa_seleccionada().unwrap().nombre,
            "Empresa Renombrada"
        );
        assert_eq!(
            core.buscar_empresas(&crate::database::queries::empresas::FiltroEmpresas {
                texto: Some("renombrada".into()),
                ..Default::default()
            })
            .unwrap()
            .len(),
            1
        );
    }

    #[test]
    fn empresas_tui_busca_cancela_y_presenta_duplicado_real() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let core = AppCore::new(connection);
        core.crear_empresa("Constructora Álvarez").unwrap();
        core.crear_empresa("Servicios Hernández").unwrap();
        let mut app = App {
            vista: Vista::MenuPrincipal,
            sesion: Some(sesion("Daniel")),
            ..App::default()
        };
        app.procesar_accion_menu_con_core(tecla(KeyCode::Char('5')), Some(&core));
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Char('/')), Some(&core));
        for c in "alvarez".chars() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core));
        }
        app.agotar_debounce_busquedas(Some(&core));
        assert_eq!(app.empresas.cantidad(), 1);
        assert_eq!(
            app.empresas.empresa_seleccionada().unwrap().nombre,
            "Constructora Álvarez"
        );

        app.procesar_tecla_vista_con_core(tecla(KeyCode::Enter), Some(&core));
        for c in " no persistir".chars() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core));
        }
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Esc), Some(&core));
        assert_eq!(
            core.buscar_empresas(&crate::database::queries::empresas::FiltroEmpresas {
                texto: Some("no persistir".into()),
                ..Default::default()
            })
            .unwrap()
            .len(),
            0
        );

        app.procesar_tecla_vista_con_core(tecla(KeyCode::Esc), Some(&core));
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Char('N')), Some(&core));
        for c in "Constructora Álvarez".chars() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core));
        }
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Enter), Some(&core));
        assert!(app.empresas.esta_en_formulario());
        assert_eq!(
            app.empresas.error_formulario_actual(),
            Some("Ya existe una empresa con ese nombre")
        );
        assert_eq!(
            core.buscar_empresas(&crate::database::queries::empresas::FiltroEmpresas::default())
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn contratistas_tui_carga_empresas_crea_edita_y_busca_con_appcore_real() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let core = AppCore::new(connection);
        let empresa_id = core.crear_empresa("Constructora Álvarez").unwrap();
        let mut app = App {
            vista: Vista::MenuPrincipal,
            sesion: Some(sesion("Daniel")),
            ..App::default()
        };
        app.procesar_accion_menu_con_core(tecla(KeyCode::Char('4')), Some(&core));
        assert_eq!(app.vista, Vista::Contratistas);
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Char('N')), Some(&core));
        for c in "001-ABC".chars() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core))
        }
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Tab), Some(&core));
        for c in "José Hernández".chars() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core))
        }
        for _ in 0..3 {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Tab), Some(&core))
        }
        for c in "31122026".chars() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core))
        }
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Tab), Some(&core));
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Enter), Some(&core));
        let items = core
            .buscar_contratistas(
                &crate::database::queries::contratistas::FiltroContratistas {
                    texto: Some("nandez".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(items.items.len(), 1);
        assert_eq!(items.items[0].empresa_id, empresa_id);
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Enter), Some(&core));
        for _ in 0.."José Hernández".chars().count() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Backspace), Some(&core))
        }
        for c in "José Álvarez".chars() {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core))
        }
        for _ in 0..5 {
            app.procesar_tecla_vista_con_core(tecla(KeyCode::Tab), Some(&core))
        }
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Enter), Some(&core));
        assert!(
            core.buscar_contratistas(
                &crate::database::queries::contratistas::FiltroContratistas {
                    texto: Some("hernandez".into()),
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .is_empty()
        );
        assert_eq!(
            core.buscar_contratistas(
                &crate::database::queries::contratistas::FiltroContratistas {
                    texto: Some("jose alvarez".into()),
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .len(),
            1
        );
        assert_eq!(
            core.buscar_contratistas(
                &crate::database::queries::contratistas::FiltroContratistas {
                    texto: Some("001-ABC".into()),
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .len(),
            1
        );
    }

    #[test]
    fn usuarios_self_edit_actualiza_sesion_segura_desde_sqlite() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let core = AppCore::new(connection);
        let id = core
            .crear_root_inicial(crate::services::usuario_service::CrearRootInicialInput {
                cedula: "ROOT-1".into(),
                nombre: "Ana".into(),
                password: "password1".into(),
            })
            .unwrap();
        core.crear_usuario(crate::services::usuario_service::CrearUsuarioInput {
            cedula: "ROOT-2".into(),
            nombre: "Respaldo".into(),
            password: "password2".into(),
            rol: RolUsuario::Root,
            activo: true,
        })
        .unwrap();
        let mut app = App {
            vista: Vista::Usuarios,
            sesion: Some(UsuarioSesion {
                id,
                cedula: "ROOT-1".into(),
                nombre: "Ana".into(),
                rol: RolUsuario::Root,
            }),
            ..App::default()
        };
        app.procesar_accion_usuarios(app.usuarios.solicitud_carga(), Some(&core));
        app.procesar_accion_usuarios(
            AccionUsuarios::Actualizar {
                id,
                input: crate::services::usuario_service::ActualizarUsuarioInput {
                    cedula: "ROOT-NUEVO".into(),
                    nombre: "Ana María".into(),
                    rol: RolUsuario::Administrador,
                },
                activo: true,
                nombre: "Ana María".into(),
            },
            Some(&core),
        );
        let sesion = app.sesion().unwrap();
        assert_eq!(sesion.cedula, "ROOT-NUEVO");
        assert_eq!(sesion.nombre, "Ana María");
        assert_eq!(sesion.rol, RolUsuario::Administrador);
    }

    #[test]
    fn f2_registra_una_salida_real_por_gafete_desde_cualquier_pantalla() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let core = AppCore::new(connection);
        let usuario_id = core
            .crear_root_inicial(crate::services::usuario_service::CrearRootInicialInput {
                cedula: "ROOT-1".into(),
                nombre: "Ana".into(),
                password: "password1".into(),
            })
            .unwrap();
        let empresa_id = core.crear_empresa("Constructora Álvarez").unwrap();
        let contratista_id = core
            .crear_contratista(crate::services::contratista_service::DatosContratista {
                cedula: "9-9999-9999".into(),
                nombre: "Persona De Prueba".into(),
                empresa_id,
                tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::PorCorreo,
                fecha_vencimiento_praind: None,
                es_personal_ruta: false,
                tiene_acceso: true,
            })
            .unwrap();
        core.registrar_ingreso(
            contratista_id,
            crate::models::medio_ingreso::MedioIngreso::Caminando,
            Some(77),
            usuario_id,
        )
        .unwrap();

        let mut app = App {
            vista: Vista::MenuPrincipal,
            sesion: Some(UsuarioSesion {
                id: usuario_id,
                cedula: "ROOT-1".into(),
                nombre: "Ana".into(),
                rol: RolUsuario::Root,
            }),
            ..App::default()
        };

        app.procesar_tecla_global(tecla(KeyCode::F(2)), Some(&core));
        assert!(app.salida_rapida.abierto());
        assert_eq!(app.vista, Vista::MenuPrincipal);

        for c in "77".chars() {
            app.procesar_tecla_global(tecla(KeyCode::Char(c)), Some(&core));
        }
        app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));

        // La confirmación queda visible hasta la siguiente tecla; el cierre real
        // en SQLite ya ocurrió.
        assert!(app.salida_rapida.abierto());
        let restantes = core
            .listar_ingresos_activos(&crate::database::queries::ingresos::FiltroIngresosActivos {
                texto: None,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(restantes.total, 0);

        app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));
        assert!(!app.salida_rapida.abierto());
    }

    #[test]
    fn f2_no_abre_sin_sesion_iniciada() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let core = AppCore::new(connection);
        let mut app = App {
            vista: Vista::Login,
            sesion: None,
            ..App::default()
        };

        app.procesar_tecla_global(tecla(KeyCode::F(2)), Some(&core));

        assert!(!app.salida_rapida.abierto());
    }

    #[test]
    fn confirmar_restauracion_deja_la_app_lista_para_salir_sin_tocar_archivos() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unico = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directorio = std::env::temp_dir().join(format!(
            "control_acceso_app_restaurar_{}_{unico}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directorio).unwrap();
        let ruta_base_datos = directorio.join("control_acceso.sqlite");
        let core = AppCore::abrir(&ruta_base_datos).unwrap();
        core.crear_root_inicial(CrearRootInicialInput {
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            password: "password1".into(),
        })
        .unwrap();
        let respaldo = core
            .crear_respaldo(crate::database::backup::TipoRespaldo::Manual)
            .unwrap();

        let mut app = App {
            vista: Vista::Configuracion,
            sesion: Some(UsuarioSesion {
                id: 1,
                cedula: "ROOT-1".into(),
                nombre: "Ana".into(),
                rol: RolUsuario::Root,
            }),
            ..App::default()
        };
        // Entra a Respaldos y carga la lista real desde el AppCore de archivo.
        let accion = app.configuracion.handle_key(tecla(KeyCode::Enter));
        app.procesar_accion_configuracion(accion, Some(&core));
        // Selecciona la única fila, pide restaurar y confirma.
        app.configuracion.handle_key(tecla(KeyCode::Char('r')));
        let accion = app.configuracion.handle_key(tecla(KeyCode::Enter));
        app.procesar_accion_configuracion(accion, Some(&core));

        assert!(app.salir);
        assert_eq!(
            app.salida,
            SalidaApp::Restaurar {
                candidata: respaldo.ruta
            }
        );
        // Restaurar el archivo de verdad es responsabilidad de main.rs, una vez
        // cerrada la conexión — App nunca debe tocar el archivo activo.
        assert!(ruta_base_datos.exists());

        std::fs::remove_dir_all(&directorio).ok();
    }

    fn core_temporal(nombre: &str) -> (AppCore, std::path::PathBuf, i64) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let unico = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directorio = std::env::temp_dir().join(format!(
            "control_acceso_app_{nombre}_{}_{unico}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directorio).unwrap();
        let ruta = directorio.join("control_acceso.sqlite");
        let core = AppCore::abrir(&ruta).unwrap();
        let root_id = core
            .crear_root_inicial(CrearRootInicialInput {
                cedula: "ROOT-1".into(),
                nombre: "Ana".into(),
                password: "password1".into(),
            })
            .unwrap();
        (core, directorio, root_id)
    }

    #[test]
    fn crear_usuario_en_hilo_aparte_termina_creado_en_sqlite_con_el_hash_correcto() {
        let (core, directorio, _root_id) = core_temporal("crear_usuario");
        let mut app = App::default();

        app.iniciar_creacion_usuario(
            crate::services::usuario_service::CrearUsuarioInput {
                cedula: "2001".into(),
                nombre: "Persona Nueva".into(),
                password: "password2".into(),
                rol: RolUsuario::Operador,
                activo: true,
            },
            "Persona Nueva".into(),
            Some(&core),
        );
        assert!(app.usuarios.guardando());

        for _ in 0..200 {
            app.recibir_hilo_usuario_si_lista(Some(&core));
            if !app.usuarios.guardando() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        assert!(!app.usuarios.guardando());
        let creado = core.buscar_usuarios(&Default::default()).unwrap();
        let persona = creado
            .iter()
            .find(|u| u.cedula == "2001")
            .expect("el usuario debía quedar creado");
        assert_eq!(persona.nombre, "Persona Nueva");
        assert!(core.autenticar("2001", "password2").is_ok());

        std::fs::remove_dir_all(&directorio).ok();
    }

    #[test]
    fn cambiar_password_en_hilo_aparte_actualiza_la_autenticacion_real() {
        let (core, directorio, root_id) = core_temporal("cambiar_password");
        let mut app = App::default();

        app.iniciar_cambio_password(root_id, "password-nuevo".into(), "Ana".into(), Some(&core));
        assert!(app.usuarios.guardando());

        for _ in 0..200 {
            app.recibir_hilo_usuario_si_lista(Some(&core));
            if !app.usuarios.guardando() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        assert!(!app.usuarios.guardando());
        assert!(core.autenticar("ROOT-1", "password-nuevo").is_ok());
        assert!(core.autenticar("ROOT-1", "password1").is_err());

        std::fs::remove_dir_all(&directorio).ok();
    }
}

/// Datos ya validados de un usuario nuevo, a la espera del hash de Argon2 —
/// no incluye `password` en texto plano, que ya se movió al hilo que calcula
/// el hash y no hace falta después.
#[derive(Debug)]
enum HiloUsuarioPendiente {
    Creacion(ReceptorHash, DatosUsuarioPendiente, String),
    CambioPassword(ReceptorHash, i64, String),
}

#[derive(Debug, Clone)]
struct DatosUsuarioPendiente {
    cedula: String,
    nombre: String,
    rol: RolUsuario,
    activo: bool,
}

/// Receptor del hilo aparte que sólo calcula un hash de Argon2 — nunca del resultado
/// final de escribir en SQLite, que ocurre después, en el hilo principal.
type ReceptorHash = std::sync::mpsc::Receiver<Result<String, PasswordError>>;

#[derive(Debug)]
pub struct App {
    vista: Vista,
    login: LoginState,
    menu: MenuPrincipalState,
    configuracion_inicial: ConfiguracionInicialState,
    activos: ActivosState,
    historial: HistorialState,
    contratistas: ContratistasState,
    empresas: EmpresasState,
    usuarios: UsuariosState,
    configuracion: ConfiguracionState,
    nuevo_ingreso: NuevoIngresoState,
    salida_rapida: SalidaRapidaState,
    salir: bool,
    salida: SalidaApp,
    sesion: Option<UsuarioSesion>,
    tema: ThemePreset,
    /// Resultado en camino de un hilo aparte que verifica la contraseña
    /// (Argon2) sin bloquear este bucle. `None` cuando no hay ningún login
    /// en curso.
    autenticacion_pendiente:
        Option<std::sync::mpsc::Receiver<Result<UsuarioSesion, AutenticacionError>>>,
    /// Hash de Argon2 en camino para crear un usuario o cambiar una
    /// contraseña. Un único `Option` en vez de dos campos independientes: la
    /// exclusión mutua entre ambos flujos es estructural (no puede haber
    /// creación y cambio de contraseña en vuelo a la vez), no depende de que
    /// nada valide `UsuariosState::guardando` desde aquí.
    hilo_usuario_pendiente: Option<HiloUsuarioPendiente>,
    /// Hash de Argon2 en camino para crear el usuario ROOT inicial.
    root_inicial_pendiente: Option<(ReceptorHash, SolicitudRoot)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            vista: Vista::Login,
            login: LoginState::default(),
            menu: MenuPrincipalState::default(),
            configuracion_inicial: ConfiguracionInicialState::default(),
            activos: ActivosState::default(),
            historial: HistorialState::default(),
            contratistas: ContratistasState::default(),
            empresas: EmpresasState::default(),
            usuarios: UsuariosState::default(),
            configuracion: ConfiguracionState::default(),
            nuevo_ingreso: NuevoIngresoState::default(),
            salida_rapida: SalidaRapidaState::default(),
            salir: false,
            salida: SalidaApp::Cerrar,
            sesion: None,
            tema: ThemePreset::Brisas,
            autenticacion_pendiente: None,
            hilo_usuario_pendiente: None,
            root_inicial_pendiente: None,
        }
    }
}

impl App {
    pub fn new(requiere_configuracion_inicial: bool, mensaje_inicial: Option<String>) -> Self {
        let mut app = Self {
            vista: if requiere_configuracion_inicial {
                Vista::ConfiguracionInicial
            } else {
                Vista::Login
            },
            ..Self::default()
        };
        if let Some(mensaje) = mensaje_inicial {
            app.login.preset_error(mensaje);
        }
        app
    }

    pub fn run<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<SalidaApp> {
        self.run_internal(terminal, None)
    }

    pub fn run_with_core<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
        core: &AppCore,
    ) -> io::Result<SalidaApp> {
        self.run_internal(terminal, Some(core))
    }

    fn run_internal<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
        core: Option<&AppCore>,
    ) -> io::Result<SalidaApp> {
        while !self.salir {
            let theme = self.tema.theme();
            terminal.draw(|frame| {
                match self.vista {
                    Vista::ConfiguracionInicial => configuracion_inicial::render(
                        frame,
                        frame.area(),
                        &self.configuracion_inicial,
                        theme,
                    ),
                    Vista::Login => login::render(frame, frame.area(), &self.login, theme),
                    Vista::MenuPrincipal => {
                        if let Some(sesion) = &self.sesion {
                            menu_principal::render(frame, frame.area(), &self.menu, sesion, theme)
                        }
                    }
                    Vista::IngresosActivos => {
                        if let Some(sesion) = &self.sesion {
                            activos::render(frame, frame.area(), &self.activos, sesion, theme)
                        }
                    }
                    Vista::Historial => {
                        if let Some(sesion) = &self.sesion {
                            historial::render(frame, frame.area(), &self.historial, sesion, theme)
                        }
                    }
                    Vista::Contratistas => {
                        if let Some(sesion) = &self.sesion {
                            contratistas::render(
                                frame,
                                frame.area(),
                                &self.contratistas,
                                sesion,
                                theme,
                            )
                        }
                    }
                    Vista::Empresas => {
                        if let Some(sesion) = &self.sesion {
                            empresas::render(frame, frame.area(), &self.empresas, sesion, theme)
                        }
                    }
                    Vista::Usuarios => {
                        if let Some(sesion) = &self.sesion {
                            usuarios::render(frame, frame.area(), &self.usuarios, sesion, theme)
                        }
                    }
                    Vista::Configuracion => {
                        configuracion::render(frame, frame.area(), &self.configuracion, theme)
                    }
                    Vista::NuevoIngreso => {
                        if let Some(sesion) = &self.sesion {
                            nuevo_ingreso::render(
                                frame,
                                frame.area(),
                                &self.nuevo_ingreso,
                                sesion,
                                theme,
                            )
                        }
                    }
                }
                salida_rapida::render(frame, frame.area(), &self.salida_rapida, theme);
            })?;

            if event::poll(EVENT_POLL)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.procesar_tecla_global(key, core);
            }

            let ahora = std::time::Instant::now();
            self.configuracion_inicial.tick(ahora);
            self.login.tick(ahora);
            // Sondeo de los 4 hilos de Argon2 en vuelo, siempre en el mismo lugar
            // del bucle (después de leer teclas): login, ROOT inicial, crear
            // usuario/cambiar contraseña.
            self.recibir_autenticacion_si_lista();
            match core {
                Some(core) => {
                    self.procesar_configuracion_pendiente(core);
                    self.recibir_root_inicial_si_lista(core);
                }
                None => self.abortar_configuracion_inicial_sin_core(),
            }
            self.recibir_hilo_usuario_si_lista(core);

            // Búsquedas con debounce: cada pantalla decide si ya pasó el
            // tiempo sin tecla nueva; si no, `tick` devuelve `Ninguna` y el
            // despacho de siempre es un no-op.
            let accion = self.historial.tick(ahora);
            self.procesar_accion_historial(accion, core);
            let accion = self.contratistas.tick(ahora);
            self.procesar_accion_contratistas(accion, core);
            let accion = self.activos.tick(ahora);
            self.procesar_accion_activos(accion, core);
            let accion = self.empresas.tick(ahora);
            self.procesar_accion_empresas(accion, core);
            let accion = self.usuarios.tick(ahora);
            self.procesar_accion_usuarios(accion, core);
            let accion = self.nuevo_ingreso.tick(ahora);
            self.procesar_accion_nuevo_ingreso(accion, core);
            let accion = self.salida_rapida.tick(ahora);
            self.procesar_accion_salida_rapida(accion, core);
        }

        Ok(self.salida.clone())
    }

    /// Revisa sin bloquear si el hilo de verificación de contraseña (Argon2) ya terminó.
    fn recibir_autenticacion_si_lista(&mut self) {
        let Some(receptor) = &self.autenticacion_pendiente else {
            return;
        };
        let Ok(resultado) = receptor.try_recv() else {
            return;
        };
        self.autenticacion_pendiente = None;
        match resultado {
            Ok(sesion) => {
                self.login.completar_validacion(None);
                self.iniciar_sesion(sesion);
            }
            Err(error) => self.login.completar_validacion(Some(error.to_string())),
        }
    }

    /// Resuelve la cédula de inmediato (rápido, sólo SQLite) y, si existe y está activo,
    /// verifica la contraseña en un hilo aparte para no congelar la UI mientras Argon2 calcula.
    fn iniciar_autenticacion(&mut self, cedula: String, password: String, core: Option<&AppCore>) {
        let Some(core) = core else {
            self.login.completar_validacion(None);
            self.iniciar_sesion(UsuarioSesion {
                id: 0,
                cedula: cedula.clone(),
                nombre: cedula,
                rol: RolUsuario::Operador,
            });
            return;
        };
        match core.buscar_candidato_autenticacion(&cedula) {
            Ok(candidato) => {
                let (emisor, receptor) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let resultado = crate::services::autenticacion_service::verificar_candidato(
                        candidato, &password,
                    );
                    let _ = emisor.send(resultado);
                });
                self.autenticacion_pendiente = Some(receptor);
            }
            Err(error) => self.login.completar_validacion(Some(error.to_string())),
        }
    }

    /// Calcula el hash de Argon2 de `password` en un hilo aparte y devuelve el
    /// receptor para sondear el resultado sin bloquear — usado por los 3 flujos
    /// que crean/cambian una credencial (crear usuario, cambiar contraseña,
    /// ROOT inicial).
    fn generar_hash_en_hilo(password: String) -> ReceptorHash {
        let (emisor, receptor) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = emisor.send(crate::services::password::generar_hash(&password));
        });
        receptor
    }

    /// Valida rápido (sólo SQLite) y, si pasa, calcula el hash de Argon2 en un hilo
    /// aparte — la escritura real ocurre después, en el hilo principal, cuando llega.
    fn iniciar_creacion_usuario(
        &mut self,
        input: crate::services::usuario_service::CrearUsuarioInput,
        nombre: String,
        core: Option<&AppCore>,
    ) {
        let Some(core) = core else {
            let recarga = self.usuarios.completar_guardado(
                Err("No se pudo guardar el usuario".into()),
                None,
                &nombre,
            );
            self.procesar_recarga_usuarios(recarga, core);
            return;
        };
        if let Err(error) = core.validar_datos_para_crear_usuario(&input) {
            let recarga =
                self.usuarios
                    .completar_guardado(Err(mensaje_usuario(error)), None, &nombre);
            self.procesar_recarga_usuarios(recarga, Some(core));
            return;
        }
        let datos = DatosUsuarioPendiente {
            cedula: input.cedula,
            nombre: input.nombre,
            rol: input.rol,
            activo: input.activo,
        };
        let receptor = Self::generar_hash_en_hilo(input.password);
        self.hilo_usuario_pendiente = Some(HiloUsuarioPendiente::Creacion(receptor, datos, nombre));
        self.usuarios.marcar_guardando();
    }

    /// Mismo patrón que `iniciar_creacion_usuario`: valida rápido, hashea en un hilo aparte.
    fn iniciar_cambio_password(
        &mut self,
        id: i64,
        password: String,
        nombre: String,
        core: Option<&AppCore>,
    ) {
        let Some(core) = core else {
            self.usuarios
                .completar_password(Err("No se pudo cambiar la contraseña".into()), &nombre);
            return;
        };
        if let Err(error) = core.validar_password_para_cambio(id, &password) {
            self.usuarios
                .completar_password(Err(mensaje_usuario(error)), &nombre);
            return;
        }
        let receptor = Self::generar_hash_en_hilo(password);
        self.hilo_usuario_pendiente =
            Some(HiloUsuarioPendiente::CambioPassword(receptor, id, nombre));
        self.usuarios.marcar_guardando();
    }

    /// Revisa sin bloquear si el hilo de Argon2 de creación de usuario o cambio de
    /// contraseña ya terminó — a lo sumo uno de los dos puede estar en vuelo a la
    /// vez, ver el comentario de `hilo_usuario_pendiente`.
    fn recibir_hilo_usuario_si_lista(&mut self, core: Option<&AppCore>) {
        let receptor = match &self.hilo_usuario_pendiente {
            Some(HiloUsuarioPendiente::Creacion(receptor, ..)) => receptor,
            Some(HiloUsuarioPendiente::CambioPassword(receptor, ..)) => receptor,
            None => return,
        };
        let Ok(resultado_hash) = receptor.try_recv() else {
            return;
        };
        match self.hilo_usuario_pendiente.take() {
            Some(HiloUsuarioPendiente::Creacion(_, datos, nombre)) => {
                let resultado = match resultado_hash {
                    Ok(hash) => core
                        .ok_or_else(|| "No se pudo guardar el usuario".to_owned())
                        .and_then(|core| {
                            core.crear_usuario_con_hash(
                                &datos.cedula,
                                &datos.nombre,
                                datos.rol,
                                datos.activo,
                                hash,
                            )
                            .map(Some)
                            .map_err(mensaje_usuario)
                        }),
                    Err(error) => Err(error.to_string()),
                };
                let recarga = self.usuarios.completar_guardado(resultado, None, &nombre);
                self.procesar_recarga_usuarios(recarga, core);
            }
            Some(HiloUsuarioPendiente::CambioPassword(_, id, nombre)) => {
                let resultado = match resultado_hash {
                    Ok(hash) => core
                        .ok_or_else(|| "No se pudo cambiar la contraseña".to_owned())
                        .and_then(|core| {
                            core.cambiar_password_usuario_con_hash(id, &hash)
                                .map_err(mensaje_usuario)
                        }),
                    Err(error) => Err(error.to_string()),
                };
                self.usuarios.completar_password(resultado, &nombre);
            }
            None => {}
        }
    }

    /// Mismo patrón para el ROOT inicial: valida rápido (sin la comprobación de "ya
    /// existe un ROOT", que sigue siendo atómica con el insert), hashea aparte, y crea
    /// el usuario cuando llega el hash — ver `recibir_root_inicial_si_lista`.
    fn iniciar_root_inicial(&mut self, solicitud: SolicitudRoot, core: &AppCore) {
        if let Err(error) = core.validar_datos_para_root_inicial(&CrearRootInicialInput {
            cedula: solicitud.cedula.clone(),
            nombre: solicitud.nombre.clone(),
            password: solicitud.password.clone(),
        }) {
            self.configuracion_inicial
                .completar_con_error(error.to_string());
            return;
        }
        let receptor = Self::generar_hash_en_hilo(solicitud.password.clone());
        self.root_inicial_pendiente = Some((receptor, solicitud));
    }

    fn recibir_root_inicial_si_lista(&mut self, core: &AppCore) {
        let Some((receptor, ..)) = &self.root_inicial_pendiente else {
            return;
        };
        let Ok(resultado_hash) = receptor.try_recv() else {
            return;
        };
        let Some((_, solicitud)) = self.root_inicial_pendiente.take() else {
            return;
        };
        match resultado_hash {
            Ok(hash) => {
                let input = CrearRootInicialInput {
                    cedula: solicitud.cedula,
                    nombre: solicitud.nombre,
                    password: solicitud.password,
                };
                match core.crear_root_inicial_con_hash(input, hash) {
                    Ok(_) | Err(UsuarioServiceError::ConfiguracionInicialYaRealizada) => {
                        self.configuracion_inicial.limpiar_secretos();
                        self.vista = Vista::Login;
                    }
                    Err(UsuarioServiceError::Database(_)) => self
                        .configuracion_inicial
                        .completar_con_error("No se pudo crear el usuario ROOT"),
                    Err(error) => self
                        .configuracion_inicial
                        .completar_con_error(error.to_string()),
                }
            }
            Err(error) => self
                .configuracion_inicial
                .completar_con_error(error.to_string()),
        }
    }

    #[cfg(test)]
    fn procesar_tecla_vista(&mut self, key: crossterm::event::KeyEvent) {
        self.procesar_tecla_global(key, None);
    }

    /// Fuerza que se dispare cualquier búsqueda con debounce pendiente,
    /// simulando que pasó tiempo de sobra desde la última tecla. Para
    /// pruebas que necesitan el resultado real de una búsqueda sin esperar
    /// el reloj de verdad.
    #[cfg(test)]
    fn agotar_debounce_busquedas(&mut self, core: Option<&AppCore>) {
        let futuro = std::time::Instant::now() + std::time::Duration::from_secs(1);
        let accion = self.historial.tick(futuro);
        self.procesar_accion_historial(accion, core);
        let accion = self.contratistas.tick(futuro);
        self.procesar_accion_contratistas(accion, core);
        let accion = self.activos.tick(futuro);
        self.procesar_accion_activos(accion, core);
        let accion = self.empresas.tick(futuro);
        self.procesar_accion_empresas(accion, core);
        let accion = self.usuarios.tick(futuro);
        self.procesar_accion_usuarios(accion, core);
        let accion = self.nuevo_ingreso.tick(futuro);
        self.procesar_accion_nuevo_ingreso(accion, core);
        let accion = self.salida_rapida.tick(futuro);
        self.procesar_accion_salida_rapida(accion, core);
    }

    /// Comandos transversales (salida de emergencia, tema, salida rápida) que se
    /// resuelven antes de despachar por vista, sin importar cuál esté activa.
    fn procesar_tecla_global(&mut self, key: crossterm::event::KeyEvent, core: Option<&AppCore>) {
        match standard_command(key) {
            Some(StandardCommand::EmergencyExit) => {
                self.finalizar_hilos_pendientes(core);
                self.salir = true;
                return;
            }
            Some(StandardCommand::Theme) => {
                self.tema = self.tema.next();
                return;
            }
            // Requiere sesión iniciada: en Login/ConfiguracionInicial no hay a quién
            // atribuir la salida ni personal "adentro" que buscar todavía.
            Some(StandardCommand::QuickExit)
                if !self.salida_rapida.abierto() && self.sesion.is_some() =>
            {
                let accion = self.salida_rapida.abrir();
                self.procesar_accion_salida_rapida(accion, core);
                return;
            }
            _ => {}
        }
        if self.salida_rapida.abierto() {
            let accion = self.salida_rapida.handle_key(key);
            self.procesar_accion_salida_rapida(accion, core);
            return;
        }
        self.procesar_tecla_vista_con_core(key, core);
    }

    /// Espera (bloqueando, con reintentos cortos) cualquier hilo de Argon2 en vuelo
    /// antes de la salida de emergencia — sin esto, la escritura ya validada se
    /// pierde en silencio porque el bucle principal termina sin volver a sondear el
    /// canal. El login no escribe nada y se abandona sin esperar.
    fn finalizar_hilos_pendientes(&mut self, core: Option<&AppCore>) {
        while self.hilo_usuario_pendiente.is_some() {
            self.recibir_hilo_usuario_si_lista(core);
            if self.hilo_usuario_pendiente.is_some() {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        match core {
            Some(core) => {
                while self.root_inicial_pendiente.is_some() {
                    self.recibir_root_inicial_si_lista(core);
                    if self.root_inicial_pendiente.is_some() {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                }
            }
            None => self.root_inicial_pendiente = None,
        }
    }

    fn procesar_accion_salida_rapida(
        &mut self,
        accion: AccionSalidaRapida,
        core: Option<&AppCore>,
    ) {
        match accion {
            AccionSalidaRapida::Ninguna => {}
            AccionSalidaRapida::Buscar { texto } => {
                let resultado = core
                    .ok_or_else(|| "No se pudieron cargar los ingresos activos".into())
                    .and_then(|c| {
                        c.listar_ingresos_activos(
                            &crate::database::queries::ingresos::FiltroIngresosActivos {
                                texto,
                                ..Default::default()
                            },
                        )
                        .map_err(|_| "No se pudieron cargar los ingresos activos".into())
                    });
                self.salida_rapida.completar_busqueda(resultado);
            }
            AccionSalidaRapida::Confirmar {
                registro_id,
                nombre,
            } => {
                let resultado = match (&self.sesion, core) {
                    (Some(s), _) if s.id == 0 => {
                        Err("La sesión de desarrollo no puede registrar movimientos reales".into())
                    }
                    (Some(s), Some(c)) => c
                        .registrar_salida(registro_id, s.id)
                        .map(|()| format!("✓ Salida registrada — {nombre}"))
                        .map_err(mensaje_salida),
                    _ => Err("No se pudo registrar la salida".into()),
                };
                self.salida_rapida.completar_confirmacion(resultado);
                if self.vista == Vista::IngresosActivos {
                    let recarga = self.activos.solicitud_carga();
                    self.procesar_accion_activos(recarga, core);
                }
            }
        }
    }

    fn procesar_tecla_vista_con_core(
        &mut self,
        key: crossterm::event::KeyEvent,
        core: Option<&AppCore>,
    ) {
        match self.vista {
            Vista::ConfiguracionInicial => {
                if self.configuracion_inicial.handle_key(key) == AccionConfiguracion::Salir {
                    self.salir = true;
                }
            }
            Vista::Login => match self.login.handle_key(key) {
                AccionLogin::Salir => self.salir = true,
                AccionLogin::Autenticar { cedula, password } => {
                    self.iniciar_autenticacion(cedula, password, core)
                }
                AccionLogin::Ninguna => {}
            },
            Vista::MenuPrincipal => self.procesar_accion_menu_con_core(key, core),
            Vista::IngresosActivos => {
                let accion = self.activos.handle_key(key);
                self.procesar_accion_activos(accion, core);
            }
            Vista::Historial => {
                let accion = self.historial.handle_key(key);
                self.procesar_accion_historial(accion, core);
            }
            Vista::Contratistas => {
                let accion = self.contratistas.handle_key(key);
                self.procesar_accion_contratistas(accion, core);
            }
            Vista::Empresas => {
                let accion = self.empresas.handle_key(key);
                self.procesar_accion_empresas(accion, core);
            }
            Vista::Usuarios => {
                let accion = self.usuarios.handle_key(key);
                self.procesar_accion_usuarios(accion, core);
            }
            Vista::Configuracion => {
                let accion = self.configuracion.handle_key(key);
                self.procesar_accion_configuracion(accion, core);
            }
            Vista::NuevoIngreso => {
                let accion = self.nuevo_ingreso.handle_key(key);
                self.procesar_accion_nuevo_ingreso(accion, core);
            }
        }
    }

    pub fn sesion(&self) -> Option<&UsuarioSesion> {
        self.sesion.as_ref()
    }

    #[cfg(test)]
    fn procesar_accion_menu(&mut self, key: crossterm::event::KeyEvent) {
        self.procesar_accion_menu_con_core(key, None);
    }

    fn procesar_accion_menu_con_core(
        &mut self,
        key: crossterm::event::KeyEvent,
        core: Option<&AppCore>,
    ) {
        let rol = self
            .sesion
            .as_ref()
            .map_or(RolUsuario::Operador, |sesion| sesion.rol);
        match self.menu.handle_key(key, rol) {
            AccionMenu::Ninguna => {}
            AccionMenu::Abrir(opcion) => {
                self.menu.seleccion = opcion;
                self.vista = match opcion {
                    OpcionMenu::NuevoIngreso => {
                        // El menú sólo es alcanzable con `self.sesion` ya
                        // establecida (`Vista::MenuPrincipal` no renderiza sin
                        // ella) — este fallback es defensivo, no debería
                        // dispararse nunca en un flujo real.
                        self.nuevo_ingreso = NuevoIngresoState::new();
                        if core.is_some() {
                            self.procesar_accion_nuevo_ingreso(
                                self.nuevo_ingreso.solicitud_carga(),
                                core,
                            );
                        }
                        Vista::NuevoIngreso
                    }
                    OpcionMenu::IngresosActivos => {
                        if let Some(core) = core {
                            self.activos.completar_empresas(
                                core.listar_empresas()
                                    .map_err(|_| "No se pudieron cargar las empresas".into()),
                            );
                            self.procesar_accion_activos(self.activos.solicitud_carga(), Some(core))
                        }
                        Vista::IngresosActivos
                    }
                    OpcionMenu::Historial => {
                        if let Some(core) = core {
                            self.historial.completar_empresas(
                                core.listar_empresas()
                                    .map_err(|_| "No se pudieron cargar las empresas".into()),
                            );
                            let accion = self.historial.solicitud_carga();
                            self.procesar_accion_historial(accion, Some(core));
                        }
                        Vista::Historial
                    }
                    OpcionMenu::Contratistas => {
                        if let Some(core) = core {
                            self.contratistas.completar_empresas(
                                core.listar_empresas()
                                    .map_err(|_| "No se pudieron cargar las empresas".into()),
                            );
                            self.procesar_accion_contratistas(
                                self.contratistas.solicitud_carga(),
                                Some(core),
                            );
                        }
                        Vista::Contratistas
                    }
                    OpcionMenu::Empresas => {
                        if core.is_some() {
                            self.procesar_accion_empresas(self.empresas.solicitar_carga(), core);
                        }
                        Vista::Empresas
                    }
                    OpcionMenu::Usuarios => {
                        if core.is_some() {
                            self.procesar_accion_usuarios(self.usuarios.solicitud_carga(), core);
                        }
                        Vista::Usuarios
                    }
                    OpcionMenu::Configuracion => {
                        self.configuracion = ConfiguracionState::default();
                        Vista::Configuracion
                    }
                    OpcionMenu::CerrarSesion | OpcionMenu::Salir => Vista::MenuPrincipal,
                };
            }
            AccionMenu::CerrarSesion => {
                self.sesion = None;
                self.login.reiniciar();
                self.vista = Vista::Login;
            }
            AccionMenu::Salir => self.salir = true,
        }
    }

    fn procesar_accion_empresas(&mut self, accion: AccionEmpresas, core: Option<&AppCore>) {
        match accion {
            AccionEmpresas::Ninguna => {}
            AccionEmpresas::Volver => self.vista = Vista::MenuPrincipal,
            AccionEmpresas::Buscar {
                texto,
                seleccionar_id,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cargar la base de empresas".to_owned())
                    .and_then(|core| {
                        core.buscar_empresas(&crate::database::queries::empresas::FiltroEmpresas {
                            texto,
                            ..Default::default()
                        })
                        .map_err(|_| "No se pudo cargar la base de empresas".to_owned())
                    });
                self.empresas.completar_busqueda(resultado, seleccionar_id);
            }
            AccionEmpresas::Crear { nombre } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar la empresa".to_owned())
                    .and_then(|core| core.crear_empresa(&nombre).map_err(mensaje_empresa));
                let recarga = self.empresas.completar_creacion(resultado, &nombre);
                if !matches!(recarga, AccionEmpresas::Ninguna) {
                    self.procesar_accion_empresas(recarga, core);
                }
            }
            AccionEmpresas::Actualizar { id, nombre } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar la empresa".to_owned())
                    .and_then(|core| {
                        core.actualizar_empresa(id, &nombre)
                            .map_err(mensaje_empresa)
                    });
                let recarga = self
                    .empresas
                    .completar_actualizacion(resultado, id, &nombre);
                if !matches!(recarga, AccionEmpresas::Ninguna) {
                    self.procesar_accion_empresas(recarga, core);
                }
            }
            AccionEmpresas::EstablecerActivo {
                id,
                activar,
                nombre,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo actualizar el estado de la empresa".into())
                    .and_then(|core| {
                        if activar {
                            core.activar_empresa(id)
                        } else {
                            core.desactivar_empresa(id)
                        }
                        .map_err(mensaje_empresa)
                    });
                let recarga = self
                    .empresas
                    .completar_estado(resultado, id, activar, &nombre);
                if !matches!(recarga, AccionEmpresas::Ninguna) {
                    self.procesar_accion_empresas(recarga, core);
                }
            }
        }
    }

    fn procesar_accion_contratistas(&mut self, accion: AccionContratistas, core: Option<&AppCore>) {
        match accion {
            AccionContratistas::Ninguna => {}
            AccionContratistas::Volver => self.vista = Vista::MenuPrincipal,
            AccionContratistas::Buscar {
                texto,
                seleccionar_id,
                empresa_id,
                tipos,
                praind,
                personal_ruta,
                tiene_acceso,
                offset,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cargar la base de contratistas".into())
                    .and_then(|core| {
                        core.buscar_contratistas(
                            &crate::database::queries::contratistas::FiltroContratistas {
                                texto,
                                empresa_id,
                                tipos_incluidos: tipos,
                                praind,
                                personal_ruta,
                                tiene_acceso,
                                offset,
                                ..Default::default()
                            },
                        )
                        .map_err(|_| "No se pudo cargar la base de contratistas".into())
                    });
                self.contratistas
                    .completar_busqueda(resultado, seleccionar_id);
            }
            AccionContratistas::Crear { datos, nombre } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar el contratista".into())
                    .and_then(|core| {
                        core.crear_contratista(datos)
                            .map(Some)
                            .map_err(mensaje_contratista)
                    });
                let recarga = self
                    .contratistas
                    .completar_guardado(resultado, None, &nombre);
                if !matches!(recarga, AccionContratistas::Ninguna) {
                    self.procesar_accion_contratistas(recarga, core);
                }
            }
            AccionContratistas::Actualizar { id, datos, nombre } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar el contratista".into())
                    .and_then(|core| {
                        core.actualizar_contratista(id, datos)
                            .map(|_| None)
                            .map_err(mensaje_contratista)
                    });
                let recarga = self
                    .contratistas
                    .completar_guardado(resultado, Some(id), &nombre);
                if !matches!(recarga, AccionContratistas::Ninguna) {
                    self.procesar_accion_contratistas(recarga, core);
                }
            }
        }
    }

    fn procesar_accion_usuarios(&mut self, accion: AccionUsuarios, core: Option<&AppCore>) {
        match accion {
            AccionUsuarios::Ninguna => {}
            AccionUsuarios::Volver => self.vista = Vista::MenuPrincipal,
            AccionUsuarios::Buscar {
                texto,
                seleccionar_id,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cargar la base de usuarios".into())
                    .and_then(|core| {
                        core.buscar_usuarios(&crate::database::queries::usuarios::FiltroUsuarios {
                            texto,
                            ..Default::default()
                        })
                        .map_err(|_| "No se pudo cargar la base de usuarios".into())
                    });
                self.usuarios.completar_busqueda(resultado, seleccionar_id);
            }
            AccionUsuarios::Crear { input, nombre } => {
                self.iniciar_creacion_usuario(input, nombre, core)
            }
            AccionUsuarios::Actualizar {
                id,
                input,
                activo,
                nombre,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar el usuario".into())
                    .and_then(|core| {
                        core.actualizar_usuario(id, input, activo)
                            .map_err(mensaje_usuario)
                    })
                    .map(|_| None);
                let recarga = self
                    .usuarios
                    .completar_guardado(resultado, Some(id), &nombre);
                self.procesar_recarga_usuarios(recarga, core);
                self.actualizar_sesion_desde_tabla(id);
            }
            AccionUsuarios::CambiarPassword {
                id,
                password,
                nombre,
            } => self.iniciar_cambio_password(id, password, nombre, core),
            AccionUsuarios::EstablecerActivo {
                id,
                activar,
                nombre,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo actualizar el estado del usuario".into())
                    .and_then(|core| {
                        if activar {
                            core.activar_usuario(id)
                        } else {
                            core.desactivar_usuario(id)
                        }
                        .map_err(mensaje_usuario)
                    });
                let recarga = self
                    .usuarios
                    .completar_estado(resultado, id, activar, &nombre);
                self.procesar_recarga_usuarios(recarga, core);
            }
        }
    }

    fn procesar_recarga_usuarios(&mut self, accion: AccionUsuarios, core: Option<&AppCore>) {
        if !matches!(accion, AccionUsuarios::Ninguna) {
            self.procesar_accion_usuarios(accion, core);
        }
    }

    fn procesar_accion_configuracion(&mut self, accion: AccionAjustes, core: Option<&AppCore>) {
        match accion {
            AccionAjustes::Ninguna => {}
            AccionAjustes::Volver => self.vista = Vista::MenuPrincipal,
            AccionAjustes::Respaldos(accion) => self.procesar_accion_respaldos(accion, core),
        }
    }

    fn procesar_accion_respaldos(&mut self, accion: AccionRespaldos, core: Option<&AppCore>) {
        match accion {
            AccionRespaldos::Ninguna | AccionRespaldos::Volver => {}
            AccionRespaldos::Cargar => {
                let resultado = core
                    .ok_or_else(|| "No se pudieron listar los respaldos".to_owned())
                    .and_then(|core| core.listar_respaldos().map_err(|error| error.to_string()));
                self.configuracion.completar_listado(resultado);
            }
            AccionRespaldos::Crear => {
                let resultado = core
                    .ok_or_else(|| "No se pudo crear el respaldo".to_owned())
                    .and_then(|core| {
                        core.crear_respaldo(crate::database::backup::TipoRespaldo::Manual)
                            .map_err(|error| error.to_string())
                    });
                self.configuracion.completar_creacion(resultado);
            }
            AccionRespaldos::Revalidar { ruta } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo validar el respaldo".to_owned())
                    .and_then(|core| {
                        core.validar_respaldo(&ruta)
                            .map_err(|error| error.to_string())
                    });
                self.configuracion.completar_validacion(&ruta, resultado);
            }
            AccionRespaldos::Exportar { ruta, destino } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo exportar el respaldo".to_owned())
                    .and_then(|core| {
                        core.exportar_respaldo(&ruta, &destino)
                            .map_err(|error| error.to_string())
                    });
                self.configuracion
                    .completar_exportacion(resultado, &destino);
            }
            AccionRespaldos::Restaurar { ruta } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo respaldar la base antes de restaurar".to_owned())
                    .and_then(|core| {
                        core.crear_respaldo(crate::database::backup::TipoRespaldo::PreRestauracion)
                            .map_err(|error| error.to_string())
                    });
                match resultado {
                    Ok(_) => {
                        self.salida = SalidaApp::Restaurar { candidata: ruta };
                        self.salir = true;
                    }
                    Err(error) => self.configuracion.completar_creacion(Err(error)),
                }
            }
        }
    }

    fn actualizar_sesion_desde_tabla(&mut self, id: i64) {
        let Some(sesion) = &mut self.sesion else {
            return;
        };
        if sesion.id != id {
            return;
        }
        if let Some(usuario) = self.usuarios.resumen_por_id(id) {
            sesion.cedula = usuario.cedula.clone();
            sesion.nombre = usuario.nombre.clone();
            sesion.rol = usuario.rol;
        }
    }

    fn procesar_accion_nuevo_ingreso(
        &mut self,
        accion: AccionNuevoIngreso,
        core: Option<&AppCore>,
    ) {
        match accion {
            AccionNuevoIngreso::Ninguna => {}
            AccionNuevoIngreso::Volver => self.vista = Vista::MenuPrincipal,
            AccionNuevoIngreso::Buscar { texto } => {
                let r = core
                    .ok_or_else(|| "No se pudieron cargar los contratistas".into())
                    .and_then(|c| {
                        c.buscar_contratistas(
                            &crate::database::queries::contratistas::FiltroContratistas {
                                texto,
                                ..Default::default()
                            },
                        )
                        .map_err(|_| "No se pudieron cargar los contratistas".into())
                    });
                self.nuevo_ingreso.completar_busqueda(r);
            }
            AccionNuevoIngreso::Preparar { contratista_id } => {
                let r = core
                    .ok_or_else(|| "No se pudo preparar el ingreso".into())
                    .and_then(|c| c.preparar_ingreso(contratista_id).map_err(mensaje_ingreso));
                self.nuevo_ingreso.completar_preparacion(r);
            }
            AccionNuevoIngreso::Registrar {
                contratista_id,
                medio,
                gafete,
            } => {
                let resultado = match (&self.sesion, core) {
                    (Some(s), _) if s.id == 0 => {
                        Err("La sesión de desarrollo no puede registrar movimientos reales".into())
                    }
                    (Some(s), Some(c)) => c
                        .registrar_ingreso(contratista_id, medio, gafete, s.id)
                        .map(|r| r.registro_id)
                        .map_err(mensaje_ingreso),
                    _ => Err("No se pudo registrar el ingreso".into()),
                };
                if self.nuevo_ingreso.completar_registro(resultado) {
                    self.activos.filtro.clear();
                    self.procesar_accion_activos(self.activos.buscar(None), core);
                    self.vista = Vista::IngresosActivos;
                }
            }
        }
    }

    fn procesar_accion_activos(&mut self, accion: AccionActivos, core: Option<&AppCore>) {
        match accion {
            AccionActivos::Ninguna => {}
            AccionActivos::Volver => self.vista = Vista::MenuPrincipal,
            AccionActivos::Buscar {
                texto,
                seleccionar_id,
                empresa_id,
                tipos,
                gafete,
                medio,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudieron cargar los ingresos activos".into())
                    .and_then(|c| {
                        c.listar_ingresos_activos(
                            &crate::database::queries::ingresos::FiltroIngresosActivos {
                                texto,
                                empresa_id,
                                tipos_incluidos: tipos,
                                gafete_numero: gafete,
                                medio_ingreso: medio,
                                ..Default::default()
                            },
                        )
                        .map_err(|_| "No se pudieron cargar los ingresos activos".into())
                    });
                self.activos.completar_busqueda(resultado, seleccionar_id);
            }
            AccionActivos::RegistrarSalida {
                registro_id,
                nombre,
            } => {
                let resultado = match (&self.sesion, core) {
                    (Some(s), _) if s.id == 0 => {
                        Err("La sesión de desarrollo no puede registrar movimientos reales".into())
                    }
                    (Some(s), Some(c)) => c
                        .registrar_salida(registro_id, s.id)
                        .map_err(mensaje_salida),
                    _ => Err("No se pudo registrar la salida".into()),
                };
                let recarga = self
                    .activos
                    .completar_salida(resultado, registro_id, &nombre);
                self.procesar_accion_activos(recarga, core);
            }
        }
    }

    fn procesar_accion_historial(&mut self, accion: AccionHistorial, core: Option<&AppCore>) {
        match accion {
            AccionHistorial::Ninguna => {}
            AccionHistorial::Volver => self.vista = Vista::MenuPrincipal,
            AccionHistorial::Consultar(filtro) => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cargar el historial".into())
                    .and_then(|core| {
                        core.buscar_historial(&filtro)
                            .map_err(|_| "No se pudo cargar el historial".into())
                    });
                self.historial.completar(resultado);
            }
        }
    }

    fn iniciar_sesion(&mut self, sesion: UsuarioSesion) {
        self.sesion = Some(sesion);
        self.menu.nueva_sesion();
        self.vista = Vista::MenuPrincipal;
    }

    /// Contraparte de `procesar_configuracion_pendiente` cuando no hay `core`
    /// (`App::run`, sin base de datos): sin esto, un ROOT inicial enviado se
    /// queda para siempre en "Creando" — `EstadoConfiguracion::Creando`
    /// bloquea hasta el `Esc` porque nadie vuelve a tomar la solicitud pendiente.
    fn abortar_configuracion_inicial_sin_core(&mut self) {
        if self.vista != Vista::ConfiguracionInicial {
            return;
        }
        if self.configuracion_inicial.tomar_solicitud().is_some() {
            self.configuracion_inicial
                .completar_con_error("No se pudo crear el usuario ROOT");
        }
    }

    fn procesar_configuracion_pendiente(&mut self, core: &AppCore) {
        if self.vista != Vista::ConfiguracionInicial {
            return;
        }
        let Some(solicitud) = self.configuracion_inicial.tomar_solicitud() else {
            return;
        };
        self.iniciar_root_inicial(solicitud, core);
    }
}

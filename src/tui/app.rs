use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{Terminal, backend::Backend};

use crate::application::AppCore;
use crate::models::usuario::RolUsuario;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::UsuarioServiceError;
use crate::services::usuario_service::CrearRootInicialInput;

use super::{
    activos::{self, AccionActivos, ActivosState},
    configuracion_inicial::{self, AccionConfiguracion, ConfiguracionInicialState},
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
    NuevoIngreso,
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
        let mut app = App::new(true);
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
                    Ok(vec![
                        crate::database::queries::contratistas::ContratistaResumen {
                            id: 1,
                            empresa_id: 1,
                            cedula: "1".into(),
                            nombre: "Contratista".into(),
                            empresa_nombre: "Empresa".into(),
                            tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::Swat,
                            fecha_vencimiento_praind: None,
                            es_personal_ruta: false,
                            tiene_acceso: true,
                        },
                    ]),
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
        app.nuevo_ingreso.completar_busqueda(Ok(vec![
            crate::database::queries::contratistas::ContratistaResumen {
                id: 1,
                empresa_id: 1,
                cedula: "1".into(),
                nombre: "Persona".into(),
                empresa_nombre: "Empresa".into(),
                tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::Swat,
                fecha_vencimiento_praind: None,
                es_personal_ruta: false,
                tiene_acceso: true,
            },
        ]));
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
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].empresa_id, empresa_id);
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

        app.procesar_tecla_global(tecla(KeyCode::Char(' ')), Some(&core));
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
}

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
    nuevo_ingreso: NuevoIngresoState,
    salida_rapida: SalidaRapidaState,
    salir: bool,
    sesion: Option<UsuarioSesion>,
    tema: ThemePreset,
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
            nuevo_ingreso: NuevoIngresoState::default(),
            salida_rapida: SalidaRapidaState::default(),
            salir: false,
            sesion: None,
            tema: ThemePreset::Brisas,
        }
    }
}

impl App {
    pub fn new(requiere_configuracion_inicial: bool) -> Self {
        Self {
            vista: if requiere_configuracion_inicial {
                Vista::ConfiguracionInicial
            } else {
                Vista::Login
            },
            ..Self::default()
        }
    }

    pub fn run<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<()> {
        self.run_internal(terminal, None)
    }

    pub fn run_with_core<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
        core: &AppCore,
    ) -> io::Result<()> {
        self.run_internal(terminal, Some(core))
    }

    fn run_internal<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
        core: Option<&AppCore>,
    ) -> io::Result<()> {
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
                        activos::render(frame, frame.area(), &self.activos, theme)
                    }
                    Vista::Historial => {
                        historial::render(frame, frame.area(), &self.historial, theme)
                    }
                    Vista::Contratistas => {
                        contratistas::render(frame, frame.area(), &self.contratistas, theme)
                    }
                    Vista::Empresas => empresas::render(frame, frame.area(), &self.empresas, theme),
                    Vista::Usuarios => usuarios::render(frame, frame.area(), &self.usuarios, theme),
                    Vista::NuevoIngreso => {
                        nuevo_ingreso::render(frame, frame.area(), &self.nuevo_ingreso, theme)
                    }
                }
                salida_rapida::render(frame, frame.area(), &self.salida_rapida, theme);
            })?;

            if let Some(core) = core {
                self.procesar_configuracion_pendiente(core);
            }

            if event::poll(EVENT_POLL)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.procesar_tecla_global(key, core);
            }

            let ahora = std::time::Instant::now();
            self.configuracion_inicial.tick(ahora);
            self.login.tick(ahora);
            if let Some((cedula, password)) = self.login.credenciales_si_validacion_lista(ahora) {
                if let Some(core) = core {
                    match core.autenticar(&cedula, &password) {
                        Ok(sesion) => {
                            self.login.completar_validacion(None);
                            self.iniciar_sesion(sesion);
                        }
                        Err(error) => self.login.completar_validacion(Some(error.to_string())),
                    }
                } else {
                    self.login.completar_validacion(None);
                    self.iniciar_sesion(UsuarioSesion {
                        id: 0,
                        cedula: cedula.clone(),
                        nombre: cedula,
                        rol: RolUsuario::Operador,
                    });
                }
            }
        }

        Ok(())
    }

    #[cfg(test)]
    fn procesar_tecla_vista(&mut self, key: crossterm::event::KeyEvent) {
        self.procesar_tecla_global(key, None);
    }

    /// Comandos transversales (salida de emergencia, tema, salida rápida) que se
    /// resuelven antes de despachar por vista, sin importar cuál esté activa.
    fn procesar_tecla_global(&mut self, key: crossterm::event::KeyEvent, core: Option<&AppCore>) {
        match standard_command(key) {
            Some(StandardCommand::EmergencyExit) => {
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

    fn procesar_accion_salida_rapida(&mut self, accion: AccionSalidaRapida, core: Option<&AppCore>) {
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
                        .map(|pagina| pagina.items)
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
            Vista::Login => {
                if self.login.handle_key(key) == AccionLogin::Salir {
                    self.salir = true;
                }
            }
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
        match self.menu.handle_key(key) {
            AccionMenu::Ninguna => {}
            AccionMenu::Abrir(opcion) => {
                self.menu.seleccion = opcion;
                self.vista = match opcion {
                    OpcionMenu::NuevoIngreso => {
                        let usuario = self
                            .sesion
                            .as_ref()
                            .map_or("Quintana", |s| s.nombre.as_str());
                        self.nuevo_ingreso = NuevoIngresoState::new(usuario);
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
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar el usuario".into())
                    .and_then(|core| core.crear_usuario(input).map(Some).map_err(mensaje_usuario));
                let recarga = self.usuarios.completar_guardado(resultado, None, &nombre);
                self.procesar_recarga_usuarios(recarga, core);
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
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cambiar la contraseña".into())
                    .and_then(|core| {
                        core.cambiar_password_usuario(id, &password)
                            .map_err(mensaje_usuario)
                    });
                self.usuarios.completar_password(resultado, &nombre);
            }
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
            self.usuarios.set_usuario_nombre(sesion.nombre.clone());
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
        self.contratistas.set_usuario_nombre(sesion.nombre.clone());
        self.empresas.set_usuario_nombre(sesion.nombre.clone());
        self.usuarios.set_usuario_nombre(sesion.nombre.clone());
        self.historial.set_usuario_nombre(sesion.nombre.clone());
        self.sesion = Some(sesion);
        self.menu.nueva_sesion();
        self.vista = Vista::MenuPrincipal;
    }

    fn procesar_configuracion_pendiente(&mut self, core: &AppCore) {
        if self.vista != Vista::ConfiguracionInicial {
            return;
        }
        let Some(solicitud) = self.configuracion_inicial.tomar_solicitud() else {
            return;
        };
        match core.crear_root_inicial(CrearRootInicialInput {
            cedula: solicitud.cedula,
            nombre: solicitud.nombre,
            password: solicitud.password,
        }) {
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
}

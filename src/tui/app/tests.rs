use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusqlite::Connection;

use super::*;
use crate::database::schema::initialize_database;
use crate::services::usuario_service::CrearRootInicialInput;
use crate::tui::activos::AccionActivos;
use crate::tui::contratistas::AccionContratistas;
use crate::tui::empresas::AccionEmpresas;
use crate::tui::nuevo_ingreso::AccionNuevoIngreso;
use crate::tui::usuarios::AccionUsuarios;

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

fn crear_root_real(core: &AppCore) -> UsuarioSesion {
    core.crear_root_inicial(crate::services::usuario_service::CrearRootInicialInput {
        cedula: "ROOT-TEST".into(),
        nombre: "Root de prueba".into(),
        password: "password-test".into(),
    })
    .unwrap();
    core.autenticar("ROOT-TEST", "password-test").unwrap()
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
    assert_eq!(app.tema, ThemePreset::Negro);

    app.procesar_tecla_vista(tecla(KeyCode::F(7)));
    assert_eq!(app.tema, ThemePreset::Classic);

    app.procesar_tecla_vista(tecla(KeyCode::F(7)));
    assert_eq!(app.tema, ThemePreset::Brisas);
}

/// El Menú Principal y las pestañas son modos excluyentes atados al tema:
/// sólo Negro usa pestañas. Con sesión activa, entrar a Negro debe saltar
/// directo a la primera pestaña (el Menú no es alcanzable ahí), y salir de
/// Negro debe volver al Menú con la selección sincronizada a la pantalla
/// que se estaba viendo.
#[test]
fn f7_hacia_negro_salta_del_menu_a_la_primera_pestana_y_de_vuelta_sincroniza_la_seleccion() {
    let mut app = App {
        vista: Vista::MenuPrincipal,
        sesion: Some(sesion("Root")),
        ..App::default()
    };
    assert_eq!(app.tema, ThemePreset::Brisas);

    app.procesar_tecla_vista(tecla(KeyCode::F(7)));
    assert_eq!(app.tema, ThemePreset::Negro);
    assert_eq!(app.vista, Vista::NuevoIngreso, "primera pestaña visible");

    app.procesar_tecla_global(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL), None);
    assert_eq!(
        app.vista,
        Vista::IngresosActivos,
        "se movió dentro de Negro"
    );

    app.procesar_tecla_vista(tecla(KeyCode::F(7)));
    assert_eq!(app.tema, ThemePreset::Classic);
    assert_eq!(app.vista, Vista::MenuPrincipal, "Classic no tiene pestañas");
    assert_eq!(
        app.menu.seleccion,
        OpcionMenu::IngresosActivos,
        "la selección del menú queda sincronizada con lo último visto"
    );
}

/// Con Esc en Cambiar contraseña ("Volver"), el destino depende del tema:
/// en Negro no hay Menú al que volver, así que va a la primera pestaña; en
/// Classic/Brisas vuelve al Menú de siempre.
#[test]
fn volver_desde_cambiar_password_depende_del_tema() {
    let mut app = App {
        vista: Vista::CambiarPassword,
        sesion: Some(sesion("Root")),
        tema: ThemePreset::Negro,
        ..App::default()
    };
    app.procesar_tecla_vista(tecla(KeyCode::Esc));
    assert_eq!(app.vista, Vista::NuevoIngreso);

    let mut app = App {
        vista: Vista::CambiarPassword,
        sesion: Some(sesion("Root")),
        tema: ThemePreset::Classic,
        ..App::default()
    };
    app.procesar_tecla_vista(tecla(KeyCode::Esc));
    assert_eq!(app.vista, Vista::MenuPrincipal);
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

/// Atajo sin documentar (a propósito, no aparece en ninguna ayuda F1):
/// Ctrl+1..Ctrl+9 saltan directo a la pantalla correspondiente desde
/// cualquier vista, sin pasar por el menú principal primero.
#[test]
fn ctrl_numero_salta_directo_a_la_pantalla_sin_pasar_por_el_menu() {
    let mut app = App {
        vista: Vista::Contratistas,
        sesion: Some(sesion("Ana")),
        tema: ThemePreset::Negro,
        ..App::default()
    };

    app.procesar_tecla_global(
        KeyEvent::new(KeyCode::Char('3'), KeyModifiers::CONTROL),
        None,
    );
    assert_eq!(app.vista, Vista::Historial);
}

/// Reusa la misma tabla y el mismo chequeo de rol que
/// `MenuPrincipalState::handle_key` — un Operador no puede saltar a
/// Usuarios con Ctrl+6, igual que no puede con un '6' suelto parado en
/// el menú.
#[test]
fn ctrl_numero_respeta_el_rol_igual_que_el_menu() {
    let mut app = App {
        vista: Vista::NuevoIngreso,
        sesion: Some(UsuarioSesion {
            id: 1,
            cedula: "1".into(),
            nombre: "Operador".into(),
            rol: crate::models::usuario::RolUsuario::Operador,
        }),
        tema: ThemePreset::Negro,
        ..App::default()
    };

    app.procesar_tecla_global(
        KeyEvent::new(KeyCode::Char('6'), KeyModifiers::CONTROL),
        None,
    );
    assert_eq!(app.vista, Vista::NuevoIngreso);
}

#[test]
fn ctrl_numero_no_funciona_sin_sesion_ni_con_ctrl_alt_ni_con_salida_rapida_abierta() {
    let mut app = App {
        vista: Vista::Login,
        sesion: None,
        ..App::default()
    };
    app.procesar_tecla_global(
        KeyEvent::new(KeyCode::Char('3'), KeyModifiers::CONTROL),
        None,
    );
    assert_eq!(app.vista, Vista::Login);

    let mut app = App {
        vista: Vista::Contratistas,
        sesion: Some(sesion("Ana")),
        tema: ThemePreset::Negro,
        ..App::default()
    };
    // Ctrl+Alt+número queda reservado (Windows Terminal lo usa para
    // cambiar de pestaña) — el atajo sólo reacciona a Ctrl solo.
    app.procesar_tecla_global(
        KeyEvent::new(
            KeyCode::Char('3'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ),
        None,
    );
    assert_eq!(app.vista, Vista::Contratistas);

    app.salida_rapida.abrir();
    app.procesar_tecla_global(
        KeyEvent::new(KeyCode::Char('3'), KeyModifiers::CONTROL),
        None,
    );
    assert_eq!(
        app.vista,
        Vista::Contratistas,
        "no debe saltar con el overlay de salida rápida abierto"
    );
}

/// Un dígito suelto (sin Ctrl) sigue siendo texto libre normal en
/// cualquier campo de búsqueda — el atajo global no debe robárselo.
#[test]
fn digito_suelto_sin_ctrl_no_dispara_el_salto() {
    let mut app = App {
        vista: Vista::Contratistas,
        sesion: Some(sesion("Ana")),
        tema: ThemePreset::Negro,
        ..App::default()
    };
    app.procesar_tecla_global(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE), None);
    assert_eq!(app.vista, Vista::Contratistas);
}

#[test]
fn ctrl_flechas_recorrer_pestanas_envuelve_y_respeta_el_rol() {
    let mut root = App {
        vista: Vista::NuevoIngreso,
        sesion: Some(sesion("Root")),
        tema: ThemePreset::Negro,
        ..App::default()
    };
    root.procesar_tecla_global(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL), None);
    assert_eq!(root.vista, Vista::CambiarPassword);
    root.procesar_tecla_global(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL), None);
    assert_eq!(root.vista, Vista::NuevoIngreso);

    let mut operador = App {
        vista: Vista::Empresas,
        sesion: Some(UsuarioSesion {
            id: 2,
            cedula: "2".into(),
            nombre: "Operador".into(),
            rol: RolUsuario::Operador,
        }),
        tema: ThemePreset::Negro,
        ..App::default()
    };
    operador.procesar_tecla_global(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL), None);
    assert_eq!(operador.vista, Vista::CambiarPassword);
}

#[test]
fn cambiar_de_pestana_conserva_el_estado_de_la_pantalla() {
    let mut app = App {
        vista: Vista::Contratistas,
        sesion: Some(sesion("Root")),
        tema: ThemePreset::Negro,
        ..App::default()
    };
    app.pestanas_visitadas[OpcionMenu::Contratistas.indice_pestana().unwrap()] = true;
    app.procesar_tecla_global(tecla(KeyCode::Char('/')), None);
    for caracter in "filtro vivo".chars() {
        app.procesar_tecla_global(tecla(KeyCode::Char(caracter)), None);
    }
    let antes = format!("{:?}", app.contratistas);

    app.procesar_tecla_global(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL), None);
    assert_eq!(app.vista, Vista::Empresas);
    app.procesar_tecla_global(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL), None);

    assert_eq!(app.vista, Vista::Contratistas);
    assert_eq!(format!("{:?}", app.contratistas), antes);
}

#[test]
fn ctrl_flechas_no_salen_del_inicio_ni_atraviesan_el_overlay() {
    let mut app = App {
        vista: Vista::MenuPrincipal,
        sesion: Some(sesion("Root")),
        ..App::default()
    };
    app.procesar_tecla_global(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL), None);
    assert_eq!(app.vista, Vista::MenuPrincipal);

    app.vista = Vista::Historial;
    app.salida_rapida.abrir();
    app.procesar_tecla_global(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL), None);
    assert_eq!(app.vista, Vista::Historial);
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
// Matriz de navegación que garantiza el mismo contrato de Escape en todas las
// vistas; dividirla duplicaría la preparación y debilitaría la comparación.
#[allow(clippy::too_many_lines)]
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
                                resultado_registrado: crate::models::registro_ingreso::ResultadoIngresoRegistrado::Permitido,
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
                        tiene_ingreso_activo: false,
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
                tiene_ingreso_activo: false,
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
            gafetes_deuda: Vec::new(),
        },
    ));
    app.procesar_tecla_vista(tecla(KeyCode::Esc));
    assert_eq!(app.vista, Vista::NuevoIngreso);
    app.procesar_tecla_vista(tecla(KeyCode::Esc));
    assert_eq!(app.vista, Vista::MenuPrincipal);
}

#[test]
fn login_exitoso_entra_directo_al_menu_con_nuevo_ingreso_seleccionado() {
    let mut app = App::default();
    app.menu.seleccion = OpcionMenu::Usuarios;
    app.iniciar_sesion(sesion("Daniel Quintana"), None);
    // La elección de entorno es explícita y persistente: "Modo comandos" en
    // el Menú, o `/clasico` en comandos. Login entra directo a operar.
    assert_eq!(app.vista, Vista::MenuPrincipal);
    assert_eq!(app.menu.seleccion, OpcionMenu::NuevoIngreso);
    assert_eq!(app.sesion().unwrap().nombre, "Daniel Quintana");
}

#[test]
fn modo_cli_desde_menu_pide_reinicio() {
    let mut app = App {
        vista: Vista::MenuPrincipal,
        sesion: Some(sesion("Daniel Quintana")),
        ..App::default()
    };
    app.iniciar_sesion(sesion("Daniel Quintana"), None);

    app.procesar_tecla_vista(tecla(KeyCode::Char('m')));
    assert!(!app.salir);
    app.procesar_tecla_vista(tecla(KeyCode::Enter));

    assert!(app.salir);
    assert_eq!(app.salida, SalidaApp::ReiniciarEnCli);
}

#[test]
fn empresas_tui_crea_busca_edita_y_recarga_desde_appcore_real() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let core = AppCore::new(connection);
    let actor = crear_root_real(&core);
    let mut app = App {
        vista: Vista::MenuPrincipal,
        sesion: Some(actor),
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
    let actor = crear_root_real(&core);
    core.crear_empresa(&actor, "Constructora Álvarez").unwrap();
    core.crear_empresa(&actor, "Servicios Hernández").unwrap();
    let mut app = App {
        vista: Vista::MenuPrincipal,
        sesion: Some(actor),
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
// Escenario de integración TUI/AppCore: crear, editar y buscar deben ocurrir
// sobre la misma conexión y el mismo estado de pantalla.
#[allow(clippy::too_many_lines)]
fn contratistas_tui_carga_empresas_crea_edita_y_busca_con_appcore_real() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let core = AppCore::new(connection);
    let actor = crear_root_real(&core);
    let empresa_id = core.crear_empresa(&actor, "Constructora Álvarez").unwrap();
    let mut app = App {
        vista: Vista::MenuPrincipal,
        sesion: Some(actor),
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
    for _ in 0.."001-ABC".chars().count() {
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Backspace), Some(&core))
    }
    for c in "009-XYZ".chars() {
        app.procesar_tecla_vista_con_core(tecla(KeyCode::Char(c)), Some(&core))
    }
    app.procesar_tecla_vista_con_core(tecla(KeyCode::Tab), Some(&core));
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
    assert!(
        core.buscar_contratistas(
            &crate::database::queries::contratistas::FiltroContratistas {
                texto: Some("001-ABC".into()),
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
                texto: Some("009-XYZ".into()),
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
    let actor = core.autenticar("ROOT-1", "password1").unwrap();
    core.crear_usuario(
        &actor,
        crate::services::usuario_service::CrearUsuarioInput {
            cedula: "ROOT-2".into(),
            nombre: "Respaldo".into(),
            password: "password2".into(),
            rol: RolUsuario::Root,
            activo: true,
        },
    )
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
    connection
        .execute(
            "INSERT INTO gafetes (numero, estado) VALUES (77, 'DISPONIBLE')",
            [],
        )
        .unwrap();
    let core = AppCore::new(connection);
    let usuario_id = core
        .crear_root_inicial(crate::services::usuario_service::CrearRootInicialInput {
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            password: "password1".into(),
        })
        .unwrap();
    let actor = core.autenticar("ROOT-1", "password1").unwrap();
    let empresa_id = core.crear_empresa(&actor, "Constructora Álvarez").unwrap();
    let contratista_id = core
        .crear_contratista(
            &actor,
            crate::services::contratista_service::DatosContratista {
                cedula: "9-9999-9999".into(),
                nombre: "Persona De Prueba".into(),
                empresa_id,
                tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::PorCorreo,
                fecha_vencimiento_praind: None,
                es_personal_ruta: false,
                tiene_acceso: true,
            },
        )
        .unwrap();
    core.registrar_ingreso(
        &actor,
        contratista_id,
        crate::models::medio_ingreso::MedioIngreso::Caminando,
        Some(77),
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
    // El primer Enter sólo pide confirmar (`Estado::ConfirmarSalida`);
    // todavía no toca SQLite.
    app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));
    assert!(app.salida_rapida.abierto());
    let antes = core
        .listar_ingresos_activos(&crate::database::queries::ingresos::FiltroIngresosActivos {
            texto: None,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(antes.total, 1);

    // El segundo Enter (sobre la confirmación) sí registra la salida real.
    app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));
    let restantes = core
        .listar_ingresos_activos(&crate::database::queries::ingresos::FiltroIngresosActivos {
            texto: None,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(restantes.total, 0);

    // El overlay vuelve a quedar abierto (no cerrado) mostrando la lista
    // recargada de quienes siguen dentro — regresión de "tras sacar a
    // alguien no se ven los demás contratistas".
    assert!(app.salida_rapida.abierto());

    app.procesar_tecla_global(tecla(KeyCode::Esc), Some(&core));
    assert!(!app.salida_rapida.abierto());
}

/// Regresión de "registrar una salida por F2 no actualiza en tiempo
/// real otras pantallas": antes sólo se recargaba Ingresos Activos si
/// era la vista actual — quedándose en Historial, un movimiento recién
/// cerrado seguía mostrándose como abierto hasta navegar a otra
/// pantalla y volver.
#[test]
fn f2_registra_salida_refresca_historial_sin_navegar() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO gafetes (numero, estado) VALUES (77, 'DISPONIBLE')",
            [],
        )
        .unwrap();
    let core = AppCore::new(connection);
    let usuario_id = core
        .crear_root_inicial(crate::services::usuario_service::CrearRootInicialInput {
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            password: "password1".into(),
        })
        .unwrap();
    let actor = core.autenticar("ROOT-1", "password1").unwrap();
    let empresa_id = core.crear_empresa(&actor, "Constructora Álvarez").unwrap();
    let contratista_id = core
        .crear_contratista(
            &actor,
            crate::services::contratista_service::DatosContratista {
                cedula: "9-9999-9999".into(),
                nombre: "Persona De Prueba".into(),
                empresa_id,
                tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::PorCorreo,
                fecha_vencimiento_praind: None,
                es_personal_ruta: false,
                tiene_acceso: true,
            },
        )
        .unwrap();
    core.registrar_ingreso(
        &actor,
        contratista_id,
        crate::models::medio_ingreso::MedioIngreso::Caminando,
        Some(77),
    )
    .unwrap();

    let mut app = App {
        vista: Vista::Historial,
        sesion: Some(UsuarioSesion {
            id: usuario_id,
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            rol: RolUsuario::Root,
        }),
        ..App::default()
    };
    let carga = app.historial.solicitud_carga();
    app.procesar_accion_historial(carga, Some(&core));
    for c in "estado:activos".chars() {
        app.historial
            .handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    let futuro = std::time::Instant::now() + std::time::Duration::from_millis(300);
    let accion = app.historial.tick(futuro);
    app.procesar_accion_historial(accion, Some(&core));
    assert_eq!(app.historial.total(), 1, "debía ver el movimiento abierto");

    app.procesar_tecla_global(tecla(KeyCode::F(2)), Some(&core));
    for c in "77".chars() {
        app.procesar_tecla_global(tecla(KeyCode::Char(c)), Some(&core));
    }
    app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));
    app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));

    assert_eq!(app.vista, Vista::Historial, "no debía moverse de pantalla");
    assert_eq!(
        app.historial.total(),
        0,
        "Historial debía refrescarse solo, sin navegar, tras la salida por F2"
    );
}

/// Regresión de "registrar un ingreso interrumpe el flujo saltando a
/// Ingresos Activos": antes cada registro exitoso cambiaba
/// `self.vista` a `Vista::IngresosActivos`, obligando a volver a
/// navegar para registrar al siguiente contratista. Ahora se queda en
/// Nuevo Ingreso — el ingreso sí quedó real en SQLite.
#[test]
fn registrar_ingreso_se_queda_en_nuevo_ingreso_en_vez_de_saltar_a_activos() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO gafetes (numero, estado) VALUES (77, 'DISPONIBLE')",
            [],
        )
        .unwrap();
    let core = AppCore::new(connection);
    let usuario_id = core
        .crear_root_inicial(crate::services::usuario_service::CrearRootInicialInput {
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            password: "password1".into(),
        })
        .unwrap();
    let actor = core.autenticar("ROOT-1", "password1").unwrap();
    let empresa_id = core.crear_empresa(&actor, "Constructora Álvarez").unwrap();
    let contratista_id = core
        .crear_contratista(
            &actor,
            crate::services::contratista_service::DatosContratista {
                cedula: "9-9999-9999".into(),
                nombre: "Persona De Prueba".into(),
                empresa_id,
                tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::PorCorreo,
                fecha_vencimiento_praind: None,
                es_personal_ruta: false,
                tiene_acceso: true,
            },
        )
        .unwrap();

    let mut app = App {
        vista: Vista::NuevoIngreso,
        sesion: Some(UsuarioSesion {
            id: usuario_id,
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            rol: RolUsuario::Root,
        }),
        ..App::default()
    };

    app.procesar_accion_nuevo_ingreso(
        AccionNuevoIngreso::Registrar {
            contratista_id,
            medio: crate::models::medio_ingreso::MedioIngreso::Caminando,
            gafete: Some(77),
        },
        Some(&core),
    );

    assert_eq!(app.vista, Vista::NuevoIngreso);
    let activos = core
        .listar_ingresos_activos(&crate::database::queries::ingresos::FiltroIngresosActivos {
            texto: None,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(activos.total, 1, "el ingreso sí debía quedar registrado");
}

/// Regresión puntual del reporte de usuario: con varios contratistas
/// filtrados en pantalla, registrar a uno no debía vaciar la lista de
/// los demás ni mandar al menú principal con un ESC — el operador
/// tiene que poder seguir con el siguiente sin volver a escribir la
/// búsqueda.
#[test]
fn registrar_no_vacia_la_lista_ni_manda_al_menu_con_mas_por_procesar() {
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
    let actor = core.autenticar("ROOT-1", "password1").unwrap();
    let empresa_id = core.crear_empresa(&actor, "Constructora Álvarez").unwrap();
    for (cedula, nombre) in [("1001", "Persona Uno"), ("1002", "Persona Dos")] {
        core.crear_contratista(
            &actor,
            crate::services::contratista_service::DatosContratista {
                cedula: cedula.into(),
                nombre: nombre.into(),
                empresa_id,
                tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::PorCorreo,
                fecha_vencimiento_praind: None,
                es_personal_ruta: false,
                tiene_acceso: true,
            },
        )
        .unwrap();
    }

    let mut app = App {
        vista: Vista::NuevoIngreso,
        sesion: Some(UsuarioSesion {
            id: usuario_id,
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            rol: RolUsuario::Root,
        }),
        ..App::default()
    };

    app.procesar_tecla_global(tecla(KeyCode::Char('/')), Some(&core));
    for c in "Persona".chars() {
        app.procesar_tecla_global(tecla(KeyCode::Char(c)), Some(&core));
    }
    let futuro = std::time::Instant::now() + std::time::Duration::from_millis(300);
    let accion = app.nuevo_ingreso.tick(futuro);
    app.procesar_accion_nuevo_ingreso(accion, Some(&core));
    assert_eq!(app.nuevo_ingreso.cantidad(), 2);

    // Selecciona al resaltado, completa el gafete (PorCorreo lo exige)
    // y registra.
    app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));
    app.procesar_tecla_global(tecla(KeyCode::Tab), Some(&core));
    for c in "50".chars() {
        app.procesar_tecla_global(tecla(KeyCode::Char(c)), Some(&core));
    }
    app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));

    assert_eq!(
        app.nuevo_ingreso.cantidad(),
        2,
        "los dos deben seguir visibles; el registrado ahora con ingreso activo"
    );

    app.procesar_tecla_global(tecla(KeyCode::Esc), Some(&core));
    assert_eq!(
        app.vista,
        Vista::NuevoIngreso,
        "ESC con filtro activo debía limpiar el filtro, no salir al menú"
    );
}

/// Regresión: "en Historial actualiza cuando saco a alguien, pero en
/// Nuevo Ingreso no" — sacar a alguien por F2 mientras el operador
/// tiene una búsqueda abierta en Nuevo Ingreso debía refrescar
/// `tiene_ingreso_activo` en la fila ya visible, no dejarla mostrando
/// "DENTRO" después de que la persona ya salió.
#[test]
// Regresión integrada: la salida y el refresco de la búsqueda visible son dos
// mitades del mismo comportamiento y necesitan compartir el estado real.
#[allow(clippy::too_many_lines)]
fn f2_registra_salida_refresca_nuevo_ingreso_sin_navegar() {
    use crate::tui::ui_kit::ThemePreset;
    use ratatui::{Terminal, backend::TestBackend};

    fn texto(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO gafetes (numero, estado) VALUES (77, 'DISPONIBLE')",
            [],
        )
        .unwrap();
    let core = AppCore::new(connection);
    let usuario_id = core
        .crear_root_inicial(crate::services::usuario_service::CrearRootInicialInput {
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            password: "password1".into(),
        })
        .unwrap();
    let actor = core.autenticar("ROOT-1", "password1").unwrap();
    let empresa_id = core.crear_empresa(&actor, "Constructora Álvarez").unwrap();
    let contratista_id = core
        .crear_contratista(
            &actor,
            crate::services::contratista_service::DatosContratista {
                cedula: "9-9999-9999".into(),
                nombre: "Persona De Prueba".into(),
                empresa_id,
                tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::PorCorreo,
                fecha_vencimiento_praind: None,
                es_personal_ruta: false,
                tiene_acceso: true,
            },
        )
        .unwrap();
    core.registrar_ingreso(
        &actor,
        contratista_id,
        crate::models::medio_ingreso::MedioIngreso::Caminando,
        Some(77),
    )
    .unwrap();

    let mut app = App {
        vista: Vista::NuevoIngreso,
        sesion: Some(UsuarioSesion {
            id: usuario_id,
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            rol: RolUsuario::Root,
        }),
        ..App::default()
    };
    let theme = ThemePreset::Brisas.theme();
    let sesion = app.sesion().unwrap().clone();

    app.procesar_tecla_global(tecla(KeyCode::Char('/')), Some(&core));
    for c in "Persona".chars() {
        app.procesar_tecla_global(tecla(KeyCode::Char(c)), Some(&core));
    }
    let futuro = std::time::Instant::now() + std::time::Duration::from_millis(300);
    let accion = app.nuevo_ingreso.tick(futuro);
    app.procesar_accion_nuevo_ingreso(accion, Some(&core));
    assert_eq!(app.nuevo_ingreso.cantidad(), 1);

    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    terminal
        .draw(|frame| {
            nuevo_ingreso::render(frame, frame.area(), &app.nuevo_ingreso, &sesion, theme)
        })
        .unwrap();
    assert!(
        texto(&terminal).contains("DENTRO · tiene un ingreso activo"),
        "antes de la salida debía verse como dentro"
    );

    // Saca a la misma persona por F2, sin navegar de Nuevo Ingreso.
    app.procesar_tecla_global(tecla(KeyCode::F(2)), Some(&core));
    for c in "77".chars() {
        app.procesar_tecla_global(tecla(KeyCode::Char(c)), Some(&core));
    }
    app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));
    app.procesar_tecla_global(tecla(KeyCode::Enter), Some(&core));
    app.procesar_tecla_global(tecla(KeyCode::Esc), Some(&core));

    assert_eq!(
        app.vista,
        Vista::NuevoIngreso,
        "no debía moverse de pantalla"
    );
    terminal
        .draw(|frame| {
            nuevo_ingreso::render(frame, frame.area(), &app.nuevo_ingreso, &sesion, theme)
        })
        .unwrap();
    let despues = texto(&terminal);
    assert!(
        despues.contains("FUERA · sin ingreso activo"),
        "debía reflejar la salida sin que el operador navegara a otra pantalla: {despues}"
    );
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
    let actor = core.autenticar("ROOT-1", "password1").unwrap();
    let respaldo = core
        .crear_respaldo(&actor, crate::database::backup::TipoRespaldo::Manual)
        .unwrap();

    let mut app = App {
        vista: Vista::Respaldos,
        sesion: Some(actor),
        ..App::default()
    };
    // Carga la lista real desde el AppCore de archivo.
    let accion = app.configuracion.reiniciar();
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

fn core_temporal(nombre: &str) -> (AppCore, std::path::PathBuf, UsuarioSesion) {
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
    let actor = core.autenticar("ROOT-1", "password1").unwrap();
    assert_eq!(actor.id, root_id);
    (core, directorio, actor)
}

#[test]
fn crear_usuario_en_hilo_aparte_termina_creado_en_sqlite_con_el_hash_correcto() {
    let (core, directorio, actor) = core_temporal("crear_usuario");
    let mut app = App {
        sesion: Some(actor.clone()),
        ..App::default()
    };

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
    let creado = core
        .buscar_usuarios(
            &actor,
            &crate::database::queries::usuarios::FiltroUsuarios::default(),
        )
        .unwrap();
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
    let (core, directorio, actor) = core_temporal("cambiar_password");
    let objetivo = core
        .crear_usuario(
            &actor,
            crate::services::usuario_service::CrearUsuarioInput {
                cedula: "USR-RESET".into(),
                nombre: "Usuario".into(),
                password: "password1".into(),
                rol: RolUsuario::Operador,
                activo: true,
            },
        )
        .unwrap();
    let mut app = App {
        sesion: Some(actor),
        ..App::default()
    };

    app.iniciar_cambio_password(
        objetivo,
        "password-nuevo".into(),
        "Usuario".into(),
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
    assert!(core.autenticar("USR-RESET", "password-nuevo").is_ok());
    assert!(core.autenticar("USR-RESET", "password1").is_err());

    std::fs::remove_dir_all(&directorio).ok();
}

#[test]
fn crear_respaldo_manual_en_hilo_aparte_termina_creado_en_disco() {
    let (core, directorio, actor) = core_temporal("crear_respaldo_manual");
    let mut app = App {
        sesion: Some(actor.clone()),
        vista: Vista::Respaldos,
        ..App::default()
    };

    app.iniciar_creacion_respaldo_manual(Some(&core));
    assert!(app.configuracion.creando_respaldo());

    for _ in 0..200 {
        app.recibir_respaldo_manual_si_listo();
        if !app.configuracion.creando_respaldo() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert!(!app.configuracion.creando_respaldo());
    let listado = core.listar_respaldos(&actor).unwrap();
    assert_eq!(listado.len(), 1);
    assert_eq!(
        listado[0].tipo,
        crate::database::backup::TipoRespaldo::Manual
    );
    assert!(listado[0].ruta.exists());

    std::fs::remove_dir_all(&directorio).ok();
}

#[test]
fn exportar_historial_en_hilo_aparte_termina_con_el_archivo_real_en_disco() {
    use crate::database::queries::ingresos::FiltroHistorial;
    use crate::historial::ColumnaHistorial;
    use chrono::TimeZone;

    let (core, directorio, actor) = core_temporal("exportar_historial");
    let mut app = App {
        sesion: Some(actor),
        vista: Vista::Historial,
        ..App::default()
    };
    let destino = directorio.join("export.xlsx");
    let filtro = FiltroHistorial::nuevo(
        chrono::Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
        chrono::Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap(),
    );

    app.iniciar_exportacion_historial(
        filtro,
        ColumnaHistorial::ALL.to_vec(),
        destino.clone(),
        Some(&core),
    );
    assert!(app.historial.exportando());

    for _ in 0..200 {
        app.recibir_exportacion_historial_si_lista();
        if !app.historial.exportando() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert!(!app.historial.exportando());
    assert!(destino.exists(), "el archivo real debía quedar en disco");

    std::fs::remove_dir_all(&directorio).ok();
}

/// Regresión del hallazgo #5 de `docs/auditoria-dominio-2026-08-20.md`:
/// "El login diferido puede aceptar credenciales revocadas". La cuenta se
/// desactiva *después* de arrancar el hilo de Argon2 pero *antes* de que
/// termine — `desactivar_usuario` corre en el hilo principal, antes de
/// cualquier sondeo del canal, así que la carrera queda determinista sin
/// depender de cuánto tarde Argon2 realmente.
#[test]
fn login_rechaza_una_cuenta_desactivada_mientras_argon2_verificaba() {
    let (core, directorio, actor) = core_temporal("login_revalidacion");
    let id = core
        .crear_usuario(
            &actor,
            crate::services::usuario_service::CrearUsuarioInput {
                cedula: "3001".into(),
                nombre: "Operador".into(),
                password: "password3".into(),
                rol: RolUsuario::Operador,
                activo: true,
            },
        )
        .unwrap();

    let mut app = App::default();
    app.iniciar_autenticacion("3001".into(), "password3".into(), Some(&core));
    assert!(app.autenticacion_pendiente.is_some());

    core.desactivar_usuario(&actor, id).unwrap();

    for _ in 0..200 {
        app.recibir_autenticacion_si_lista(Some(&core));
        if app.autenticacion_pendiente.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert!(
        app.autenticacion_pendiente.is_none(),
        "el hilo no terminó a tiempo"
    );
    assert!(
        app.sesion().is_none(),
        "no debe iniciar sesión con una cuenta desactivada durante la verificación"
    );
    assert!(matches!(
        app.login.estado(),
        crate::tui::login::EstadoLogin::Error(_)
    ));

    std::fs::remove_dir_all(&directorio).ok();
}

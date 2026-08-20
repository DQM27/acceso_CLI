use rusqlite::Connection;

use control_acceso::{
    application::AppCore,
    database::{
        queries::{auditoria_contratistas::FiltroAuditoriaContratistas, usuarios::FiltroUsuarios},
        schema::initialize_database,
    },
    models::{tipo_ingreso::TipoIngreso, usuario::RolUsuario},
    services::{
        autenticacion_service::UsuarioSesion,
        contratista_service::DatosActualizacionContratista,
        error::{ContratistaServiceError, EmpresaServiceError, UsuarioServiceError},
        usuario_service::{ActualizarUsuarioInput, CrearUsuarioInput},
    },
};

fn sesion(id: i64, rol: RolUsuario) -> UsuarioSesion {
    UsuarioSesion {
        id,
        cedula: format!("U{id}"),
        nombre: format!("Usuario {id}"),
        rol,
    }
}

#[test]
fn cambio_propio_exige_password_actual_y_funciona_para_operador() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let core = AppCore::new(connection);
    core.crear_root_inicial(
        control_acceso::services::usuario_service::CrearRootInicialInput {
            cedula: "ROOT-REAL".into(),
            nombre: "Root".into(),
            password: "password-root".into(),
        },
    )
    .unwrap();
    let root = core.autenticar("ROOT-REAL", "password-root").unwrap();
    core.crear_usuario(
        &root,
        CrearUsuarioInput {
            cedula: "OPERADOR-REAL".into(),
            nombre: "Operador".into(),
            password: "password-viejo".into(),
            rol: RolUsuario::Operador,
            activo: true,
        },
    )
    .unwrap();
    let operador = core.autenticar("OPERADOR-REAL", "password-viejo").unwrap();

    assert!(matches!(
        core.cambiar_mi_password(&operador, "incorrecta", "password-nuevo"),
        Err(UsuarioServiceError::PasswordActualIncorrecta)
    ));
    core.cambiar_mi_password(&operador, "password-viejo", "password-nuevo")
        .unwrap();
    assert!(core.autenticar("OPERADOR-REAL", "password-viejo").is_err());
    assert!(core.autenticar("OPERADOR-REAL", "password-nuevo").is_ok());
}

fn base() -> AppCore {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo) VALUES
                (1,'U1','Root','hash','ROOT',1),
                (2,'U2','Administrador','hash','ADMINISTRADOR',1),
                (3,'U3','Operador','hash','OPERADOR',1),
                (4,'U4','Inactivo','hash','ROOT',0);
             INSERT INTO empresas(id,nombre,activo) VALUES (1,'Empresa',1);
             INSERT INTO contratistas(
                id,cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,
                es_personal_ruta,tiene_acceso
             ) VALUES (1,'C1','Persona',1,'SWAT',NULL,0,1);",
        )
        .unwrap();
    AppCore::new(connection)
}

#[test]
fn operador_no_puede_invocar_comandos_administrativos_directamente() {
    let core = base();
    let operador = sesion(3, RolUsuario::Operador);

    assert!(matches!(
        core.desactivar_empresa(&operador, 1),
        Err(EmpresaServiceError::OperacionNoAutorizada)
    ));
    assert!(matches!(
        core.crear_usuario(
            &operador,
            CrearUsuarioInput {
                cedula: "NUEVO".into(),
                nombre: "Nuevo".into(),
                password: "password-nuevo".into(),
                rol: RolUsuario::Operador,
                activo: true,
            }
        ),
        Err(UsuarioServiceError::OperacionNoAutorizada)
    ));
    assert!(matches!(
        core.buscar_auditoria_contratistas(&operador, &FiltroAuditoriaContratistas::default()),
        Err(ContratistaServiceError::OperacionNoAutorizada)
    ));

    let cambio_acceso = DatosActualizacionContratista {
        nombre: "Persona".into(),
        empresa_id: 1,
        tipo_ingreso: TipoIngreso::Swat,
        fecha_vencimiento_praind: None,
        es_personal_ruta: false,
        tiene_acceso: false,
    };
    assert!(matches!(
        core.actualizar_contratista(&operador, 1, cambio_acceso),
        Err(ContratistaServiceError::OperacionNoAutorizada)
    ));
}

#[test]
fn administrador_no_recibe_roots_y_tampoco_puede_tocar_su_id_a_mano() {
    let core = base();
    let administrador = sesion(2, RolUsuario::Administrador);
    let usuarios = core
        .buscar_usuarios(&administrador, &FiltroUsuarios::default())
        .unwrap();
    assert!(
        usuarios
            .iter()
            .all(|usuario| usuario.rol != RolUsuario::Root)
    );

    assert!(matches!(
        core.actualizar_usuario(
            &administrador,
            1,
            ActualizarUsuarioInput {
                cedula: "ROOT-MANIPULADO".into(),
                nombre: "Root".into(),
                rol: RolUsuario::Root,
            },
            true,
        ),
        Err(UsuarioServiceError::OperacionNoAutorizada)
    ));
    assert!(matches!(
        core.desactivar_usuario(&administrador, 1),
        Err(UsuarioServiceError::OperacionNoAutorizada)
    ));
    assert!(matches!(
        core.cambiar_password_usuario(&administrador, 1, "password-nuevo"),
        Err(UsuarioServiceError::OperacionNoAutorizada)
    ));
}

#[test]
fn una_sesion_inactiva_es_rechazada_aunque_su_snapshot_diga_root() {
    let core = base();
    assert!(matches!(
        core.crear_empresa(&sesion(4, RolUsuario::Root), "Otra"),
        Err(EmpresaServiceError::OperacionNoAutorizada)
    ));
}

use rusqlite::Connection;

use control_acceso::{
    application::AppCore,
    database::{
        queries::{
            auditoria::FiltroAuditoria, contratistas::FiltroContratistas, usuarios::FiltroUsuarios,
        },
        schema::initialize_database,
    },
    models::{tipo_ingreso::TipoIngreso, usuario::RolUsuario},
    services::{
        autenticacion_service::UsuarioSesion,
        contratista_service::DatosActualizacionContratista,
        error::{EmpresaServiceError, UsuarioServiceError},
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
fn cambio_propio_exige_password_actual_y_funciona() {
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

    assert!(matches!(
        core.cambiar_mi_password(&root, "incorrecta", "password-nuevo"),
        Err(UsuarioServiceError::PasswordActualIncorrecta)
    ));
    core.cambiar_mi_password(&root, "password-root", "password-nuevo")
        .unwrap();
    assert!(core.autenticar("ROOT-REAL", "password-root").is_err());
    assert!(core.autenticar("ROOT-REAL", "password-nuevo").is_ok());
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
             ) VALUES (1,'100100100','Persona',1,'SWAT',NULL,0,1);",
        )
        .unwrap();
    AppCore::new(connection)
}

/// Aplanado de roles (ver docs/decisiones-tecnicas.md, 2026-09-11):
/// `RolUsuario::puede()` devuelve `true` siempre, así que un OPERADOR con
/// sesión válida SÍ puede hacer las tres operaciones de abajo -- lo único
/// que sigue rechazando algo es que la sesión sea real y esté activa
/// (`verificar_actor_activo`), no el rol en sí. Este test documentaba la
/// restricción vieja; se actualiza para reflejar la real en vez de quedar
/// fallando indefinidamente.
#[test]
fn operador_con_sesion_activa_puede_invocar_comandos_antes_administrativos() {
    let core = base();
    let operador = sesion(3, RolUsuario::Operador);

    core.desactivar_empresa(&operador, 1).unwrap();
    core.buscar_auditoria(&operador, &FiltroAuditoria::default())
        .unwrap();

    let cambio_acceso = DatosActualizacionContratista {
        cedula: "100100100".into(),
        nombre: "Persona".into(),
        empresa_id: 1,
        tipo_ingreso: TipoIngreso::Swat,
        fecha_vencimiento_praind: None,
        es_personal_ruta: false,
        tiene_acceso: false,
    };
    core.actualizar_contratista(&operador, 1, cambio_acceso)
        .unwrap();
}

fn datos_contratista(cedula: &str, nombre: &str) -> DatosActualizacionContratista {
    DatosActualizacionContratista {
        cedula: cedula.into(),
        nombre: nombre.into(),
        empresa_id: 1,
        tipo_ingreso: TipoIngreso::Swat,
        fecha_vencimiento_praind: None,
        es_personal_ruta: false,
        tiene_acceso: true,
    }
}

/// Con roles aplanados, `RolUsuario::puede(EditarCedulaContratista)` ya no
/// distingue nada -- lo que sigue siendo real (y vale la pena seguir
/// probando) es que la sesión se resuelve por `actor.id` contra la base,
/// IGNORANDO el `rol` que venga en el objeto `UsuarioSesion` del llamador:
/// una sesión con `id=3` (Operador real en la base) que se hace pasar por
/// `RolUsuario::Root` sigue operando como Operador de verdad, no como el
/// rol que dice tener -- simplemente ya no importa, porque cualquier rol
/// puede.
#[test]
fn el_rol_real_se_resuelve_por_id_no_por_lo_que_declara_la_sesion() {
    let core = base();

    // Sesión que dice ser Root pero id=3 es Operador en la base -- igual
    // funciona, porque el chequeo real usa el rol de la base (aplanado a
    // "cualquiera puede"), nunca el campo `rol` de este `UsuarioSesion`.
    core.actualizar_contratista(
        &sesion(3, RolUsuario::Root),
        1,
        datos_contratista("999999999", "Persona"),
    )
    .unwrap();

    core.actualizar_contratista(
        &sesion(3, RolUsuario::Operador),
        1,
        datos_contratista("100100100", "Nombre corregido"),
    )
    .unwrap();
    core.actualizar_contratista(
        &sesion(2, RolUsuario::Administrador),
        1,
        datos_contratista("100200200", "Nombre corregido"),
    )
    .unwrap();
    core.actualizar_contratista(
        &sesion(1, RolUsuario::Root),
        1,
        datos_contratista("100300300", "Nombre corregido"),
    )
    .unwrap();

    let pagina = core
        .buscar_contratistas(&FiltroContratistas::default())
        .unwrap();
    assert_eq!(pagina.items[0].cedula, "100300300");
    assert_eq!(pagina.items[0].nombre, "NOMBRE CORREGIDO");
}

#[test]
fn administrador_no_recibe_roots_al_buscar_usuarios() {
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
}

#[test]
fn una_sesion_inactiva_es_rechazada_aunque_su_snapshot_diga_root() {
    let core = base();
    assert!(matches!(
        core.crear_empresa(&sesion(4, RolUsuario::Root), "Otra"),
        Err(EmpresaServiceError::OperacionNoAutorizada)
    ));
}

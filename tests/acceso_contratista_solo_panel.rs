//! Desde el escritorio nadie da ni quita el acceso a un contratista: eso es
//! sólo del panel web (decisión del dueño 2026-10-24). El escritorio guarda
//! con `AppCore::actualizar_datos_contratista`, que conserva el acceso que
//! ya tenía sin importar lo que llegue.

use rusqlite::Connection;

use control_acceso::{
    application::AppCore,
    database::{queries::contratistas::FiltroContratistas, schema::initialize_database},
    models::{tipo_ingreso::TipoIngreso, usuario::RolUsuario},
    services::{
        autenticacion_service::UsuarioSesion, contratista_service::DatosActualizacionContratista,
    },
};

fn administrador() -> UsuarioSesion {
    UsuarioSesion {
        id: 1,
        cedula: "U1".into(),
        nombre: "Administrador".into(),
        rol: RolUsuario::Administrador,
    }
}

/// Dos contratistas: el 1 con acceso y el 2 sin acceso.
fn base() -> AppCore {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo) VALUES
                (1,'U1','Administrador','hash','ADMINISTRADOR',1);
             INSERT INTO empresas(id,nombre,activo) VALUES (1,'Empresa',1);
             INSERT INTO contratistas(
                id,cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,
                es_personal_ruta,tiene_acceso
             ) VALUES
                (1,'100100100','Con acceso',1,'SWAT',NULL,0,1),
                (2,'200200200','Sin acceso',1,'SWAT',NULL,0,0);",
        )
        .unwrap();
    AppCore::new(connection)
}

fn datos(cedula: &str, nombre: &str, tiene_acceso: bool) -> DatosActualizacionContratista {
    DatosActualizacionContratista {
        cedula: cedula.into(),
        nombre: nombre.into(),
        empresa_id: 1,
        tipo_ingreso: TipoIngreso::Swat,
        fecha_vencimiento_praind: None,
        es_personal_ruta: false,
        tiene_acceso,
    }
}

/// (nombre, tiene acceso) del contratista con ese id.
fn estado(core: &AppCore, id: i64) -> (String, bool) {
    let pagina = core
        .buscar_contratistas(&FiltroContratistas::default())
        .unwrap();
    let fila = pagina.items.into_iter().find(|c| c.id == id).unwrap();
    (fila.nombre, fila.tiene_acceso)
}

#[test]
fn guardar_desde_el_escritorio_no_quita_el_acceso() {
    let core = base();
    core.actualizar_datos_contratista(
        &administrador(),
        1,
        datos("100100100", "Nombre corregido", false),
    )
    .unwrap();
    // El nombre sí se guarda (el núcleo lo pasa a mayúsculas); el acceso no.
    assert_eq!(estado(&core, 1), ("NOMBRE CORREGIDO".into(), true));
}

#[test]
fn guardar_desde_el_escritorio_no_devuelve_el_acceso() {
    let core = base();
    core.actualizar_datos_contratista(&administrador(), 2, datos("200200200", "Sin acceso", true))
        .unwrap();
    assert_eq!(estado(&core, 2), ("SIN ACCESO".into(), false));
}

#[test]
fn el_camino_general_si_cambia_el_acceso() {
    // `actualizar_contratista` (lo que no es el escritorio) sigue igual.
    let core = base();
    core.actualizar_contratista(&administrador(), 1, datos("100100100", "Con acceso", false))
        .unwrap();
    assert!(!estado(&core, 1).1);
}

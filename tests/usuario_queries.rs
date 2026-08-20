use rusqlite::Connection;

use control_acceso::database::queries::usuarios::{
    FiltroUsuarios, SqliteUsuariosQuery, UsuariosQuery,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::usuario_service::UsuarioConsultaService;

fn base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
}

fn insertar(connection: &Connection, cedula: &str, nombre: &str, rol: &str, activo: bool) -> i64 {
    connection.execute("INSERT INTO usuarios(cedula,nombre,password_hash,rol,activo) VALUES (?1,?2,'hash-secreto',?3,?4)", rusqlite::params![cedula, nombre, rol, activo as i64]).unwrap();
    connection.last_insert_rowid()
}

#[test]
fn resumen_devuelve_campos_seguros_roles_y_ambos_estados() {
    let connection = base();
    let root = insertar(&connection, "1", "Ana", "ROOT", true);
    insertar(&connection, "2", "Beto", "ADMINISTRADOR", false);
    insertar(&connection, "3", "Caro", "OPERADOR", true);
    let items = SqliteUsuariosQuery::new(&connection)
        .buscar(&FiltroUsuarios::default())
        .unwrap();

    assert_eq!(items.len(), 3);
    assert_eq!(items[0].id, root);
    assert_eq!(items[0].cedula, "1");
    assert_eq!(items[0].nombre, "Ana");
    assert_eq!(items[0].rol, RolUsuario::Root);
    assert!(items[0].activo);
    assert_eq!(items[1].rol, RolUsuario::Administrador);
    assert!(!items[1].activo);
    assert_eq!(items[2].rol, RolUsuario::Operador);
}

#[test]
fn busca_por_cedula_y_nombre_parcial_case_insensitive_con_trim() {
    let connection = base();
    insertar(&connection, "ABC-1001", "MARÍA MORA", "OPERADOR", true);
    let query = SqliteUsuariosQuery::new(&connection);
    for texto in ["  abc-100  ", "  mora  "] {
        let items = query
            .buscar(&FiltroUsuarios {
                texto: Some(texto.into()),
                ..FiltroUsuarios::default()
            })
            .unwrap();
        assert_eq!(items.len(), 1);
    }
}

/// Regresión de "Búsquedas de 1-2 caracteres no pliegan tildes ni Ñ"
/// (`docs/hallazgos-buscador.md`): "ri" (2 caracteres) no aparece como
/// subcadena literal en "María" (tiene í, no i) — sólo matchea tras plegar
/// diacríticos vía la función SQL `PLEGAR`.
#[test]
fn busqueda_corta_pliega_tildes() {
    let connection = base();
    insertar(&connection, "1", "María Mora", "OPERADOR", true);
    let items = SqliteUsuariosQuery::new(&connection)
        .buscar(&FiltroUsuarios {
            texto: Some("ri".into()),
            ..FiltroUsuarios::default()
        })
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].nombre, "María Mora");
}

#[test]
fn texto_vacio_equivale_a_sin_filtro() {
    let connection = base();
    insertar(&connection, "1", "Ana", "OPERADOR", true);
    insertar(&connection, "2", "Beto", "OPERADOR", false);
    let query = SqliteUsuariosQuery::new(&connection);
    let todos = query.buscar(&FiltroUsuarios::default()).unwrap();
    let blancos = query
        .buscar(&FiltroUsuarios {
            texto: Some("   ".into()),
            ..FiltroUsuarios::default()
        })
        .unwrap();
    assert_eq!(todos, blancos);
}

#[test]
fn limit_offset_y_orden_nombre_id_son_estables() {
    let connection = base();
    let primero = insertar(&connection, "1", "ana", "OPERADOR", true);
    let segundo = insertar(&connection, "2", "Ana", "OPERADOR", true);
    insertar(&connection, "3", "Beto", "OPERADOR", true);
    let query = SqliteUsuariosQuery::new(&connection);
    let pagina_uno = query
        .buscar(&FiltroUsuarios {
            texto: None,
            limite: 1,
            offset: 0,
        })
        .unwrap();
    let pagina_dos = query
        .buscar(&FiltroUsuarios {
            texto: None,
            limite: 1,
            offset: 1,
        })
        .unwrap();
    assert_eq!(pagina_uno[0].id, primero);
    assert_eq!(pagina_dos[0].id, segundo);
}

#[test]
fn service_devuelve_usuario_resumen_sin_perder_datos() {
    let connection = base();
    let id = insertar(&connection, "9001", "Operador Seguro", "OPERADOR", false);
    let query = SqliteUsuariosQuery::new(&connection);
    let items = UsuarioConsultaService::new(&query)
        .buscar_para_tabla(&FiltroUsuarios::default())
        .unwrap();
    assert_eq!(items[0].id, id);
    assert_eq!(items[0].nombre, "Operador Seguro");
    assert_eq!(items[0].rol, RolUsuario::Operador);
    assert!(!items[0].activo);
}

#[test]
fn rol_persistido_invalido_es_error_tecnico_sin_panico() {
    let connection = base();
    connection
        .execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    insertar(&connection, "1", "Inválido", "ROL_INVALIDO", true);
    assert!(
        SqliteUsuariosQuery::new(&connection)
            .buscar(&FiltroUsuarios::default())
            .is_err()
    );
}

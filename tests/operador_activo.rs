use rusqlite::Connection;

use control_acceso::application::AppCore;
use control_acceso::database::schema::initialize_database;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::autenticacion_service::UsuarioSesion;
use control_acceso::services::error::RegistroIngresoServiceError;

/// Regresión del hallazgo #2 de `docs/auditoria-dominio-2026-08-20.md`:
/// "Una sesión revocada puede continuar registrando movimientos". Antes,
/// `registrar_ingreso`/`registrar_salida` sólo exigían que `usuario_id`
/// existiera (por la FK de SQLite) — nunca que la cuenta siguiera activa.
fn base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO empresas(id,nombre) VALUES (1,'Empresa');
             INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo)
             VALUES
                (1,'U1','Operador Activo','hash','OPERADOR',1),
                (2,'U2','Operador Desactivado','hash','OPERADOR',0);
             INSERT INTO contratistas(
                 id,cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso
             ) VALUES (1,'C1','Persona',1,'SWAT',0,1);",
        )
        .unwrap();
    connection
}

fn actor(id: i64) -> UsuarioSesion {
    UsuarioSesion {
        id,
        cedula: format!("U{id}"),
        nombre: "Operador".into(),
        rol: RolUsuario::Operador,
    }
}

#[test]
fn registrar_ingreso_rechaza_un_usuario_desactivado() {
    let core = AppCore::new(base());
    let resultado = core.registrar_ingreso(&actor(2), 1, MedioIngreso::Caminando, None);
    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::OperadorNoAutorizado)
    ));
    assert!(
        core.listar_ingresos_activos(&Default::default())
            .unwrap()
            .items
            .is_empty(),
        "un operador desactivado no debe poder crear el movimiento"
    );
}

#[test]
fn registrar_ingreso_rechaza_un_usuario_inexistente() {
    let core = AppCore::new(base());
    let resultado = core.registrar_ingreso(&actor(999), 1, MedioIngreso::Caminando, None);
    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::OperadorNoAutorizado)
    ));
}

#[test]
fn registrar_salida_rechaza_un_usuario_desactivado_aunque_el_ingreso_sea_valido() {
    let core = AppCore::new(base());
    let entrada = core
        .registrar_ingreso(&actor(1), 1, MedioIngreso::Caminando, None)
        .unwrap();

    let resultado = core.registrar_salida(&actor(2), entrada.registro_id);
    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::OperadorNoAutorizado)
    ));
    assert_eq!(
        core.listar_ingresos_activos(&Default::default())
            .unwrap()
            .items
            .len(),
        1,
        "el ingreso debe seguir activo — la salida no se registró"
    );
}

#[test]
fn registrar_ingreso_y_salida_funcionan_con_un_operador_activo() {
    let core = AppCore::new(base());
    let entrada = core
        .registrar_ingreso(&actor(1), 1, MedioIngreso::Caminando, None)
        .unwrap();
    core.registrar_salida(&actor(1), entrada.registro_id)
        .unwrap();
}

//! La misma persona no puede quedar adentro dos veces en el mismo sitio:
//! una por la PC y otra por el teléfono. Antes el chequeo miraba sólo la
//! base del propio equipo; el ingreso abierto por el otro dispositivo vive
//! en la caché `ingresos_remotos` y no se consultaba.

use rusqlite::Connection;

use control_acceso::application::AppCore;
use control_acceso::database::queries::contratistas::FiltroContratistas;
use control_acceso::database::schema::initialize_database;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::autenticacion_service::UsuarioSesion;
use control_acceso::services::error::RegistroIngresoServiceError;
use control_acceso::services::registro_ingreso_service::BloqueoIngreso;

fn base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO empresas(id,nombre) VALUES (1,'Empresa');
             INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo)
             VALUES (1,'U1','Operador','hash','OPERADOR',1);
             INSERT INTO contratistas(
                 id,cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso
             ) VALUES
                 (1,'101110111','PERSONA ADENTRO EN EL OTRO EQUIPO',1,'SWAT',0,1),
                 (2,'202220222','PERSONA AFUERA',1,'SWAT',0,1);",
        )
        .unwrap();
    connection
}

/// Núcleo con lo que deja la sincronización cuando el otro dispositivo del
/// sitio tiene abierto un ingreso con esa cédula.
fn core_con_abierto_en_el_otro_dispositivo(cedula: &str) -> AppCore {
    let connection = base();
    connection
        .execute(
            "INSERT INTO ingresos_remotos (
                 uuid, sitio_id, contratista_nombre, hora_entrada,
                 dispositivo_entrada_id, actualizado_en, contratista_cedula
             ) VALUES ('u-otro', 's1', 'PERSONA', '2026-09-27T12:00:00Z',
                       'otro-dispositivo', '2026-09-27T12:00:00Z', ?1)",
            [cedula],
        )
        .unwrap();
    AppCore::new(connection)
}

fn actor() -> UsuarioSesion {
    UsuarioSesion {
        id: 1,
        cedula: "U1".into(),
        nombre: "Operador".into(),
        rol: RolUsuario::Operador,
    }
}

#[test]
fn preparar_bloquea_a_quien_esta_adentro_por_el_otro_dispositivo() {
    let core = core_con_abierto_en_el_otro_dispositivo("101110111");

    let adentro = core.preparar_ingreso(1).unwrap();
    assert_eq!(
        adentro.bloqueo(),
        Some(BloqueoIngreso::IngresoActivoEnOtroDispositivo)
    );
    assert_eq!(core.preparar_ingreso(2).unwrap().bloqueo(), None);
}

#[test]
fn registrar_rechaza_a_quien_esta_adentro_por_el_otro_dispositivo() {
    let core = core_con_abierto_en_el_otro_dispositivo("101110111");

    let resultado = core.registrar_ingreso(&actor(), 1, MedioIngreso::Caminando, None, None);

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::IngresoActivoEnOtroDispositivo)
    ));
    core.registrar_ingreso(&actor(), 2, MedioIngreso::Caminando, None, None)
        .unwrap();
}

#[test]
fn el_buscador_marca_adentro_a_quien_entro_por_el_otro_dispositivo() {
    let core = core_con_abierto_en_el_otro_dispositivo("101110111");

    let adentro: Vec<(String, bool)> = core
        .buscar_contratistas(&FiltroContratistas::default())
        .unwrap()
        .items
        .into_iter()
        .map(|fila| (fila.cedula, fila.tiene_ingreso_activo))
        .collect();

    assert!(adentro.contains(&("101110111".to_string(), true)));
    assert!(adentro.contains(&("202220222".to_string(), false)));
}

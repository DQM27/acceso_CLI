//! Veto por persona (`personas_vetadas`): una cédula vetada no entra por
//! ninguna puerta, sin importar el rol ni el formato en que se escriba.
//! Antes el veto estaba atado al rol (`contratistas.tiene_acceso`) y la
//! persona entraba como proveedor o como visita con la misma cédula. Ver
//! `docs/features-futuras/plan-veto-por-persona.md`.
//!
//! Que un veto levantado deje de bloquear lo cubren
//! `database::queries::personas_vetadas` (sólo cuenta `vigente = 1`) y
//! `nube::en_vivo` (el aviso del levantamiento deja `vigente = 0`).

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use rusqlite::Connection;

use control_acceso::application::AppCore;
use control_acceso::database::schema::initialize_database;
use control_acceso::mensajes::MENSAJE_PERSONA_VETADA;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::autenticacion_service::UsuarioSesion;
use control_acceso::services::error::{
    CitaServiceError, IngresoProveedorServiceError, RegistroIngresoServiceError,
};
use control_acceso::services::registro_ingreso_service::BloqueoIngreso;
use control_acceso::tiempo::RelojFijo;

const VETADA: &str = "112340567";
const LIBRE: &str = "202220222";

/// Una misma persona (`VETADA`) aparece como contratista, tiene una visita
/// agendada con su cédula escrita con guiones, y además tiene un veto
/// vigente. `LIBRE` es otra persona, sin veto, en los mismos lugares.
fn nucleo() -> AppCore {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(&format!(
            "INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo)
             VALUES (1,'U1','Operador','hash','OPERADOR',1);
             INSERT INTO empresas(id,nombre) VALUES (1,'Empresa');
             INSERT INTO contratistas(
                 id,cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso
             ) VALUES
                 (1,'{VETADA}','PERSONA VETADA',1,'SWAT',0,1),
                 (2,'{LIBRE}','PERSONA LIBRE',1,'SWAT',0,1);
             INSERT INTO empresas_proveedor(id,nombre,activo,uuid) VALUES (1,'Proveedora',1,'ep-1');
             INSERT INTO gafetes(numero,tipo,estado) VALUES (7,'PROVEEDOR','DISPONIBLE'),(8,'PROVEEDOR','DISPONIBLE');
             INSERT INTO citas (id, uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                 anfitrion_correo, estado, creado_en)
             VALUES (1, 'cita-1', '2026-09-01', '2026-09-30', 'Anfitrión',
                 'anfitrion@ejemplo.com', 'VIGENTE', '2026-09-01T00:00:00Z');
             INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre) VALUES
                 (1, 'visitante-1', 1, '1-1234-0567', 'Persona Vetada'),
                 (2, 'visitante-2', 1, '{LIBRE}', 'Persona Libre');
             INSERT INTO personas_vetadas (uuid, cedula, vigente, actualizado_en)
             VALUES ('veto-1', '{VETADA}', 1, '2026-09-28T10:00:00Z');"
        ))
        .unwrap();
    let reloj = Arc::new(RelojFijo::new(
        Utc.with_ymd_and_hms(2026, 9, 28, 12, 0, 0).unwrap(),
    ));
    AppCore::con_reloj(connection, reloj)
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
fn contratista_vetado_no_pasa_de_la_preparacion_ni_del_registro() {
    let core = nucleo();

    let preparacion = core.preparar_ingreso(1).unwrap();
    assert_eq!(preparacion.bloqueo(), Some(BloqueoIngreso::PersonaVetada));
    assert_eq!(
        control_acceso::mensajes::mensaje_bloqueo_ingreso(&BloqueoIngreso::PersonaVetada),
        MENSAJE_PERSONA_VETADA
    );
    assert!(matches!(
        core.registrar_ingreso(&actor(), 1, MedioIngreso::Caminando, None, None),
        Err(RegistroIngresoServiceError::PersonaVetada)
    ));

    assert_eq!(core.preparar_ingreso(2).unwrap().bloqueo(), None);
    core.registrar_ingreso(&actor(), 2, MedioIngreso::Caminando, None, None)
        .unwrap();
}

#[test]
fn la_misma_persona_tampoco_entra_como_proveedor_en_ningun_formato() {
    let core = nucleo();

    for formato in [VETADA, "1-1234-0567", "01-1234-0567"] {
        assert!(
            matches!(
                core.registrar_ingreso_proveedor(&actor(), formato, "Persona", 1, None, 7),
                Err(IngresoProveedorServiceError::PersonaVetada)
            ),
            "{formato}"
        );
    }
    core.registrar_ingreso_proveedor(&actor(), LIBRE, "Persona Libre", 1, None, 8)
        .unwrap();
}

#[test]
fn ni_como_visita_aunque_tenga_la_cita_agendada() {
    let core = nucleo();

    assert!(matches!(
        core.verificar_check_in_visita("112340567"),
        Err(CitaServiceError::PersonaVetada)
    ));
    core.verificar_check_in_visita(LIBRE).unwrap();
}

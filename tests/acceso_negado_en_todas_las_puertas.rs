//! El acceso se niega con el mismo interruptor de siempre (`tiene_acceso`
//! del contratista, en el panel web o el escritorio). Antes sólo lo miraba
//! la puerta de contratistas: la persona entraba igual como proveedor o
//! como visita con la misma cédula. Ahora las tres puertas lo respetan, y
//! no se esquiva cambiando el formato de la cédula.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use rusqlite::Connection;

use control_acceso::application::AppCore;
use control_acceso::database::schema::initialize_database;
use control_acceso::domain::resultado_acceso::MotivoDenegacion;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::autenticacion_service::UsuarioSesion;
use control_acceso::services::error::{
    CitaServiceError, IngresoProveedorServiceError, RegistroIngresoServiceError,
};
use control_acceso::tiempo::RelojFijo;

const NEGADA: &str = "112340567";
const LIBRE: &str = "202220222";

/// `NEGADA` es contratista con el acceso apagado, y además tiene una visita
/// agendada con la cédula escrita con guiones. `LIBRE` es otra persona con
/// el acceso encendido, en los mismos lugares.
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
                 (1,'{NEGADA}','PERSONA NEGADA',1,'SWAT',0,0),
                 (2,'{LIBRE}','PERSONA LIBRE',1,'SWAT',0,1);
             INSERT INTO empresas_proveedor(id,nombre,activo,uuid) VALUES (1,'Proveedora',1,'ep-1');
             INSERT INTO gafetes(numero,tipo,estado)
             VALUES (7,'PROVEEDOR','DISPONIBLE'),(8,'PROVEEDOR','DISPONIBLE');
             INSERT INTO citas (id, uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                 anfitrion_correo, estado, creado_en)
             VALUES (1, 'cita-1', '2026-09-01', '2026-09-30', 'Anfitrión',
                 'anfitrion@ejemplo.com', 'VIGENTE', '2026-09-01T00:00:00Z');
             INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre) VALUES
                 (1, 'visitante-1', 1, '1-1234-0567', 'Persona Negada'),
                 (2, 'visitante-2', 1, '{LIBRE}', 'Persona Libre');"
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
fn como_contratista_ya_lo_bloqueaba_el_interruptor() {
    let core = nucleo();

    assert!(matches!(
        core.registrar_ingreso(&actor(), 1, MedioIngreso::Caminando, None, None),
        Err(RegistroIngresoServiceError::AccesoDenegado(
            MotivoDenegacion::SinAcceso
        ))
    ));
}

#[test]
fn tampoco_entra_como_proveedor_en_ningun_formato() {
    let core = nucleo();

    for formato in [NEGADA, "1-1234-0567", "01-1234-0567"] {
        assert!(
            matches!(
                core.registrar_ingreso_proveedor(&actor(), formato, "Persona", 1, None, 7),
                Err(IngresoProveedorServiceError::AccesoNegado)
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
        core.verificar_check_in_visita(NEGADA),
        Err(CitaServiceError::AccesoNegado)
    ));
    core.verificar_check_in_visita(LIBRE).unwrap();
}

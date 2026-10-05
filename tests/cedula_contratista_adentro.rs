//! A un contratista que está adentro no se le cambia la cédula: su ingreso
//! abierto quedaría con la vieja y, con la nueva, el "¿ya está adentro?"
//! (que compara por cédula) no lo frenaría al volver a entrar.

use rusqlite::Connection;

use control_acceso::{
    application::AppCore,
    database::schema::initialize_database,
    mensajes::mensaje_contratista,
    models::{tipo_ingreso::TipoIngreso, usuario::RolUsuario},
    services::{
        autenticacion_service::UsuarioSesion, contratista_service::DatosActualizacionContratista,
        error::ContratistaServiceError,
    },
};

fn sesion() -> UsuarioSesion {
    UsuarioSesion {
        id: 1,
        cedula: "U1".into(),
        nombre: "Operador".into(),
        rol: RolUsuario::Operador,
    }
}

/// Base con un contratista (cédula 100100100) y, según `adentro`, un
/// ingreso abierto suyo en este equipo o en el otro de la unidad.
fn base(adentro: Option<&str>) -> AppCore {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo) VALUES
                (1,'U1','Operador','hash','OPERADOR',1);
             INSERT INTO empresas(id,nombre,activo) VALUES (1,'Empresa',1);
             INSERT INTO contratistas(
                id,cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,
                es_personal_ruta,tiene_acceso
             ) VALUES (1,'100100100','PERSONA',1,'SWAT',NULL,0,1);",
        )
        .unwrap();
    match adentro {
        Some("este_equipo") => connection
            .execute_batch(
                "INSERT INTO registro_ingresos (
                    contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
                    usuario_ingreso_id, contratista_cedula, contratista_nombre, empresa_nombre,
                    usuario_ingreso_nombre, es_personal_ruta, tiene_acceso, resultado_acceso,
                    reglas_version, uuid
                 ) VALUES (
                    1, 1, '2026-10-05T08:00:00Z', 'CAMINANDO', 'SWAT',
                    1, '100100100', 'PERSONA', 'Empresa',
                    'Operador', 0, 1, 'PERMITIDO', 1, 'uuid-ingreso'
                 )",
            )
            .unwrap(),
        Some("otro_equipo") => connection
            .execute_batch(
                "INSERT INTO ingresos_remotos (
                    uuid, sitio_id, contratista_nombre, hora_entrada,
                    dispositivo_entrada_id, actualizado_en, contratista_cedula
                 ) VALUES ('u-remoto', 's1', 'PERSONA', '2026-10-05T08:00:00Z', 'otro',
                           '2026-10-05T08:00:00Z', '1-0010-0100')",
            )
            .unwrap(),
        _ => {}
    }
    AppCore::new(connection)
}

fn con_cedula(cedula: &str) -> DatosActualizacionContratista {
    DatosActualizacionContratista {
        cedula: cedula.into(),
        nombre: "Persona".into(),
        empresa_id: 1,
        tipo_ingreso: TipoIngreso::Swat,
        fecha_vencimiento_praind: None,
        es_personal_ruta: false,
        tiene_acceso: true,
    }
}

#[test]
fn afuera_se_le_puede_cambiar_la_cedula() {
    let core = base(None);
    core.actualizar_contratista(&sesion(), 1, con_cedula("200200200"))
        .unwrap();
}

#[test]
fn adentro_en_este_equipo_no_se_le_cambia_la_cedula() {
    let core = base(Some("este_equipo"));
    let error = core
        .actualizar_contratista(&sesion(), 1, con_cedula("200200200"))
        .unwrap_err();
    assert!(matches!(
        error,
        ContratistaServiceError::CedulaConIngresoActivo
    ));
    assert_eq!(
        mensaje_contratista(error),
        "No se puede cambiar la cédula mientras está adentro — registre primero la salida"
    );
}

#[test]
fn adentro_en_el_otro_equipo_tampoco_aunque_alla_la_cedula_venga_con_guiones() {
    let core = base(Some("otro_equipo"));
    assert!(matches!(
        core.actualizar_contratista(&sesion(), 1, con_cedula("200200200")),
        Err(ContratistaServiceError::CedulaConIngresoActivo)
    ));
}

#[test]
fn adentro_igual_se_puede_editar_lo_demas_y_la_misma_cedula_escrita_distinto() {
    let core = base(Some("este_equipo"));
    // La misma cédula con guiones no es un cambio: se guarda igual.
    core.actualizar_contratista(&sesion(), 1, con_cedula("1-0010-0100"))
        .unwrap();
}

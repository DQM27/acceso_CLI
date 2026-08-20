use chrono::NaiveDate;
use rusqlite::Connection;

use control_acceso::{
    application::AppCore,
    database::{
        queries::auditoria_contratistas::FiltroAuditoriaContratistas, schema::initialize_database,
    },
    models::{tipo_ingreso::TipoIngreso, usuario::RolUsuario},
    services::{
        autenticacion_service::UsuarioSesion, contratista_service::DatosActualizacionContratista,
    },
};

fn actor() -> UsuarioSesion {
    UsuarioSesion {
        id: 1,
        cedula: "ROOT".into(),
        nombre: "Root".into(),
        rol: RolUsuario::Root,
    }
}

fn conexion() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo)
             VALUES(1,'ROOT','Root','hash','ROOT',1);
             INSERT INTO empresas(id,nombre) VALUES(1,'Empresa');
             INSERT INTO contratistas(
                id,cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,
                es_personal_ruta,tiene_acceso
             ) VALUES(1,'C1','Persona',1,'SWAT',NULL,0,1);",
        )
        .unwrap();
    connection
}

fn datos(tipo: TipoIngreso, fecha: Option<NaiveDate>) -> DatosActualizacionContratista {
    DatosActualizacionContratista {
        nombre: "Persona".into(),
        empresa_id: 1,
        tipo_ingreso: tipo,
        fecha_vencimiento_praind: fecha,
        es_personal_ruta: false,
        tiene_acceso: true,
    }
}

fn datos_con_acceso(tiene_acceso: bool) -> DatosActualizacionContratista {
    DatosActualizacionContratista {
        tiene_acceso,
        ..datos(TipoIngreso::Swat, None)
    }
}

#[test]
fn solo_registra_cambios_reales_con_actor_y_valores_correctos() {
    let core = AppCore::new(conexion());
    let actor = actor();
    core.actualizar_contratista(&actor, 1, datos(TipoIngreso::Swat, None))
        .unwrap();
    assert_eq!(
        core.buscar_auditoria_contratistas(&actor, &Default::default())
            .unwrap()
            .total,
        0
    );

    let fecha = NaiveDate::from_ymd_opt(2027, 1, 2).unwrap();
    core.actualizar_contratista(&actor, 1, datos(TipoIngreso::Praind, Some(fecha)))
        .unwrap();
    let pagina = core
        .buscar_auditoria_contratistas(&actor, &FiltroAuditoriaContratistas::default())
        .unwrap();
    assert_eq!(pagina.total, 2);
    assert!(
        pagina
            .items
            .iter()
            .all(|cambio| cambio.usuario_id == actor.id)
    );
    assert!(pagina.items.iter().any(|cambio| {
        cambio.campo == "tipo_ingreso"
            && cambio.valor_anterior.as_deref() == Some("SWAT")
            && cambio.valor_nuevo.as_deref() == Some("PRAIND")
    }));
    assert!(pagina.items.iter().any(|cambio| {
        cambio.campo == "fecha_vencimiento_praind"
            && cambio.valor_anterior.is_none()
            && cambio.valor_nuevo.as_deref() == Some("2027-01-02")
    }));
}

#[test]
fn fallo_de_auditoria_revierte_tambien_la_actualizacion() {
    let connection = conexion();
    connection
        .execute_batch(
            "CREATE TRIGGER bloquear_auditoria
             BEFORE INSERT ON auditoria_contratistas
             BEGIN SELECT RAISE(ABORT,'fallo forzado'); END;",
        )
        .unwrap();
    let core = AppCore::new(connection);
    let actor = actor();
    assert!(
        core.actualizar_contratista(&actor, 1, datos(TipoIngreso::InHouse, None))
            .is_err()
    );
    let contratista = core
        .buscar_contratistas(
            &control_acceso::database::queries::contratistas::FiltroContratistas {
                texto: Some("C1".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(contratista.items[0].tipo_ingreso, TipoIngreso::Swat);
}

#[test]
fn registra_cada_desactivacion_y_reactivacion_del_acceso() {
    let core = AppCore::new(conexion());
    let actor = actor();

    core.actualizar_contratista(&actor, 1, datos_con_acceso(false))
        .unwrap();
    core.actualizar_contratista(&actor, 1, datos_con_acceso(true))
        .unwrap();

    let pagina = core
        .buscar_auditoria_contratistas(&actor, &FiltroAuditoriaContratistas::default())
        .unwrap();
    assert_eq!(pagina.total, 2);
    assert_eq!(pagina.items[0].campo, "tiene_acceso");
    assert_eq!(
        pagina.items[0].valor_anterior.as_deref(),
        Some("DESHABILITADO")
    );
    assert_eq!(pagina.items[0].valor_nuevo.as_deref(), Some("HABILITADO"));
    assert_eq!(pagina.items[1].campo, "tiene_acceso");
    assert_eq!(
        pagina.items[1].valor_anterior.as_deref(),
        Some("HABILITADO")
    );
    assert_eq!(
        pagina.items[1].valor_nuevo.as_deref(),
        Some("DESHABILITADO")
    );
}

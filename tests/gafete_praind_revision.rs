//! Revisión del gafete PRAIND leído con OCR contra la ficha, y la
//! actualización del vencimiento cuando el gafete es más nuevo
//! (`AppCore::revisar_gafete_praind` / `actualizar_vencimiento_praind`).

use chrono::NaiveDate;
use rusqlite::Connection;

use control_acceso::{
    application::AppCore,
    database::{queries::contratistas::FiltroContratistas, schema::initialize_database},
    domain::gafete_praind::{Comparacion, DatosGafeteLeido, FechaContraFicha, VencimientoGafete},
    models::usuario::RolUsuario,
    services::{autenticacion_service::UsuarioSesion, error::ContratistaServiceError},
};

fn operador() -> UsuarioSesion {
    UsuarioSesion {
        id: 1,
        cedula: "U1".into(),
        nombre: "Operador".into(),
        rol: RolUsuario::Administrador,
    }
}

fn fecha(texto: &str) -> NaiveDate {
    NaiveDate::parse_from_str(texto, "%Y-%m-%d").unwrap()
}

/// Un contratista PRAIND de Sodexo que vence el 30/05/2030 (lejos de hoy:
/// la prueba no depende del día en que corre).
fn base() -> AppCore {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo) VALUES
                (1,'U1','Operador','hash','ADMINISTRADOR',1);
             INSERT INTO empresas(id,nombre,activo) VALUES (1,'SODEXO',1);
             INSERT INTO contratistas(
                id,cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,
                es_personal_ruta,tiene_acceso
             ) VALUES (1,'305450949','HAZEL JOHANNA ANGULO AGUILAR',1,'PRAIND','2030-05-30',0,1);",
        )
        .unwrap();
    AppCore::new(connection)
}

fn gafete(induccion: &str, vence: &str) -> DatosGafeteLeido {
    DatosGafeteLeido {
        cedula: Some("305450949".into()),
        nombre: Some("Hazel Johanna Angulo Aguilar".into()),
        empresa: Some("Sodexo".into()),
        induccion: Some(fecha(induccion)),
        vencimiento: Some(fecha(vence)),
    }
}

/// (vencimiento, tiene acceso, nombre) del contratista 1.
fn ficha(core: &AppCore) -> (Option<NaiveDate>, bool, String) {
    let fila = core
        .buscar_contratistas(&FiltroContratistas::default())
        .unwrap()
        .items
        .into_iter()
        .find(|c| c.id == 1)
        .unwrap();
    (
        fila.fecha_vencimiento_praind,
        fila.tiene_acceso,
        fila.nombre,
    )
}

#[test]
fn revisa_contra_la_ficha_con_su_empresa() {
    let core = base();
    let revision = core
        .revisar_gafete_praind(1, &gafete("2030-06-07", "2032-06-06"))
        .unwrap();
    assert_eq!(revision.cedula, Comparacion::Coincide);
    assert_eq!(revision.empresa, Comparacion::Coincide);
    assert_eq!(revision.vencimiento_ficha, Some(fecha("2030-05-30")));
    assert_eq!(revision.contra_ficha, FechaContraFicha::MasNuevo);
    assert_eq!(revision.fecha_para_ficha, Some(fecha("2032-06-06")));

    let viejo = core
        .revisar_gafete_praind(1, &gafete("2019-06-19", "2020-06-19"))
        .unwrap();
    assert_eq!(
        viejo.vencimiento,
        VencimientoGafete::Vencido(fecha("2020-06-19"))
    );
    assert_eq!(viejo.fecha_para_ficha, None);
}

#[test]
fn actualizar_el_vencimiento_deja_el_resto_de_la_ficha_igual() {
    let core = base();
    core.actualizar_vencimiento_praind(&operador(), 1, fecha("2032-06-06"))
        .unwrap();
    assert_eq!(
        ficha(&core),
        (
            Some(fecha("2032-06-06")),
            true,
            "HAZEL JOHANNA ANGULO AGUILAR".into()
        )
    );
}

#[test]
fn no_deja_poner_un_vencimiento_ya_pasado() {
    let core = base();
    let error = core
        .actualizar_vencimiento_praind(&operador(), 1, fecha("2020-06-19"))
        .unwrap_err();
    assert!(
        matches!(error, ContratistaServiceError::PraindVencido),
        "se esperaba el rechazo de la regla del contratista, llegó {error:?}"
    );
    assert_eq!(ficha(&core).0, Some(fecha("2030-05-30")));
}

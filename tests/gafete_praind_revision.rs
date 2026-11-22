//! Revisión del gafete PRAIND leído con OCR contra la ficha, y la
//! actualización del vencimiento cuando el gafete es más nuevo
//! (`AppCore::revisar_gafete_praind` / `actualizar_vencimiento_praind`).

use chrono::NaiveDate;
use rusqlite::Connection;

use control_acceso::{
    application::{AppCore, CorreccionDesdeGafete},
    database::{queries::contratistas::FiltroContratistas, schema::initialize_database},
    domain::gafete_praind::{
        Comparacion, DatosGafeteLeido, EmpresaPropuesta, FechaContraFicha, VencimientoGafete,
    },
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
             INSERT INTO empresas(id,nombre,activo) VALUES (1,'SODEXO',1),
                (2,'MULTISERVICIOS DE ALDAMA DE SAN RAFAEL',1);
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

#[test]
fn propone_corregir_el_nombre_y_la_empresa_que_difieren() {
    let core = base();
    let leido = DatosGafeteLeido {
        nombre: Some("Hazel Johana Angulo Aguilar".into()),
        empresa: Some("Multiservicios de Aldama de San Rafael".into()),
        ..gafete("2030-06-07", "2032-06-06")
    };
    let revision = core.revisar_gafete_praind(1, &leido).unwrap();
    assert_eq!(
        revision.nombre_para_ficha.as_deref(),
        Some("HAZEL JOHANA ANGULO AGUILAR")
    );
    assert_eq!(
        revision.empresa_para_ficha,
        Some(EmpresaPropuesta {
            id: 2,
            nombre: "MULTISERVICIOS DE ALDAMA DE SAN RAFAEL".into()
        })
    );

    // Lo que coincide no propone nada.
    let igual = core
        .revisar_gafete_praind(1, &gafete("2030-06-07", "2032-06-06"))
        .unwrap();
    assert_eq!(igual.nombre_para_ficha, None);
    assert_eq!(igual.empresa_para_ficha, None);
}

#[test]
fn una_empresa_que_no_esta_en_el_catalogo_no_se_propone() {
    let core = base();
    let leido = DatosGafeteLeido {
        empresa: Some("Empresa Desconocida S.A.".into()),
        ..gafete("2030-06-07", "2032-06-06")
    };
    let revision = core.revisar_gafete_praind(1, &leido).unwrap();
    assert!(matches!(revision.empresa, Comparacion::Difiere { .. }));
    assert_eq!(revision.empresa_para_ficha, None);
}

#[test]
fn corregir_la_ficha_cambia_solo_lo_elegido_en_una_edicion() {
    let core = base();
    core.corregir_ficha_desde_gafete(
        &operador(),
        1,
        CorreccionDesdeGafete {
            nombre: Some("HAZEL JOHANA ANGULO AGUILAR".into()),
            empresa_id: Some(2),
            vencimiento: None,
        },
    )
    .unwrap();
    assert_eq!(
        ficha(&core),
        (
            Some(fecha("2030-05-30")),
            true,
            "HAZEL JOHANA ANGULO AGUILAR".into()
        )
    );
    let empresa = core
        .buscar_contratistas(&FiltroContratistas::default())
        .unwrap()
        .items
        .into_iter()
        .find(|c| c.id == 1)
        .unwrap()
        .empresa_nombre;
    assert_eq!(empresa, "MULTISERVICIOS DE ALDAMA DE SAN RAFAEL");
}

#[test]
fn alta_desde_gafete_encuentra_la_empresa_del_catalogo() {
    let core = base();
    let leido = DatosGafeteLeido {
        empresa: Some("Multiservicios de Aldama de San Rafael S.A.".into()),
        ..gafete("2026-06-19", "2028-06-19")
    };
    let datos = core.datos_alta_desde_gafete(&leido, None).unwrap();
    assert_eq!(datos.cedula.as_deref(), Some("305450949"));
    assert_eq!(datos.nombre.as_deref(), Some("HAZEL JOHANNA ANGULO AGUILAR"));
    assert_eq!(
        datos.empresa_catalogo,
        Some(EmpresaPropuesta {
            id: 2,
            nombre: "MULTISERVICIOS DE ALDAMA DE SAN RAFAEL".into()
        })
    );
    assert_eq!(datos.vencimiento, Some(fecha("2028-06-19")));
}

#[test]
fn alta_desde_gafete_sin_empresa_conocida_la_deja_como_texto() {
    let core = base();
    let leido = DatosGafeteLeido {
        empresa: Some("Empresa Que No Existe".into()),
        ..gafete("2026-06-19", "2028-06-19")
    };
    let datos = core.datos_alta_desde_gafete(&leido, None).unwrap();
    assert_eq!(datos.empresa.as_deref(), Some("Empresa Que No Existe"));
    assert_eq!(datos.empresa_catalogo, None);
}

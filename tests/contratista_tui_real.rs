use chrono::NaiveDate;
use control_acceso::{
    application::AppCore,
    database::{connection::open_database, queries::contratistas::FiltroContratistas},
    models::{tipo_ingreso::TipoIngreso, usuario::RolUsuario},
    services::autenticacion_service::UsuarioSesion,
    services::contratista_service::{DatosActualizacionContratista, DatosContratista},
    services::error::ContratistaServiceError,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn datos(
    cedula: &str,
    nombre: &str,
    empresa_id: i64,
    tipo: TipoIngreso,
    fecha: Option<NaiveDate>,
    ruta: bool,
) -> DatosContratista {
    DatosContratista {
        cedula: cedula.into(),
        nombre: nombre.into(),
        empresa_id,
        tipo_ingreso: tipo,
        fecha_vencimiento_praind: fecha,
        es_personal_ruta: ruta,
        tiene_acceso: true,
    }
}
fn actualizacion(
    cedula: &str,
    nombre: &str,
    empresa_id: i64,
    tipo: TipoIngreso,
    fecha: Option<NaiveDate>,
    ruta: bool,
) -> DatosActualizacionContratista {
    DatosActualizacionContratista {
        cedula: cedula.into(),
        nombre: nombre.into(),
        empresa_id,
        tipo_ingreso: tipo,
        fecha_vencimiento_praind: fecha,
        es_personal_ruta: ruta,
        tiene_acceso: true,
    }
}
fn filtro(t: &str) -> FiltroContratistas {
    FiltroContratistas {
        texto: Some(t.into()),
        ..Default::default()
    }
}

fn root(core: &AppCore) -> UsuarioSesion {
    let id = core
        .crear_root_inicial(
            control_acceso::services::usuario_service::CrearRootInicialInput {
                cedula: "ROOT".into(),
                nombre: "Root".into(),
                password: "password-root".into(),
            },
        )
        .unwrap();
    UsuarioSesion {
        id,
        cedula: "ROOT".into(),
        nombre: "Root".into(),
        rol: RolUsuario::Root,
    }
}

#[test]
fn persistencia_busqueda_fts_y_empresa_id_sobreviven_reapertura() {
    let sello = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let ruta = std::env::temp_dir().join(format!("brisas_contratista_{sello}.db"));
    let id;
    {
        let core = AppCore::new(open_database(&ruta).unwrap());
        let actor = root(&core);
        let e = core.crear_empresa(&actor, "Constructora Álvarez").unwrap();
        id = core
            .crear_contratista(
                &actor,
                datos(
                    "001-09",
                    "José Hernández",
                    e,
                    TipoIngreso::Praind,
                    NaiveDate::from_ymd_opt(2027, 1, 1),
                    false,
                ),
            )
            .unwrap();
        for q in ["jose", "hernandez", "nandez", "alvarez", "tructora"] {
            let r = core.buscar_contratistas(&filtro(q)).unwrap().items;
            assert_eq!(r[0].id, id);
            assert_eq!(r[0].empresa_id, e);
        }
    }
    {
        let core = AppCore::new(open_database(&ruta).unwrap());
        assert_eq!(
            core.buscar_contratistas(&filtro("001-09")).unwrap().items[0].id,
            id
        );
    }
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn actualizar_refresca_fts_duplicado_y_empresa_inexistente_son_semanticos() {
    let ruta = std::env::temp_dir().join(format!(
        "brisas_contratista_sem_{}.db",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let core = AppCore::new(open_database(&ruta).unwrap());
    let actor = root(&core);
    let e = core.crear_empresa(&actor, "Empresa").unwrap();
    let a = core
        .crear_contratista(
            &actor,
            datos(
                "CEDULA-VIEJA-997",
                "José Hernández",
                e,
                TipoIngreso::PorCorreo,
                None,
                false,
            ),
        )
        .unwrap();
    let _b = core
        .crear_contratista(
            &actor,
            datos("B", "Otro", e, TipoIngreso::Swat, None, false),
        )
        .unwrap();
    core.actualizar_contratista(
        &actor,
        a,
        actualizacion(
            "CEDULA-NUEVA-998",
            "José Álvarez",
            e,
            TipoIngreso::PorCorreo,
            None,
            false,
        ),
    )
    .unwrap();
    assert!(
        core.buscar_contratistas(&filtro("hernandez"))
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        core.buscar_contratistas(&filtro("alvarez")).unwrap().items[0].id,
        a
    );
    assert!(
        core.buscar_contratistas(&filtro("CEDULA-VIEJA-997"))
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        core.buscar_contratistas(&filtro("CEDULA-NUEVA-998"))
            .unwrap()
            .items[0]
            .id,
        a
    );
    assert!(matches!(
        core.actualizar_contratista(
            &actor,
            a,
            actualizacion("B", "No debe persistir", e, TipoIngreso::Swat, None, false)
        ),
        Err(ContratistaServiceError::CedulaDuplicada)
    ));
    let conservado = &core
        .buscar_contratistas(&filtro("CEDULA-NUEVA-998"))
        .unwrap()
        .items[0];
    assert_eq!(conservado.nombre, "José Álvarez");
    assert_eq!(conservado.tipo_ingreso, TipoIngreso::PorCorreo);
    assert!(matches!(
        core.crear_contratista(
            &actor,
            datos("D", "Sin empresa", 999, TipoIngreso::Swat, None, false)
        ),
        Err(ContratistaServiceError::EmpresaNoEncontrada)
    ));
    drop(core);
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn matrices_praind_ruta_acceso_y_cedula_string_se_persisten() {
    let ruta = std::env::temp_dir().join(format!(
        "brisas_contratista_reglas_{}.db",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let core = AppCore::new(open_database(&ruta).unwrap());
    let actor = root(&core);
    let e = core.crear_empresa(&actor, "Empresa").unwrap();
    let casos = [
        (TipoIngreso::Praind, false, true),
        (TipoIngreso::InHouse, false, true),
        (TipoIngreso::PorCorreo, false, false),
        (TipoIngreso::Swat, false, false),
        (TipoIngreso::PorCorreo, true, true),
    ];
    for (i, (tipo, ruta_personal, requiere)) in casos.into_iter().enumerate() {
        let fecha = requiere.then(|| NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
        let cedula = format!("00-{i}");
        core.crear_contratista(
            &actor,
            datos(
                &cedula,
                &format!("Persona {i}"),
                e,
                tipo,
                fecha,
                ruta_personal,
            ),
        )
        .unwrap();
        let r = &core.buscar_contratistas(&filtro(&cedula)).unwrap().items[0];
        assert_eq!(r.cedula, cedula);
        assert_eq!(r.fecha_vencimiento_praind.is_some(), requiere);
        assert_eq!(r.es_personal_ruta, ruta_personal);
    }
    drop(core);
    std::fs::remove_file(ruta).unwrap();
}

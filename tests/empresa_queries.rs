use std::time::{SystemTime, UNIX_EPOCH};

use control_acceso::{
    application::AppCore,
    database::{
        connection::open_database,
        queries::empresas::{EmpresasQuery, FiltroEmpresas, SqliteEmpresasQuery},
        schema::initialize_database,
    },
    models::usuario::RolUsuario,
    services::autenticacion_service::UsuarioSesion,
    services::error::EmpresaServiceError,
};
use rusqlite::{Connection, params};

fn filtro(texto: Option<&str>) -> FiltroEmpresas {
    FiltroEmpresas {
        texto: texto.map(str::to_owned),
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
fn busca_fts_sin_acentos_mayusculas_y_por_substring() {
    let c = Connection::open_in_memory().unwrap();
    initialize_database(&c).unwrap();
    c.execute(
        "INSERT INTO empresas(nombre) VALUES ('Constructora Álvarez')",
        [],
    )
    .unwrap();
    let q = SqliteEmpresasQuery::new(&c);
    for texto in ["alvarez", "ÁLVAREZ", "tructora", "varez"] {
        let items = q.buscar(&filtro(Some(texto))).unwrap();
        assert_eq!(items.len(), 1, "{texto}");
        assert_eq!(items[0].nombre, "Constructora Álvarez");
    }
}

/// Regresión de "Búsquedas de 1-2 caracteres no pliegan tildes ni Ñ"
/// (`docs/hallazgos-buscador.md`): "al" (2 caracteres) no aparece como
/// subcadena literal en "Álvarez" (empieza con `á`, no `a`) — sólo matchea
/// tras plegar diacríticos vía la función SQL `PLEGAR`.
#[test]
fn busqueda_corta_pliega_tildes() {
    let c = Connection::open_in_memory().unwrap();
    initialize_database(&c).unwrap();
    c.execute(
        "INSERT INTO empresas(nombre) VALUES ('Constructora Álvarez')",
        [],
    )
    .unwrap();
    let items = SqliteEmpresasQuery::new(&c)
        .buscar(&filtro(Some("al")))
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].nombre, "Constructora Álvarez");
}

#[test]
fn conteo_real_incluye_empresas_con_cero_sin_n_mas_uno() {
    let c = Connection::open_in_memory().unwrap();
    initialize_database(&c).unwrap();
    c.execute_batch(
        "PRAGMA foreign_keys=ON; INSERT INTO empresas(id,nombre) VALUES(1,'A'),(2,'B'),(3,'C');",
    )
    .unwrap();
    for (cedula, empresa) in [("a1", 1), ("a2", 1), ("a3", 1), ("b1", 2)] {
        c.execute("INSERT INTO contratistas(cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso) VALUES(?1,?1,?2,'SWAT',0,1)", params![cedula,empresa]).unwrap();
    }
    let items = SqliteEmpresasQuery::new(&c)
        .buscar(&FiltroEmpresas::default())
        .unwrap();
    assert_eq!(
        items
            .iter()
            .map(|e| (e.nombre.as_str(), e.contratistas))
            .collect::<Vec<_>>(),
        [("A", 3), ("B", 1), ("C", 0)]
    );
}

#[test]
fn renombrado_actualiza_fts_y_duplicado_conserva_sqlite() {
    let c = Connection::open_in_memory().unwrap();
    initialize_database(&c).unwrap();
    let core = AppCore::new(c);
    let actor = root(&core);
    let uno = core.crear_empresa(&actor, "Constructora Álvarez").unwrap();
    let dos = core.crear_empresa(&actor, "Empresa Dos").unwrap();
    core.actualizar_empresa(&actor, uno, "Constructora Hernández")
        .unwrap();
    assert!(
        core.buscar_empresas(&filtro(Some("alvarez")))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        core.buscar_empresas(&filtro(Some("hernandez"))).unwrap()[0].id,
        uno
    );
    assert!(matches!(
        core.actualizar_empresa(&actor, dos, "Constructora Hernández"),
        Err(EmpresaServiceError::NombreDuplicado)
    ));
    assert_eq!(
        core.buscar_empresas(&filtro(Some("empresa dos"))).unwrap()[0].id,
        dos
    );
}

#[test]
fn limite_offset_y_planes_representativos_son_correctos() {
    let c = Connection::open_in_memory().unwrap();
    initialize_database(&c).unwrap();
    for nombre in ["Alfa", "Beta", "Gamma"] {
        c.execute("INSERT INTO empresas(nombre) VALUES(?1)", [nombre])
            .unwrap();
    }
    let q = SqliteEmpresasQuery::new(&c);
    let pagina = q
        .buscar(&FiltroEmpresas {
            texto: None,
            limite: 1,
            offset: 1,
        })
        .unwrap();
    assert_eq!(pagina[0].nombre, "Beta");
    let plan_fts: Vec<String> = c.prepare("EXPLAIN QUERY PLAN SELECT e.id FROM empresas_fts INNER JOIN empresas e ON e.id=empresas_fts.rowid WHERE empresas_fts MATCH 'alf'").unwrap().query_map([], |r| r.get(3)).unwrap().collect::<Result<_,_>>().unwrap();
    assert!(plan_fts.iter().any(|p| p.contains("VIRTUAL TABLE INDEX")));
    let plan_conteo: Vec<String> = c.prepare("EXPLAIN QUERY PLAN SELECT e.id,COUNT(c.id) FROM empresas e LEFT JOIN contratistas c ON c.empresa_id=e.id GROUP BY e.id").unwrap().query_map([], |r| r.get(3)).unwrap().collect::<Result<_,_>>().unwrap();
    assert!(
        plan_conteo
            .iter()
            .any(|p| p.contains("idx_contratistas_empresa"))
    );
}

#[test]
fn empresa_persiste_al_cerrar_y_reabrir_appcore() {
    let sello = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let ruta = std::env::temp_dir().join(format!("brisas_empresas_{sello}.db"));
    {
        let core = AppCore::new(open_database(&ruta).unwrap());
        let actor = root(&core);
        core.crear_empresa(&actor, "Empresa Persistente").unwrap();
    }
    {
        let core = AppCore::new(open_database(&ruta).unwrap());
        let items = core.buscar_empresas(&filtro(Some("persistente"))).unwrap();
        assert_eq!(items[0].nombre, "Empresa Persistente");
        assert_eq!(items[0].contratistas, 0);
    }
    std::fs::remove_file(ruta).unwrap();
}

use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{Connection, params};

use control_acceso::database::error::DatabaseError;
use control_acceso::database::queries::ingresos::{
    EstadoMovimiento, FiltroHistorial, FiltroIngresosActivos, IngresoActivoLectura, IngresosQuery,
    ListaIngresosActivosLectura, PaginaHistorial, SqliteIngresosQuery,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::domain::resultado_acceso::ResultadoAcceso;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::services::error::RegistroIngresoServiceError;
use control_acceso::services::registro_ingreso_service::RegistroIngresoConsultaService;

struct Base {
    connection: Connection,
    empresa_uno: i64,
    empresa_dos: i64,
    contratista_uno: i64,
    contratista_dos: i64,
    usuario_entrada: i64,
    usuario_salida: i64,
}

fn preparar_base() -> Base {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas(nombre) VALUES ('Constructora Alfa'), ('Servicios Beta')",
            [],
        )
        .unwrap();
    let empresa_uno = 1;
    let empresa_dos = 2;
    connection.execute("INSERT INTO usuarios(cedula,nombre,password_hash,rol,activo) VALUES ('u1','Operador Entrada','hash','OPERADOR',1), ('u2','Operador Salida','hash','OPERADOR',1)", []).unwrap();
    let usuario_entrada = 1;
    let usuario_salida = 2;
    connection.execute("INSERT INTO contratistas(cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,es_personal_ruta,tiene_acceso) VALUES ('1001','Ana Solano',1,'PRAIND','2026-08-20',0,1), ('2002','Beto Rojas',2,'SWAT',NULL,0,1)", []).unwrap();
    Base {
        connection,
        empresa_uno,
        empresa_dos,
        contratista_uno: 1,
        contratista_dos: 2,
        usuario_entrada,
        usuario_salida,
    }
}

#[allow(clippy::too_many_arguments)]
fn insertar(
    base: &Base,
    contratista: i64,
    empresa: i64,
    ingreso: &str,
    medio: &str,
    tipo: &str,
    gafete: Option<i64>,
    salida: Option<&str>,
    usuario_salida: Option<i64>,
) -> i64 {
    base.connection
        .execute(
            "INSERT INTO registro_ingresos(
        contratista_id,empresa_id,fecha_hora_ingreso,medio_ingreso,tipo_ingreso,
        gafete_numero,usuario_ingreso_id,fecha_hora_salida,usuario_salida_id,
        contratista_cedula,contratista_nombre,empresa_nombre,
        usuario_ingreso_nombre,usuario_salida_nombre,fecha_vencimiento_praind,
        es_personal_ruta,tiene_acceso,resultado_acceso,motivo_resultado,reglas_version
    ) SELECT ?1,?2,?3,?4,?5,?6,?7,?8,?9,
        c.cedula,c.nombre,e.nombre,ui.nombre,us.nombre,c.fecha_vencimiento_praind,
        c.es_personal_ruta,c.tiene_acceso,'MIGRADO','DATOS_RECONSTRUIDOS',0
      FROM contratistas c
      INNER JOIN empresas e ON e.id=?2
      INNER JOIN usuarios ui ON ui.id=?7
      LEFT JOIN usuarios us ON us.id=?9
      WHERE c.id=?1",
            params![
                contratista,
                empresa,
                ingreso,
                medio,
                tipo,
                gafete,
                base.usuario_entrada,
                salida,
                usuario_salida
            ],
        )
        .unwrap();
    base.connection.last_insert_rowid()
}

fn dt(valor: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(valor, "%Y-%m-%d %H:%M:%S").unwrap()
}

fn filtro_historial() -> FiltroHistorial {
    FiltroHistorial::nuevo(dt("2026-08-01 00:00:00"), dt("2026-09-01 00:00:00"))
}

#[test]
fn activos_solo_devuelve_abiertos_con_fila_compuesta() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(12),
        None,
        None,
    );
    insertar(
        &base,
        base.contratista_dos,
        base.empresa_dos,
        "2026-08-12 08:00:00",
        "VEHICULO",
        "SWAT",
        None,
        Some("2026-08-12 09:00:00"),
        Some(base.usuario_salida),
    );
    let items = SqliteIngresosQuery::new(&base.connection)
        .listar_activos(&FiltroIngresosActivos::default())
        .unwrap();
    assert_eq!(items.total, 1);
    assert_eq!(items.items.len(), 1);
    let item = &items.items[0];
    assert_eq!(item.cedula, "1001");
    assert_eq!(item.contratista_nombre, "Ana Solano");
    assert_eq!(item.empresa_nombre, "Constructora Alfa");
    assert_eq!(item.tipo_ingreso, TipoIngreso::Praind);
    assert_eq!(item.medio_ingreso, MedioIngreso::Caminando);
    assert_eq!(item.fecha_hora_ingreso, dt("2026-08-12 07:00:00"));
    assert_eq!(item.gafete_numero, Some(12));
    assert_eq!(item.usuario_ingreso_nombre, "Operador Entrada");
}

#[test]
fn activos_admite_gafete_none_y_ambos_medios() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(1),
        None,
        None,
    );
    insertar(
        &base,
        base.contratista_dos,
        base.empresa_dos,
        "2026-08-12 08:00:00",
        "VEHICULO",
        "SWAT",
        None,
        None,
        None,
    );
    let items = SqliteIngresosQuery::new(&base.connection)
        .listar_activos(&FiltroIngresosActivos::default())
        .unwrap();
    assert_eq!(items.items[0].medio_ingreso, MedioIngreso::Vehiculo);
    assert_eq!(items.items[0].gafete_numero, None);
    assert_eq!(items.items[1].medio_ingreso, MedioIngreso::Caminando);
}

#[test]
fn activos_busca_por_cedula_nombre_empresa_y_gafete() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(47),
        None,
        None,
    );
    let query = SqliteIngresosQuery::new(&base.connection);
    for texto in ["100", "solano", "constructora", "47"] {
        let items = query
            .listar_activos(&FiltroIngresosActivos {
                texto: Some(format!("  {texto}  ")),
            })
            .unwrap();
        assert_eq!(items.items.len(), 1, "filtro {texto}");
        assert_eq!(items.total, 1);
    }
    assert_eq!(
        query
            .listar_activos(&FiltroIngresosActivos {
                texto: Some("   ".into()),
            })
            .unwrap()
            .items
            .len(),
        1
    );
}

#[test]
fn activos_devuelve_mas_de_cien_sin_recorte_y_con_orden_estable() {
    let base = preparar_base();
    for id in 3..=125 {
        base.connection
            .execute(
                "INSERT INTO contratistas(
                    cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso
                 ) VALUES (?1,?2,1,'SWAT',0,1)",
                params![format!("C-{id}"), format!("Persona {id}")],
            )
            .unwrap();
    }
    let mut ids = Vec::new();
    for contratista_id in 1..=125 {
        ids.push(insertar(
            &base,
            contratista_id,
            base.empresa_uno,
            "2026-08-12 07:00:00",
            "VEHICULO",
            "SWAT",
            None,
            None,
            None,
        ));
    }
    let query = SqliteIngresosQuery::new(&base.connection);
    let resultado = query
        .listar_activos(&FiltroIngresosActivos::default())
        .unwrap();
    assert_eq!(resultado.total, 125);
    assert_eq!(resultado.items.len(), 125);
    assert_eq!(resultado.items[0].registro_id, *ids.last().unwrap());
    assert_eq!(resultado.items[124].registro_id, ids[0]);
}

#[test]
fn activos_filtrados_conservan_total_real_y_gafete_devuelve_registro_completo() {
    let base = preparar_base();
    let registro_id = insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(47),
        None,
        None,
    );
    insertar(
        &base,
        base.contratista_dos,
        base.empresa_dos,
        "2026-08-12 08:00:00",
        "VEHICULO",
        "SWAT",
        None,
        None,
        None,
    );
    let query = SqliteIngresosQuery::new(&base.connection);
    let filtrados = query
        .listar_activos(&FiltroIngresosActivos {
            texto: Some("Ana".into()),
        })
        .unwrap();
    assert_eq!(filtrados.items.len(), 1);
    assert_eq!(filtrados.total, 2);

    let encontrado = query.buscar_activo_por_gafete(47).unwrap().unwrap();
    assert_eq!(encontrado.registro_id, registro_id);
    assert_eq!(encontrado.contratista_nombre, "Ana Solano");
    assert_eq!(encontrado.empresa_nombre, "Constructora Alfa");
    assert!(query.buscar_activo_por_gafete(99).unwrap().is_none());
}

#[test]
fn activos_convierte_los_cuatro_tipos() {
    let base = preparar_base();
    base.connection.execute("INSERT INTO contratistas(cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,es_personal_ruta,tiene_acceso) VALUES ('3003','Caro',1,'POR_CORREO',NULL,0,1), ('4004','Dani',2,'SWAT',NULL,0,1)", []).unwrap();
    for (i, tipo) in ["PRAIND", "IN_HOUSE", "POR_CORREO", "SWAT"]
        .iter()
        .enumerate()
    {
        insertar(
            &base,
            i as i64 + 1,
            if i % 2 == 0 {
                base.empresa_uno
            } else {
                base.empresa_dos
            },
            &format!("2026-08-{:02} 07:00:00", 10 + i),
            "CAMINANDO",
            tipo,
            Some(i as i64 + 1),
            None,
            None,
        );
    }
    let tipos: Vec<_> = SqliteIngresosQuery::new(&base.connection)
        .listar_activos(&FiltroIngresosActivos::default())
        .unwrap()
        .items
        .into_iter()
        .map(|i| i.tipo_ingreso)
        .collect();
    assert_eq!(
        tipos,
        vec![
            TipoIngreso::Swat,
            TipoIngreso::PorCorreo,
            TipoIngreso::InHouse,
            TipoIngreso::Praind
        ]
    );
}

#[test]
fn service_calcula_advertencia_praind_actual_sin_sql_de_dominio() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(1),
        None,
        None,
    );
    let query = SqliteIngresosQuery::new(&base.connection);
    let service = RegistroIngresoConsultaService::new(&query);
    let items = service
        .listar_activos(
            &FiltroIngresosActivos::default(),
            NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
        )
        .unwrap();
    assert_eq!(
        items.items[0].resultado_acceso,
        ResultadoAcceso::PermitidoConAdvertencia
    );
}

#[test]
fn historial_todos_incluye_activos_cerrados_y_usuarios_opcionales() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(1),
        None,
        None,
    );
    insertar(
        &base,
        base.contratista_dos,
        base.empresa_dos,
        "2026-08-12 08:00:00",
        "VEHICULO",
        "SWAT",
        None,
        Some("2026-08-12 09:00:00"),
        Some(base.usuario_salida),
    );
    let pagina = SqliteIngresosQuery::new(&base.connection)
        .buscar_historial(&filtro_historial())
        .unwrap();
    assert_eq!(pagina.total, 2);
    assert_eq!(pagina.items[0].usuario_ingreso_nombre, "Operador Entrada");
    assert_eq!(
        pagina.items[0].usuario_salida_nombre.as_deref(),
        Some("Operador Salida")
    );
    assert_eq!(pagina.items[1].usuario_salida_nombre, None);
    assert_eq!(pagina.items[0].gafete_numero, None);
}

#[test]
fn historial_filtra_estado_activos_y_cerrados() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(1),
        None,
        None,
    );
    insertar(
        &base,
        base.contratista_dos,
        base.empresa_dos,
        "2026-08-12 08:00:00",
        "VEHICULO",
        "SWAT",
        None,
        Some("2026-08-12 09:00:00"),
        Some(base.usuario_salida),
    );
    let query = SqliteIngresosQuery::new(&base.connection);
    let mut filtro = filtro_historial();
    filtro.estado = EstadoMovimiento::Activos;
    assert_eq!(query.buscar_historial(&filtro).unwrap().total, 1);
    filtro.estado = EstadoMovimiento::Cerrados;
    assert_eq!(query.buscar_historial(&filtro).unwrap().total, 1);
}

#[test]
fn historial_rango_es_inclusivo_desde_y_exclusivo_hasta() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-01 00:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(1),
        None,
        None,
    );
    insertar(
        &base,
        base.contratista_dos,
        base.empresa_dos,
        "2026-09-01 00:00:00",
        "VEHICULO",
        "SWAT",
        None,
        None,
        None,
    );
    let pagina = SqliteIngresosQuery::new(&base.connection)
        .buscar_historial(&filtro_historial())
        .unwrap();
    assert_eq!(pagina.total, 1);
    assert_eq!(
        pagina.items[0].fecha_hora_ingreso,
        dt("2026-08-01 00:00:00")
    );
}

#[test]
fn historial_busca_persona_por_cedula_y_nombre_con_trim_y_case_insensitive() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(1),
        None,
        None,
    );
    let query = SqliteIngresosQuery::new(&base.connection);
    for texto in ["  1001  ", "  ana solano  "] {
        let mut filtro = filtro_historial();
        filtro.texto_persona = Some(texto.into());
        assert_eq!(query.buscar_historial(&filtro).unwrap().total, 1);
    }
}

#[test]
fn historial_filtra_empresa_tipo_y_gafete_con_and() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(31),
        None,
        None,
    );
    insertar(
        &base,
        base.contratista_dos,
        base.empresa_dos,
        "2026-08-12 08:00:00",
        "VEHICULO",
        "SWAT",
        Some(32),
        None,
        None,
    );
    let mut filtro = filtro_historial();
    filtro.empresa_id = Some(base.empresa_uno);
    filtro.tipo_ingreso = Some(TipoIngreso::Praind);
    filtro.gafete_numero = Some(31);
    filtro.estado = EstadoMovimiento::Activos;
    let pagina = SqliteIngresosQuery::new(&base.connection)
        .buscar_historial(&filtro)
        .unwrap();
    assert_eq!(pagina.total, 1);
    assert_eq!(pagina.items[0].contratista_nombre, "Ana Solano");
}

#[test]
fn historial_pagina_en_sql_y_total_no_depende_de_limit_offset() {
    let base = preparar_base();
    for i in 0..17 {
        insertar(
            &base,
            base.contratista_uno,
            base.empresa_uno,
            &format!("2026-08-{:02} 07:00:00", i + 1),
            "CAMINANDO",
            "PRAIND",
            Some(i + 1),
            Some(&format!("2026-08-{:02} 08:00:00", i + 1)),
            Some(base.usuario_salida),
        );
    }
    let query = SqliteIngresosQuery::new(&base.connection);
    let mut filtro = filtro_historial();
    filtro.limite = 5;
    filtro.offset = 5;
    let pagina = query.buscar_historial(&filtro).unwrap();
    assert_eq!(pagina.total, 17);
    assert_eq!(pagina.items.len(), 5);
    filtro.offset = 30;
    let vacia = query.buscar_historial(&filtro).unwrap();
    assert_eq!(vacia.total, 17);
    assert!(vacia.items.is_empty());
}

#[test]
fn historial_orden_fecha_id_es_estable_en_empates() {
    let base = preparar_base();
    let primero = insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(1),
        None,
        None,
    );
    let segundo = insertar(
        &base,
        base.contratista_dos,
        base.empresa_dos,
        "2026-08-12 07:00:00",
        "VEHICULO",
        "SWAT",
        None,
        None,
        None,
    );
    let items = SqliteIngresosQuery::new(&base.connection)
        .buscar_historial(&filtro_historial())
        .unwrap()
        .items;
    assert_eq!(
        items.iter().map(|i| i.registro_id).collect::<Vec<_>>(),
        vec![segundo, primero]
    );
}

#[test]
fn historial_convierte_tipos_medios_y_fechas_seguras() {
    let base = preparar_base();
    for (i, tipo) in ["PRAIND", "IN_HOUSE", "POR_CORREO", "SWAT"]
        .iter()
        .enumerate()
    {
        insertar(
            &base,
            base.contratista_uno,
            base.empresa_uno,
            &format!("2026-08-{:02} 07:00:00", 10 + i),
            if i % 2 == 0 { "CAMINANDO" } else { "VEHICULO" },
            tipo,
            Some(i as i64 + 1),
            Some(&format!("2026-08-{:02} 08:00:00", 10 + i)),
            Some(base.usuario_salida),
        );
    }
    let items = SqliteIngresosQuery::new(&base.connection)
        .buscar_historial(&filtro_historial())
        .unwrap()
        .items;
    assert_eq!(items.len(), 4);
    assert!(items.iter().any(
        |i| i.tipo_ingreso == TipoIngreso::InHouse && i.medio_ingreso == MedioIngreso::Vehiculo
    ));
}

#[test]
fn service_rechaza_rango_invalido_y_devuelve_pagina_valida() {
    let base = preparar_base();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "CAMINANDO",
        "PRAIND",
        Some(1),
        None,
        None,
    );
    let query = SqliteIngresosQuery::new(&base.connection);
    let service = RegistroIngresoConsultaService::new(&query);
    assert_eq!(
        service.buscar_historial(&filtro_historial()).unwrap().total,
        1
    );
    let mut invalido = filtro_historial();
    invalido.hasta = invalido.desde;
    assert!(matches!(
        service.buscar_historial(&invalido),
        Err(RegistroIngresoServiceError::RangoFechasInvalido)
    ));
}

struct QueryConError;

impl IngresosQuery for QueryConError {
    fn listar_activos(
        &self,
        _: &FiltroIngresosActivos,
    ) -> Result<ListaIngresosActivosLectura, DatabaseError> {
        Err(DatabaseError::Sqlite(rusqlite::Error::InvalidQuery))
    }

    fn buscar_activo_por_gafete(
        &self,
        _: i64,
    ) -> Result<Option<IngresoActivoLectura>, DatabaseError> {
        Err(DatabaseError::Sqlite(rusqlite::Error::InvalidQuery))
    }

    fn buscar_historial(&self, _: &FiltroHistorial) -> Result<PaginaHistorial, DatabaseError> {
        Err(DatabaseError::Sqlite(rusqlite::Error::InvalidQuery))
    }
}

#[test]
fn service_propaga_error_de_database() {
    let service = RegistroIngresoConsultaService::new(&QueryConError);
    let resultado = service.listar_activos(
        &FiltroIngresosActivos::default(),
        NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
    );
    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::Database(_))
    ));
}

#[test]
fn query_rechaza_enum_persistido_invalido_sin_panico() {
    let base = preparar_base();
    base.connection
        .execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    insertar(
        &base,
        base.contratista_uno,
        base.empresa_uno,
        "2026-08-12 07:00:00",
        "MEDIO_INVALIDO",
        "PRAIND",
        Some(1),
        None,
        None,
    );
    let resultado = SqliteIngresosQuery::new(&base.connection)
        .listar_activos(&FiltroIngresosActivos::default());
    assert!(resultado.is_err());
}

use chrono::NaiveDate;
use rusqlite::{Connection, params};

use control_acceso::database::queries::Igualdad;
use control_acceso::database::queries::contratistas::{
    ContratistasQuery, FiltroContratistas, SqliteContratistasQuery,
};
use control_acceso::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use control_acceso::database::repositories::empresa_repository::{
    EmpresaRepository, SqliteEmpresaRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::contratista::Contratista;
use control_acceso::models::empresa::Empresa;
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::services::contratista_service::ContratistaConsultaService;

fn preparar_base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
}

fn crear_empresa(connection: &Connection, nombre: &str) -> i64 {
    SqliteEmpresaRepository::new(connection)
        .crear(&Empresa {
            id: 0,
            nombre: nombre.to_owned(),
            activo: true,
        })
        .unwrap()
}

// Los argumentos reflejan directamente las columnas cuya conversión verifica cada prueba.
#[allow(clippy::too_many_arguments)]
fn crear_contratista(
    connection: &Connection,
    cedula: &str,
    nombre: &str,
    empresa_id: i64,
    tipo_ingreso: TipoIngreso,
    fecha: Option<NaiveDate>,
    ruta: bool,
    acceso: bool,
) -> i64 {
    SqliteContratistaRepository::new(connection)
        .crear(&Contratista {
            id: 0,
            cedula: cedula.to_owned(),
            nombre: nombre.to_owned(),
            empresa_id,
            tipo_ingreso,
            fecha_vencimiento_praind: fecha,
            es_personal_ruta: ruta,
            tiene_acceso: acceso,
            empresa_activa: true,
        })
        .unwrap()
}

fn buscar(
    connection: &Connection,
    texto: Option<&str>,
) -> Vec<control_acceso::database::queries::contratistas::ContratistaResumen> {
    SqliteContratistasQuery::new(connection)
        .buscar(&FiltroContratistas {
            texto: texto.map(str::to_owned),
            ..FiltroContratistas::default()
        })
        .unwrap()
        .items
}

#[test]
fn devuelve_contratista_con_nombre_de_empresa_en_un_solo_resultado() {
    let connection = preparar_base();
    let empresa_id = crear_empresa(&connection, "Constructora Alfa");
    crear_contratista(
        &connection,
        "1001",
        "Ana Solano",
        empresa_id,
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2027, 9, 20).unwrap()),
        false,
        true,
    );

    let resultados = buscar(&connection, None);

    assert_eq!(resultados.len(), 1);
    assert_eq!(resultados[0].empresa_nombre, "Constructora Alfa");
    assert_eq!(resultados[0].cedula, "1001");
}

#[test]
fn busca_por_cedula() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    crear_contratista(
        &connection,
        "155824",
        "Ana",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    crear_contratista(
        &connection,
        "999999",
        "Beatriz",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    assert_eq!(buscar(&connection, Some("5824"))[0].cedula, "155824");
}

#[test]
fn busca_parcialmente_por_nombre() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    crear_contratista(
        &connection,
        "1",
        "María Fernanda Mora",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    assert_eq!(
        buscar(&connection, Some("Fernanda"))[0].nombre,
        "María Fernanda Mora"
    );
}

#[test]
fn busca_por_nombre_de_empresa() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Servicios Electromecánicos");
    crear_contratista(
        &connection,
        "1",
        "Carlos",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    assert_eq!(
        buscar(&connection, Some("Electromecánicos"))[0].empresa_nombre,
        "Servicios Electromecánicos"
    );
}

#[test]
fn busqueda_ascii_es_case_insensitive() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "CONSTRUCTORA ALFA");
    crear_contratista(
        &connection,
        "1",
        "CARLOS ROJAS",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    assert_eq!(buscar(&connection, Some("carlos rojas")).len(), 1);
    assert_eq!(buscar(&connection, Some("constructora alfa")).len(), 1);
}

/// Regresión de "Búsquedas de 1-2 caracteres no pliegan tildes ni Ñ"
/// (`docs/hallazgos-buscador.md`): `COLLATE NOCASE` sólo pliega ASCII A-Z,
/// así que "os" no encontraba "Óscar" antes de la función SQL `PLEGAR`.
#[test]
fn busqueda_corta_pliega_tildes() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    crear_contratista(
        &connection,
        "1",
        "Óscar Peña",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    assert_eq!(buscar(&connection, Some("os")).len(), 1);
}

#[test]
fn texto_vacio_o_blancos_equivale_a_sin_filtro() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    crear_contratista(
        &connection,
        "1",
        "Ana",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    crear_contratista(
        &connection,
        "2",
        "Beto",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    assert_eq!(buscar(&connection, None), buscar(&connection, Some("")));
    assert_eq!(buscar(&connection, None), buscar(&connection, Some("   ")));
}

#[test]
fn aplica_trim_al_texto() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    crear_contratista(
        &connection,
        "1",
        "Persona Buscada",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    assert_eq!(buscar(&connection, Some("  Buscada  ")).len(), 1);
}

#[test]
fn limite_funciona_y_se_acota_a_un_minimo_seguro() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    for indice in 0..4 {
        crear_contratista(
            &connection,
            &format!("{indice}"),
            &format!("Persona {indice}"),
            empresa,
            TipoIngreso::Swat,
            None,
            false,
            true,
        );
    }
    let query = SqliteContratistasQuery::new(&connection);
    let pagina = query
        .buscar(&FiltroContratistas {
            texto: None,
            limite: 2,
            offset: 0,
            ..FiltroContratistas::default()
        })
        .unwrap();
    // El conteo real no se recorta por `limite` — así la UI puede avisar
    // "2 de 4" en vez de dejar los otros dos fuera en silencio.
    assert_eq!(pagina.items.len(), 2);
    assert_eq!(pagina.total, 4);
    assert_eq!(
        query
            .buscar(&FiltroContratistas {
                texto: None,
                limite: 0,
                offset: 0,
                ..FiltroContratistas::default()
            })
            .unwrap()
            .items
            .len(),
        1
    );
}

#[test]
fn filtra_por_empresa_tipo_praind_ruta_y_acceso() {
    use control_acceso::database::queries::contratistas::FiltroPraind;

    let connection = preparar_base();
    let empresa_uno = crear_empresa(&connection, "Brisas");
    let empresa_dos = crear_empresa(&connection, "Expenic");
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();

    // PRAIND vencido, de ruta, sin acceso, empresa uno.
    crear_contratista(
        &connection,
        "1",
        "Ana",
        empresa_uno,
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        true,
        false,
    );
    // PRAIND próximo a vencer (dentro de 30 días), empresa dos.
    crear_contratista(
        &connection,
        "2",
        "Beto",
        empresa_dos,
        TipoIngreso::Praind,
        Some(hoy + chrono::Duration::days(10)),
        false,
        true,
    );
    // Sin fecha PRAIND (no la requiere), empresa uno.
    crear_contratista(
        &connection,
        "3",
        "Caro",
        empresa_uno,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    // Cambió a un tipo que ya no requiere PRAIND pero conserva una fecha
    // vencida sin limpiar — no debe contar como "vencido" (empresa dos).
    crear_contratista(
        &connection,
        "4",
        "Dani",
        empresa_dos,
        TipoIngreso::PorCorreo,
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        false,
        true,
    );

    let query = SqliteContratistasQuery::new(&connection);

    let base = || FiltroContratistas::default();

    assert_eq!(
        query
            .buscar(&FiltroContratistas {
                empresa_id: Some(Igualdad::Incluye(empresa_uno)),
                ..base()
            })
            .unwrap()
            .items
            .len(),
        2
    );

    assert_eq!(
        query
            .buscar(&FiltroContratistas {
                tipos_incluidos: Some(vec![TipoIngreso::Praind, TipoIngreso::Swat]),
                ..base()
            })
            .unwrap()
            .items
            .len(),
        3
    );
    assert_eq!(
        query
            .buscar(&FiltroContratistas {
                tipos_incluidos: Some(vec![TipoIngreso::InHouse]),
                ..base()
            })
            .unwrap()
            .items
            .len(),
        0
    );

    let vencidos = query
        .buscar(&FiltroContratistas {
            praind: Some(FiltroPraind::Vencido { hoy }),
            ..base()
        })
        .unwrap()
        .items;
    assert_eq!(vencidos.len(), 1);
    assert_eq!(vencidos[0].nombre, "Ana");
    assert!(
        !vencidos.iter().any(|c| c.nombre == "Dani"),
        "Dani ya no requiere PRAIND (PorCorreo) aunque conserve una fecha vencida sin limpiar"
    );

    let proximos = query
        .buscar(&FiltroContratistas {
            praind: Some(FiltroPraind::ProximoAVencer { hoy }),
            ..base()
        })
        .unwrap()
        .items;
    assert_eq!(proximos.len(), 1);
    assert_eq!(proximos[0].nombre, "Beto");

    let sin_fecha = query
        .buscar(&FiltroContratistas {
            praind: Some(FiltroPraind::SinFecha),
            ..base()
        })
        .unwrap()
        .items;
    assert_eq!(sin_fecha.len(), 1);
    assert_eq!(sin_fecha[0].nombre, "Caro");

    assert_eq!(
        query
            .buscar(&FiltroContratistas {
                personal_ruta: Some(true),
                ..base()
            })
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        query
            .buscar(&FiltroContratistas {
                tiene_acceso: Some(false),
                ..base()
            })
            .unwrap()
            .items
            .len(),
        1
    );
}

#[test]
fn el_total_se_mantiene_igual_al_avanzar_de_pagina() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    for indice in 0..5 {
        crear_contratista(
            &connection,
            &format!("{indice}"),
            &format!("Persona {indice}"),
            empresa,
            TipoIngreso::Swat,
            None,
            false,
            true,
        );
    }
    let query = SqliteContratistasQuery::new(&connection);
    let filtro = |offset| FiltroContratistas {
        texto: None,
        limite: 2,
        offset,
        ..FiltroContratistas::default()
    };

    let primera = query.buscar(&filtro(0)).unwrap();
    let segunda = query.buscar(&filtro(2)).unwrap();
    let tercera = query.buscar(&filtro(4)).unwrap();

    assert_eq!(primera.total, 5);
    assert_eq!(segunda.total, 5);
    assert_eq!(tercera.total, 5);
    assert_eq!(primera.items.len(), 2);
    assert_eq!(segunda.items.len(), 2);
    assert_eq!(tercera.items.len(), 1);
    // Sin solapamiento ni huecos entre páginas.
    let nombres: Vec<_> = primera
        .items
        .iter()
        .chain(segunda.items.iter())
        .chain(tercera.items.iter())
        .map(|c| c.nombre.clone())
        .collect();
    assert_eq!(nombres.len(), 5);
    let mut unicos = nombres.clone();
    unicos.sort();
    unicos.dedup();
    assert_eq!(unicos.len(), 5);
}

#[test]
fn offset_funciona_sobre_el_orden_estable() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    crear_contratista(
        &connection,
        "1",
        "Ana",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    crear_contratista(
        &connection,
        "2",
        "Beto",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    let resultado = SqliteContratistasQuery::new(&connection)
        .buscar(&FiltroContratistas {
            texto: None,
            limite: 1,
            offset: 1,
            ..FiltroContratistas::default()
        })
        .unwrap()
        .items;
    assert_eq!(resultado[0].nombre, "Beto");
}

#[test]
fn ordena_por_nombre_sin_distinguir_mayusculas_y_desempata_por_id() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    let primero = crear_contratista(
        &connection,
        "1",
        "ana",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    let segundo = crear_contratista(
        &connection,
        "2",
        "Ana",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    crear_contratista(
        &connection,
        "3",
        "Beto",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    let ids: Vec<_> = buscar(&connection, None)
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(ids, vec![primero, segundo, 3]);
}

#[test]
fn convierte_fecha_some_y_none() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    let fecha = NaiveDate::from_ymd_opt(2027, 12, 31).unwrap();
    crear_contratista(
        &connection,
        "1",
        "Con fecha",
        empresa,
        TipoIngreso::Praind,
        Some(fecha),
        false,
        true,
    );
    crear_contratista(
        &connection,
        "2",
        "Sin fecha",
        empresa,
        TipoIngreso::Swat,
        None,
        false,
        true,
    );
    let resultados = buscar(&connection, None);
    assert_eq!(resultados[0].fecha_vencimiento_praind, Some(fecha));
    assert_eq!(resultados[1].fecha_vencimiento_praind, None);
}

#[test]
fn convierte_booleanos_de_ruta_y_acceso() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    crear_contratista(
        &connection,
        "1",
        "Ruta bloqueada",
        empresa,
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
        true,
        false,
    );
    let resultado = &buscar(&connection, None)[0];
    assert!(resultado.es_personal_ruta);
    assert!(!resultado.tiene_acceso);
}

#[test]
fn convierte_los_cuatro_tipos_de_ingreso() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Brisas");
    for (indice, tipo) in [
        TipoIngreso::Praind,
        TipoIngreso::InHouse,
        TipoIngreso::PorCorreo,
        TipoIngreso::Swat,
    ]
    .into_iter()
    .enumerate()
    {
        crear_contratista(
            &connection,
            &format!("{indice}"),
            &format!("Persona {indice}"),
            empresa,
            tipo,
            None,
            false,
            true,
        );
    }
    let tipos: Vec<_> = buscar(&connection, None)
        .into_iter()
        .map(|c| c.tipo_ingreso)
        .collect();
    assert_eq!(
        tipos,
        vec![
            TipoIngreso::Praind,
            TipoIngreso::InHouse,
            TipoIngreso::PorCorreo,
            TipoIngreso::Swat
        ]
    );
}

#[test]
fn foreign_key_impide_estado_con_empresa_inexistente() {
    let connection = preparar_base();
    let resultado = connection.execute("INSERT INTO contratistas (cedula, nombre, empresa_id, tipo_ingreso, fecha_vencimiento_praind, es_personal_ruta, tiene_acceso) VALUES (?1, ?2, 999, 'SWAT', NULL, 0, 1)", params!["1", "Inválido"]);
    assert!(resultado.is_err());
    assert!(buscar(&connection, None).is_empty());
}

#[test]
fn service_devuelve_read_model_sin_perder_datos() {
    let connection = preparar_base();
    let empresa = crear_empresa(&connection, "Empresa completa");
    let fecha = NaiveDate::from_ymd_opt(2028, 6, 30).unwrap();
    let id = crear_contratista(
        &connection,
        "7001",
        "Persona Completa",
        empresa,
        TipoIngreso::InHouse,
        Some(fecha),
        true,
        false,
    );
    let query = SqliteContratistasQuery::new(&connection);
    let servicio = ContratistaConsultaService::new(&query);

    let resultado = servicio
        .buscar_para_tabla(&FiltroContratistas::default())
        .unwrap()
        .items;

    assert_eq!(resultado[0].id, id);
    assert_eq!(resultado[0].empresa_nombre, "Empresa completa");
    assert_eq!(resultado[0].fecha_vencimiento_praind, Some(fecha));
    assert_eq!(resultado[0].tipo_ingreso, TipoIngreso::InHouse);
    assert!(resultado[0].es_personal_ruta);
    assert!(!resultado[0].tiene_acceso);
}

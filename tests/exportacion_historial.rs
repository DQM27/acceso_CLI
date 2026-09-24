use chrono::{NaiveDate, Utc};
use rusqlite::{Connection, params};

use control_acceso::{
    application::{AppCore, ExportarHistorialError, exportar_tabla_xlsx},
    database::{queries::ingresos::FiltroHistorial, schema::initialize_database},
    historial::{ColumnaHistorial, exportacion::ColumnaTabla},
    models::{medio_ingreso::MedioIngreso, tipo_ingreso::TipoIngreso},
    tiempo::serializar_utc,
};

fn instante(dia: u32, hora: u32) -> chrono::DateTime<Utc> {
    chrono::DateTime::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2026, 8, dia)
            .unwrap()
            .and_hms_opt(hora, 0, 0)
            .unwrap(),
        Utc,
    )
}

fn uuid_local(numero: usize) -> String {
    format!("local-{numero}")
}

fn uuids(valores: &[&str]) -> Vec<String> {
    valores.iter().map(|valor| (*valor).to_owned()).collect()
}

/// Un movimiento de otro dispositivo del sitio, tal como lo deja
/// `recibir_historial_del_sitio` en `historial_sitio`. `tipo`/`medio` en
/// `None` reproducen filas viejas sincronizadas antes de que la nube
/// cargara esas columnas.
fn insertar_remoto(
    connection: &Connection,
    uuid: &str,
    hora_entrada: chrono::DateTime<Utc>,
    tipo: Option<&str>,
    medio: Option<&str>,
) {
    connection
        .execute(
            "INSERT INTO historial_sitio(uuid,sitio_id,contratista_cedula,contratista_nombre,\
             empresa_nombre,tipo_ingreso,medio_ingreso,hora_entrada,hora_salida,gafete_numero,\
             usuario_entrada_nombre,usuario_salida_nombre,dispositivo_entrada_id,actualizado_en) \
             VALUES (?1,'sitio-1','00202','Beto Rojas','Brisas',?2,?3,?4,NULL,3,\
             'Celular',NULL,'disp-movil',?4)",
            params![uuid, tipo, medio, serializar_utc(hora_entrada)],
        )
        .unwrap();
}

fn core_con_movimientos(cantidad: usize) -> AppCore {
    AppCore::new(conexion_con_movimientos(cantidad))
}

fn conexion_con_movimientos(cantidad: usize) -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute("INSERT INTO empresas(nombre) VALUES ('Brisas')", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO usuarios(cedula,nombre,password_hash,rol,activo) \
             VALUES ('u1','Quintana','hash','OPERADOR',1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO contratistas(cedula,nombre,empresa_id,tipo_ingreso,\
             fecha_vencimiento_praind,es_personal_ruta,tiene_acceso) \
             VALUES ('00101','Ana Solano',1,'PRAIND','2027-01-01',0,1)",
            [],
        )
        .unwrap();

    for indice in 0..cantidad {
        let ingreso = serializar_utc(instante(20, 8));
        let salida = serializar_utc(instante(20, 9));
        connection
            .execute(
                "INSERT INTO registro_ingresos(contratista_id,empresa_id,\
                 fecha_hora_ingreso,medio_ingreso,tipo_ingreso,gafete_numero,\
                 usuario_ingreso_id,fecha_hora_salida,usuario_salida_id,\
                 contratista_cedula,contratista_nombre,empresa_nombre,\
                 usuario_ingreso_nombre,usuario_salida_nombre,fecha_vencimiento_praind,\
                 es_personal_ruta,tiene_acceso,resultado_acceso,motivo_resultado,reglas_version,uuid) \
                 VALUES (1,1,?1,'CAMINANDO','PRAIND',?2,1,?3,1,\
                 '00101','Ana Solano','Brisas','Quintana','Quintana','2027-01-01',\
                 0,1,'PERMITIDO',NULL,1,?4)",
                params![
                    ingreso,
                    i64::try_from(indice).unwrap_or(i64::MAX) + 1,
                    salida,
                    uuid_local(indice + 1)
                ],
            )
            .unwrap();
    }
    connection
}

/// La GUI (`buscar_historial_completo`) trae todo el conjunto de una vez
/// para que AG Grid virtualice del lado del cliente — tiene que funcionar
/// aunque el total cruce el límite de una página SQL (200), mismo criterio
/// que `buscar_auditoria_completo_trae_todo_aunque_supere_una_pagina_sql`
/// (`tests/auditoria_contratistas.rs`).
#[test]
fn buscar_historial_completo_trae_todo_aunque_supere_una_pagina_sql() {
    let core = core_con_movimientos(205);
    let filtro = FiltroHistorial::nuevo(instante(1, 0), instante(31, 23));

    let todos = core.buscar_historial_completo(&filtro).unwrap();

    assert_eq!(todos.items.len(), 205);
    assert!(!todos.truncado);
}

#[test]
fn exporta_todos_los_resultados_aunque_superen_una_pagina_sql() {
    let core = core_con_movimientos(205);
    let filtro = FiltroHistorial::nuevo(instante(1, 0), instante(31, 23));
    let directorio = tempfile::tempdir().unwrap();
    let destino = directorio.path().join("historial.xlsx");

    let exportados = core
        .exportar_historial(
            &filtro,
            &[ColumnaHistorial::FechaIngreso, ColumnaHistorial::Nombre],
            &destino,
        )
        .unwrap();

    assert_eq!(exportados, 205);
    let contenido = std::fs::read(&destino).unwrap();
    assert!(contenido.starts_with(b"PK"));
    assert!(matches!(
        core.exportar_historial(&filtro, &[ColumnaHistorial::Nombre], &destino),
        Err(ExportarHistorialError::DestinoExiste(_))
    ));
}

/// La GUI filtra del lado del cliente (AG Grid) y manda sólo los `uuid`
/// que quedaron visibles — `exportar_historial_seleccion` tiene que
/// recortar a esos movimientos aunque el conjunto sin acotar cruce el
/// límite de una página SQL (200), es decir que el recorte no puede
/// depender de que todo entre en una sola página.
#[test]
fn exporta_solo_los_uuids_seleccionados_aunque_crucen_una_pagina_sql() {
    let core = core_con_movimientos(205);
    let filtro = FiltroHistorial::nuevo(instante(1, 0), instante(31, 23));
    let directorio = tempfile::tempdir().unwrap();
    let destino = directorio.path().join("historial.xlsx");

    // Se elige un subconjunto que cruza el límite de página (200): algunos
    // antes, algunos después. Orden a propósito fuera de lo cronológico:
    // `exportar_historial_seleccion` debe escribir en ESTE orden, no en el
    // que devuelve la consulta SQL.
    let seleccion = uuids(&[
        "local-199",
        "local-1",
        "local-205",
        "local-100",
        "local-200",
        "local-201",
    ]);

    let exportados = core
        .exportar_historial_seleccion(
            &filtro,
            Some(&seleccion),
            &[ColumnaHistorial::FechaIngreso, ColumnaHistorial::Nombre],
            &destino,
        )
        .unwrap();

    assert_eq!(exportados, seleccion.len());
    let contenido = std::fs::read(&destino).unwrap();
    assert!(contenido.starts_with(b"PK"));
}

/// `rust_xlsxwriter` sólo escribe, no lee — así que el orden de escritura
/// se prueba a nivel de `movimientos_en_orden` (lo que
/// `exportar_historial_seleccion` usa por dentro cuando hay `uuids`) en vez
/// de abrir el XLSX resultante. Orden a propósito NO cronológico (la GUI
/// manda el orden visible en pantalla tras un reordenamiento de columnas,
/// no el orden de la consulta SQL).
#[test]
fn movimientos_en_orden_respeta_el_orden_de_uuids_no_el_de_la_consulta() {
    let core = core_con_movimientos(205);
    let filtro = FiltroHistorial::nuevo(instante(1, 0), instante(31, 23));
    let seleccion = uuids(&["local-199", "local-1", "local-205", "local-100"]);

    let movimientos = core.movimientos_en_orden(&filtro, &seleccion).unwrap();

    let resultantes: Vec<String> = movimientos.into_iter().map(|m| m.uuid).collect();
    assert_eq!(resultantes, seleccion);
}

/// Un uuid que no existe en el conjunto filtrado (foto vieja de la grilla)
/// se omite en silencio en vez de fallar toda la exportación.
#[test]
fn movimientos_en_orden_omite_uuids_inexistentes() {
    let core = core_con_movimientos(5);
    let filtro = FiltroHistorial::nuevo(instante(1, 0), instante(31, 23));
    let seleccion = uuids(&["local-3", "no-existe", "local-1"]);

    let movimientos = core.movimientos_en_orden(&filtro, &seleccion).unwrap();

    let resultantes: Vec<String> = movimientos.into_iter().map(|m| m.uuid).collect();
    assert_eq!(resultantes, uuids(&["local-3", "local-1"]));
}

/// Bug reportado por el usuario 2026-09-23: Excel/PDF sólo traían los
/// ingresos registrados en esta PC, los de otro dispositivo del sitio
/// (`historial_sitio`) quedaban afuera aunque se vieran en la grilla.
#[test]
fn movimientos_en_orden_incluye_movimientos_de_otro_dispositivo() {
    let connection = conexion_con_movimientos(3);
    insertar_remoto(
        &connection,
        "remoto-1",
        instante(21, 10),
        Some("SWAT"),
        Some("VEHICULO"),
    );
    // Fila vieja sin tipo/medio: igual sale, sin inventar valores.
    insertar_remoto(&connection, "remoto-2", instante(22, 10), None, None);
    let core = AppCore::new(connection);
    let filtro = FiltroHistorial::nuevo(instante(1, 0), instante(31, 23));
    let seleccion = uuids(&["remoto-2", "local-1", "remoto-1"]);

    let movimientos = core.movimientos_en_orden(&filtro, &seleccion).unwrap();

    let resultantes: Vec<String> = movimientos.iter().map(|m| m.uuid.clone()).collect();
    assert_eq!(resultantes, seleccion);
    assert_eq!(movimientos[0].tipo_ingreso, None);
    assert_eq!(movimientos[0].medio_ingreso, None);
    assert_eq!(movimientos[2].tipo_ingreso, Some(TipoIngreso::Swat));
    assert_eq!(movimientos[2].medio_ingreso, Some(MedioIngreso::Vehiculo));
    assert_eq!(movimientos[2].contratista_nombre, "Beto Rojas");
}

/// `historial_sitio` también refleja los movimientos de ESTE dispositivo
/// (respaldo ante reinstalación) — para esos manda la fila local, y nunca
/// deben salir duplicados en el archivo.
#[test]
fn movimientos_completos_mezcla_remotos_sin_duplicar_los_propios() {
    let connection = conexion_con_movimientos(2);
    insertar_remoto(
        &connection,
        "local-1",
        instante(20, 8),
        Some("PRAIND"),
        Some("CAMINANDO"),
    );
    insertar_remoto(
        &connection,
        "remoto-1",
        instante(25, 10),
        Some("PRAIND"),
        Some("CAMINANDO"),
    );
    let core = AppCore::new(connection);
    let filtro = FiltroHistorial::nuevo(instante(1, 0), instante(31, 23));

    let movimientos = core.movimientos_completos(&filtro).unwrap();

    let resultantes: Vec<String> = movimientos.iter().map(|m| m.uuid.clone()).collect();
    // Más nuevo primero, igual que la grilla.
    assert_eq!(resultantes[0], "remoto-1");
    assert_eq!(resultantes.len(), 3);
    let propio = movimientos.iter().find(|m| m.uuid == "local-1").unwrap();
    assert_eq!(propio.contratista_nombre, "Ana Solano");
}

/// El camino sin recorte (`uuids: None`, rango truncado en la GUI) también
/// incluye lo de otros dispositivos, sin duplicar los propios.
#[test]
fn exportar_todo_el_rango_incluye_movimientos_de_otro_dispositivo() {
    let connection = conexion_con_movimientos(4);
    insertar_remoto(
        &connection,
        "local-2",
        instante(20, 8),
        Some("PRAIND"),
        Some("CAMINANDO"),
    );
    insertar_remoto(
        &connection,
        "remoto-1",
        instante(21, 10),
        Some("PRAIND"),
        Some("CAMINANDO"),
    );
    insertar_remoto(
        &connection,
        "remoto-fuera",
        instante(1, 0) - chrono::Duration::days(1),
        None,
        None,
    );
    let core = AppCore::new(connection);
    let filtro = FiltroHistorial::nuevo(instante(1, 0), instante(31, 23));
    let directorio = tempfile::tempdir().unwrap();
    let destino = directorio.path().join("historial.xlsx");

    let exportados = core
        .exportar_historial(&filtro, &[ColumnaHistorial::Nombre], &destino)
        .unwrap();

    assert_eq!(exportados, 5);
}

/// Tabla genérica (historial de proveedores y de KOF en escritorio): arma
/// el XLSX, rechaza una tabla sin columnas y nunca pisa un archivo que ya
/// existe (mismo criterio que el exportador de Historial).
#[test]
fn exporta_una_tabla_generica_a_xlsx() {
    let directorio = tempfile::tempdir().unwrap();
    let destino = directorio.path().join("proveedores.xlsx");
    let columnas = vec![
        ColumnaTabla {
            titulo: "NOMBRE".into(),
            izquierda: true,
        },
        ColumnaTabla {
            titulo: "EMPRESA".into(),
            izquierda: false,
        },
    ];
    let filas = vec![
        vec!["Ana Solano".to_owned(), "BELCA".to_owned()],
        vec!["Beto Rojas".to_owned(), "BOCATA".to_owned()],
    ];

    assert_eq!(exportar_tabla_xlsx(&columnas, &filas, &destino).unwrap(), 2);
    assert!(std::fs::read(&destino).unwrap().starts_with(b"PK"));
    assert!(matches!(
        exportar_tabla_xlsx(&columnas, &filas, &destino),
        Err(ExportarHistorialError::DestinoExiste(_))
    ));
    assert!(matches!(
        exportar_tabla_xlsx(&[], &filas, &directorio.path().join("otra.xlsx")),
        Err(ExportarHistorialError::SinColumnas)
    ));
}

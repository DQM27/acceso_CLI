use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use rusqlite::{Connection, params};

use crate::database::schema::initialize_database;

use super::paginado::TAMANO_PAGINA_REMOTA;
use super::*;

#[test]
fn token_dispositivo_vencido_solo_detecta_401() {
    assert!(
        SincronizacionError::RespuestaInesperada {
            status: 401,
            cuerpo: "jwt expired".to_string(),
        }
        .token_dispositivo_vencido()
    );
    assert!(
        !SincronizacionError::RespuestaInesperada {
            status: 403,
            cuerpo: "forbidden".to_string(),
        }
        .token_dispositivo_vencido()
    );
    assert!(!SincronizacionError::FechaInvalida("x".to_string()).token_dispositivo_vencido());
}

fn servidor_de_una_respuesta(respuesta: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
    let direccion = listener.local_addr().expect("dirección local");
    thread::spawn(move || {
        let Ok((mut conexion, _)) = listener.accept() else {
            return;
        };
        conexion
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("set_read_timeout");
        let mut buffer = [0_u8; 4096];
        loop {
            match conexion.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_leidos) => {}
            }
        }
        let _ = conexion.write_all(respuesta.as_bytes());
        let _ = conexion.flush();
    });
    format!("http://{direccion}")
}

/// Como `servidor_de_una_respuesta`, pero para pruebas que disparan más
/// de un `GET` (`recibir_catalogo_del_sitio` pide primero empresas y
/// luego contratistas) -- una respuesta por conexión aceptada, en
/// orden. Cada respuesta debe traer `Connection: close` para que el
/// cliente abra una conexión nueva en el siguiente pedido.
fn servidor_de_respuestas(respuestas: Vec<&'static str>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
    let direccion = listener.local_addr().expect("dirección local");
    thread::spawn(move || {
        for respuesta in respuestas {
            let Ok((mut conexion, _)) = listener.accept() else {
                return;
            };
            conexion
                .set_read_timeout(Some(Duration::from_millis(200)))
                .expect("set_read_timeout");
            let mut buffer = [0_u8; 4096];
            loop {
                match conexion.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(_leidos) => {}
                }
            }
            let _ = conexion.write_all(respuesta.as_bytes());
            let _ = conexion.flush();
        }
    });
    format!("http://{direccion}")
}

/// Prueba directa del hallazgo R-06: antes, `pendientes()` filtraba con
/// una expresión (`datetime(actualizado_en, ...)`) que `SQLite` no podía
/// resolver con ningún índice -- cada `drenar_cola` escaneaba toda
/// `cola_salida` pendiente. Con `proximo_intento_en` como columna
/// generada e indexada (`MIGRACION_27`), el plan de consulta real de
/// `pendientes()` debe usar `idx_cola_salida_pendientes` en vez de un
/// `SCAN cola_salida`.
#[test]
fn la_consulta_de_pendientes_usa_el_indice_no_un_escaneo_completo() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN
             SELECT id, entidad, entidad_uuid, operacion, intentos FROM cola_salida
             WHERE estado = 'pendiente' AND proximo_intento_en <= datetime('now')
             ORDER BY creado_en
             LIMIT 200",
            [],
            |row| row.get::<_, String>(3),
        )
        .unwrap();

    assert!(
        plan.contains("idx_cola_salida_pendientes"),
        "esperaba que el plan usara el índice, se obtuvo: {plan}"
    );
    assert!(
        !plan.to_uppercase().contains("SCAN"),
        "esperaba una búsqueda por índice, no un escaneo completo: {plan}"
    );
}

fn conexion_con_contratista() -> (Connection, String) {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute("INSERT INTO empresas (nombre) VALUES ('Brisas')", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso,
                es_personal_ruta, tiene_acceso, uuid
            ) VALUES ('1-2345', 'Persona de prueba', 1, 'SWAT', 0, 1, 'uuid-contratista')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('contratista', 'uuid-contratista', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    (connection, "uuid-contratista".to_string())
}

fn contexto(base_url: &str) -> ContextoSincronizacion<'_> {
    ContextoSincronizacion {
        base_url,
        apikey: "clave-de-prueba",
        token: "token-de-prueba",
        dispositivo_id: "dispositivo-1",
        sitio_id: "sitio-1",
    }
}

#[test]
fn envia_una_empresa_pendiente_y_la_marca_enviada() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas (nombre, activo, uuid)
             VALUES ('Brisas', 1, 'uuid-empresa')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('empresa', 'uuid-empresa', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    let estado: String = connection
        .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
        .unwrap();
    assert_eq!(estado, "enviado");
}

#[test]
fn envia_un_gafete_pendiente_y_lo_marca_enviado() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO gafetes (numero, tipo, estado, uuid)
             VALUES (5, 'CONTRATISTA', 'DISPONIBLE', 'uuid-gafete')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('gafete', 'uuid-gafete', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    let estado: String = connection
        .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
        .unwrap();
    assert_eq!(estado, "enviado");
}

#[test]
fn envia_un_contratista_pendiente_y_lo_marca_enviado() {
    let (connection, _uuid) = conexion_con_contratista();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    let estado: String = connection
        .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
        .unwrap();
    assert_eq!(estado, "enviado");
}

/// Tres empresas nuevas -- sin relación entre sí, así que ninguna
/// depende de que otra fila del lote se haya aplicado antes.
fn conexion_con_tres_empresas() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    for n in 1..=3 {
        connection
            .execute(
                &format!(
                    "INSERT INTO empresas (nombre, activo, uuid)
                     VALUES ('Empresa {n}', 1, 'uuid-empresa-{n}')"
                ),
                [],
            )
            .unwrap();
        connection
            .execute(
                &format!(
                    "INSERT INTO cola_salida (
                        entidad, entidad_uuid, operacion, creado_en, actualizado_en
                    ) VALUES ('empresa', 'uuid-empresa-{n}', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
                ),
                [],
            )
            .unwrap();
    }
    connection
}

#[test]
fn agrupa_varias_filas_del_mismo_tipo_y_las_manda_en_un_solo_lote() {
    let connection = conexion_con_tres_empresas();
    // Una sola respuesta -- si `drenar_cola` mandara una petición por
    // fila (comportamiento viejo), la segunda y tercera empresa se
    // quedarían sin servidor que les conteste y la prueba fallaría por
    // timeout/error de red en vez de pasar.
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 3,
            fallidos: 0,
            ..Default::default()
        }
    );
    let enviadas: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM cola_salida WHERE estado = 'enviado'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(enviadas, 3);
}

#[test]
fn si_el_lote_completo_falla_cae_a_mandar_cada_fila_por_separado() {
    let connection = conexion_con_tres_empresas();
    // Primera respuesta (al intento de lote): error -- simula que
    // Postgres rechazó el array entero por una sola fila mala. El
    // fallback reintenta las tres filas por separado, pero acá sólo se
    // preparan dos respuestas más a propósito: la prueba verifica que
    // esas dos se marcan `enviado` igual, y que a la tercera (sin
    // respuesta esperándola, como un error de red real) no se la
    // pierde ni se la cuenta como enviada -- queda `pendiente` para el
    // próximo intento.
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"error\":\"fila invalida\"}",
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    ]);

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    // El lote falló una vez (no cuenta como fallo por fila), y las tres
    // filas individuales se reintentaron dentro de la misma llamada:
    // dos con éxito. La tercera no tiene respuesta preparada -- se
    // queda pendiente, igual que pasaría con un error de red real.
    assert_eq!(resumen.enviados, 2);
    let pendientes: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM cola_salida WHERE estado = 'pendiente'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        pendientes, 1,
        "la tercera fila queda para reintentar, no perdida ni duplicada"
    );
}

#[test]
fn error_del_receptor_deja_la_fila_pendiente_para_reintentar_con_el_motivo_guardado() {
    let (connection, _uuid) = conexion_con_contratista();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\
         Connection: close\r\n\r\n{\"error\":\"boom\"}",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 0,
            fallidos: 1,
            ..Default::default()
        }
    );
    let (estado, intentos, ultimo_error): (String, i64, Option<String>) = connection
        .query_row(
            "SELECT estado, intentos, ultimo_error FROM cola_salida",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    // "pendiente", no "fallido" -- un solo fallo todavía se reintenta
    // solo (con backoff), no es un fallo permanente.
    assert_eq!(estado, "pendiente");
    assert_eq!(intentos, 1);
    assert!(ultimo_error.unwrap().contains("500"));
}

#[test]
fn una_fila_recien_fallida_no_se_reintenta_de_inmediato_por_el_backoff() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, estado, intentos,
                creado_en, actualizado_en
            ) VALUES (
                'contratista', 'uuid-x', 'crear', 'pendiente', 1,
                '2026-01-01T00:00:00Z', strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            )",
            [],
        )
        .unwrap();
    // Nunca levanta un servidor: si `pendientes()` la trajera igual, la
    // conexión fallaría y el test lo detectaría por el resumen.
    let resumen = drenar_cola(&connection, &contexto("http://127.0.0.1:1"), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 0,
            fallidos: 0,
            ..Default::default()
        }
    );
}

#[test]
fn tras_agotar_los_reintentos_la_fila_queda_fallida_de_forma_permanente() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, estado, intentos,
                creado_en, actualizado_en
            ) VALUES (
                'contratista', 'uuid-x', 'crear', 'pendiente',
                ?1, '2026-01-01T00:00:00Z', '2020-01-01T00:00:00Z'
            )",
            params![INTENTOS_ANTES_DE_FALLO_PERMANENTE - 1],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\
         Connection: close\r\n\r\n{\"error\":\"boom\"}",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 0,
            fallidos: 1,
            ..Default::default()
        }
    );
    let estado: String = connection
        .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
        .unwrap();
    assert_eq!(estado, "fallido");
    assert_eq!(contar_fallos_permanentes(&connection).unwrap(), 1);
}

/// Cita + visitante + movimiento de visita completo (con snapshot),
/// listo para encolar -- devuelve el `uuid` del movimiento.
fn conexion_con_movimiento_de_visita(fecha_hora_salida: Option<&str>) -> (Connection, String) {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
             VALUES ('1', 'Guardia', 'h', 'OPERADOR', 1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO citas (uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                anfitrion_correo, estado, creado_en)
             VALUES ('uuid-cita', '2026-01-01', '2026-01-02', 'Anfitrión',
                'anfitrion@ejemplo.com', 'VIGENTE', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cita_visitantes (uuid, cita_id, cedula, nombre)
             VALUES ('uuid-visitante', 1, '1-2345', 'Persona Visitante')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO movimientos_visita (
                uuid, cita_visitante_id, gafete_numero, fecha_hora_entrada,
                fecha_hora_salida, usuario_entrada_id, usuario_entrada_nombre,
                visitante_cedula, visitante_nombre, empresa, anfitrion_nombre, motivo
            ) VALUES (
                'uuid-movimiento', 1, 12, '2026-01-01T08:00:00Z',
                ?1, 1, 'Guardia',
                '1-2345', 'Persona Visitante', 'Brisas', 'Anfitrión', 'Auditoría'
            )",
            params![fecha_hora_salida],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES (
                'movimiento_visita', 'uuid-movimiento', ?1,
                '2026-01-01T08:00:00Z', '2026-01-01T08:00:00Z'
            )",
            params![if fecha_hora_salida.is_some() {
                "cerrar"
            } else {
                "crear"
            }],
        )
        .unwrap();
    (connection, "uuid-movimiento".to_string())
}

#[test]
fn envia_la_apertura_de_un_movimiento_de_visita() {
    let (connection, _) = conexion_con_movimiento_de_visita(None);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.starts_with("POST /rest/v1/movimientos_visita "));
        let cuerpo = "[]";
        write!(
            socket,
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    servidor.join().unwrap();
}

#[test]
fn envia_el_cierre_de_un_movimiento_de_visita() {
    let (connection, _) = conexion_con_movimiento_de_visita(Some("2026-01-01T09:00:00Z"));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.starts_with(
            "PATCH /rest/v1/movimientos_visita?id=eq.uuid-movimiento&hora_salida=is.null "
        ));
        write!(
            socket,
            "HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
        )
        .unwrap();
    });

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    servidor.join().unwrap();
}

#[test]
fn envia_una_ruta_pendiente_y_la_marca_enviada() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO rutas (numero, activo, uuid) VALUES (79, 1, 'uuid-ruta-79')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('ruta', 'uuid-ruta-79', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    let estado: String = connection
        .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
        .unwrap();
    assert_eq!(estado, "enviado");
}

#[test]
fn envia_un_vehiculo_ruta_pendiente_y_lo_marca_enviado() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO vehiculos_ruta (numero_unidad, placa, activo, uuid)
             VALUES ('22906', 'C12345', 1, 'uuid-vehiculo')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('vehiculo_ruta', 'uuid-vehiculo', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    let estado: String = connection
        .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
        .unwrap();
    assert_eq!(estado, "enviado");
}

#[test]
fn envia_un_encargado_ruta_pendiente_y_lo_marca_enviado() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO encargados_ruta (codigo_empleado, nombre, activo, uuid)
             VALUES ('5040017', 'Michael Araya Retana', 1, 'uuid-encargado')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('encargado_ruta', 'uuid-encargado', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    let estado: String = connection
        .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
        .unwrap();
    assert_eq!(estado, "enviado");
}

#[test]
fn envia_una_empresa_proveedor_pendiente_y_la_marca_enviada() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas_proveedor (nombre, activo, uuid)
             VALUES ('Maika', 1, 'uuid-empresa-proveedor')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('empresa_proveedor', 'uuid-empresa-proveedor', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    let estado: String = connection
        .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
        .unwrap();
    assert_eq!(estado, "enviado");
}

/// Ingreso de proveedor listo para encolar, mismo criterio que
/// `conexion_con_movimiento_de_visita` -- `fecha_hora_salida` en `Some`
/// simula un ingreso ya cerrado, listo para el cierre.
fn conexion_con_ingreso_proveedor(fecha_hora_salida: Option<&str>) -> (Connection, String) {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
             VALUES ('1', 'Guardia', 'h', 'OPERADOR', 1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO empresas_proveedor (id, nombre, activo, uuid)
             VALUES (1, 'Maika', 1, 'uuid-empresa-proveedor')",
            [],
        )
        .unwrap();
    let usuario_salida_id = fecha_hora_salida.is_some().then_some(1_i64);
    let usuario_salida_nombre = fecha_hora_salida.is_some().then_some("Guardia");
    connection
        .execute(
            "INSERT INTO registro_ingresos_proveedor (
                uuid, cedula, nombre, empresa_id, empresa_nombre, placa, gafete_numero,
                fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre,
                fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
            ) VALUES (
                'uuid-ingreso-proveedor', '1-2345', 'Juan Perez', 1, 'Maika', NULL, 12,
                '2026-01-01T08:00:00Z', 1, 'Guardia',
                ?1, ?2, ?3
            )",
            params![fecha_hora_salida, usuario_salida_id, usuario_salida_nombre],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES (
                'ingreso_proveedor', 'uuid-ingreso-proveedor', ?1,
                '2026-01-01T08:00:00Z', '2026-01-01T08:00:00Z'
            )",
            params![if fecha_hora_salida.is_some() {
                "cerrar"
            } else {
                "crear"
            }],
        )
        .unwrap();
    (connection, "uuid-ingreso-proveedor".to_string())
}

#[test]
fn envia_la_apertura_de_un_ingreso_proveedor() {
    let (connection, _) = conexion_con_ingreso_proveedor(None);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.starts_with("POST /rest/v1/ingresos_proveedor "));
        let cuerpo = "[]";
        write!(
            socket,
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    servidor.join().unwrap();
}

#[test]
fn envia_el_cierre_de_un_ingreso_proveedor() {
    let (connection, _) = conexion_con_ingreso_proveedor(Some("2026-01-01T09:00:00Z"));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.starts_with(
            "PATCH /rest/v1/ingresos_proveedor?id=eq.uuid-ingreso-proveedor&hora_salida=is.null "
        ));
        write!(
            socket,
            "HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
        )
        .unwrap();
    });

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    servidor.join().unwrap();
}

#[test]
fn gafete_de_proveedor_ocupado_en_otro_dispositivo_excluye_este_dispositivo() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.contains("/rest/v1/ingresos_proveedor?"));
        assert!(pedido.contains("sitio_id=eq.sitio-1"));
        assert!(pedido.contains("dispositivo_entrada_id=neq.dispositivo-1"));
        assert!(pedido.contains("gafete_numero=eq.12"));
        let cuerpo = "[{\"id\":\"uuid-ingreso-proveedor\"}]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    let ocupado =
        gafete_de_proveedor_ocupado_en_otro_dispositivo(&contexto(&base_url), 12).unwrap();

    assert!(ocupado);
    servidor.join().unwrap();
}

#[test]
fn gafete_de_proveedor_ocupado_en_otro_dispositivo_sin_conflicto_devuelve_false() {
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let ocupado =
        gafete_de_proveedor_ocupado_en_otro_dispositivo(&contexto(&base_url), 12).unwrap();

    assert!(!ocupado);
}

/// Salida de ruta lista para encolar (sin match de catálogo, mismo
/// caso más común según `RutaService`) -- devuelve el `uuid` de la
/// salida. `fecha_hora_retorno` en `Some` simula una salida que ya
/// está lista para el cierre.
fn conexion_con_salida_ruta(fecha_hora_retorno: Option<&str>) -> (Connection, String) {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
             VALUES ('1', 'Guardia', 'h', 'OPERADOR', 1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rutas (id, numero, uuid) VALUES (1, 79, 'uuid-ruta-79')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO salidas_ruta (
                uuid, vehiculo_placa, vehiculo_numero_unidad, encargado_nombre,
                ruta_id, numero_ruta, sub_numero, numero_documento, fecha_documento,
                resultado, fecha_hora_salida, usuario_salida_id, usuario_salida_nombre,
                fecha_hora_retorno, usuario_retorno_id, usuario_retorno_nombre
            ) VALUES (
                'uuid-salida', 'C12345', '22906', 'Carlos Balmaceda',
                1, 79, 1, '700101452', '2026-09-15',
                'PERMITIDO', '2026-09-15T12:00:00Z', 1, 'Guardia',
                ?1, ?2, ?3
            )",
            params![
                fecha_hora_retorno,
                fecha_hora_retorno.map(|_| 1_i64),
                fecha_hora_retorno.map(|_| "Guardia"),
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES (
                'salida_ruta', 'uuid-salida', ?1,
                '2026-09-15T12:00:00Z', '2026-09-15T12:00:00Z'
            )",
            params![if fecha_hora_retorno.is_some() {
                "cerrar"
            } else {
                "crear"
            }],
        )
        .unwrap();
    (connection, "uuid-salida".to_string())
}

#[test]
fn envia_la_apertura_de_una_salida_ruta() {
    let (connection, _) = conexion_con_salida_ruta(None);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.starts_with("POST /rest/v1/salidas_ruta "));
        let cuerpo = "[]";
        write!(
            socket,
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    servidor.join().unwrap();
}

#[test]
fn envia_el_cierre_de_una_salida_ruta() {
    let (connection, _) = conexion_con_salida_ruta(Some("2026-09-15T18:00:00Z"));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(
            pedido
                .starts_with("PATCH /rest/v1/salidas_ruta?id=eq.uuid-salida&hora_retorno=is.null ")
        );
        write!(
            socket,
            "HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
        )
        .unwrap();
    });

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
    servidor.join().unwrap();
}

#[test]
fn envia_la_apertura_de_un_ingreso() {
    let (connection, contratista_uuid) = conexion_con_contratista();
    connection.execute("DELETE FROM cola_salida", []).unwrap();
    connection
        .execute("INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES ('1', 'Op', 'h', 'OPERADOR', 1)", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO registro_ingresos (
                contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
                usuario_ingreso_id, contratista_cedula, contratista_nombre, empresa_nombre,
                usuario_ingreso_nombre, es_personal_ruta, tiene_acceso, resultado_acceso,
                reglas_version, uuid
            ) VALUES (
                1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO', 'SWAT',
                1, '1-2345', 'Persona de prueba', 'Brisas',
                'Op', 0, 1, 'PERMITIDO', 1, 'uuid-ingreso'
            )",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('ingreso', 'uuid-ingreso', 'crear', '2026-01-01T08:00:00Z', '2026-01-01T08:00:00Z')",
            [],
        )
        .unwrap();
    let _ = &contratista_uuid;
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(
        resumen,
        ResumenDrenado {
            enviados: 1,
            fallidos: 0,
            ..Default::default()
        }
    );
}

/// Fase 3: el `409` que trae `code: "23505"` y nombra
/// `ingresos_gafete_activo_sitio_idx` en el cuerpo debe marcar la fila
/// `fallido` DE INMEDIATO (no `pendiente` para reintentar) y reportar
/// el conflicto con los datos correctos -- no cualquier `409`, sólo
/// éste, y sin esperar los 20 reintentos normales.
#[test]
fn apertura_de_ingreso_con_gafete_ya_activo_en_otro_dispositivo_queda_fallida_de_inmediato() {
    let (connection, contratista_uuid) = conexion_con_contratista();
    connection.execute("DELETE FROM cola_salida", []).unwrap();
    connection
        .execute("INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES ('1', 'Op', 'h', 'OPERADOR', 1)", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO registro_ingresos (
                contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
                usuario_ingreso_id, contratista_cedula, contratista_nombre, empresa_nombre,
                usuario_ingreso_nombre, es_personal_ruta, tiene_acceso, resultado_acceso,
                reglas_version, gafete_numero, uuid
            ) VALUES (
                1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO', 'PRAIND',
                1, '1-2345', 'Persona de prueba', 'Brisas',
                'Op', 0, 1, 'PERMITIDO', 1, 77, 'uuid-ingreso'
            )",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('ingreso', 'uuid-ingreso', 'crear', '2026-01-01T08:00:00Z', '2026-01-01T08:00:00Z')",
            [],
        )
        .unwrap();
    let _ = &contratista_uuid;
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         {\"code\":\"23505\",\"details\":\"Key (sitio_id, gafete_numero)=(s1, 77) already exists.\",\
         \"hint\":null,\"message\":\"duplicate key value violates unique constraint \\\"ingresos_gafete_activo_sitio_idx\\\"\"}",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.enviados, 0);
    assert_eq!(resumen.fallidos, 1);
    assert_eq!(
        resumen.conflictos_gafete,
        vec![ConflictoGafeteActivo {
            tipo: TipoMovimientoGafete::Contratista,
            nombre: "Persona de prueba".to_string(),
            gafete_numero: 77,
            fecha_hora: "2026-01-01T08:00:00Z".to_string(),
        }]
    );

    let (estado, intentos): (String, i64) = connection
        .query_row(
            "SELECT estado, intentos FROM cola_salida WHERE entidad_uuid = 'uuid-ingreso'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(estado, "fallido");
    assert_eq!(intentos, 1);
}

/// Espejo del test anterior, pero con un `409` que NO nombra este
/// índice -- un choque de unicidad genérico (o cualquier otro `409`)
/// no debe tratarse como conflicto de gafete: sigue el camino normal
/// de reintento (`pendiente`, no `fallido`).
#[test]
fn un_409_que_no_nombra_el_indice_de_gafete_sigue_el_camino_normal_de_reintento() {
    let (connection, contratista_uuid) = conexion_con_contratista();
    connection.execute("DELETE FROM cola_salida", []).unwrap();
    connection
        .execute("INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES ('1', 'Op', 'h', 'OPERADOR', 1)", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO registro_ingresos (
                contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
                usuario_ingreso_id, contratista_cedula, contratista_nombre, empresa_nombre,
                usuario_ingreso_nombre, es_personal_ruta, tiene_acceso, resultado_acceso,
                reglas_version, gafete_numero, uuid
            ) VALUES (
                1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO', 'PRAIND',
                1, '1-2345', 'Persona de prueba', 'Brisas',
                'Op', 0, 1, 'PERMITIDO', 1, 77, 'uuid-ingreso'
            )",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            ) VALUES ('ingreso', 'uuid-ingreso', 'crear', '2026-01-01T08:00:00Z', '2026-01-01T08:00:00Z')",
            [],
        )
        .unwrap();
    let _ = &contratista_uuid;
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         {\"message\":\"otro choque cualquiera, no el de gafete\"}",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.fallidos, 1);
    assert!(resumen.conflictos_gafete.is_empty());
    let estado: String = connection
        .query_row(
            "SELECT estado FROM cola_salida WHERE entidad_uuid = 'uuid-ingreso'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(estado, "pendiente");
}

#[test]
fn recibe_ingresos_abiertos_del_otro_dispositivo_y_los_cachea() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-remoto\",\"contratista_nombre\":\"Persona Remota\",\
         \"hora_entrada\":\"2026-01-01T08:00:00Z\",\"usuario_entrada_nombre\":\"Op PC\",\
         \"dispositivo_entrada_id\":\"otro-dispositivo\"}]",
    );

    let recibidos = recibir_ingresos_abiertos(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidos.len(), 1);
    assert_eq!(recibidos[0].uuid, "uuid-remoto");
    assert_eq!(recibidos[0].contratista_nombre, "Persona Remota");
    let cacheados: i64 = connection
        .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(cacheados, 1);
}

#[test]
fn recibe_ingresos_proveedor_abiertos_del_otro_dispositivo_y_los_cachea() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-remoto\",\"cedula\":\"1-1111\",\"nombre\":\"Juan Perez\",\
         \"empresa_nombre\":\"Maika\",\"placa\":null,\"gafete_numero\":9,\
         \"hora_entrada\":\"2026-01-01T08:00:00Z\",\"usuario_entrada_nombre\":\"Op PC\",\
         \"dispositivo_entrada_id\":\"otro-dispositivo\"}]",
    );

    let recibidos = recibir_ingresos_proveedor_abiertos(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidos.len(), 1);
    assert_eq!(recibidos[0].uuid, "uuid-remoto");
    assert_eq!(recibidos[0].nombre, "Juan Perez");
    assert_eq!(recibidos[0].gafete_numero, 9);
    let cacheados: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ingresos_proveedor_remotos",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(cacheados, 1);
}

#[test]
fn recibir_reemplaza_la_cache_del_sitio_por_completo() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO ingresos_remotos (
                uuid, sitio_id, contratista_nombre, hora_entrada,
                usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
            ) VALUES (
                'ya-cerrado', 'sitio-1', 'Otra Persona', '2026-01-01T07:00:00Z',
                NULL, 'otro-dispositivo', '2026-01-01T07:00:00Z'
            )",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    recibir_ingresos_abiertos(&connection, &contexto(&base_url)).unwrap();

    let cacheados: i64 = connection
        .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        cacheados, 0,
        "lo que ya no viene en la respuesta se borra de la caché"
    );
}

#[test]
fn recibe_historial_del_sitio_y_guarda_el_tipo_de_dispositivo_embebido() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"mov-1\",\"contratista_nombre\":\"Persona Remota\",\
         \"hora_entrada\":\"2026-01-01T08:00:00Z\",\
         \"dispositivo_entrada_id\":\"otro-dispositivo\",\
         \"updated_at\":\"2026-01-01T08:00:05Z\",\
         \"dispositivo_entrada\":{\"tipo\":\"mobile\"}}]",
    );

    let recibidos = recibir_historial_del_sitio(&connection, &contexto(&base_url), true).unwrap();

    assert_eq!(recibidos, 1);
    let tipo: Option<String> = connection
        .query_row(
            "SELECT dispositivo_entrada_tipo FROM historial_sitio WHERE uuid = 'mov-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tipo.as_deref(), Some("mobile"));
}

/// Servidor que contesta `respuestas` en orden (una por conexión) y anota
/// la línea de cada pedido ("GET /rest/v1/...").
fn servidor_que_anota(respuestas: Vec<String>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
    let direccion = listener.local_addr().expect("dirección local");
    let pedidos = Arc::new(Mutex::new(Vec::new()));
    let anotados = Arc::clone(&pedidos);
    thread::spawn(move || {
        for cuerpo in respuestas {
            let Ok((mut socket, _)) = listener.accept() else {
                return;
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("set_read_timeout");
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                match socket.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(leidos) => pedido.extend_from_slice(&buffer[..leidos]),
                }
            }
            let pedido = String::from_utf8_lossy(&pedido);
            anotados
                .lock()
                .unwrap()
                .push(pedido.lines().next().unwrap_or_default().to_string());
            let _ = write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
                cuerpo.len()
            );
        }
    });
    (format!("http://{direccion}"), pedidos)
}

/// `historial_sitio` con dos movimientos ya guardados y la marca en
/// 13:00: `mov-igual` al día con la nube, `mov-viejo` desactualizado.
fn conexion_con_historial_guardado() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO historial_sitio (
                 uuid, sitio_id, contratista_nombre, hora_entrada, dispositivo_entrada_id,
                 actualizado_en
             ) VALUES
                 ('mov-igual', 'sitio-1', 'Igual', '2026-09-27T11:00:00Z', 'otro',
                  '2026-09-27T12:00:00.500000Z'),
                 ('mov-viejo', 'sitio-1', 'Viejo', '2026-09-27T11:00:00Z', 'otro',
                  '2026-09-27T12:00:00Z');
             UPDATE sincronizacion_estado
                SET historial_actualizado_hasta = '2026-09-27T13:00:00.000000Z' WHERE id = 1;",
        )
        .unwrap();
    connection
}

fn fila_historial_json(id: &str, updated_at: &str) -> String {
    format!(
        r#"{{"id":"{id}","contratista_nombre":"Persona","hora_entrada":"2026-09-27T11:00:00Z","hora_salida":"2026-09-27T12:30:00Z","dispositivo_entrada_id":"otro","updated_at":"{updated_at}"}}"#
    )
}

#[test]
fn la_reconciliacion_compara_el_indice_y_trae_solo_lo_que_falta_o_cambio() {
    let connection = conexion_con_historial_guardado();
    let indice = r#"[
        {"id":"mov-igual","updated_at":"2026-09-27T12:00:00.5+00:00"},
        {"id":"mov-viejo","updated_at":"2026-09-27T12:30:00+00:00"},
        {"id":"mov-nuevo","updated_at":"2026-09-27T14:00:00+00:00"}
    ]"#;
    let filas = format!(
        "[{},{}]",
        fila_historial_json("mov-viejo", "2026-09-27T12:30:00+00:00"),
        fila_historial_json("mov-nuevo", "2026-09-27T14:00:00+00:00"),
    );
    let (base_url, pedidos) = servidor_que_anota(vec![indice.to_string(), filas]);

    let recibidos = recibir_historial_del_sitio(&connection, &contexto(&base_url), true).unwrap();

    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 2, "{pedidos:#?}");
    assert!(pedidos[0].contains("select=id,updated_at"), "{pedidos:#?}");
    assert!(pedidos[1].contains("id=in."), "{pedidos:#?}");
    assert!(
        pedidos[1].contains("mov-viejo") && pedidos[1].contains("mov-nuevo"),
        "{pedidos:#?}"
    );
    assert!(
        !pedidos[1].contains("mov-igual"),
        "lo que ya está al día no se vuelve a pedir"
    );
    assert_eq!(recibidos, 2);
    let guardado: String = connection
        .query_row(
            "SELECT actualizado_en FROM historial_sitio WHERE uuid = 'mov-nuevo'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        guardado, "2026-09-27T14:00:00.000000Z",
        "el updated_at del servidor"
    );
    let marca: String = connection
        .query_row(
            "SELECT historial_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(marca, "2026-09-27T14:00:00.000000Z");
}

#[test]
fn la_reconciliacion_sin_diferencias_solo_pide_el_indice() {
    let connection = conexion_con_historial_guardado();
    let indice = r#"[{"id":"mov-igual","updated_at":"2026-09-27T12:00:00.5+00:00"}]"#;
    let (base_url, pedidos) = servidor_que_anota(vec![indice.to_string()]);

    let recibidos = recibir_historial_del_sitio(&connection, &contexto(&base_url), true).unwrap();

    assert_eq!(recibidos, 0);
    assert_eq!(pedidos.lock().unwrap().len(), 1);
    let marca: String = connection
        .query_row(
            "SELECT historial_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        marca, "2026-09-27T13:00:00.000000Z",
        "la marca no retrocede"
    );
}

#[test]
fn fuera_del_arranque_no_se_pide_indice_sino_lo_cambiado() {
    let connection = conexion_con_historial_guardado();
    let (base_url, pedidos) = servidor_que_anota(vec!["[]".to_string()]);

    recibir_historial_del_sitio(&connection, &contexto(&base_url), false).unwrap();

    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 1);
    assert!(
        !pedidos[0].contains("select=id,updated_at&"),
        "{pedidos:#?}"
    );
    // Marca (13:00) menos el traslape corto (5 minutos).
    assert!(
        pedidos[0].contains("updated_at=gt.2026-09-27T12:55:00.000000Z")
            || pedidos[0].contains("updated_at=gt.2026-09-27T12%3A55%3A00.000000Z"),
        "{pedidos:#?}"
    );
}

#[test]
fn recibir_historial_del_sitio_incluye_movimientos_del_dispositivo_actual() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.contains("/ingresos?sitio_id=eq.sitio-1"));
        assert!(
            !pedido.contains("dispositivo_entrada_id=neq."),
            "una instalación nueva debe poder repoblar lo que antes generó este mismo dispositivo"
        );
        let cuerpo = "[]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    recibir_historial_del_sitio(&connection, &contexto(&base_url), true).unwrap();
    servidor.join().unwrap();
}

#[test]
fn recibir_historial_paginado_persiste_todas_las_paginas_y_la_marca_de_agua_es_el_maximo_global() {
    use std::fmt::Write as _;

    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    // Página 1: exactamente TAMANO_PAGINA_REMOTA filas -- señal de que
    // hay una página más. `updated_at` sube con cada fila, hasta
    // 2026-01-01T00:08:19Z en la última (fila 499, +499 segundos).
    let mut filas_pagina_1 = String::new();
    for i in 0..TAMANO_PAGINA_REMOTA {
        if i > 0 {
            filas_pagina_1.push(',');
        }
        let minutos = i / 60;
        let segundos = i % 60;
        write!(
            filas_pagina_1,
            "{{\"id\":\"mov-{i:04}\",\"contratista_nombre\":\"Persona {i}\",\
             \"hora_entrada\":\"2026-01-01T00:00:00Z\",\
             \"dispositivo_entrada_id\":\"dispositivo-1\",\
             \"updated_at\":\"2026-01-01T00:{minutos:02}:{segundos:02}Z\"}}"
        )
        .unwrap();
    }
    let cuerpo_pagina_1 = format!("[{filas_pagina_1}]");

    // Página 2: una sola fila (menos que TAMANO_PAGINA_REMOTA, así que
    // es la última) con un `updated_at` MÁS VIEJO que el máximo de la
    // página 1 -- las páginas vienen ordenadas por `id`, no por
    // `updated_at`, así que esto puede pasar en la práctica. Si la
    // marca de agua final quedara en el valor de la última página en
    // vez del máximo global, esta prueba lo detecta.
    let cuerpo_pagina_2 = "[{\"id\":\"mov-pagina-2\",\"contratista_nombre\":\"Persona tardía\",\
         \"hora_entrada\":\"2026-01-01T00:00:00Z\",\
         \"dispositivo_entrada_id\":\"dispositivo-1\",\
         \"updated_at\":\"2026-01-01T00:00:01Z\"}]"
        .to_string();

    let respuesta_pagina_1 = Box::leak(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo_pagina_1}",
            cuerpo_pagina_1.len()
        )
        .into_boxed_str(),
    ) as &'static str;
    let respuesta_pagina_2 = Box::leak(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo_pagina_2}",
            cuerpo_pagina_2.len()
        )
        .into_boxed_str(),
    ) as &'static str;
    let base_url = servidor_de_respuestas(vec![respuesta_pagina_1, respuesta_pagina_2]);

    let recibidos = recibir_historial_del_sitio(&connection, &contexto(&base_url), true).unwrap();

    assert_eq!(recibidos, u32::try_from(TAMANO_PAGINA_REMOTA).unwrap() + 1);
    let guardadas: i64 = connection
        .query_row("SELECT COUNT(*) FROM historial_sitio", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        guardadas,
        i64::try_from(TAMANO_PAGINA_REMOTA).unwrap() + 1,
        "las filas de ambas páginas quedan persistidas, no sólo la última"
    );
    let marca: String = connection
        .query_row(
            "SELECT historial_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        marca, "2026-01-01T00:08:19.000000Z",
        "la marca de agua es el máximo de TODAS las páginas, no el de la última"
    );
}

#[test]
fn marca_historial_para_consulta_retrocede_una_semana() {
    let ahora = crate::tiempo::parsear_utc("2026-09-09T12:00:00Z").unwrap();

    let marca = marca_historial_para_consulta(
        Some("2026-09-09T10:00:00Z"),
        ahora,
        traslape_historial(true),
    );

    assert_eq!(
        marca,
        Some(crate::tiempo::parsear_utc("2026-09-02T10:00:00Z").unwrap())
    );
}

#[test]
fn marca_historial_para_consulta_sanea_marcas_en_futuro() {
    let ahora = crate::tiempo::parsear_utc("2026-09-09T12:00:00Z").unwrap();

    let marca = marca_historial_para_consulta(
        Some("2026-12-01T00:00:00Z"),
        ahora,
        traslape_historial(true),
    );

    assert_eq!(
        marca,
        Some(crate::tiempo::parsear_utc("2026-09-02T12:00:00Z").unwrap())
    );
}

#[test]
fn fuera_del_arranque_el_historial_retrocede_solo_minutos() {
    // El pulso y los avisos no vuelven a bajar la semana entera (visto en
    // producción: ~4 s reescribiendo 390 ingresos en cada aviso).
    let ahora = crate::tiempo::parsear_utc("2026-09-09T12:00:00Z").unwrap();

    let marca = marca_historial_para_consulta(
        Some("2026-09-09T10:00:00.123456Z"),
        ahora,
        traslape_historial(false),
    );

    assert_eq!(
        marca,
        Some(crate::tiempo::parsear_utc("2026-09-09T09:55:00.123456Z").unwrap())
    );
}

#[test]
fn recibe_historial_del_sitio_sin_dispositivo_embebido_no_falla() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"mov-2\",\"contratista_nombre\":\"Persona Remota\",\
         \"hora_entrada\":\"2026-01-01T08:00:00Z\",\
         \"dispositivo_entrada_id\":\"otro-dispositivo\",\
         \"updated_at\":\"2026-01-01T08:00:05Z\"}]",
    );

    let recibidos = recibir_historial_del_sitio(&connection, &contexto(&base_url), true).unwrap();

    assert_eq!(recibidos, 1);
    let tipo: Option<String> = connection
        .query_row(
            "SELECT dispositivo_entrada_tipo FROM historial_sitio WHERE uuid = 'mov-2'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tipo, None, "sin embed, queda NULL en vez de fallar");
}

#[test]
fn recibe_una_cita_con_su_grupo_de_visitantes_y_el_nombre_del_anfitrion_embebido() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"cita-1\",\"motivo\":\"Auditoría\",\
         \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-12\",\
         \"anfitrion_correo\":\"kof@brisas.com\",\
         \"anfitrion\":{\"nombre\":\"Persona Anfitriona\"},\
         \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:00Z\",\
         \"cita_visitantes\":[\
         {\"id\":\"visitante-1\",\"cedula\":\"1-1111\",\"nombre\":\"Visitante Uno\",\
         \"empresa\":null,\"placa_vehiculo\":null}]}]",
    );

    let recibidas = recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidas, 1);
    let (anfitrion_nombre, estado): (String, String) = connection
        .query_row(
            "SELECT anfitrion_nombre, estado FROM citas WHERE uuid = 'cita-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(anfitrion_nombre, "Persona Anfitriona");
    assert_eq!(estado, "VIGENTE");
    let cedula: String = connection
        .query_row(
            "SELECT cedula FROM cita_visitantes WHERE uuid = 'visitante-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    // La web la guardó con guiones; baja en su forma única (`domain::cedula`).
    assert_eq!(cedula, "11111");
}

#[test]
fn recibe_la_hora_estimada_de_una_cita_y_la_admite_ausente() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"cita-hora\",\"motivo\":null,\
         \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
         \"hora_estimada\":\"10:00:00\",\
         \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
         \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:00Z\",\
         \"cita_visitantes\":[]},\
         {\"id\":\"cita-sin-hora\",\"motivo\":null,\
         \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
         \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
         \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:01Z\",\
         \"cita_visitantes\":[]}]",
    );

    let recibidas = recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidas, 2);
    let hora_estimada: Option<String> = connection
        .query_row(
            "SELECT hora_estimada FROM citas WHERE uuid = 'cita-hora'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(hora_estimada.as_deref(), Some("10:00:00"));
    let sin_hora: Option<String> = connection
        .query_row(
            "SELECT hora_estimada FROM citas WHERE uuid = 'cita-sin-hora'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        sin_hora, None,
        "campo ausente en el JSON no debe fallar, sólo queda NULL"
    );
}

#[test]
fn recibe_cita_sin_anfitrion_embebido_no_falla() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"cita-2\",\"motivo\":null,\
         \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
         \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
         \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:00Z\",\
         \"cita_visitantes\":[]}]",
    );

    let recibidas = recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidas, 1);
    let anfitrion_nombre: String = connection
        .query_row(
            "SELECT anfitrion_nombre FROM citas WHERE uuid = 'cita-2'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        anfitrion_nombre, "—",
        "sin embed (RLS lo filtró o no aplica), queda un placeholder en vez de fallar"
    );
}

#[test]
fn una_fila_de_cita_con_fecha_ilegible_se_omite_sin_abortar_las_demas() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"cita-mala\",\"motivo\":null,\
         \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
         \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
         \"estado\":\"VIGENTE\",\"updated_at\":\"no-es-una-fecha\",\
         \"cita_visitantes\":[]},\
         {\"id\":\"cita-buena\",\"motivo\":null,\
         \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
         \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
         \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:00Z\",\
         \"cita_visitantes\":[]}]",
    );

    let recibidas = recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidas, 1, "la fila con updated_at ilegible no cuenta");
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM citas", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total, 1, "solo se guardó la fila válida");
}

#[test]
fn segunda_sincronizacion_de_citas_pide_solo_lo_actualizado_desde_la_marca_previa() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "UPDATE sincronizacion_estado SET citas_actualizado_hasta = '2026-09-09T08:00:00Z' WHERE id = 1",
            [],
        )
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(
            pedido.contains("updated_at=gt.2026-09-09T08%3A00%3A00")
                || pedido.contains("updated_at=gt.2026-09-09T08:00:00")
        );
        assert!(
            !pedido.contains("sitio_id="),
            "sin filtro explícito de sitio -- RLS ya lo resuelve del lado del servidor"
        );
        let cuerpo = "[]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();
    servidor.join().unwrap();
}

#[test]
fn recibe_el_historial_de_visitas_del_sitio_y_lo_guarda_local() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"mov-visita-1\",\"visitante_cedula\":\"1-1111\",\
         \"visitante_nombre\":\"Visitante Remoto\",\"empresa\":\"Brisas\",\
         \"anfitrion_nombre\":\"Anfitrión\",\"motivo\":\"Auditoría\",\
         \"gafete_numero\":9,\"hora_entrada\":\"2026-01-01T08:00:00Z\",\
         \"hora_salida\":null,\"usuario_entrada_nombre\":\"Guardia\",\
         \"usuario_salida_nombre\":null,\"dispositivo_entrada_id\":\"otro-dispositivo\",\
         \"dispositivo_salida_id\":null,\"updated_at\":\"2026-01-01T08:00:05Z\"}]",
    );

    let recibidos =
        recibir_historial_visitas_del_sitio(&connection, &contexto(&base_url), true).unwrap();

    assert_eq!(recibidos, 1);
    let (cedula, nombre, empresa, anfitrion): (String, String, Option<String>, Option<String>) =
        connection
            .query_row(
                "SELECT visitante_cedula, visitante_nombre, empresa, anfitrion_nombre
             FROM historial_visitas_sitio WHERE uuid = 'mov-visita-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
    assert_eq!(cedula, "1-1111");
    assert_eq!(nombre, "Visitante Remoto");
    assert_eq!(empresa.as_deref(), Some("Brisas"));
    assert_eq!(anfitrion.as_deref(), Some("Anfitrión"));
}

#[test]
fn una_fila_de_historial_de_visitas_con_fecha_ilegible_se_omite_sin_abortar_las_demas() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"mov-mala\",\"visitante_cedula\":\"1-1111\",\"visitante_nombre\":\"X\",\
         \"empresa\":null,\"anfitrion_nombre\":null,\"motivo\":null,\"gafete_numero\":null,\
         \"hora_entrada\":\"no-es-una-fecha\",\"hora_salida\":null,\
         \"usuario_entrada_nombre\":null,\"usuario_salida_nombre\":null,\
         \"dispositivo_entrada_id\":\"otro-dispositivo\",\"dispositivo_salida_id\":null,\
         \"updated_at\":\"2026-01-01T08:00:00Z\"},\
         {\"id\":\"mov-buena\",\"visitante_cedula\":\"1-2222\",\"visitante_nombre\":\"Y\",\
         \"empresa\":null,\"anfitrion_nombre\":null,\"motivo\":null,\"gafete_numero\":null,\
         \"hora_entrada\":\"2026-01-01T08:00:00Z\",\"hora_salida\":null,\
         \"usuario_entrada_nombre\":null,\"usuario_salida_nombre\":null,\
         \"dispositivo_entrada_id\":\"otro-dispositivo\",\"dispositivo_salida_id\":null,\
         \"updated_at\":\"2026-01-01T08:00:05Z\"}]",
    );

    let recibidos =
        recibir_historial_visitas_del_sitio(&connection, &contexto(&base_url), true).unwrap();

    assert_eq!(recibidos, 1, "la fila con hora_entrada ilegible no cuenta");
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM historial_visitas_sitio", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 1, "solo se guardó la fila válida");
}

#[test]
fn segunda_sincronizacion_de_historial_de_visitas_pide_solo_lo_actualizado_desde_la_marca_previa() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "UPDATE sincronizacion_estado SET historial_visitas_actualizado_hasta = '2026-09-09T08:00:00Z' WHERE id = 1",
            [],
        )
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.contains("/movimientos_visita?sitio_id=eq.sitio-1"));
        let cuerpo = "[]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    recibir_historial_visitas_del_sitio(&connection, &contexto(&base_url), true).unwrap();
    servidor.join().unwrap();
}

#[test]
fn recibe_el_historial_de_ingresos_proveedor_del_sitio_y_lo_guarda_local() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"ingreso-proveedor-1\",\"cedula\":\"1-1111\",\
         \"nombre\":\"Proveedor Remoto\",\"empresa_nombre\":\"Maika\",\"placa\":null,\
         \"gafete_numero\":9,\"hora_entrada\":\"2026-01-01T08:00:00Z\",\
         \"hora_salida\":null,\"usuario_entrada_nombre\":\"Guardia\",\
         \"usuario_salida_nombre\":null,\"dispositivo_entrada_id\":\"otro-dispositivo\",\
         \"dispositivo_salida_id\":null,\"updated_at\":\"2026-01-01T08:00:05Z\"}]",
    );

    let recibidos =
        recibir_historial_ingresos_proveedor_del_sitio(&connection, &contexto(&base_url), true)
            .unwrap();

    assert_eq!(recibidos, 1);
    let (cedula, nombre, empresa): (String, String, Option<String>) = connection
        .query_row(
            "SELECT cedula, nombre, empresa_nombre
             FROM historial_ingresos_proveedor_sitio WHERE uuid = 'ingreso-proveedor-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(cedula, "1-1111");
    assert_eq!(nombre, "Proveedor Remoto");
    assert_eq!(empresa.as_deref(), Some("Maika"));
}

#[test]
fn una_fila_de_historial_de_ingresos_proveedor_con_fecha_ilegible_se_omite_sin_abortar_las_demas() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"prov-malo\",\"cedula\":\"1-1111\",\"nombre\":\"X\",\
         \"empresa_nombre\":null,\"placa\":null,\"gafete_numero\":null,\
         \"hora_entrada\":\"no-es-una-fecha\",\"hora_salida\":null,\
         \"usuario_entrada_nombre\":null,\"usuario_salida_nombre\":null,\
         \"dispositivo_entrada_id\":\"otro-dispositivo\",\"dispositivo_salida_id\":null,\
         \"updated_at\":\"2026-01-01T08:00:00Z\"},\
         {\"id\":\"prov-bueno\",\"cedula\":\"1-2222\",\"nombre\":\"Y\",\
         \"empresa_nombre\":null,\"placa\":null,\"gafete_numero\":null,\
         \"hora_entrada\":\"2026-01-01T08:00:00Z\",\"hora_salida\":null,\
         \"usuario_entrada_nombre\":null,\"usuario_salida_nombre\":null,\
         \"dispositivo_entrada_id\":\"otro-dispositivo\",\"dispositivo_salida_id\":null,\
         \"updated_at\":\"2026-01-01T08:00:05Z\"}]",
    );

    let recibidos =
        recibir_historial_ingresos_proveedor_del_sitio(&connection, &contexto(&base_url), true)
            .unwrap();

    assert_eq!(recibidos, 1, "la fila con hora_entrada ilegible no cuenta");
    let total: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM historial_ingresos_proveedor_sitio",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(total, 1, "solo se guardó la fila válida");
}

#[test]
fn segunda_sincronizacion_de_historial_de_ingresos_proveedor_pide_solo_lo_actualizado_desde_la_marca_previa()
 {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "UPDATE sincronizacion_estado SET historial_ingresos_proveedor_actualizado_hasta = '2026-09-09T08:00:00Z' WHERE id = 1",
            [],
        )
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.contains("/ingresos_proveedor?sitio_id=eq.sitio-1"));
        assert!(pedido.contains("updated_at=gt."));
        let cuerpo = "[]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    recibir_historial_ingresos_proveedor_del_sitio(&connection, &contexto(&base_url), true)
        .unwrap();
    servidor.join().unwrap();
}

#[test]
fn usuario_sigue_activo_remoto_lee_el_booleano_de_una_sola_fila() {
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"activo\":false}]",
    );

    let activo = usuario_sigue_activo_remoto(&contexto(&base_url), "999999999").unwrap();

    assert!(!activo);
}

#[test]
fn usuario_sigue_activo_remoto_sin_fila_asume_activo() {
    // Un usuario que este dispositivo creó y todavía no subió (o que
    // subió hace un instante y el receptor todavía no lo ve) -- la nube
    // no tiene nada que decir de él, no hay motivo para expulsarlo por
    // eso.
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let activo = usuario_sigue_activo_remoto(&contexto(&base_url), "ROOT1").unwrap();

    assert!(activo);
}

#[test]
fn contratista_con_ingreso_activo_busca_en_todos_los_sitios() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.contains("contratista_cedula=eq.2001"));
        assert!(!pedido.contains("sitio_id=neq"));
        assert!(pedido.contains("hora_salida=is.null"));
        let cuerpo = "[{\"sitio_id\":\"otro\",\"sitios\":{\"nombre\":\"Cartago\"}}]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    let activo = contratista_con_ingreso_activo(&contexto(&base_url), "2001").unwrap();

    assert_eq!(
        activo,
        Some(IngresoActivoEnLaNube {
            mismo_sitio: false,
            sitio_nombre: "Cartago".to_string(),
        })
    );
    servidor.join().unwrap();
}

#[test]
fn contratista_con_ingreso_activo_sin_ingresos_abiertos_devuelve_none() {
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let activo = contratista_con_ingreso_activo(&contexto(&base_url), "2001").unwrap();

    assert_eq!(activo, None);
}

fn conexion_con_dos_ingresos_activos() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "
            INSERT INTO empresas (id, nombre, uuid) VALUES (1, 'Brisas', 'uuid-empresa');
            INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
            VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
            INSERT INTO contratistas (
                id, cedula, nombre, empresa_id, tipo_ingreso,
                es_personal_ruta, tiene_acceso, uuid
            ) VALUES
                (1, '2001', 'Persona Uno', 1, 'SWAT', 0, 1, 'uuid-c1'),
                (2, '2002', 'Persona Dos', 1, 'SWAT', 0, 1, 'uuid-c2');
            INSERT INTO registro_ingresos (
                id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso,
                tipo_ingreso, gafete_numero, usuario_ingreso_id, contratista_cedula,
                contratista_nombre, empresa_nombre, usuario_ingreso_nombre,
                fecha_vencimiento_praind, es_personal_ruta, tiene_acceso,
                resultado_acceso, motivo_resultado, reglas_version,
                empresa_activa_snapshot, uuid
            ) VALUES
                (1, 1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO', 'SWAT', NULL, 1, '2001',
                 'Persona Uno', 'Brisas', 'Operador', NULL, 0, 1, 'PERMITIDO', NULL, 1, 1,
                 'uuid-i1'),
                (2, 2, 1, '2026-01-01T08:00:00Z', 'CAMINANDO', 'SWAT', NULL, 1, '2002',
                 'Persona Dos', 'Brisas', 'Operador', NULL, 0, 1, 'PERMITIDO', NULL, 1, 1,
                 'uuid-i2');
            ",
        )
        .unwrap();
    connection
}

#[test]
fn contratistas_con_conflicto_activo_solo_incluye_a_quien_de_verdad_choca() {
    let connection = conexion_con_dos_ingresos_activos();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"contratista_cedula\":\"2001\",\"sitios\":{\"nombre\":\"Cartago\"}}]",
    );

    let conflictos = contratistas_con_conflicto_activo(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(
        conflictos,
        vec![ConflictoIngresoActivo {
            cedula: "2001".to_string(),
            contratista_nombre: "Persona Uno".to_string(),
            sitio_conflicto: "Cartago".to_string(),
        }]
    );
}

#[test]
fn contratistas_con_conflicto_activo_sin_nada_local_no_llama_a_la_nube() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    // Puerto sin nada escuchando: si igual intentara la red, esto
    // fallaría con un error de conexión en vez de devolver `Ok(vec![])`.
    let conflictos =
        contratistas_con_conflicto_activo(&connection, &contexto("http://127.0.0.1:1")).unwrap();

    assert_eq!(conflictos, Vec::new());
}

#[test]
fn gafete_de_visita_ocupado_en_otro_dispositivo_excluye_este_dispositivo() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.contains("sitio_id=eq.sitio-1"));
        assert!(pedido.contains("dispositivo_entrada_id=neq.dispositivo-1"));
        assert!(pedido.contains("gafete_numero=eq.9"));
        let cuerpo = "[{\"id\":\"uuid-movimiento\"}]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    let ocupado = gafete_de_visita_ocupado_en_otro_dispositivo(&contexto(&base_url), 9).unwrap();

    assert!(ocupado);
    servidor.join().unwrap();
}

#[test]
fn gafete_de_visita_ocupado_en_otro_dispositivo_sin_conflicto_devuelve_false() {
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let ocupado = gafete_de_visita_ocupado_en_otro_dispositivo(&contexto(&base_url), 9).unwrap();

    assert!(!ocupado);
}

#[test]
fn gafete_provisional_ocupado_en_otro_dispositivo_consulta_la_tabla_y_columnas_correctas() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.contains("GET /rest/v1/prestamos_gafete_provisional?"));
        assert!(pedido.contains("sitio_id=eq.sitio-1"));
        assert!(pedido.contains("dispositivo_entrega_id=neq.dispositivo-1"));
        assert!(pedido.contains("hora_devolucion=is.null"));
        assert!(pedido.contains("gafete_numero=eq.12"));
        let cuerpo = "[{\"id\":\"uuid-prestamo\"}]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    let ocupado = gafete_provisional_ocupado_en_otro_dispositivo(&contexto(&base_url), 12).unwrap();

    assert!(ocupado);
    servidor.join().unwrap();
}

#[test]
fn gafete_provisional_ocupado_en_otro_dispositivo_sin_conflicto_devuelve_false() {
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let ocupado = gafete_provisional_ocupado_en_otro_dispositivo(&contexto(&base_url), 12).unwrap();

    assert!(!ocupado);
}

#[test]
fn visitante_activo_en_otro_sitio_excluye_el_sitio_actual_en_la_url() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.contains("visitante_cedula=eq.1-2345"));
        assert!(pedido.contains("sitio_id=neq.sitio-1"));
        assert!(pedido.contains("hora_salida=is.null"));
        let cuerpo = "[{\"sitios\":{\"nombre\":\"Cartago\"}}]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    let sitio = visitante_activo_en_otro_sitio(&contexto(&base_url), "1-2345").unwrap();

    assert_eq!(sitio, Some("Cartago".to_string()));
    servidor.join().unwrap();
}

#[test]
fn visitante_activo_en_otro_sitio_sin_conflicto_devuelve_none() {
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let sitio = visitante_activo_en_otro_sitio(&contexto(&base_url), "1-2345").unwrap();

    assert_eq!(sitio, None);
}

fn conexion_con_dos_movimientos_visita_activos() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "
            INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
            VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
            INSERT INTO citas (id, uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                anfitrion_correo, estado, creado_en)
            VALUES (1, 'uuid-cita-1', '2026-08-01', '2026-08-08', 'Ana',
                'ana@acme.com', 'VIGENTE', '2026-08-01T00:00:00Z');
            INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre) VALUES
                (1, 'uuid-v1', 1, '1-2345', 'Visitante Uno'),
                (2, 'uuid-v2', 1, '6-7890', 'Visitante Dos');
            INSERT INTO movimientos_visita (
                id, uuid, cita_visitante_id, gafete_numero, fecha_hora_entrada,
                usuario_entrada_id, usuario_entrada_nombre,
                visitante_cedula, visitante_nombre, anfitrion_nombre
            ) VALUES
                (1, 'uuid-m1', 1, NULL, '2026-08-01T08:00:00Z', 1, 'Operador',
                 '1-2345', 'Visitante Uno', 'Ana'),
                (2, 'uuid-m2', 2, NULL, '2026-08-01T08:00:00Z', 1, 'Operador',
                 '6-7890', 'Visitante Dos', 'Ana');
            ",
        )
        .unwrap();
    connection
}

#[test]
fn visitantes_con_conflicto_activo_solo_incluye_a_quien_de_verdad_choca() {
    let connection = conexion_con_dos_movimientos_visita_activos();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"visitante_cedula\":\"1-2345\",\"sitios\":{\"nombre\":\"Cartago\"}}]",
    );

    let conflictos = visitantes_con_conflicto_activo(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(
        conflictos,
        vec![ConflictoMovimientoVisitaActivo {
            cedula: "1-2345".to_string(),
            visitante_nombre: "Visitante Uno".to_string(),
            sitio_conflicto: "Cartago".to_string(),
        }]
    );
}

#[test]
fn visitantes_con_conflicto_activo_sin_nada_local_no_llama_a_la_nube() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let conflictos =
        visitantes_con_conflicto_activo(&connection, &contexto("http://127.0.0.1:1")).unwrap();

    assert_eq!(conflictos, Vec::new());
}

#[test]
fn proveedor_activo_en_otro_sitio_excluye_el_sitio_actual_en_la_url() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.contains("cedula=eq.1-2345"));
        assert!(pedido.contains("sitio_id=neq.sitio-1"));
        assert!(pedido.contains("hora_salida=is.null"));
        let cuerpo = "[{\"sitios\":{\"nombre\":\"Cartago\"}}]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    let sitio = proveedor_activo_en_otro_sitio(&contexto(&base_url), "1-2345").unwrap();

    assert_eq!(sitio, Some("Cartago".to_string()));
    servidor.join().unwrap();
}

#[test]
fn proveedor_activo_en_otro_sitio_sin_conflicto_devuelve_none() {
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let sitio = proveedor_activo_en_otro_sitio(&contexto(&base_url), "1-2345").unwrap();

    assert_eq!(sitio, None);
}

fn conexion_con_un_ingreso_proveedor_activo() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "
            INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
            VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
            INSERT INTO empresas_proveedor (id, nombre, activo, uuid)
            VALUES (1, 'Maika', 1, 'uuid-empresa-proveedor');
            INSERT INTO registro_ingresos_proveedor (
                id, cedula, nombre, empresa_id, empresa_nombre, gafete_numero,
                fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre, uuid
            ) VALUES (
                1, '1-2345', 'Juan Perez', 1, 'Maika', 9,
                '2026-08-01T08:00:00Z', 1, 'Operador', 'uuid-ingreso-proveedor'
            );
            ",
        )
        .unwrap();
    connection
}

#[test]
fn proveedores_con_conflicto_activo_solo_incluye_a_quien_de_verdad_choca() {
    let connection = conexion_con_un_ingreso_proveedor_activo();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"cedula\":\"1-2345\",\"sitios\":{\"nombre\":\"Cartago\"}}]",
    );

    let conflictos = proveedores_con_conflicto_activo(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(
        conflictos,
        vec![ConflictoIngresoProveedorActivo {
            cedula: "1-2345".to_string(),
            nombre: "Juan Perez".to_string(),
            sitio_conflicto: "Cartago".to_string(),
        }]
    );
}

#[test]
fn proveedores_con_conflicto_activo_sin_nada_local_no_llama_a_la_nube() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let conflictos =
        proveedores_con_conflicto_activo(&connection, &contexto("http://127.0.0.1:1")).unwrap();

    assert_eq!(conflictos, Vec::new());
}

#[test]
fn recibe_cierres_de_ingresos_propios_sin_reencolar() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "
            INSERT INTO empresas (id, nombre, uuid) VALUES (1, 'Brisas', 'uuid-empresa');
            INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
            VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
            INSERT INTO contratistas (
                id, cedula, nombre, empresa_id, tipo_ingreso,
                es_personal_ruta, tiene_acceso, uuid
            ) VALUES (1, '2001', 'Persona', 1, 'SWAT', 0, 1, 'uuid-contratista');
            INSERT INTO registro_ingresos (
                id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso,
                tipo_ingreso, gafete_numero, usuario_ingreso_id, contratista_cedula,
                contratista_nombre, empresa_nombre, usuario_ingreso_nombre,
                fecha_vencimiento_praind, es_personal_ruta, tiene_acceso,
                resultado_acceso, motivo_resultado, reglas_version,
                empresa_activa_snapshot, uuid
            ) VALUES (
                1, 1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO',
                'SWAT', NULL, 1, '2001', 'Persona', 'Brisas', 'Operador',
                NULL, 0, 1, 'PERMITIDO', NULL, 1, 1, 'uuid-ingreso'
            );
            ",
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-ingreso\",\"hora_salida\":\"2026-01-01T10:00:00Z\",\
         \"usuario_salida_nombre\":\"Operador remoto\"}]",
    );

    let aplicados = recibir_cierres_de_ingresos_propios(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(aplicados, 1);
    let (salida, usuario_id, usuario_nombre): (String, Option<i64>, String) = connection
        .query_row(
            "SELECT fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
             FROM registro_ingresos WHERE uuid = 'uuid-ingreso'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(salida, "2026-01-01T10:00:00Z");
    assert_eq!(usuario_id, None);
    assert_eq!(usuario_nombre, "Operador remoto");
    let reencolados: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM cola_salida
             WHERE entidad = 'ingreso' AND entidad_uuid = 'uuid-ingreso'
               AND operacion = 'cerrar'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reencolados, 0);
}

#[test]
fn recibe_cierres_de_ingresos_propios_proveedor_sin_reencolar() {
    let connection = conexion_con_un_ingreso_proveedor_activo();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-ingreso-proveedor\",\"hora_salida\":\"2026-08-01T10:00:00Z\",\
         \"usuario_salida_nombre\":\"Operador remoto\"}]",
    );

    let aplicados =
        recibir_cierres_de_ingresos_propios_proveedor(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(aplicados, 1);
    let (salida, usuario_id, usuario_nombre): (String, Option<i64>, String) = connection
        .query_row(
            "SELECT fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
             FROM registro_ingresos_proveedor WHERE uuid = 'uuid-ingreso-proveedor'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(salida, "2026-08-01T10:00:00Z");
    assert_eq!(usuario_id, None);
    assert_eq!(usuario_nombre, "Operador remoto");
}

#[test]
fn recibe_cierres_de_ingresos_propios_proveedor_sin_nada_local_no_llama_a_la_nube() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    let aplicados =
        recibir_cierres_de_ingresos_propios_proveedor(&connection, &contexto("http://127.0.0.1:1"))
            .unwrap();

    assert_eq!(aplicados, 0);
}

#[test]
fn recibe_prestamos_gafete_provisional_abiertos_del_otro_dispositivo_y_los_cachea() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-remoto\",\"encargado_nombre\":\"Kendall Morales\",\
         \"encargado_codigo_empleado\":\"5366536\",\"gafete_numero\":4,\
         \"hora_entrega\":\"2026-01-01T08:00:00Z\",\"usuario_entrega_nombre\":\"Op PC\"}]",
    );

    let recibidos =
        recibir_prestamos_gafete_provisional_abiertos(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidos.len(), 1);
    assert_eq!(recibidos[0].uuid, "uuid-remoto");
    assert_eq!(recibidos[0].encargado_nombre, "Kendall Morales");
    assert_eq!(recibidos[0].gafete_numero, 4);
    let cacheados: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM prestamos_gafete_provisional_remotos",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(cacheados, 1);
}

#[test]
fn recibe_historial_gafetes_provisionales_del_sitio_y_lo_guarda_local() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-remoto\",\"encargado_nombre\":\"Kendall Morales\",\
         \"encargado_codigo_empleado\":\"5366536\",\"gafete_numero\":4,\
         \"hora_entrega\":\"2026-01-01T08:00:00Z\",\"usuario_entrega_nombre\":\"Op PC\",\
         \"hora_devolucion\":\"2026-01-01T09:00:00Z\",\
         \"usuario_devolucion_nombre\":\"Op Movil\",\
         \"updated_at\":\"2026-01-01T09:00:05Z\"}]",
    );

    let recibidos =
        recibir_historial_gafetes_provisionales_del_sitio(&connection, &contexto(&base_url))
            .unwrap();

    assert_eq!(recibidos, 1);
    let (nombre, devolucion): (String, Option<String>) = connection
        .query_row(
            "SELECT encargado_nombre, usuario_devolucion_nombre
             FROM prestamos_gafete_provisional_historial_sitio WHERE uuid = 'uuid-remoto'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(nombre, "Kendall Morales");
    assert_eq!(devolucion.as_deref(), Some("Op Movil"));
}

#[test]
fn segunda_sincronizacion_de_historial_gafetes_provisionales_pide_solo_lo_actualizado_desde_la_marca_previa()
 {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "UPDATE sincronizacion_estado
             SET gafetes_provisionales_historial_actualizado_hasta = '2026-09-09T08:00:00Z'
             WHERE id = 1",
            [],
        )
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0);
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(
            pedido.contains("updated_at=gt.2026-09-09T08%3A00%3A00")
                || pedido.contains("updated_at=gt.2026-09-09T08:00:00")
        );
        let cuerpo = "[]";
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
    });

    recibir_historial_gafetes_provisionales_del_sitio(&connection, &contexto(&base_url)).unwrap();
    servidor.join().unwrap();
}

fn conexion_con_un_prestamo_gafete_provisional_activo() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "
            INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
            VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
            INSERT INTO encargados_ruta (id, codigo_empleado, nombre, activo, uuid)
            VALUES (1, '5366536', 'Kendall Morales', 1, 'uuid-encargado');
            INSERT INTO prestamos_gafete_provisional (
                id, encargado_id, encargado_nombre, encargado_codigo_empleado,
                gafete_numero, fecha_hora_entrega, usuario_entrega_id,
                usuario_entrega_nombre, uuid
            ) VALUES (
                1, 1, 'Kendall Morales', '5366536', 4,
                '2026-08-01T08:00:00Z', 1, 'Operador', 'uuid-prestamo'
            );
            ",
        )
        .unwrap();
    connection
}

#[test]
fn recibe_devoluciones_propias_gafete_provisional_sin_reencolar() {
    let connection = conexion_con_un_prestamo_gafete_provisional_activo();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-prestamo\",\"hora_devolucion\":\"2026-08-01T10:00:00Z\",\
         \"usuario_devolucion_nombre\":\"Operador remoto\"}]",
    );

    let aplicados =
        recibir_devoluciones_propias_gafete_provisional(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(aplicados, 1);
    let (devolucion, usuario_id, usuario_nombre): (String, Option<i64>, String) = connection
        .query_row(
            "SELECT fecha_hora_devolucion, usuario_devolucion_id, usuario_devolucion_nombre
             FROM prestamos_gafete_provisional WHERE uuid = 'uuid-prestamo'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(devolucion, "2026-08-01T10:00:00Z");
    assert_eq!(usuario_id, None);
    assert_eq!(usuario_nombre, "Operador remoto");
}

#[test]
fn recibe_devoluciones_propias_gafete_provisional_sin_nada_local_no_llama_a_la_nube() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    let aplicados = recibir_devoluciones_propias_gafete_provisional(
        &connection,
        &contexto("http://127.0.0.1:1"),
    )
    .unwrap();

    assert_eq!(aplicados, 0);
}

#[test]
fn sin_nada_abierto_localmente_no_llama_a_la_nube() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    // Base vacía -- ni una fila en `registro_ingresos`. Puerto sin nada
    // escuchando: si la función igual intentara pegarle a la red, esto
    // fallaría con un error de conexión en vez de devolver `Ok(0)`.
    let contexto = contexto("http://127.0.0.1:1");

    let aplicados = recibir_cierres_de_ingresos_propios(&connection, &contexto).unwrap();

    assert_eq!(aplicados, 0);
}

#[test]
fn normaliza_la_fecha_de_salida_que_devuelve_postgrest_antes_de_guardarla() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "
            INSERT INTO empresas (id, nombre, uuid) VALUES (1, 'Brisas', 'uuid-empresa');
            INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
            VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
            INSERT INTO contratistas (
                id, cedula, nombre, empresa_id, tipo_ingreso,
                es_personal_ruta, tiene_acceso, uuid
            ) VALUES (1, '2001', 'Persona', 1, 'SWAT', 0, 1, 'uuid-contratista');
            INSERT INTO registro_ingresos (
                id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso,
                tipo_ingreso, gafete_numero, usuario_ingreso_id, contratista_cedula,
                contratista_nombre, empresa_nombre, usuario_ingreso_nombre,
                fecha_vencimiento_praind, es_personal_ruta, tiene_acceso,
                resultado_acceso, motivo_resultado, reglas_version,
                empresa_activa_snapshot, uuid
            ) VALUES (
                1, 1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO',
                'SWAT', NULL, 1, '2001', 'Persona', 'Brisas', 'Operador',
                NULL, 0, 1, 'PERMITIDO', NULL, 1, 1, 'uuid-ingreso'
            );
            ",
        )
        .unwrap();
    // Formato real que devuelve `PostgREST` para un `timestamptz`
    // (fracción de segundo + offset "+00:00", no el "...Z" sin fracción
    // que exige `registro_ingresos_salida_utc`) -- este caso rompía la
    // sincronización en vivo aunque los tests con formato ya-canónico
    // pasaran.
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-ingreso\",\"hora_salida\":\"2026-01-01T10:00:00.123456+00:00\",\
         \"usuario_salida_nombre\":\"Operador remoto\"}]",
    );

    let aplicados = recibir_cierres_de_ingresos_propios(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(aplicados, 1);
    let salida: String = connection
        .query_row(
            "SELECT fecha_hora_salida FROM registro_ingresos WHERE uuid = 'uuid-ingreso'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(salida, "2026-01-01T10:00:00Z");
}

#[test]
fn cierra_un_ingreso_remoto_y_lo_saca_de_la_cache() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO ingresos_remotos (
                uuid, sitio_id, contratista_nombre, hora_entrada,
                usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
            ) VALUES (
                'uuid-remoto', 'sitio-1', 'Persona Remota', '2026-01-01T08:00:00Z',
                'Op PC', 'otro-dispositivo', '2026-01-01T08:00:00Z'
            )",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    cerrar_ingreso_remoto(
        &connection,
        &contexto(&base_url),
        "uuid-remoto",
        "Op Celular",
        chrono::Utc::now(),
    )
    .unwrap();

    let cacheados: i64 = connection
        .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(cacheados, 0);
}

/// Servidor de un solo pedido que devuelve el pedido COMPLETO (headers y
/// cuerpo) por el canal.
fn servidor_que_captura_el_cuerpo() -> (String, std::sync::mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
    let direccion = listener.local_addr().expect("dirección local");
    let (enviar, recibir) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let Ok((mut socket, _)) = listener.accept() else {
            return;
        };
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("set_read_timeout");
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let texto = String::from_utf8_lossy(&pedido).to_string();
            if let Some((cabecera, cuerpo)) = texto.split_once("\r\n\r\n") {
                let largo = cabecera
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                    })
                    .unwrap_or(0);
                if cuerpo.len() >= largo {
                    break;
                }
            }
            match socket.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(leidos) => pedido.extend_from_slice(&buffer[..leidos]),
            }
        }
        let _ = enviar.send(String::from_utf8_lossy(&pedido).to_string());
        let _ = socket.write_all(
            b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        );
    });
    (format!("http://{direccion}"), recibir)
}

#[test]
fn el_cierre_remoto_sella_la_hora_que_le_pasan_no_la_del_reloj_crudo() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let (base_url, pedido) = servidor_que_captura_el_cuerpo();
    // Un reloj corregido que difiere del crudo: la hora enviada tiene que
    // ser ésta, no `Utc::now()`.
    let hora = crate::tiempo::parsear_utc("2026-01-01T08:00:30Z").unwrap();

    cerrar_ingreso_remoto(
        &connection,
        &contexto(&base_url),
        "uuid-remoto",
        "Op PC",
        hora,
    )
    .unwrap();

    let pedido = pedido
        .recv_timeout(Duration::from_secs(3))
        .expect("pedido capturado");
    let esperado = format!(
        "\"hora_salida\":\"{}\"",
        crate::tiempo::serializar_utc(hora)
    );
    assert!(pedido.contains(&esperado), "{pedido}");
}

#[test]
fn cierra_un_ingreso_proveedor_remoto_y_lo_saca_de_la_cache() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO ingresos_proveedor_remotos (
                uuid, sitio_id, cedula, nombre, empresa_nombre, placa, gafete_numero,
                hora_entrada, usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
            ) VALUES (
                'uuid-remoto', 'sitio-1', '1-1111', 'Juan Perez', 'Maika', NULL, 9,
                '2026-01-01T08:00:00Z', 'Op PC', 'otro-dispositivo', '2026-01-01T08:00:00Z'
            )",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    cerrar_ingreso_proveedor_remoto(
        &connection,
        &contexto(&base_url),
        "uuid-remoto",
        "Op Celular",
        chrono::Utc::now(),
    )
    .unwrap();

    let cacheados: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ingresos_proveedor_remotos",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(cacheados, 0);
}

fn fila_gafete(
    id: &str,
    numero: i64,
    estado: &str,
    deudor_id: Option<&str>,
    actualizado: &str,
) -> FilaGafeteRemota {
    FilaGafeteRemota {
        id: id.to_string(),
        numero,
        tipo: "CONTRATISTA".to_string(),
        estado: estado.to_string(),
        contratista_portador_id: deudor_id.map(str::to_string),
        contratista_portador_nombre: None,
        visita_portador_id: None,
        updated_at: actualizado.to_string(),
    }
}

fn fila_gafete_visita(
    id: &str,
    numero: i64,
    estado: &str,
    visita_portador_id: Option<&str>,
    actualizado: &str,
) -> FilaGafeteRemota {
    FilaGafeteRemota {
        id: id.to_string(),
        numero,
        tipo: "VISITA".to_string(),
        estado: estado.to_string(),
        contratista_portador_id: None,
        contratista_portador_nombre: None,
        visita_portador_id: visita_portador_id.map(str::to_string),
        updated_at: actualizado.to_string(),
    }
}

#[test]
fn guardar_gafetes_marca_de_agua_avanza_al_mas_nuevo_cuando_nada_queda_pendiente() {
    let (connection, uuid_contratista) = conexion_con_contratista();
    let transaction = connection.unchecked_transaction().unwrap();
    let gafetes = vec![
        fila_gafete("g1", 1, "DISPONIBLE", None, "2026-01-01T00:00:00Z"),
        fila_gafete(
            "g2",
            2,
            "PERDIDO",
            Some(&uuid_contratista),
            "2026-01-02T00:00:00Z",
        ),
    ];

    let (recibidos, marca) = guardar_gafetes(&transaction, &gafetes).unwrap();

    assert_eq!(recibidos, 2);
    assert_eq!(
        marca.map(crate::tiempo::serializar_utc).as_deref(),
        Some("2026-01-02T00:00:00Z")
    );
}

#[test]
fn guardar_gafetes_perdido_sin_deudor_resoluble_se_omite_y_no_avanza_la_marca() {
    let (connection, uuid_contratista) = conexion_con_contratista();
    let transaction = connection.unchecked_transaction().unwrap();
    let gafetes = vec![
        // Este sí resuelve y se guarda -- pero no debe hacer avanzar la
        // marca, porque el de abajo (más nuevo) se queda pendiente.
        fila_gafete(
            "g1",
            1,
            "PERDIDO",
            Some(&uuid_contratista),
            "2026-01-01T00:00:00Z",
        ),
        // Deudor que no existe localmente todavía -- se salta (violaría
        // el CHECK de la tabla) en vez de fallar toda la sincronización.
        fila_gafete(
            "g2",
            2,
            "PERDIDO",
            Some("uuid-contratista-que-no-llego-todavia"),
            "2026-01-02T00:00:00Z",
        ),
    ];

    let (recibidos, marca) = guardar_gafetes(&transaction, &gafetes).unwrap();

    // g1 se guardó (violaría el CHECK si no) pero la marca de agua no
    // avanza nada este ciclo -- si avanzara hasta el `updated_at` de g1
    // (o más), el próximo sync (`updated_at=gt.marca`) ya no volvería a
    // pedir g2, y su deuda quedaría sin resolver para siempre aunque el
    // contratista faltante llegue después.
    assert_eq!(recibidos, 1);
    let guardado: i64 = transaction
        .query_row("SELECT COUNT(*) FROM gafetes WHERE numero = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(guardado, 1);
    let pendiente: i64 = transaction
        .query_row("SELECT COUNT(*) FROM gafetes WHERE numero = 2", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(pendiente, 0);
    assert_eq!(marca, None);
}

#[test]
fn gafete_de_visita_perdido_sin_visitante_local_queda_pendiente_y_resuelve_despues() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    // Primer intento: el `cita_visitante` todavía no llegó localmente
    // (mismo escenario que un contratista pendiente) -- `recibir_catalogo_del_sitio`
    // corre antes que `recibir_citas_del_sitio` en todos los call sites
    // actuales, así que esto es el caso normal, no uno raro.
    let transaction = connection.unchecked_transaction().unwrap();
    let gafetes_pendientes = vec![fila_gafete_visita(
        "g1",
        9,
        "PERDIDO",
        Some("uuid-visitante-1"),
        "2026-01-01T00:00:00Z",
    )];
    let (recibidos, marca) = guardar_gafetes(&transaction, &gafetes_pendientes).unwrap();
    assert_eq!(recibidos, 0);
    assert_eq!(marca, None);
    transaction.commit().unwrap();

    // Ahora sí llegó la cita/visitante (simulando que corrió
    // `recibir_citas_del_sitio` en el ciclo siguiente).
    connection
        .execute(
            "INSERT INTO citas (uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                anfitrion_correo, estado, creado_en)
             VALUES ('uuid-cita-1', '2026-08-01', '2026-08-08', 'Ana', 'ana@acme.com',
                'VIGENTE', '2026-08-01T00:00:00Z')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cita_visitantes (uuid, cita_id, cedula, nombre)
             VALUES ('uuid-visitante-1', 1, '1-2345', 'Jenna')",
            [],
        )
        .unwrap();

    let transaction = connection.unchecked_transaction().unwrap();
    let (recibidos, marca) = guardar_gafetes(&transaction, &gafetes_pendientes).unwrap();

    assert_eq!(recibidos, 1);
    assert_eq!(
        marca.map(crate::tiempo::serializar_utc).as_deref(),
        Some("2026-01-01T00:00:00Z")
    );
    let visita_portador_id: i64 = transaction
        .query_row(
            "SELECT visita_portador_id FROM gafetes WHERE numero = 9",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(visita_portador_id, 1);
}

#[test]
fn recibe_estado_de_gafete_desde_nube_aunque_el_cursor_local_ya_exista() {
    let (connection, _) = conexion_con_contratista();
    connection
        .execute(
            "INSERT INTO gafetes (numero, tipo, estado) VALUES (26, 'CONTRATISTA', 'DISPONIBLE')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE sincronizacion_estado SET catalogo_actualizado_hasta = '2099-01-01T00:00:00Z'",
            [],
        )
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        // 6 pedidos por sync: empresas/contratistas/usuarios/gafetes/
        // rutas/empresas_proveedor.
        for paso in 0..6 {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                let leidos = socket.read(&mut buffer).unwrap();
                assert!(leidos > 0);
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            let cuerpo = if paso == 3 {
                let pedido = String::from_utf8(pedido).unwrap();
                assert!(pedido.contains("/gafetes?sitio_id=eq.sitio-1"));
                // `updated_at` viaja en el `select=` (se necesita para la
                // marca propia de gafetes), pero acá lo que importa es
                // que NO haya filtro `updated_at=gt.` -- `gafetes_actualizado_hasta`
                // nunca se tocó en este test (sólo `catalogo_actualizado_hasta`,
                // la de arriba), así que su propio cursor sigue en NULL y
                // pide todo, sin importar qué tan adelantado esté el otro.
                assert!(
                    !pedido.contains("updated_at=gt."),
                    "el cursor de gafetes no debe heredar el de catálogo"
                );
                r#"[{"id":"gafete-remoto","numero":26,"tipo":"CONTRATISTA","estado":"PERDIDO","contratista_portador_id":"uuid-contratista","contratista_portador_nombre":null,"visita_portador_id":null,"visita_portador_nombre":null,"updated_at":"2026-01-01T00:00:00Z"}]"#
            } else {
                "[]"
            };
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}", cuerpo.len()).unwrap();
        }
    });
    let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();
    servidor.join().unwrap();
    assert_eq!(resumen.gafetes_recibidos, 1);
    let estado: (String, i64) = connection
        .query_row(
            "SELECT estado, contratista_portador_id FROM gafetes WHERE numero=26",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(estado, ("PERDIDO".to_string(), 1));
}

#[test]
fn segundo_sync_de_catalogo_pide_gafetes_con_su_propia_marca_guardada() {
    let (connection, _) = conexion_con_contratista();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        // Dos syncs completos = 12 pedidos (empresas/contratistas/
        // usuarios/gafetes/rutas/empresas_proveedor, dos veces). Sólo el
        // de gafetes trae algo, para que la marca de agua realmente
        // tenga algo que guardar.
        for paso in 0..12 {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                let leidos = socket.read(&mut buffer).unwrap();
                assert!(leidos > 0);
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            let cuerpo = if paso == 3 {
                let pedido = String::from_utf8(pedido).unwrap();
                assert!(
                    !pedido.contains("updated_at=gt."),
                    "primer sync: sin marca todavía, tiene que pedir todo"
                );
                r#"[{"id":"gafete-remoto","numero":26,"tipo":"CONTRATISTA","estado":"DISPONIBLE","contratista_portador_id":null,"contratista_portador_nombre":null,"visita_portador_id":null,"visita_portador_nombre":null,"updated_at":"2026-01-05T00:00:00Z"}]"#
            } else if paso == 9 {
                let pedido = String::from_utf8(pedido).unwrap();
                assert!(
                    pedido.contains("updated_at=gt.2026-01-05T00%3A00%3A00")
                        || pedido.contains("updated_at=gt.2026-01-05T00:00:00"),
                    "segundo sync: tiene que arrastrar la marca que dejó el primero -- pedido real: {pedido}"
                );
                "[]"
            } else {
                "[]"
            };
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}", cuerpo.len()).unwrap();
        }
    });

    recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();
    let marca_guardada: Option<String> = connection
        .query_row(
            "SELECT gafetes_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        marca_guardada.as_deref(),
        Some("2026-01-05T00:00:00.000000Z")
    );

    recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();
    servidor.join().unwrap();
}

#[test]
fn recibe_catalogo_del_sitio_y_lo_guarda_local() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-empresa-remota\",\"nombre\":\"Empresa Remota\",\"activa\":true,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-contratista-remoto\",\"nombre\":\"Persona Remota\",\
         \"identificacion\":\"1-1111\",\"empresa_id\":\"uuid-empresa-remota\",\
         \"empresa_nombre\":\"Empresa Remota\",\"activo\":true,\"tipo_ingreso\":\"SWAT\",\
         \"fecha_vencimiento_praind\":null,\"es_personal_ruta\":false,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    ]);

    let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(
        resumen,
        ResumenCatalogo {
            empresas_recibidas: 1,
            contratistas_recibidos: 1,
            usuarios_recibidos: 0,
            gafetes_recibidos: 0,
            rutas_recibidas: 0,
            empresas_proveedor_recibidas: 0,
        }
    );
    let (nombre_empresa, uuid_empresa): (String, Option<String>) = connection
        .query_row("SELECT nombre, uuid FROM empresas", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(nombre_empresa, "Empresa Remota");
    assert_eq!(uuid_empresa.as_deref(), Some("uuid-empresa-remota"));

    let (cedula, tipo_ingreso, uuid_contratista): (String, String, Option<String>) = connection
        .query_row(
            "SELECT cedula, tipo_ingreso, uuid FROM contratistas",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(cedula, "1-1111");
    assert_eq!(tipo_ingreso, "SWAT");
    assert_eq!(uuid_contratista.as_deref(), Some("uuid-contratista-remoto"));
}

#[test]
fn recibe_numeros_de_ruta_del_sitio_dentro_del_catalogo_general() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-ruta-remota\",\"numero\":79,\"activo\":true,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    ]);

    let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(resumen.rutas_recibidas, 1);
    let (numero, activo, uuid): (i64, i64, Option<String>) = connection
        .query_row("SELECT numero, activo, uuid FROM rutas", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(numero, 79);
    assert_eq!(activo, 1);
    assert_eq!(uuid.as_deref(), Some("uuid-ruta-remota"));
}

/// El pull que le faltaba a `empresas_proveedor`: antes sólo se
/// empujaba (local → nube), nunca se traía de vuelta -- un catálogo
/// creado en un dispositivo nunca aparecía en el otro (bug reportado
/// en pruebas reales, 2026-09-17).
#[test]
fn recibe_empresas_proveedor_del_sitio_dentro_del_catalogo_general() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-empresa-proveedor-remota\",\"nombre\":\"MayCorp\",\"activa\":true,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
    ]);

    let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(resumen.empresas_proveedor_recibidas, 1);
    let (nombre, activo, uuid): (String, i64, String) = connection
        .query_row(
            "SELECT nombre, activo, uuid FROM empresas_proveedor",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(nombre, "MayCorp");
    assert_eq!(activo, 1);
    assert_eq!(uuid, "uuid-empresa-proveedor-remota");
}

/// Mismo criterio que `recibir_catalogo_fusiona_con_una_fila_local_existente_sin_duplicarla`,
/// pero para empresas de proveedor -- una empresa creada en ESTE
/// dispositivo no debe duplicarse cuando la nube confirma la misma
/// fila (por nombre), sólo completarle el `uuid`.
#[test]
fn recibe_empresa_proveedor_fusiona_con_una_fila_local_existente_sin_duplicarla() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas_proveedor (nombre, uuid) VALUES ('MayCorp', 'uuid-local-temporal')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-empresa-proveedor-remota\",\"nombre\":\"MayCorp\",\"activa\":true,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
    ]);

    recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM empresas_proveedor", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 1, "no duplica la empresa que ya tenía por nombre");
    let uuid: String = connection
        .query_row("SELECT uuid FROM empresas_proveedor", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        uuid, "uuid-local-temporal",
        "COALESCE no pisa un uuid que ya tenía"
    );
}

/// Bug real reportado por el usuario 2026-09-21: un `UPDATE` directo en
/// Supabase que sube el nombre a mayúsculas nunca bajaba a los
/// dispositivos. Causa: con un solo `ON CONFLICT(nombre)`, el nombre
/// remoto ya no coincidía (case-sensitive) con la fila local existente
/// -- sin conflicto por nombre, el INSERT chocaba en cambio contra el
/// índice único de `uuid` con un error que abortaba el pull entero.
#[test]
fn recibe_empresa_proveedor_renombrada_en_la_nube_actualiza_la_fila_ya_sincronizada() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas_proveedor (nombre, uuid) VALUES ('Dos Pinos', 'uuid-dos-pinos')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-dos-pinos\",\"nombre\":\"DOS PINOS\",\"activa\":true,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
    ]);

    recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM empresas_proveedor", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 1, "no duplica la fila, la actualiza por uuid");
    let nombre: String = connection
        .query_row(
            "SELECT nombre FROM empresas_proveedor WHERE uuid = 'uuid-dos-pinos'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(nombre, "DOS PINOS");
}

/// Reproduce el incidente de producción tal cual (2026-09-21, reportado
/// con captura del error real en mobile): una empresa creada LOCAL antes
/// de sincronizar ("Mayca", con un uuid provisorio propio de este
/// dispositivo) y la misma empresa ya normalizada en la nube ("MAYCA",
/// con su uuid real) -- ni coinciden por `uuid` ni por nombre exacto,
/// sólo por `PLEGAR`. Sin la tercera rama del `ON CONFLICT` esto
/// abortaba el pull con "UNIQUE constraint failed:
/// `idx_empresas_proveedor_nombre_plegado`".
#[test]
fn recibe_empresa_proveedor_con_grafia_distinta_a_una_fila_local_se_fusiona_por_nombre_plegado() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas_proveedor (nombre, uuid) VALUES ('Mayca', 'uuid-local-provisorio')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-mayca-real\",\"nombre\":\"MAYCA\",\"activa\":true,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
    ]);

    recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM empresas_proveedor", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 1, "no deja las dos grafías como filas separadas");
    let (nombre, uuid): (String, String) = connection
        .query_row("SELECT nombre, uuid FROM empresas_proveedor", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(nombre, "MAYCA", "queda con la grafía que manda la nube");
    assert_eq!(
        uuid, "uuid-local-provisorio",
        "COALESCE no pisa el uuid que la fila local ya tenía"
    );
}

#[test]
fn recibe_catalogo_rutas_del_sitio_y_lo_guarda_local() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-vehiculo-remoto\",\"numero_unidad\":\"22906\",\"placa\":\"C12345\",\
         \"activo\":true,\"updated_at\":\"2026-01-01T00:00:00Z\"}]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-encargado-remoto\",\"codigo_empleado\":\"5040017\",\
         \"nombre\":\"Michael Araya Retana\",\"activo\":true,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
    ]);

    let resumen = recibir_catalogo_rutas_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(
        resumen,
        ResumenCatalogoRutas {
            vehiculos_recibidos: 1,
            encargados_recibidos: 1,
        }
    );
    let (placa, uuid_vehiculo): (String, Option<String>) = connection
        .query_row("SELECT placa, uuid FROM vehiculos_ruta", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(placa, "C12345");
    assert_eq!(uuid_vehiculo.as_deref(), Some("uuid-vehiculo-remoto"));
    let (codigo, nombre, cedula): (String, String, Option<String>) = connection
        .query_row(
            "SELECT codigo_empleado, nombre, cedula FROM encargados_ruta",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(codigo, "5040017");
    assert_eq!(nombre, "Michael Araya Retana");
    assert_eq!(cedula, None, "el catálogo remoto nunca trae cédula");
    let marca_guardada: Option<String> = connection
        .query_row(
            "SELECT catalogo_rutas_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        marca_guardada.as_deref(),
        Some("2026-01-01T00:00:00.000000Z")
    );
}

#[test]
fn recibir_catalogo_rutas_fusiona_un_vehiculo_local_existente_por_placa_sin_duplicarlo() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO vehiculos_ruta (numero_unidad, placa, uuid)
             VALUES (NULL, 'C12345', 'uuid-local-viejo')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-vehiculo-remoto\",\"numero_unidad\":\"22906\",\"placa\":\"C12345\",\
         \"activo\":true,\"updated_at\":\"2026-01-01T00:00:00Z\"}]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    ]);

    recibir_catalogo_rutas_del_sitio(&connection, &contexto(&base_url)).unwrap();

    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM vehiculos_ruta", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total, 1, "no duplica el vehículo que ya tenía por placa");
    let (numero_unidad, uuid): (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT numero_unidad, uuid FROM vehiculos_ruta",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        numero_unidad.as_deref(),
        Some("22906"),
        "la fila local se actualiza con lo remoto"
    );
    assert_eq!(
        uuid.as_deref(),
        Some("uuid-local-viejo"),
        "un uuid local ya existente nunca se pisa"
    );
}

#[test]
fn segundo_sync_de_catalogo_rutas_pide_solo_lo_nuevo_con_la_marca_guardada() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        // Primer sync = 2 pedidos (vehículos/encargados) sin marca;
        // segundo sync = 2 más, ya con `updated_at=gt.` de la marca que
        // dejó el primero.
        for paso in 0..4 {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                let leidos = socket.read(&mut buffer).unwrap();
                assert!(leidos > 0);
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            let cuerpo = if paso == 0 {
                let pedido = String::from_utf8(pedido).unwrap();
                assert!(
                    !pedido.contains("updated_at=gt."),
                    "primer sync: sin marca todavía, tiene que pedir todo"
                );
                r#"[{"id":"uuid-vehiculo-remoto","numero_unidad":"22906","placa":"C12345","activo":true,"updated_at":"2026-01-05T00:00:00Z"}]"#
            } else if paso == 2 {
                let pedido = String::from_utf8(pedido).unwrap();
                assert!(
                    pedido.contains("updated_at=gt.2026-01-05T00%3A00%3A00")
                        || pedido.contains("updated_at=gt.2026-01-05T00:00:00"),
                    "segundo sync: tiene que arrastrar la marca que dejó el primero -- pedido real: {pedido}"
                );
                "[]"
            } else {
                "[]"
            };
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}", cuerpo.len()).unwrap();
        }
    });

    recibir_catalogo_rutas_del_sitio(&connection, &contexto(&base_url)).unwrap();
    let marca_guardada: Option<String> = connection
        .query_row(
            "SELECT catalogo_rutas_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        marca_guardada.as_deref(),
        Some("2026-01-05T00:00:00.000000Z")
    );

    recibir_catalogo_rutas_del_sitio(&connection, &contexto(&base_url)).unwrap();
    servidor.join().unwrap();
}

#[test]
fn recibir_catalogo_fusiona_con_una_fila_local_existente_sin_duplicarla() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas (nombre) VALUES ('Empresa Remota')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
            ) VALUES ('1-1111', 'Persona Local', 1, 'SWAT', 0, 1)",
            [],
        )
        .unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-empresa-remota\",\"nombre\":\"Empresa Remota\",\"activa\":true,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-contratista-remoto\",\"nombre\":\"Persona Remota\",\
         \"identificacion\":\"1-1111\",\"empresa_id\":\"uuid-empresa-remota\",\
         \"empresa_nombre\":\"Empresa Remota\",\"activo\":true,\"tipo_ingreso\":\"SWAT\",\
         \"fecha_vencimiento_praind\":null,\"es_personal_ruta\":false,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    ]);

    recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    let total_empresas: i64 = connection
        .query_row("SELECT COUNT(*) FROM empresas", [], |row| row.get(0))
        .unwrap();
    let total_contratistas: i64 = connection
        .query_row("SELECT COUNT(*) FROM contratistas", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        total_empresas, 1,
        "no duplica la empresa que ya tenía por nombre"
    );
    assert_eq!(
        total_contratistas, 1,
        "no duplica el contratista que ya tenía por cédula"
    );
    let (nombre_final, uuid_final): (String, Option<String>) = connection
        .query_row("SELECT nombre, uuid FROM contratistas", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(
        nombre_final, "Persona Remota",
        "la fila local se actualiza con lo remoto"
    );
    assert_eq!(
        uuid_final.as_deref(),
        Some("uuid-contratista-remoto"),
        "le completa el uuid"
    );
}

/// Mismo bug real que `recibe_empresa_proveedor_renombrada_en_la_nube_actualiza_la_fila_ya_sincronizada`,
/// pero para `empresas` (contratistas) -- comparten el mismo patrón de
/// `guardar_*` con `ON CONFLICT` encadenado.
#[test]
fn recibe_empresa_de_contratistas_renombrada_en_la_nube_actualiza_la_fila_ya_sincronizada() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas (nombre, uuid) VALUES ('Mi Empresa', 'uuid-mi-empresa')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-mi-empresa\",\"nombre\":\"MI EMPRESA\",\"activa\":true,\
         \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    ]);

    recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM empresas", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total, 1, "no duplica la fila, la actualiza por uuid");
    let nombre: String = connection
        .query_row(
            "SELECT nombre FROM empresas WHERE uuid = 'uuid-mi-empresa'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(nombre, "MI EMPRESA");
}

#[test]
fn recibir_catalogo_salta_un_contratista_remoto_sin_identificacion_o_tipo_ingreso() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-incompleto\",\"nombre\":\"Persona Incompleta\",\
         \"identificacion\":null,\"empresa_id\":null,\"empresa_nombre\":null,\
         \"activo\":true,\"tipo_ingreso\":null,\"fecha_vencimiento_praind\":null,\
         \"es_personal_ruta\":null,\"updated_at\":\"2026-01-01T00:00:00Z\"}]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    ]);

    let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(resumen.contratistas_recibidos, 0);
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM contratistas", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        total, 0,
        "una fila sin datos suficientes para las reglas de acceso no se inventa"
    );
}

/// N6: el catálogo baja página por página y cada página se guarda en su
/// propia transacción. Si la red se corta a mitad, lo que ya llegó queda
/// guardado, pero la marca de agua NO avanza: el próximo sync vuelve a
/// pedir desde la marca anterior y no se pierde nada de lo que faltó.
#[test]
fn catalogo_cortado_a_mitad_conserva_lo_bajado_y_no_avanza_la_marca() {
    use std::fmt::Write as _;

    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    let mut filas = String::new();
    for i in 0..TAMANO_PAGINA_REMOTA {
        if i > 0 {
            filas.push(',');
        }
        write!(
            filas,
            "{{\"id\":\"uuid-vehiculo-{i}\",\"numero_unidad\":\"{i}\",\"placa\":\"P{i:05}\",\
             \"activo\":true,\"updated_at\":\"2026-01-01T00:00:00Z\"}}"
        )
        .unwrap();
    }
    let cuerpo = format!("[{filas}]");
    let pagina_1 = Box::leak(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .into_boxed_str(),
    ) as &'static str;
    let base_url = servidor_de_respuestas(vec![
        pagina_1,
        "HTTP/1.1 503 Service Unavailable\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
    ]);

    let resultado = recibir_catalogo_rutas_del_sitio(&connection, &contexto(&base_url));

    assert!(resultado.is_err(), "la segunda página falló: {resultado:?}");
    let guardados: i64 = connection
        .query_row("SELECT COUNT(*) FROM vehiculos_ruta", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        guardados,
        i64::try_from(TAMANO_PAGINA_REMOTA).unwrap(),
        "la página que sí llegó queda guardada"
    );
    let marca: Option<String> = connection
        .query_row(
            "SELECT catalogo_rutas_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(marca, None, "sin todas las páginas, la marca no avanza");
}

/// Respuesta de `PostgREST` cuando el índice único de "gafete en uso"
/// `indice` rechaza la apertura (`'static` como pide `servidor_de_una_respuesta`).
fn respuesta_gafete_en_uso(indice: &str) -> &'static str {
    let respuesta = format!(
        "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         {{\"code\":\"23505\",\"details\":null,\"hint\":null,\
         \"message\":\"duplicate key value violates unique constraint \\\"{indice}\\\"\"}}"
    );
    Box::leak(respuesta.into_boxed_str())
}

fn estado_en_cola(connection: &Connection, entidad_uuid: &str) -> String {
    connection
        .query_row(
            "SELECT estado FROM cola_salida WHERE entidad_uuid = ?1",
            params![entidad_uuid],
            |row| row.get(0),
        )
        .unwrap()
}

/// Mismo bloqueo cruzado que en contratistas: la apertura de un ingreso de
/// proveedor con un gafete que otro dispositivo del sitio ya tiene activo
/// queda fallida de inmediato y se reporta como conflicto de proveedor.
#[test]
fn ingreso_proveedor_con_gafete_ya_activo_en_otro_dispositivo_queda_fallido_de_inmediato() {
    let (connection, uuid) = conexion_con_ingreso_proveedor(None);
    let base_url = servidor_de_una_respuesta(respuesta_gafete_en_uso(
        "ingresos_proveedor_gafete_activo_sitio_idx",
    ));

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.fallidos, 1);
    assert_eq!(
        resumen.conflictos_gafete,
        vec![ConflictoGafeteActivo {
            tipo: TipoMovimientoGafete::Proveedor,
            nombre: "Juan Perez".to_string(),
            gafete_numero: 12,
            fecha_hora: "2026-01-01T08:00:00Z".to_string(),
        }]
    );
    assert_eq!(estado_en_cola(&connection, &uuid), "fallido");
}

/// Lo mismo para la entrega de un gafete provisional KOF.
#[test]
fn prestamo_kof_con_gafete_ya_activo_en_otro_dispositivo_queda_fallido_de_inmediato() {
    let connection = conexion_con_un_prestamo_gafete_provisional_activo();
    connection
        .execute(
            "INSERT INTO cola_salida (entidad, entidad_uuid, operacion, creado_en, actualizado_en)
             VALUES ('prestamo_gafete_provisional', 'uuid-prestamo', 'crear',
                     '2026-08-01T08:00:00Z', '2026-08-01T08:00:00Z')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(respuesta_gafete_en_uso(
        "prestamos_gafete_provisional_gafete_activo_sitio_idx",
    ));

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.fallidos, 1);
    assert_eq!(
        resumen.conflictos_gafete,
        vec![ConflictoGafeteActivo {
            tipo: TipoMovimientoGafete::ProvisionalKof,
            nombre: "Kendall Morales".to_string(),
            gafete_numero: 4,
            fecha_hora: "2026-08-01T08:00:00Z".to_string(),
        }]
    );
    assert_eq!(estado_en_cola(&connection, "uuid-prestamo"), "fallido");
}

/// El índice de OTRA tabla no cuenta como conflicto de esta: un proveedor
/// que choca con algo que no es su índice de "gafete en uso" sigue el
/// camino normal de reintento.
#[test]
fn ingreso_proveedor_con_un_409_de_otro_indice_sigue_el_camino_normal() {
    let (connection, uuid) = conexion_con_ingreso_proveedor(None);
    let base_url =
        servidor_de_una_respuesta(respuesta_gafete_en_uso("ingresos_gafete_activo_sitio_idx"));

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert!(resumen.conflictos_gafete.is_empty());
    assert_eq!(estado_en_cola(&connection, &uuid), "pendiente");
}

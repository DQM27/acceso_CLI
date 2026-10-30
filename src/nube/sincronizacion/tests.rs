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

/// Respuesta `200` con `cuerpo` JSON y su `Content-Length` (`'static` como
/// pide `servidor_de_respuestas`).
fn respuesta_json(cuerpo: &str) -> &'static str {
    Box::leak(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\
             Content-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .into_boxed_str(),
    )
}

/// Como `servidor_de_respuestas`, pero además anota cada pedido completo
/// (línea, cabeceras y cuerpo) para que la prueba revise qué se pidió.
fn servidor_que_anota_pedidos(respuestas: Vec<&'static str>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
    let direccion = listener.local_addr().expect("dirección local");
    let pedidos = Arc::new(Mutex::new(Vec::new()));
    let anotados = Arc::clone(&pedidos);
    thread::spawn(move || {
        for respuesta in respuestas {
            let Ok((mut conexion, _)) = listener.accept() else {
                return;
            };
            conexion
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("set_read_timeout");
            let mut pedido = Vec::new();
            let mut buffer = [0_u8; 8192];
            let fin_cabeceras = loop {
                let leidos = conexion.read(&mut buffer).unwrap_or(0);
                if leidos == 0 {
                    break pedido.len();
                }
                pedido.extend_from_slice(&buffer[..leidos]);
                if let Some(posicion) = pedido.windows(4).position(|w| w == b"\r\n\r\n") {
                    break posicion + 4;
                }
            };
            let cabeceras = String::from_utf8_lossy(&pedido[..fin_cabeceras]).to_lowercase();
            let largo: usize = cabeceras
                .lines()
                .find_map(|linea| linea.strip_prefix("content-length:"))
                .and_then(|valor| valor.trim().parse().ok())
                .unwrap_or(0);
            while pedido.len() < fin_cabeceras + largo {
                let leidos = conexion.read(&mut buffer).unwrap_or(0);
                if leidos == 0 {
                    break;
                }
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            anotados
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&pedido).into_owned());
            let _ = conexion.write_all(respuesta.as_bytes());
            let _ = conexion.flush();
        }
    });
    (format!("http://{direccion}"), pedidos)
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

    // El lote falló una vez (no cuenta como fallo por fila), y las filas
    // se reintentaron de a una dentro de la misma llamada: dos con éxito.
    // La tercera no tiene respuesta preparada: un error de red, que corta
    // la pasada sin castigarla (ver `cola::es_fallo_de_la_conexion`).
    let error = drenar_cola(&connection, &contexto(&base_url), 10)
        .expect_err("la falta de red corta la pasada");
    assert!(error.es_falta_de_conexion(), "{error}");

    let contar = |estado: &str| -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM cola_salida WHERE estado = ?1",
                params![estado],
                |row| row.get(0),
            )
            .unwrap()
    };
    assert_eq!(contar("enviado"), 2);
    assert_eq!(
        contar("pendiente"),
        1,
        "la tercera fila queda para reintentar, no perdida ni duplicada"
    );
    let intentos: i64 = connection
        .query_row(
            "SELECT intentos FROM cola_salida WHERE estado = 'pendiente'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        intentos, 0,
        "sin intento ni espera: la red no es culpa de la fila"
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
    // Agotar los reintentos sí es un error real: prende el aviso rojo.
    assert_eq!(estado_bandeja_salida(&connection).unwrap().fallidos, 1);
    let detalle = detalle_fallidos_cola(&connection, 40).unwrap();
    assert_eq!(
        (detalle[0].motivo.as_str(), detalle[0].intentos),
        ("error", INTENTOS_ANTES_DE_FALLO_PERMANENTE)
    );
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
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![
        RESPUESTA_201,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-empresa-proveedor\",\"nombre\":\"MAIKA\"}]",
    ]);

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
    // El alta no pisa una empresa que ya esté en la nube con ese nombre
    // (ignora el duplicado) y después confirma cuál quedó.
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 2, "{pedidos:#?}");
    assert!(
        pedidos[0].starts_with("POST /rest/v1/empresas_proveedor?on_conflict=sitio_id,nombre"),
        "{pedidos:#?}"
    );
    assert!(
        pedidos[0]
            .to_lowercase()
            .contains("resolution=ignore-duplicates"),
        "{pedidos:#?}"
    );
    assert!(
        pedidos[1].starts_with("GET /rest/v1/empresas_proveedor?sitio_id=eq.sitio-1"),
        "{pedidos:#?}"
    );
}

/// Dos equipos dieron de alta la misma empresa (cada uno con su `uuid`). El
/// segundo ya no le cambia el `id` a la de la nube -- eso dejaba colgados
/// los ingresos del primero --: adopta el `uuid` de la nube, y sus ingresos
/// (que apuntan a la empresa por el `id` local) viajan con ese `uuid`.
#[test]
fn si_el_nombre_ya_esta_en_la_nube_adopta_su_uuid_y_los_ingresos_lo_usan() {
    let (connection, _) = conexion_con_ingreso_proveedor(None);
    connection
        .execute_batch(
            "DELETE FROM cola_salida;
             INSERT INTO cola_salida (entidad, entidad_uuid, operacion, creado_en, actualizado_en)
             VALUES ('empresa_proveedor', 'uuid-empresa-proveedor', 'crear',
                     '2026-01-01T07:00:00Z', '2026-01-01T07:00:00Z'),
                    ('ingreso_proveedor', 'uuid-ingreso-proveedor', 'crear',
                     '2026-01-01T08:00:00Z', '2026-01-01T08:00:00Z');",
        )
        .unwrap();
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![
        RESPUESTA_201,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-de-la-nube\",\"nombre\":\"MAIKA\"},{\"id\":\"otra\",\"nombre\":\"Otra\"}]",
        RESPUESTA_201,
    ]);

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!((resumen.enviados, resumen.fallidos), (2, 0));
    let uuid_local: String = connection
        .query_row(
            "SELECT uuid FROM empresas_proveedor WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(uuid_local, "uuid-de-la-nube");
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 3, "{pedidos:#?}");
    assert!(
        pedidos[2].starts_with("POST /rest/v1/ingresos_proveedor"),
        "{pedidos:#?}"
    );
    assert!(
        pedidos[2].contains("\"empresa_id\":\"uuid-de-la-nube\""),
        "{pedidos:#?}"
    );
}

/// Incidente 2026-10-30: el alta de 8 empresas de proveedor había fallado
/// para siempre y todo ingreso de esas empresas caía por la clave foránea
/// (409), sin que nada volviera a mandarlas. Ahora, ante ese 409, la cola
/// manda la empresa y reintenta el ingreso en la misma pasada.
#[test]
fn un_ingreso_de_proveedor_sin_su_empresa_en_la_nube_la_manda_y_se_reintenta() {
    let (connection, _) = conexion_con_ingreso_proveedor(None);
    connection
        .execute(
            "INSERT INTO cola_salida (
                 entidad, entidad_uuid, operacion, estado, motivo_fallo, intentos,
                 creado_en, actualizado_en, ultimo_error
             ) VALUES ('empresa_proveedor', 'uuid-empresa-proveedor', 'crear', 'fallido', 'error', 20,
                       '2026-01-01T07:00:00Z', '2026-01-01T07:00:00Z', 'El receptor respondió 403: {}')",
            [],
        )
        .unwrap();
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![
        "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         {\"code\":\"23503\",\"message\":\"insert or update on table \\\"ingresos_proveedor\\\" violates \
         foreign key constraint \\\"ingresos_proveedor_empresa_id_fkey\\\"\"}",
        RESPUESTA_201,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-empresa-proveedor\",\"nombre\":\"Maika\"}]",
        RESPUESTA_201,
    ]);

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!((resumen.enviados, resumen.fallidos), (1, 0));
    let estados: Vec<(String, String)> = connection
        .prepare("SELECT entidad, estado FROM cola_salida ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        estados,
        vec![
            ("ingreso_proveedor".to_string(), "enviado".to_string()),
            ("empresa_proveedor".to_string(), "enviado".to_string()),
        ]
    );
    assert_eq!(estado_bandeja_salida(&connection).unwrap().fallidos, 0);
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 4, "{pedidos:#?}");
    assert!(pedidos[1].starts_with("POST /rest/v1/empresas_proveedor"));
    assert!(pedidos[3].starts_with("POST /rest/v1/ingresos_proveedor"));
}

/// `empresas_proveedor` es por unidad (`sitio_id NOT NULL` y política de alta
/// que lo compara con el del token): sin `sitio_id` en el cuerpo la nube
/// responde 403 y el ingreso que la usa cae por la FK (409), en bucle.
#[test]
fn el_cuerpo_de_una_empresa_proveedor_lleva_el_sitio_del_equipo() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas_proveedor (nombre, activo, uuid)
             VALUES ('Maika', 1, 'uuid-empresa-proveedor')",
            [],
        )
        .unwrap();

    let cuerpo = construir_cuerpo_empresa_proveedor(
        &connection,
        &contexto("http://localhost"),
        "uuid-empresa-proveedor",
    )
    .unwrap();

    assert_eq!(cuerpo["sitio_id"], "sitio-1");
    assert_eq!(cuerpo["nombre"], "Maika");
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

/// Un ingreso registrado sin conexión cuyo contratista ya está adentro en
/// otra unidad: la nube lo rechaza con `ingresos_contratista_activo_idx` y la
/// fila queda `fallido` de inmediato (no se reintenta por días). No es un
/// conflicto de gafete: el aviso lo da `contratistas_con_conflicto_activo`.
#[test]
fn apertura_de_ingreso_de_contratista_ya_adentro_queda_fallida_de_inmediato() {
    let (connection, _contratista_uuid) = conexion_con_contratista();
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
                1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO', 'SWAT',
                1, '1-2345', 'Persona de prueba', 'Brisas',
                'Op', 0, 1, 'PERMITIDO', 1, NULL, 'uuid-ingreso'
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
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         {\"code\":\"23505\",\"details\":\"Key (contratista_cedula)=(1-2345) already exists.\",\
         \"hint\":null,\"message\":\"duplicate key value violates unique constraint \\\"ingresos_contratista_activo_idx\\\"\"}",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.enviados, 0);
    assert_eq!(resumen.fallidos, 1);
    assert!(resumen.conflictos_gafete.is_empty());
    let (estado, intentos): (String, i64) = connection
        .query_row(
            "SELECT estado, intentos FROM cola_salida WHERE entidad_uuid = 'uuid-ingreso'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(estado, "fallido");
    assert_eq!(intentos, 1);

    // Es un conflicto del negocio, ya avisado: no prende el aviso rojo de
    // "no se pudo subir" (incidente 2026-10-30, 28 conflictos viejos
    // mostrados como registros perdidos).
    assert_eq!(estado_bandeja_salida(&connection).unwrap().fallidos, 0);
    assert_eq!(contar_fallos_permanentes(&connection).unwrap(), 0);

    // El detalle para estudiarlo desde la nube trae sólo datos técnicos: ni
    // la cédula ni el nombre que venían en el cuerpo de la respuesta.
    let detalle = detalle_fallidos_cola(&connection, 40).unwrap();
    assert_eq!(detalle.len(), 1);
    let fallo = &detalle[0];
    assert_eq!(
        (
            fallo.entidad.as_str(),
            fallo.operacion.as_str(),
            fallo.entidad_uuid.as_str(),
            fallo.motivo.as_str(),
            fallo.estado_http,
            fallo.codigo.as_deref(),
            fallo.restriccion.as_deref(),
        ),
        (
            "ingreso",
            "crear",
            "uuid-ingreso",
            "conflicto",
            Some(409),
            Some("23505"),
            Some("ingresos_contratista_activo_idx"),
        )
    );
    assert!(!format!("{fallo:?}").contains("1-2345"), "{fallo:?}");
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

/// Va por la función `ingreso_activo_de_contratista` (POST a `/rpc`), no por
/// `/rest/v1/ingresos`: la RLS no deja ver los ingresos de otras unidades, y
/// la consulta directa siempre respondía "libre".
#[test]
fn contratista_con_ingreso_activo_pregunta_a_la_funcion_de_la_nube() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !String::from_utf8_lossy(&pedido).contains("\"p_cedula\":\"2001\"") {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0, "el pedido no trajo la cédula");
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.starts_with("POST /rest/v1/rpc/ingreso_activo_de_contratista "));
        let cuerpo = "[{\"sitio_id\":\"otro\",\"sitio_nombre\":\"Cartago\"}]";
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
         [{\"contratista_cedula\":\"2001\",\"sitio_nombre\":\"Cartago\"}]",
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

/// `estado_del_sitio` manda las cédulas que el equipo tiene adentro y trae
/// los conflictos en el mismo llamado. Al final se cruzan contra lo que
/// sigue adentro AHORA: si mientras tanto llegó el cierre de alguien (lo
/// cerró el otro equipo), su conflicto ya no se avisa.
#[test]
fn los_conflictos_llegan_en_el_estado_y_se_cruzan_con_lo_que_sigue_adentro() {
    let connection = conexion_con_dos_ingresos_activos();
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![respuesta_json(
        r#"{"ingresos_abiertos":[],
            "conflictos_ingresos":[{"contratista_cedula":"2001","sitio_nombre":"Cartago"},
                                   {"contratista_cedula":"2002","sitio_nombre":"Cartago"}]}"#,
    )]);
    let contexto = contexto(&base_url);
    let alcance = crate::nube::alcance::AlcanceSincronizacion {
        ingresos: true,
        ..crate::nube::alcance::AlcanceSincronizacion::solo_envio()
    };

    let mut estado = pedir_estado_del_sitio(&connection, &contexto, &alcance)
        .unwrap()
        .expect("la nube tiene la función");
    // Mientras tanto llegó el cierre de "Persona Dos" (salida que dio el otro
    // equipo), por el mismo camino que usa la sincronización.
    assert!(
        aplicar_cierre_de_ingreso_propio(
            &connection,
            "uuid-i2",
            "2026-01-01T09:00:00Z",
            Some("OPERADOR A2")
        )
        .unwrap()
    );
    let conflictos = contratistas_con_conflicto_activo_con(
        &connection,
        &contexto,
        Some(std::mem::take(&mut estado.conflictos_ingresos)),
    )
    .unwrap();

    assert_eq!(
        conflictos,
        vec![ConflictoIngresoActivo {
            cedula: "2001".to_string(),
            contratista_nombre: "Persona Uno".to_string(),
            sitio_conflicto: "Cartago".to_string(),
        }]
    );
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 1, "ninguna consulta aparte: {pedidos:#?}");
    assert!(
        pedidos[0].contains(r#""p_cedulas_ingresos":["2001","2002"]"#),
        "{}",
        pedidos[0]
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

/// Va por la función `visita_activa_de_visitante`: la consulta directa a
/// `/rest/v1/movimientos_visita?...&sitio_id=neq.` chocaba con la RLS (cada
/// equipo sólo lee su unidad) y siempre daba vacío.
#[test]
fn visitante_activo_en_otro_sitio_pregunta_a_la_funcion_de_la_nube() {
    let (base_url, servidor) = servidor_rpc(
        "visita_activa_de_visitante",
        "\"p_cedula\":\"1-2345\"",
        "[{\"sitio_id\":\"otro\",\"sitio_nombre\":\"Cartago\"}]",
    );

    let sitio = visitante_activo_en_otro_sitio(&contexto(&base_url), "1-2345").unwrap();

    assert_eq!(sitio, Some("Cartago".to_string()));
    servidor.join().unwrap();
}

/// Una visita abierta en ESTA unidad sólo puede ser del otro equipo (lo
/// propio lo frena antes el chequeo local): se nombra así.
#[test]
fn visitante_activo_en_esta_unidad_se_nombra_como_otro_equipo() {
    let (base_url, servidor) = servidor_rpc(
        "visita_activa_de_visitante",
        "\"p_cedula\":\"1-2345\"",
        "[{\"sitio_id\":\"sitio-1\",\"sitio_nombre\":\"Brisas\"}]",
    );

    let sitio = visitante_activo_en_otro_sitio(&contexto(&base_url), "1-2345").unwrap();

    assert_eq!(
        sitio,
        Some("Brisas (otro equipo de esta unidad)".to_string())
    );
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
    let (base_url, servidor) = servidor_rpc(
        "visitantes_activos_en_otras_unidades",
        "\"p_cedulas\":[\"1-2345\",\"6-7890\"]",
        "[{\"visitante_cedula\":\"1-2345\",\"sitio_nombre\":\"Cartago\"}]",
    );

    let conflictos = visitantes_con_conflicto_activo(&connection, &contexto(&base_url)).unwrap();
    servidor.join().unwrap();

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

/// Servidor de una sola petición que exige un POST a `/rest/v1/rpc/<funcion>`
/// con `fragmento` en el cuerpo, y responde `respuesta` (JSON).
fn servidor_rpc(
    funcion: &'static str,
    fragmento: &'static str,
    respuesta: &'static str,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !String::from_utf8_lossy(&pedido).contains(fragmento) {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0, "el pedido no trajo {fragmento}");
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.starts_with(&format!("POST /rest/v1/rpc/{funcion} ")));
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{respuesta}",
            respuesta.len()
        )
        .unwrap();
    });
    (base_url, servidor)
}

/// Va por la función `ingreso_proveedor_activo`: la RLS no deja ver los
/// ingresos de proveedor de otras unidades.
#[test]
fn proveedor_con_ingreso_activo_pregunta_a_la_funcion_de_la_nube() {
    let (base_url, servidor) = servidor_rpc(
        "ingreso_proveedor_activo",
        "\"p_cedula\":\"1-2345\"",
        "[{\"sitio_id\":\"otro\",\"sitio_nombre\":\"Cartago\"}]",
    );

    let activo = proveedor_con_ingreso_activo(&contexto(&base_url), "1-2345").unwrap();

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
fn proveedor_con_ingreso_activo_distingue_el_mismo_sitio() {
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"sitio_id\":\"sitio-1\",\"sitio_nombre\":\"Brisas\"}]",
    );

    let activo = proveedor_con_ingreso_activo(&contexto(&base_url), "1-2345").unwrap();

    assert_eq!(activo.map(|a| a.mismo_sitio), Some(true));
}

#[test]
fn proveedor_con_ingreso_activo_sin_conflicto_devuelve_none() {
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );

    let activo = proveedor_con_ingreso_activo(&contexto(&base_url), "1-2345").unwrap();

    assert_eq!(activo, None);
}

/// KOF: el encargado se busca por su código de empleado, no por su id.
#[test]
fn encargado_con_prestamo_provisional_activo_pregunta_por_codigo_de_empleado() {
    let (base_url, servidor) = servidor_rpc(
        "prestamo_provisional_activo_de_encargado",
        "\"p_codigo_empleado\":\"KOF-77\"",
        "[{\"sitio_id\":\"otro\",\"sitio_nombre\":\"Cartago\"}]",
    );

    let prestamo =
        encargado_con_prestamo_provisional_activo(&contexto(&base_url), "KOF-77").unwrap();

    assert_eq!(
        prestamo,
        Some(IngresoActivoEnLaNube {
            mismo_sitio: false,
            sitio_nombre: "Cartago".to_string(),
        })
    );
    servidor.join().unwrap();
}

/// La cola reconoce el rechazo de cada índice "una persona, una apertura" y
/// sólo el de su propia entidad; un cierre nunca cuenta.
#[test]
fn reconoce_el_rechazo_por_persona_ya_adentro_de_cada_entidad() {
    fn fila(entidad: &str, operacion: &str) -> super::cola::FilaCola {
        super::cola::FilaCola {
            id: 1,
            entidad: entidad.to_string(),
            entidad_uuid: "uuid".to_string(),
            operacion: operacion.to_string(),
            intentos: 0,
        }
    }
    fn rechazo(indice: &str) -> SincronizacionError {
        SincronizacionError::RespuestaInesperada {
            status: 409,
            cuerpo: format!(
                "{{\"code\":\"23505\",\"message\":\"duplicate key value violates unique constraint \\\"{indice}\\\"\"}}"
            ),
        }
    }
    use super::cola::es_conflicto_ingreso_activo as es_conflicto;

    assert!(es_conflicto(
        &fila("ingreso", "crear"),
        &rechazo("ingresos_contratista_activo_idx")
    ));
    assert!(es_conflicto(
        &fila("ingreso_proveedor", "crear"),
        &rechazo("ingresos_proveedor_cedula_activa_idx")
    ));
    assert!(es_conflicto(
        &fila("prestamo_gafete_provisional", "crear"),
        &rechazo("prestamos_gafete_provisional_encargado_activo_idx"),
    ));
    // El índice de otra entidad, o el del gafete, no cuenta.
    assert!(!es_conflicto(
        &fila("ingreso_proveedor", "crear"),
        &rechazo("ingresos_contratista_activo_idx")
    ));
    assert!(!es_conflicto(
        &fila("ingreso_proveedor", "crear"),
        &rechazo("ingresos_proveedor_gafete_activo_sitio_idx")
    ));
    // Un cierre nunca choca con estos índices.
    assert!(!es_conflicto(
        &fila("ingreso_proveedor", "cerrar"),
        &rechazo("ingresos_proveedor_cedula_activa_idx")
    ));
    // Otro estado HTTP tampoco.
    assert!(!es_conflicto(
        &fila("ingreso_proveedor", "crear"),
        &SincronizacionError::RespuestaInesperada {
            status: 500,
            cuerpo: "ingresos_proveedor_cedula_activa_idx".to_string()
        },
    ));
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
         [{\"cedula\":\"1-2345\",\"sitio_nombre\":\"Cartago\"}]",
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

fn uuid_de_prueba(n: u32) -> String {
    format!("00000000-0000-0000-0000-{n:012}")
}

/// Una caché con `cuantos` ingresos abiertos por el otro dispositivo, con los
/// uuid de `uuid_de_prueba(1..=cuantos)`.
fn conexion_con_ingresos_remotos(cuantos: u32) -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    for n in 1..=cuantos {
        connection
            .execute(
                "INSERT INTO ingresos_remotos (
                    uuid, sitio_id, contratista_nombre, hora_entrada,
                    usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
                ) VALUES (?1, 'sitio-1', 'Persona', '2026-01-01T08:00:00Z',
                    'Op PC', 'otro-dispositivo', '2026-01-01T08:00:00Z')",
                [uuid_de_prueba(n)],
            )
            .unwrap();
    }
    connection
}

fn en_la_cache(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn cantidad_de_lapidas(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT COUNT(*) FROM remotos_cerrados_aca", [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn cierra_varios_ingresos_remotos_con_un_solo_patch() {
    let connection = conexion_con_ingresos_remotos(3);
    let (base_url, pedidos) = servidor_que_anota(vec![String::new()]);
    let uuids: Vec<String> = (1..=3).map(uuid_de_prueba).collect();

    let resultado = cerrar_ingresos_remotos(
        &connection,
        &contexto(&base_url),
        &uuids,
        "Op Celular",
        chrono::Utc::now(),
    );

    assert_eq!(resultado.cerrados, uuids);
    assert!(resultado.invalidos.is_empty());
    assert!(resultado.fallidos.is_empty());
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(
        pedidos.len(),
        1,
        "un solo pedido para los tres: {pedidos:?}"
    );
    let esperado = format!(
        "PATCH /rest/v1/ingresos?id=in.({},{},{})&hora_salida=is.null",
        uuid_de_prueba(1),
        uuid_de_prueba(2),
        uuid_de_prueba(3),
    );
    assert!(pedidos[0].starts_with(&esperado), "{}", pedidos[0]);
    assert_eq!(en_la_cache(&connection), 0);
    assert_eq!(cantidad_de_lapidas(&connection), 3);
}

#[test]
fn el_cierre_remoto_en_lote_sella_la_hora_y_el_operador_que_le_pasan() {
    let connection = conexion_con_ingresos_remotos(2);
    let (base_url, pedido) = servidor_que_captura_el_cuerpo();
    let hora = crate::tiempo::parsear_utc("2026-01-01T08:00:30Z").unwrap();
    let uuids: Vec<String> = (1..=2).map(uuid_de_prueba).collect();

    cerrar_ingresos_remotos(&connection, &contexto(&base_url), &uuids, "Op PC", hora);

    let pedido = pedido
        .recv_timeout(Duration::from_secs(3))
        .expect("pedido capturado");
    let hora_esperada = format!(
        "\"hora_salida\":\"{}\"",
        crate::tiempo::serializar_utc(hora)
    );
    assert!(pedido.contains(&hora_esperada), "{pedido}");
    assert!(
        pedido.contains("\"usuario_salida_nombre\":\"Op PC\""),
        "{pedido}"
    );
    assert!(
        pedido.contains("\"dispositivo_salida_id\":\"dispositivo-1\""),
        "{pedido}"
    );
}

#[test]
fn un_texto_que_no_es_uuid_no_se_manda_a_la_nube() {
    let connection = conexion_con_ingresos_remotos(1);
    let (base_url, pedidos) = servidor_que_anota(vec![String::new()]);
    let malicioso = "x),id=neq.0&select=*".to_string();
    let uuids = vec![uuid_de_prueba(1), malicioso.clone(), "corto".to_string()];

    let resultado = cerrar_ingresos_remotos(
        &connection,
        &contexto(&base_url),
        &uuids,
        "Op",
        chrono::Utc::now(),
    );

    assert_eq!(resultado.cerrados, [uuid_de_prueba(1)]);
    assert_eq!(resultado.invalidos, [malicioso, "corto".to_string()]);
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 1);
    assert!(
        pedidos[0].starts_with(&format!(
            "PATCH /rest/v1/ingresos?id=in.({})&hora_salida=is.null",
            uuid_de_prueba(1)
        )),
        "{}",
        pedidos[0]
    );
}

#[test]
fn si_todos_son_invalidos_no_se_hace_ningun_pedido() {
    let connection = conexion_con_ingresos_remotos(0);
    // Sin servidor: cualquier pedido fallaría por falta de conexión y
    // aparecería en `fallidos`.
    let resultado = cerrar_ingresos_remotos(
        &connection,
        &contexto("http://127.0.0.1:1"),
        &["no-es-uuid".to_string()],
        "Op",
        chrono::Utc::now(),
    );

    assert_eq!(resultado.invalidos.len(), 1);
    assert!(resultado.cerrados.is_empty());
    assert!(resultado.fallidos.is_empty());
}

#[test]
fn los_repetidos_se_mandan_una_sola_vez() {
    let connection = conexion_con_ingresos_remotos(1);
    let (base_url, pedidos) = servidor_que_anota(vec![String::new()]);
    let uuid = uuid_de_prueba(1);

    let resultado = cerrar_ingresos_remotos(
        &connection,
        &contexto(&base_url),
        &[uuid.clone(), uuid.clone()],
        "Op",
        chrono::Utc::now(),
    );

    assert_eq!(resultado.cerrados, std::slice::from_ref(&uuid));
    assert!(
        pedidos.lock().unwrap()[0].contains(&format!("id=in.({uuid})&")),
        "el uuid aparece una vez"
    );
}

#[test]
fn si_el_patch_falla_ninguno_del_grupo_se_cierra_ni_sale_de_la_cache() {
    let connection = conexion_con_ingresos_remotos(2);
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 500 Internal Server Error\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
    );
    let uuids: Vec<String> = (1..=2).map(uuid_de_prueba).collect();

    let resultado = cerrar_ingresos_remotos(
        &connection,
        &contexto(&base_url),
        &uuids,
        "Op",
        chrono::Utc::now(),
    );

    assert!(resultado.cerrados.is_empty());
    assert_eq!(resultado.fallidos.len(), 1);
    assert_eq!(resultado.fallidos[0].uuids, uuids);
    assert!(resultado.fallidos[0].error.es_falta_de_conexion());
    assert_eq!(en_la_cache(&connection), 2, "siguen adentro en la caché");
    assert_eq!(cantidad_de_lapidas(&connection), 0, "y sin lápida");
}

#[test]
fn con_mas_de_cincuenta_se_parte_en_varios_pedidos() {
    let connection = conexion_con_ingresos_remotos(60);
    let (base_url, pedidos) = servidor_que_anota(vec![String::new(), String::new()]);
    let uuids: Vec<String> = (1..=60).map(uuid_de_prueba).collect();

    let resultado = cerrar_ingresos_remotos(
        &connection,
        &contexto(&base_url),
        &uuids,
        "Op",
        chrono::Utc::now(),
    );

    assert_eq!(resultado.cerrados, uuids, "todos, en su orden");
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 2);
    assert_eq!(pedidos[0].matches("0000-").count(), 50 * 4);
    assert_eq!(pedidos[1].matches("0000-").count(), 10 * 4);
    assert_eq!(en_la_cache(&connection), 0);
}

#[test]
fn si_falla_un_grupo_el_siguiente_se_intenta_igual() {
    let connection = conexion_con_ingresos_remotos(60);
    // El primer grupo (50) recibe un 500; el segundo (10), un 200.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
    let direccion = listener.local_addr().expect("dirección local");
    thread::spawn(move || {
        for respuesta in [
            "HTTP/1.1 500 Internal Server Error\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        ] {
            let Ok((mut socket, _)) = listener.accept() else {
                return;
            };
            socket
                .set_read_timeout(Some(Duration::from_millis(300)))
                .expect("set_read_timeout");
            let mut buffer = [0_u8; 8192];
            while let Ok(leidos) = socket.read(&mut buffer) {
                if leidos == 0 {
                    break;
                }
            }
            let _ = socket.write_all(respuesta.as_bytes());
        }
    });
    let base_url = format!("http://{direccion}");
    let uuids: Vec<String> = (1..=60).map(uuid_de_prueba).collect();

    let resultado = cerrar_ingresos_remotos(
        &connection,
        &contexto(&base_url),
        &uuids,
        "Op",
        chrono::Utc::now(),
    );

    assert_eq!(resultado.fallidos.len(), 1);
    assert_eq!(resultado.fallidos[0].uuids, uuids[..50]);
    assert_eq!(resultado.cerrados, uuids[50..]);
    assert_eq!(
        en_la_cache(&connection),
        50,
        "sólo salieron los 10 del grupo bueno"
    );
}

#[test]
fn calentar_abre_tres_conexiones_a_la_raiz_de_la_api() {
    let (base_url, pedidos) = servidor_que_anota(vec![String::new(); 3]);
    let ultimo = Mutex::new(None);

    let corrio = crate::nube::cliente::precalentar_si_corresponde(
        &ultimo,
        std::time::Instant::now(),
        &base_url,
        "clave-de-prueba",
    );

    assert!(corrio);
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 3, "una consulta por conexión: {pedidos:?}");
    assert!(
        pedidos
            .iter()
            .all(|pedido| pedido.starts_with("OPTIONS /rest/v1/ HTTP/1.1")),
        "{pedidos:?}"
    );
}

/// Una vuelta de `mantener_conexiones_vivas`: un `OPTIONS` por cada cliente
/// (sincronización y verificación), sin token.
#[test]
fn mantener_vivas_toca_los_dos_clientes_con_options() {
    let (base_url, pedidos) =
        servidor_que_anota_pedidos(vec![respuesta_json(""), respuesta_json("")]);

    crate::nube::cliente::tocar_conexiones(&base_url, "clave-de-prueba");

    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 2, "uno por cliente: {pedidos:#?}");
    for pedido in &pedidos {
        assert!(pedido.starts_with("OPTIONS /rest/v1/ HTTP/1.1"), "{pedido}");
        assert!(!pedido.to_lowercase().contains("authorization"), "{pedido}");
    }
}

#[test]
fn calentar_manda_la_clave_publica_y_ninguna_otra_credencial() {
    let (base_url, pedido) = servidor_que_captura_el_cuerpo();
    let ultimo = Mutex::new(None);

    crate::nube::cliente::precalentar_si_corresponde(
        &ultimo,
        std::time::Instant::now(),
        &base_url,
        "clave-de-prueba",
    );

    let pedido = pedido
        .recv_timeout(Duration::from_secs(3))
        .expect("pedido capturado")
        .to_ascii_lowercase();
    assert!(pedido.contains("apikey: clave-de-prueba"), "{pedido}");
    assert!(
        !pedido.contains("authorization"),
        "no hace falta token para calentar la conexión: {pedido}"
    );
}

#[test]
fn calentar_respeta_la_pausa_minima_entre_calentamientos() {
    let (base_url, pedidos) = servidor_que_anota(vec![String::new(); 6]);
    let ultimo = Mutex::new(None);
    let inicio = std::time::Instant::now();
    let calentar = |despues_de_segundos: u64| {
        crate::nube::cliente::precalentar_si_corresponde(
            &ultimo,
            inicio + Duration::from_secs(despues_de_segundos),
            &base_url,
            "clave-de-prueba",
        )
    };

    assert!(calentar(0), "la primera vez calienta");
    assert!(!calentar(5), "a los 5 s siguen calientes: no repite");
    assert!(!calentar(19), "tampoco a los 19 s");
    assert_eq!(
        pedidos.lock().unwrap().len(),
        3,
        "sólo salieron los de la primera"
    );
    assert!(calentar(25), "pasada la pausa vuelve a calentar");
    assert_eq!(pedidos.lock().unwrap().len(), 6);
}

#[test]
fn calentar_sin_nube_alcanzable_no_falla() {
    // Mejor esfuerzo: sin red, el calentamiento simplemente no logra nada.
    let ultimo = Mutex::new(None);

    let corrio = crate::nube::cliente::precalentar_si_corresponde(
        &ultimo,
        std::time::Instant::now(),
        "http://127.0.0.1:1",
        "clave-de-prueba",
    );

    assert!(corrio, "lo intentó, aunque no haya podido");
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
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![respuesta_json(
        r#"{"gafetes":[{"id":"gafete-remoto","numero":26,"tipo":"CONTRATISTA","estado":"PERDIDO","contratista_portador_id":"uuid-contratista","contratista_portador_nombre":null,"visita_portador_id":null,"updated_at":"2026-01-01T00:00:00Z"}]}"#,
    )]);
    let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();
    // Un solo llamado. `gafetes_actualizado_hasta` nunca se tocó en este test
    // (sólo `catalogo_actualizado_hasta`, la de arriba), así que su propio
    // cursor sigue en NULL y pide todo, sin importar qué tan adelantado esté
    // el otro.
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 1, "{pedidos:#?}");
    assert!(pedidos[0].starts_with("POST /rest/v1/rpc/catalogo_del_sitio "));
    assert!(
        pedidos[0].contains(r#""p_desde":"2099-01-01T00:00:00Z""#),
        "{}",
        pedidos[0]
    );
    assert!(
        pedidos[0].contains(r#""p_desde_gafetes":null"#),
        "el cursor de gafetes no debe heredar el de catálogo: {}",
        pedidos[0]
    );
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
    // Dos syncs = dos llamados. Sólo gafetes trae algo en el primero, para
    // que la marca de agua realmente tenga algo que guardar.
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![
        respuesta_json(
            r#"{"gafetes":[{"id":"gafete-remoto","numero":26,"tipo":"CONTRATISTA","estado":"DISPONIBLE","contratista_portador_id":null,"contratista_portador_nombre":null,"visita_portador_id":null,"updated_at":"2026-01-05T00:00:00Z"}]}"#,
        ),
        respuesta_json("{}"),
    ]);

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
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 2, "{pedidos:#?}");
    assert!(
        pedidos[0].contains(r#""p_desde_gafetes":null"#),
        "primer sync: sin marca todavía, tiene que pedir todo: {}",
        pedidos[0]
    );
    assert!(
        pedidos[1].contains(r#""p_desde_gafetes":"2026-01-05T00:00:00.000000Z""#),
        "segundo sync: tiene que arrastrar la marca que dejó el primero: {}",
        pedidos[1]
    );
}

#[test]
fn recibe_catalogo_del_sitio_y_lo_guarda_local() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"empresas":[{"id":"uuid-empresa-remota","nombre":"Empresa Remota","activa":true,
            "updated_at":"2026-01-01T00:00:00Z"}],
           "contratistas":[{"id":"uuid-contratista-remoto","nombre":"Persona Remota",
            "identificacion":"1-1111","empresa_id":"uuid-empresa-remota",
            "empresa_nombre":"Empresa Remota","activo":true,"tipo_ingreso":"SWAT",
            "fecha_vencimiento_praind":null,"es_personal_ruta":false,
            "updated_at":"2026-01-01T00:00:00Z"}],
           "usuarios":[],"gafetes":[],"rutas":[],"empresas_proveedor":[]}"#,
    )]);

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
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"rutas":[{"id":"uuid-ruta-remota","numero":79,"activo":true,
            "updated_at":"2026-01-01T00:00:00Z"}]}"#,
    )]);

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
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"empresas_proveedor":[{"id":"uuid-empresa-proveedor-remota","nombre":"MayCorp",
            "activa":true,"updated_at":"2026-01-01T00:00:00Z"}]}"#,
    )]);

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

/// Una fusión hecha en el panel ("FUDDY FACTORY" en "FOODIE FACTORY") llega
/// con el catálogo: la fusionada queda marcada y la portería ya no la ofrece.
#[test]
fn recibe_la_fusion_de_una_empresa_proveedor_y_deja_de_ofrecerla() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas_proveedor (nombre, uuid) VALUES ('FUDDY FACTORY', 'uuid-fuddy')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"empresas_proveedor":[
            {"id":"uuid-foodie","nombre":"FOODIE FACTORY","activa":true,"updated_at":"2026-10-31T00:00:00Z","fusionada_en":null},
            {"id":"uuid-fuddy","nombre":"FUDDY FACTORY","activa":false,"updated_at":"2026-10-31T00:00:01Z","fusionada_en":"uuid-foodie"}]}"#,
    )]);

    recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    let fusionada: Option<String> = connection
        .query_row(
            "SELECT fusionada_en FROM empresas_proveedor WHERE uuid = 'uuid-fuddy'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(fusionada.as_deref(), Some("uuid-foodie"));
    let repositorio = crate::database::repositories::empresa_proveedor_repository::SqliteEmpresaProveedorRepository::new(&connection);
    let ofrecidas =
        crate::services::empresa_proveedor_service::EmpresaProveedorService::new(&repositorio)
            .buscar_seleccionables("FUDDY")
            .unwrap();
    assert_eq!(
        ofrecidas
            .iter()
            .map(|e| e.nombre.as_str())
            .collect::<Vec<_>>(),
        vec!["FOODIE FACTORY"]
    );
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
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"empresas_proveedor":[{"id":"uuid-empresa-proveedor-remota","nombre":"MayCorp",
            "activa":true,"updated_at":"2026-01-01T00:00:00Z"}]}"#,
    )]);

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
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"empresas_proveedor":[{"id":"uuid-dos-pinos","nombre":"DOS PINOS","activa":true,
            "updated_at":"2026-01-01T00:00:00Z"}]}"#,
    )]);

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
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"empresas_proveedor":[{"id":"uuid-mayca-real","nombre":"MAYCA","activa":true,
            "updated_at":"2026-01-01T00:00:00Z"}]}"#,
    )]);

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

/// El catálogo de rutas ya sólo trae el personal KOF (`encargados_ruta`): los
/// vehículos de ruta no se piden (función retirada, su tabla se va a borrar).
#[test]
fn recibe_catalogo_rutas_del_sitio_y_lo_guarda_local() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![respuesta_json(
        r#"[{"id":"uuid-encargado-remoto","codigo_empleado":"5040017",
             "nombre":"Michael Araya Retana","activo":true,
             "updated_at":"2026-01-01T00:00:00Z"}]"#,
    )]);

    let resumen = recibir_catalogo_rutas_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(
        resumen,
        ResumenCatalogoRutas {
            vehiculos_recibidos: 0,
            encargados_recibidos: 1,
        }
    );
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 1, "{pedidos:#?}");
    assert!(
        pedidos[0].starts_with("GET /rest/v1/encargados_ruta?"),
        "{}",
        pedidos[0]
    );
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
fn segundo_sync_de_catalogo_rutas_pide_solo_lo_nuevo_con_la_marca_guardada() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![
        respuesta_json(
            r#"[{"id":"uuid-encargado-remoto","codigo_empleado":"5040017","nombre":"Michael Araya Retana","activo":true,"updated_at":"2026-01-05T00:00:00Z"}]"#,
        ),
        respuesta_json("[]"),
    ]);

    recibir_catalogo_rutas_del_sitio(&connection, &contexto(&base_url)).unwrap();
    recibir_catalogo_rutas_del_sitio(&connection, &contexto(&base_url)).unwrap();

    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 2, "{pedidos:#?}");
    assert!(
        !pedidos[0].contains("updated_at=gt."),
        "primer sync: sin marca todavía, tiene que pedir todo"
    );
    assert!(
        pedidos[1].contains("updated_at=gt.2026-01-05T00%3A00%3A00")
            || pedidos[1].contains("updated_at=gt.2026-01-05T00:00:00"),
        "segundo sync: tiene que arrastrar la marca que dejó el primero -- pedido real: {}",
        pedidos[1]
    );
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
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"empresas":[{"id":"uuid-empresa-remota","nombre":"Empresa Remota","activa":true,
            "updated_at":"2026-01-01T00:00:00Z"}],
           "contratistas":[{"id":"uuid-contratista-remoto","nombre":"Persona Remota",
            "identificacion":"1-1111","empresa_id":"uuid-empresa-remota",
            "empresa_nombre":"Empresa Remota","activo":true,"tipo_ingreso":"SWAT",
            "fecha_vencimiento_praind":null,"es_personal_ruta":false,
            "updated_at":"2026-01-01T00:00:00Z"}]}"#,
    )]);

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
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"empresas":[{"id":"uuid-mi-empresa","nombre":"MI EMPRESA","activa":true,
            "updated_at":"2026-01-01T00:00:00Z"}]}"#,
    )]);

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
    let base_url = servidor_de_respuestas(vec![respuesta_json(
        r#"{"contratistas":[{"id":"uuid-incompleto","nombre":"Persona Incompleta",
            "identificacion":null,"empresa_id":null,"empresa_nombre":null,
            "activo":true,"tipo_ingreso":null,"fecha_vencimiento_praind":null,
            "es_personal_ruta":null,"updated_at":"2026-01-01T00:00:00Z"}]}"#,
    )]);

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

/// `estado_del_sitio` pide sólo los grupos del alcance, con los ids que este
/// equipo tiene abiertos, y la etapa aplica el cierre que llegó sin pedir
/// nada por GET.
#[test]
fn estado_del_sitio_pide_los_ids_abiertos_y_la_etapa_aplica_el_cierre() {
    let connection = conexion_con_un_ingreso_proveedor_activo();
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![respuesta_json(
        r#"{"cierres_proveedor":[{"id":"uuid-ingreso-proveedor","hora_salida":"2026-08-01T10:00:00+00:00","usuario_salida_nombre":"OPERADOR A2"}],
            "proveedor_abiertos":[]}"#,
    )]);
    let contexto = contexto(&base_url);
    let alcance = crate::nube::alcance::AlcanceSincronizacion {
        ingresos_proveedor: true,
        ..crate::nube::alcance::AlcanceSincronizacion::solo_envio()
    };

    let mut estado = pedir_estado_del_sitio(&connection, &contexto, &alcance)
        .unwrap()
        .expect("la nube tiene la función");
    let aplicados = recibir_cierres_de_ingresos_propios_proveedor_con(
        &connection,
        &contexto,
        Some(std::mem::take(&mut estado.cierres_proveedor)),
    )
    .unwrap();

    assert_eq!(aplicados, 1);
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 1, "ningún GET aparte: {pedidos:#?}");
    assert!(pedidos[0].starts_with("POST /rest/v1/rpc/estado_del_sitio "));
    assert!(
        pedidos[0].contains(r#""p_grupos":["proveedor"]"#),
        "{}",
        pedidos[0]
    );
    assert!(
        pedidos[0].contains(r#""p_proveedor":["uuid-ingreso-proveedor"]"#),
        "{}",
        pedidos[0]
    );
    assert!(!pedidos[0].contains("p_ingresos"), "{}", pedidos[0]);
}

/// Sin `estado_del_sitio` en la nube (404) no hay estado: cada etapa pide lo
/// suyo por GET, como antes.
#[test]
fn sin_estado_del_sitio_en_la_nube_cada_etapa_pide_lo_suyo() {
    let connection = conexion_con_un_ingreso_proveedor_activo();
    let base_url = servidor_de_respuestas(vec![
        "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nConnection: close\r\n\
         Content-Length: 19\r\n\r\n{\"code\":\"PGRST202\"}",
    ]);
    let alcance = crate::nube::alcance::AlcanceSincronizacion::completo();

    let estado = pedir_estado_del_sitio(&connection, &contexto(&base_url), &alcance).unwrap();

    assert!(estado.is_none());
}

/// Una app nueva contra una nube sin `catalogo_del_sitio` (404): el
/// catálogo se pide tabla por tabla, como antes, y se guarda igual.
#[test]
fn sin_catalogo_del_sitio_en_la_nube_pide_tabla_por_tabla() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let vacia = respuesta_json("[]");
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![
        "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nConnection: close\r\n\
         Content-Length: 19\r\n\r\n{\"code\":\"PGRST202\"}",
        respuesta_json(
            r#"[{"id":"uuid-empresa-remota","nombre":"Empresa Remota","activa":true,"updated_at":"2026-01-01T00:00:00Z"}]"#,
        ),
        vacia,
        vacia,
        vacia,
        vacia,
        vacia,
    ]);

    let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(resumen.empresas_recibidas, 1);
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(pedidos.len(), 7, "{pedidos:#?}");
    assert!(pedidos[0].starts_with("POST /rest/v1/rpc/catalogo_del_sitio "));
    assert!(
        pedidos[1].starts_with("GET /rest/v1/empresas?"),
        "{}",
        pedidos[1]
    );
    assert!(
        pedidos[6].starts_with("GET /rest/v1/empresas_proveedor?"),
        "{}",
        pedidos[6]
    );
}

/// Una tabla que llena la primera página (primer sync de un equipo nuevo con
/// un catálogo grande) sigue con los GET desde la segunda página: no se
/// pierde nada de lo que no entró en el llamado único.
#[test]
fn catalogo_con_la_primera_pagina_llena_sigue_por_get_desde_la_segunda() {
    use std::fmt::Write as _;

    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let mut empresas = String::new();
    for i in 0..TAMANO_PAGINA_REMOTA {
        if i > 0 {
            empresas.push(',');
        }
        write!(
            empresas,
            "{{\"id\":\"uuid-empresa-{i:04}\",\"nombre\":\"Empresa {i:04}\",\"activa\":true,\
             \"updated_at\":\"2026-01-01T00:00:00Z\"}}"
        )
        .unwrap();
    }
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![
        respuesta_json(&format!("{{\"empresas\":[{empresas}]}}")),
        respuesta_json(
            r#"[{"id":"uuid-empresa-ultima","nombre":"Empresa ultima","activa":true,"updated_at":"2026-01-02T00:00:00Z"}]"#,
        ),
    ]);

    let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(
        resumen.empresas_recibidas,
        u32::try_from(TAMANO_PAGINA_REMOTA).unwrap() + 1
    );
    let pedidos = pedidos.lock().unwrap().clone();
    assert_eq!(
        pedidos.len(),
        2,
        "sólo empresas sigue por GET: {pedidos:#?}"
    );
    let segunda = pedidos[1].to_lowercase();
    assert!(segunda.starts_with("get /rest/v1/empresas?"), "{segunda}");
    assert!(segunda.contains("order=id.asc"), "{segunda}");
    assert!(
        segunda.contains(&format!("range: {TAMANO_PAGINA_REMOTA}-")),
        "pide desde la segunda página: {segunda}"
    );
    let marca: Option<String> = connection
        .query_row(
            "SELECT catalogo_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(marca.as_deref(), Some("2026-01-02T00:00:00.000000Z"));
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
            "{{\"id\":\"uuid-encargado-{i}\",\"codigo_empleado\":\"E{i:05}\",\
             \"nombre\":\"Encargado {i}\",\"activo\":true,\"updated_at\":\"2026-01-01T00:00:00Z\"}}"
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
        .query_row("SELECT COUNT(*) FROM encargados_ruta", [], |row| row.get(0))
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

// ---- Ingreso por correo ----

fn conexion_con_ingreso_correo(fecha_hora_salida: Option<&str>) -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
             VALUES ('1', 'Guardia', 'h', 'OPERADOR', 1)",
            [],
        )
        .unwrap();
    let usuario_salida_id = fecha_hora_salida.is_some().then_some(1_i64);
    let usuario_salida_nombre = fecha_hora_salida.is_some().then_some("Guardia");
    connection
        .execute(
            "INSERT INTO registro_ingresos_correo (
                uuid, cedula, nombre, motivo, placa, gafete_numero,
                fecha_hora_ingreso, usuario_ingreso_id, usuario_ingreso_nombre,
                fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
            ) VALUES (
                'uuid-correo', '112345678', 'Ana Solano', 'Entrevista RH', NULL, 5,
                '2026-01-01T08:00:00Z', 1, 'Guardia', ?1, ?2, ?3
            )",
            params![fecha_hora_salida, usuario_salida_id, usuario_salida_nombre],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (entidad, entidad_uuid, operacion, creado_en, actualizado_en)
             VALUES ('ingreso_correo', 'uuid-correo', ?1,
                     '2026-01-01T08:00:00Z', '2026-01-01T08:00:00Z')",
            params![if fecha_hora_salida.is_some() {
                "cerrar"
            } else {
                "crear"
            }],
        )
        .unwrap();
    connection
}

/// Servidor de una sola petición que exige que empiece con `inicio` y traiga
/// `fragmento` (en el cuerpo), y responde `respuesta` tal cual.
fn servidor_que_exige(
    inicio: &'static str,
    fragmento: &'static str,
    respuesta: &'static str,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let servidor = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut pedido = Vec::new();
        let mut buffer = [0; 4096];
        while !String::from_utf8_lossy(&pedido).contains(fragmento) {
            let leidos = socket.read(&mut buffer).unwrap();
            assert!(leidos > 0, "el pedido no trajo {fragmento}");
            pedido.extend_from_slice(&buffer[..leidos]);
        }
        let pedido = String::from_utf8(pedido).unwrap();
        assert!(pedido.starts_with(inicio), "{pedido}");
        socket.write_all(respuesta.as_bytes()).unwrap();
    });
    (base_url, servidor)
}

#[test]
fn envia_la_apertura_de_un_ingreso_por_correo_con_su_motivo() {
    let connection = conexion_con_ingreso_correo(None);
    let (base_url, servidor) = servidor_que_exige(
        "POST /rest/v1/ingresos_correo ",
        "\"motivo\":\"Entrevista RH\"",
        "HTTP/1.1 201 Created\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.enviados, 1);
    assert_eq!(resumen.fallidos, 0);
    servidor.join().unwrap();
}

#[test]
fn envia_el_cierre_de_un_ingreso_por_correo() {
    let connection = conexion_con_ingreso_correo(Some("2026-01-01T09:00:00Z"));
    let (base_url, servidor) = servidor_que_exige(
        "PATCH /rest/v1/ingresos_correo?id=eq.uuid-correo&hora_salida=is.null ",
        "\"hora_salida\":\"2026-01-01T09:00:00Z\"",
        "HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
    );

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.enviados, 1);
    servidor.join().unwrap();
}

#[test]
fn ingreso_por_correo_con_gafete_ya_activo_queda_fallido_con_aviso() {
    let connection = conexion_con_ingreso_correo(None);
    let base_url = servidor_de_una_respuesta(respuesta_gafete_en_uso(
        "ingresos_correo_gafete_activo_sitio_idx",
    ));

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.fallidos, 1);
    assert_eq!(
        resumen.conflictos_gafete,
        vec![ConflictoGafeteActivo {
            tipo: TipoMovimientoGafete::PorCorreo,
            nombre: "Ana Solano".to_string(),
            gafete_numero: 5,
            fecha_hora: "2026-01-01T08:00:00Z".to_string(),
        }]
    );
    assert_eq!(estado_en_cola(&connection, "uuid-correo"), "fallido");
}

#[test]
fn la_cola_reconoce_el_rechazo_por_persona_ya_adentro_por_correo() {
    let fila = super::cola::FilaCola {
        id: 1,
        entidad: "ingreso_correo".to_string(),
        entidad_uuid: "uuid".to_string(),
        operacion: "crear".to_string(),
        intentos: 0,
    };
    let rechazo = |indice: &str| SincronizacionError::RespuestaInesperada {
        status: 409,
        cuerpo: format!("duplicate key value violates unique constraint \"{indice}\""),
    };
    assert!(super::cola::es_conflicto_ingreso_activo(
        &fila,
        &rechazo("ingresos_correo_cedula_activa_idx")
    ));
    assert!(!super::cola::es_conflicto_ingreso_activo(
        &fila,
        &rechazo("ingresos_proveedor_cedula_activa_idx")
    ));
}

#[test]
fn correo_con_ingreso_activo_pregunta_a_la_funcion_de_la_nube() {
    let (base_url, servidor) = servidor_rpc(
        "ingreso_correo_activo",
        "\"p_cedula\":\"112345678\"",
        "[{\"sitio_id\":\"otro\",\"sitio_nombre\":\"Cartago\"}]",
    );

    let activo = correo_con_ingreso_activo(&contexto(&base_url), "112345678").unwrap();

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
fn recibe_ingresos_por_correo_abiertos_del_otro_dispositivo_y_los_cachea() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-remoto\",\"cedula\":\"112345678\",\"nombre\":\"Ana Solano\",\
         \"motivo\":\"Entrevista RH\",\"placa\":null,\"gafete_numero\":5,\
         \"hora_entrada\":\"2026-01-01T08:00:00Z\",\"usuario_entrada_nombre\":\"Op PC\",\
         \"dispositivo_entrada_id\":\"otro-dispositivo\"}]",
    );

    let recibidos = recibir_ingresos_correo_abiertos(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidos.len(), 1);
    assert_eq!(recibidos[0].motivo, "Entrevista RH");
    let cacheados: i64 = connection
        .query_row("SELECT COUNT(*) FROM ingresos_correo_remotos", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(cacheados, 1);
}

#[test]
fn un_ingreso_por_correo_propio_cerrado_por_otro_dispositivo_se_cierra_aca() {
    let connection = conexion_con_ingreso_correo(None);
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-correo\",\"hora_salida\":\"2026-01-01T10:00:00Z\",\
         \"usuario_salida_nombre\":\"Guardia Celular\"}]",
    );

    let aplicados =
        recibir_cierres_de_ingresos_propios_correo(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(aplicados, 1);
    let (salida, nombre): (String, String) = connection
        .query_row(
            "SELECT fecha_hora_salida, usuario_salida_nombre FROM registro_ingresos_correo
             WHERE uuid = 'uuid-correo'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(salida, "2026-01-01T10:00:00Z");
    assert_eq!(nombre, "Guardia Celular");
}

#[test]
fn correos_con_conflicto_activo_avisan_la_otra_unidad() {
    let connection = conexion_con_ingreso_correo(None);
    let (base_url, servidor) = servidor_rpc(
        "correos_activos_en_otras_unidades",
        "\"p_cedulas\":[\"112345678\"]",
        "[{\"cedula\":\"112345678\",\"sitio_nombre\":\"Cartago\"}]",
    );

    let conflictos = correos_con_conflicto_activo(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(
        conflictos,
        vec![ConflictoIngresoProveedorActivo {
            cedula: "112345678".to_string(),
            nombre: "Ana Solano".to_string(),
            sitio_conflicto: "Cartago".to_string(),
        }]
    );
    servidor.join().unwrap();
}

#[test]
fn recibe_el_historial_de_ingresos_por_correo_y_omite_las_filas_ilegibles() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"correo-malo\",\"cedula\":\"111111111\",\"nombre\":\"X\",\
         \"motivo\":null,\"placa\":null,\"gafete_numero\":null,\
         \"hora_entrada\":\"no-es-una-fecha\",\"hora_salida\":null,\
         \"usuario_entrada_nombre\":null,\"usuario_salida_nombre\":null,\
         \"dispositivo_entrada_id\":\"otro-dispositivo\",\"dispositivo_salida_id\":null,\
         \"updated_at\":\"2026-01-01T08:00:00Z\"},\
         {\"id\":\"correo-1\",\"cedula\":\"112345678\",\"nombre\":\"Ana Solano\",\
         \"motivo\":\"Entrevista RH\",\"placa\":null,\"gafete_numero\":5,\
         \"hora_entrada\":\"2026-01-01T08:00:00Z\",\"hora_salida\":\"2026-01-01T09:00:00Z\",\
         \"usuario_entrada_nombre\":\"Guardia\",\"usuario_salida_nombre\":\"Guardia\",\
         \"dispositivo_entrada_id\":\"otro-dispositivo\",\"dispositivo_salida_id\":\"otro-dispositivo\",\
         \"updated_at\":\"2026-01-01T09:00:05Z\"}]",
    );

    let recibidos =
        recibir_historial_ingresos_correo_del_sitio(&connection, &contexto(&base_url), true)
            .unwrap();

    assert_eq!(recibidos, 1);
    let (motivo, salida): (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT motivo, hora_salida FROM historial_ingresos_correo_sitio WHERE uuid = 'correo-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(motivo.as_deref(), Some("Entrevista RH"));
    assert_eq!(salida.as_deref(), Some("2026-01-01T09:00:00Z"));
    let marca: Option<String> = connection
        .query_row(
            "SELECT historial_ingresos_correo_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(marca.is_some());
}

/// Ingreso por correo ya cerrado localmente con sus dos filas en la cola:
/// el `crear` (id 1, con el estado/intentos/`actualizado_en` que se pidan) y
/// el `cerrar` (id 2, recién encolado, sin intentos).
fn conexion_con_apertura_y_cierre_en_cola(
    estado_crear: &str,
    intentos_crear: i64,
    actualizado_crear: &str,
) -> Connection {
    let connection = conexion_con_ingreso_correo(Some("2026-01-01T09:00:00Z"));
    connection.execute("DELETE FROM cola_salida", []).unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (id, entidad, entidad_uuid, operacion, estado, intentos,
                 creado_en, actualizado_en)
             VALUES (1, 'ingreso_correo', 'uuid-correo', 'crear', ?1, ?2,
                 '2026-01-01T08:00:00Z', ?3)",
            params![estado_crear, intentos_crear, actualizado_crear],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO cola_salida (id, entidad, entidad_uuid, operacion, creado_en, actualizado_en)
             VALUES (2, 'ingreso_correo', 'uuid-correo', 'cerrar',
                 '2026-01-01T09:00:00Z', '2026-01-01T09:00:00Z')",
            [],
        )
        .unwrap();
    connection
}

fn estado_e_intentos(connection: &Connection, id: i64) -> (String, i64) {
    connection
        .query_row(
            "SELECT estado, intentos FROM cola_salida WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
}

/// Carrera de la revisión del 2026-10-04: el `crear` falló una vez y espera
/// su backoff (15 min desde "ahora"), el `cerrar` ya está listo. Antes el
/// cierre salía solo, el `PATCH` no tocaba nada (204) y el `crear` posterior
/// dejaba el ingreso abierto para siempre en la nube. Ahora el cierre ni
/// siquiera se envía: queda pendiente, sin sumar intentos.
#[test]
fn el_cierre_no_sale_mientras_su_apertura_espera_el_reintento() {
    let connection = conexion_con_apertura_y_cierre_en_cola(
        "pendiente",
        1,
        &crate::tiempo::serializar_utc(chrono::Utc::now()),
    );
    // Sin respuestas: cualquier pedido a la red fallaría y sumaría un intento.
    let (base_url, pedidos) = servidor_que_anota(Vec::new());

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen, ResumenDrenado::default());
    assert!(pedidos.lock().unwrap().is_empty(), "no se mandó nada");
    assert_eq!(
        estado_e_intentos(&connection, 1),
        ("pendiente".to_string(), 1)
    );
    assert_eq!(
        estado_e_intentos(&connection, 2),
        ("pendiente".to_string(), 0),
        "el cierre espera intacto, sin gastar reintentos"
    );
}

/// Si la apertura y el cierre están listos a la vez, salen en la misma
/// pasada y en orden: primero el `POST` y después el `PATCH`.
#[test]
fn apertura_y_cierre_listos_salen_en_la_misma_pasada_y_en_orden() {
    let connection = conexion_con_apertura_y_cierre_en_cola("pendiente", 0, "2026-01-01T08:00:00Z");
    let (base_url, pedidos) = servidor_que_anota(vec![String::new(), String::new()]);

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.enviados, 2);
    let pedidos = pedidos.lock().unwrap().clone();
    assert!(
        pedidos[0].starts_with("POST /rest/v1/ingresos_correo "),
        "{pedidos:?}"
    );
    assert!(
        pedidos[1].starts_with("PATCH /rest/v1/ingresos_correo?id=eq.uuid-correo"),
        "{pedidos:?}"
    );
    assert_eq!(estado_e_intentos(&connection, 2).0, "enviado");
}

/// Una apertura que la nube rechazó (`fallido`, por ejemplo por el índice
/// único de cédula activa) no frena su cierre: el `PATCH` no toca nada y la
/// fila sale de la cola en vez de quedar pendiente para siempre.
#[test]
fn el_cierre_sale_si_su_apertura_quedo_fallida() {
    let connection = conexion_con_apertura_y_cierre_en_cola("fallido", 1, "2026-01-01T08:00:00Z");
    let (base_url, pedidos) = servidor_que_anota(vec![String::new()]);

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.enviados, 1);
    assert!(pedidos.lock().unwrap()[0].starts_with("PATCH "));
    assert_eq!(estado_e_intentos(&connection, 2).0, "enviado");
}

#[test]
fn solo_un_cierre_puede_esperar_a_su_apertura() {
    let connection = conexion_con_apertura_y_cierre_en_cola("pendiente", 0, "2026-01-01T08:00:00Z");
    let fila = |id, operacion: &str| super::cola::FilaCola {
        id,
        entidad: "ingreso_correo".to_string(),
        entidad_uuid: "uuid-correo".to_string(),
        operacion: operacion.to_string(),
        intentos: 0,
    };
    assert!(super::cola::cierre_espera_su_apertura(&connection, &fila(2, "cerrar")).unwrap());
    assert!(!super::cola::cierre_espera_su_apertura(&connection, &fila(1, "crear")).unwrap());
    // Otro registro (otro uuid) no se frena por esta apertura.
    let mut ajena = fila(2, "cerrar");
    ajena.entidad_uuid = "otro-uuid".to_string();
    assert!(!super::cola::cierre_espera_su_apertura(&connection, &ajena).unwrap());
}

/// Visitas (`20261004130000_visitas_ven_otras_unidades`): el 409 del índice
/// único de cédula activa deja la apertura fallida de inmediato, como en
/// contratistas; el cierre nunca choca con ese índice.
#[test]
fn la_cola_reconoce_el_rechazo_por_visitante_ya_adentro() {
    let fila = |operacion: &str| super::cola::FilaCola {
        id: 1,
        entidad: "movimiento_visita".to_string(),
        entidad_uuid: "uuid".to_string(),
        operacion: operacion.to_string(),
        intentos: 0,
    };
    let rechazo = SincronizacionError::RespuestaInesperada {
        status: 409,
        cuerpo: "duplicate key value violates unique constraint \"movimientos_visita_cedula_activa_idx\""
            .to_string(),
    };
    assert!(super::cola::es_conflicto_ingreso_activo(
        &fila("crear"),
        &rechazo
    ));
    assert!(!super::cola::es_conflicto_ingreso_activo(
        &fila("cerrar"),
        &rechazo
    ));
}

/// `20261004140000_gafete_de_visita_unico_en_visitas`: una visita cuyo
/// gafete ya tiene abierto el otro equipo de la unidad (otra visita, o un
/// ingreso por correo vía el trigger de gafete compartido) queda fallida de
/// inmediato y avisa como choque de gafete, con tipo `Visita`.
#[test]
fn visita_con_gafete_ya_activo_queda_fallida_con_aviso() {
    let connection = conexion_con_dos_movimientos_visita_activos();
    connection
        .execute_batch(
            "DELETE FROM cola_salida;
             INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre)
                 VALUES (3, 'uuid-v3', 1, '5-5555', 'Visitante Tres');
             INSERT INTO movimientos_visita (
                 id, uuid, cita_visitante_id, gafete_numero, fecha_hora_entrada,
                 usuario_entrada_id, usuario_entrada_nombre,
                 visitante_cedula, visitante_nombre, anfitrion_nombre
             ) VALUES (3, 'uuid-m3', 3, 7, '2026-08-01T09:00:00Z', 1, 'Operador',
                 '5-5555', 'Visitante Tres', 'Ana');
             INSERT INTO cola_salida (entidad, entidad_uuid, operacion, creado_en, actualizado_en)
             VALUES ('movimiento_visita', 'uuid-m3', 'crear',
                     '2026-08-01T09:00:00Z', '2026-08-01T09:00:00Z');",
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(respuesta_gafete_en_uso(
        "movimientos_visita_gafete_activo_sitio_idx",
    ));

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.fallidos, 1);
    assert_eq!(
        resumen.conflictos_gafete,
        vec![ConflictoGafeteActivo {
            tipo: TipoMovimientoGafete::Visita,
            nombre: "Visitante Tres".to_string(),
            gafete_numero: 7,
            fecha_hora: "2026-08-01T09:00:00Z".to_string(),
        }]
    );
    assert_eq!(estado_en_cola(&connection, "uuid-m3"), "fallido");
}

/// Una persona no puede estar adentro por dos vías: la verificación en vivo
/// pregunta a `persona_adentro_por_otra_via` con la vía propia.
#[test]
fn persona_adentro_por_otra_via_pregunta_a_la_funcion_de_la_nube() {
    let (base_url, servidor) = servidor_rpc(
        "persona_adentro_por_otra_via",
        "\"p_via\":\"POR_CORREO\"",
        "[{\"via\":\"CONTRATISTA\",\"sitio_id\":\"sitio-1\",\"sitio_nombre\":\"Brisas\"}]",
    );

    let adentro = persona_adentro_por_otra_via(
        &contexto(&base_url),
        "112345678",
        crate::models::via_ingreso::ViaIngreso::PorCorreo,
    )
    .unwrap();

    assert_eq!(
        adentro,
        Some(AdentroPorOtraViaEnLaNube {
            via: crate::models::via_ingreso::ViaIngreso::Contratista,
            sitio_nombre: "Brisas".to_string(),
            mismo_sitio: true,
        })
    );
    servidor.join().unwrap();
}

const RESPUESTA_CORREO_REMOTO_ABIERTO: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
     [{\"id\":\"uuid-remoto\",\"cedula\":\"112345678\",\"nombre\":\"Ana Solano\",\
     \"motivo\":\"Entrevista RH\",\"placa\":null,\"gafete_numero\":5,\
     \"hora_entrada\":\"2026-01-01T08:00:00Z\",\"usuario_entrada_nombre\":\"Op PC\",\
     \"dispositivo_entrada_id\":\"otro-dispositivo\"}]";

fn lapidas(connection: &Connection) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT uuid FROM remotos_cerrados_aca ORDER BY uuid")
        .unwrap();
    statement
        .query_map([], |fila| fila.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Carrera de la revisión del 2026-10-04: quien opera cierra a mano un
/// ingreso del otro equipo mientras una sincronización ya leyó la nube con
/// ese ingreso todavía abierto. Antes, el reemplazo de la caché lo volvía a
/// insertar y la persona reaparecía "adentro". Ahora el cierre deja una
/// lápida y la recepción no lo vuelve a meter.
#[test]
fn un_ingreso_remoto_cerrado_aca_no_revive_con_una_lectura_vieja() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    // 1. Primera sincronización: el ingreso del otro equipo entra a la caché.
    let base_url = servidor_de_una_respuesta(RESPUESTA_CORREO_REMOTO_ABIERTO);
    recibir_ingresos_correo_abiertos(&connection, &contexto(&base_url)).unwrap();
    // 2. Quien opera lo cierra a mano (la nube responde 204).
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
    );
    cerrar_ingreso_correo_remoto(
        &connection,
        &contexto(&base_url),
        "uuid-remoto",
        "Guardia",
        chrono::Utc::now(),
    )
    .unwrap();
    assert_eq!(lapidas(&connection), vec!["uuid-remoto".to_string()]);

    // 3. Una sincronización que leyó la nube ANTES del cierre trae el
    //    ingreso todavía abierto: no vuelve a la caché.
    let base_url = servidor_de_una_respuesta(RESPUESTA_CORREO_REMOTO_ABIERTO);
    let recibidos = recibir_ingresos_correo_abiertos(&connection, &contexto(&base_url)).unwrap();
    assert!(recibidos.is_empty(), "no debe revivir: {recibidos:?}");
    let en_cache: i64 = connection
        .query_row("SELECT COUNT(*) FROM ingresos_correo_remotos", [], |fila| {
            fila.get(0)
        })
        .unwrap();
    assert_eq!(en_cache, 0);
    assert_eq!(lapidas(&connection), vec!["uuid-remoto".to_string()]);

    // 4. Cuando la nube ya no lo devuelve abierto, la lápida se olvida.
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
    );
    recibir_ingresos_correo_abiertos(&connection, &contexto(&base_url)).unwrap();
    assert!(lapidas(&connection).is_empty());
}

/// Las lápidas se olvidan al día aunque la nube siga devolviendo el
/// registro abierto (por ejemplo, si el cierre nunca llegó a aplicarse).
#[test]
fn una_lapida_de_mas_de_un_dia_se_olvida() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO remotos_cerrados_aca (uuid, tabla, cerrado_en)
             VALUES ('uuid-remoto', 'ingresos_correo_remotos', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    let base_url = servidor_de_una_respuesta(RESPUESTA_CORREO_REMOTO_ABIERTO);

    let recibidos = recibir_ingresos_correo_abiertos(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidos.len(), 1);
    assert!(lapidas(&connection).is_empty());
}

/// Medio de ingreso de las visitas agendadas
/// (`20261005140000_movimientos_visita_placa`): la placa sube con el
/// movimiento y vuelve en el historial de la unidad; sin placa = caminando.
#[test]
fn la_placa_de_la_visita_sube_con_el_movimiento() {
    let connection = conexion_con_dos_movimientos_visita_activos();
    connection
        .execute(
            "UPDATE movimientos_visita SET placa = 'BCD123' WHERE uuid = 'uuid-m1'",
            [],
        )
        .unwrap();
    let (base_url, pedido) = servidor_que_captura_el_cuerpo();

    super::cola::enviar_movimiento_visita(
        &crate::nube::cliente::cliente_http(),
        &connection,
        &contexto(&base_url),
        "uuid-m1",
    )
    .unwrap();

    let pedido = pedido
        .recv_timeout(Duration::from_secs(3))
        .expect("pedido capturado");
    assert!(pedido.contains("\"placa\":\"BCD123\""), "{pedido}");
}

#[test]
fn el_historial_de_visitas_guarda_la_placa_y_tolera_que_no_venga() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let fila = |id: &str, placa: &str| {
        serde_json::from_str::<super::historial::FilaHistorialVisitaRemota>(&format!(
            r#"{{"id":"{id}","visitante_cedula":"1-2345","visitante_nombre":"Visitante",
                "empresa":null,"anfitrion_nombre":"Ana","motivo":null,"gafete_numero":null,
                {placa}"hora_entrada":"2026-08-01T08:00:00Z","hora_salida":null,
                "usuario_entrada_nombre":"Operador","usuario_salida_nombre":null,
                "dispositivo_entrada_id":"disp","dispositivo_salida_id":null,
                "updated_at":"2026-08-01T08:00:00Z"}}"#
        ))
        .unwrap()
    };
    let transaction = connection.unchecked_transaction().unwrap();
    for remota in [fila("m-1", r#""placa":"BCD123","#), fila("m-2", "")] {
        super::historial::guardar_fila_historial_visita(
            &transaction,
            &contexto("http://sin-red"),
            &remota,
            "2026-08-01T08:00:00Z",
        )
        .unwrap();
    }
    transaction.commit().unwrap();

    let placas: Vec<(String, Option<String>)> = connection
        .prepare("SELECT uuid, placa FROM historial_visitas_sitio ORDER BY uuid")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        placas,
        vec![
            ("m-1".to_string(), Some("BCD123".to_string())),
            ("m-2".to_string(), None)
        ]
    );
}

/// Visita abierta por el otro equipo (la PC) y otra que es de este mismo
/// equipo (`uuid-m1`): sólo la primera entra a la caché.
const RESPUESTA_VISITAS_ABIERTAS: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
     [{\"id\":\"uuid-pc\",\"visitante_cedula\":\"112345678\",\"visitante_nombre\":\"Ana Solano\",\
     \"empresa\":null,\"anfitrion_nombre\":\"Laura\",\"motivo\":\"Auditoría\",\
     \"gafete_numero\":7,\"placa\":\"BCD123\",\"hora_entrada\":\"2026-08-01T09:00:00+00:00\",\
     \"usuario_entrada_nombre\":\"Op PC\",\"dispositivo_entrada_id\":\"pc\"},\
     {\"id\":\"uuid-m1\",\"visitante_cedula\":\"1-2345\",\"visitante_nombre\":\"Visitante Uno\",\
     \"empresa\":null,\"anfitrion_nombre\":\"Ana\",\"motivo\":null,\"gafete_numero\":null,\
     \"placa\":null,\"hora_entrada\":\"2026-08-01T08:00:00Z\",\
     \"usuario_entrada_nombre\":\"Operador\",\"dispositivo_entrada_id\":\"este\"}]";

/// Pedido del dueño 2026-10-06: el teléfono muestra las visitas que están
/// adentro por la PC, como con contratistas.
#[test]
fn recibe_las_visitas_abiertas_del_otro_equipo_y_las_cachea() {
    let connection = conexion_con_dos_movimientos_visita_activos();
    let base_url = servidor_de_una_respuesta(RESPUESTA_VISITAS_ABIERTAS);

    let recibidas = recibir_movimientos_visita_abiertos(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(recibidas.len(), 1, "la propia no se duplica: {recibidas:?}");
    assert_eq!(recibidas[0].uuid, "uuid-pc");
    assert_eq!(recibidas[0].gafete_numero, Some(7));
    assert_eq!(recibidas[0].placa.as_deref(), Some("BCD123"));
    assert_eq!(recibidas[0].hora_entrada, "2026-08-01T09:00:00Z");
    let cacheadas: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM movimientos_visita_remotos",
            [],
            |fila| fila.get(0),
        )
        .unwrap();
    assert_eq!(cacheadas, 1);
}

/// Una visita de la PC a la que este equipo dio salida no reaparece con
/// una lectura vieja de la nube (lápida, como con "por correo").
#[test]
fn una_visita_del_otro_equipo_cerrada_aca_no_revive() {
    let connection = conexion_con_dos_movimientos_visita_activos();
    let base_url = servidor_de_una_respuesta(RESPUESTA_VISITAS_ABIERTAS);
    recibir_movimientos_visita_abiertos(&connection, &contexto(&base_url)).unwrap();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
    );
    cerrar_movimiento_visita_remoto(
        &connection,
        &contexto(&base_url),
        "uuid-pc",
        "Guardia",
        chrono::Utc::now(),
    )
    .unwrap();
    assert_eq!(lapidas(&connection), vec!["uuid-pc".to_string()]);

    let base_url = servidor_de_una_respuesta(RESPUESTA_VISITAS_ABIERTAS);
    let recibidas = recibir_movimientos_visita_abiertos(&connection, &contexto(&base_url)).unwrap();
    assert!(recibidas.is_empty(), "no debe revivir: {recibidas:?}");
}

/// La PC dio salida a una visita que entró por este equipo: se cierra acá
/// también, sin usuario local, con el nombre de quien la dio.
#[test]
fn una_visita_propia_con_salida_en_el_otro_equipo_se_cierra_aca() {
    let connection = conexion_con_dos_movimientos_visita_activos();
    let base_url = servidor_de_una_respuesta(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         [{\"id\":\"uuid-m1\",\"hora_salida\":\"2026-08-01T10:00:00Z\",\
         \"usuario_salida_nombre\":\"Guardia PC\"}]",
    );

    let aplicados =
        recibir_cierres_de_movimientos_visita_propios(&connection, &contexto(&base_url)).unwrap();

    assert_eq!(aplicados, 1);
    let (salida, nombre): (String, String) = connection
        .query_row(
            "SELECT fecha_hora_salida, usuario_salida_nombre FROM movimientos_visita
             WHERE uuid = 'uuid-m1'",
            [],
            |fila| Ok((fila.get(0)?, fila.get(1)?)),
        )
        .unwrap();
    assert_eq!(salida, "2026-08-01T10:00:00Z");
    assert_eq!(nombre, "Guardia PC");
}

// --- Incidente del 2026-10-29: una salida con el token vencido ----------
//
// La salida de un contratista recibió `401` (token del teléfono vencido),
// quedó castigada con la espera de 15 minutos y las salidas siguientes
// subieron primero. Ver `cola::es_fallo_de_la_conexion`.

const RESPUESTA_401: &str = "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\n\
     Connection: close\r\n\r\n{\"code\":\"PGRST301\",\"message\":\"JWT expired\"}";
const RESPUESTA_204: &str = "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n";
const RESPUESTA_201: &str =
    "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]";

/// Dos ingresos ya cerrados en el teléfono, con sus aperturas ya en la nube
/// y las dos salidas esperando en la cola, en orden: primero la de
/// `uuid-melania` (id 10) y después la de `uuid-otro` (id 11).
fn conexion_con_dos_salidas_en_cola() -> Connection {
    let (connection, _) = conexion_con_contratista();
    connection.execute("DELETE FROM cola_salida", []).unwrap();
    connection
        .execute("INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES ('1', 'Op', 'h', 'OPERADOR', 1)", [])
        .unwrap();
    for (uuid, salida) in [
        ("uuid-melania", "2026-10-30T04:00:38Z"),
        ("uuid-otro", "2026-10-30T04:04:13Z"),
    ] {
        connection
            .execute(
                "INSERT INTO registro_ingresos (
                    contratista_id, empresa_id, fecha_hora_ingreso, fecha_hora_salida, medio_ingreso,
                    tipo_ingreso, usuario_ingreso_id, contratista_cedula, contratista_nombre,
                    empresa_nombre, usuario_ingreso_nombre, usuario_salida_nombre, es_personal_ruta,
                    tiene_acceso, resultado_acceso, reglas_version, uuid
                ) VALUES (
                    1, 1, '2026-10-29T18:58:37Z', ?2, 'CAMINANDO', 'SWAT', 1, '1-2345',
                    'Persona de prueba', 'Brisas', 'Op', 'Op', 0, 1, 'PERMITIDO', 1, ?1
                )",
                params![uuid, salida],
            )
            .unwrap();
    }
    for (id, uuid, operacion, estado, creado) in [
        (
            1,
            "uuid-melania",
            "crear",
            "enviado",
            "2026-10-29T18:58:38Z",
        ),
        (2, "uuid-otro", "crear", "enviado", "2026-10-29T19:00:00Z"),
        (
            10,
            "uuid-melania",
            "cerrar",
            "pendiente",
            "2026-10-30T04:00:38Z",
        ),
        (
            11,
            "uuid-otro",
            "cerrar",
            "pendiente",
            "2026-10-30T04:04:13Z",
        ),
    ] {
        connection
            .execute(
                "INSERT INTO cola_salida (id, entidad, entidad_uuid, operacion, estado,
                     creado_en, actualizado_en)
                 VALUES (?1, 'ingreso', ?2, ?3, ?4, ?5, ?5)",
                params![id, uuid, operacion, estado, creado],
            )
            .unwrap();
    }
    connection
}

/// El incidente tal cual: con el token vencido, la salida NO se castiga ni
/// se adelanta la siguiente. La pasada se corta con un error que las apps
/// reconocen como "renovar el token y reintentar", y en ese reintento las
/// dos salen enseguida y en orden.
#[test]
fn una_salida_con_el_token_vencido_no_se_castiga_y_sale_en_el_reintento() {
    let connection = conexion_con_dos_salidas_en_cola();
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![RESPUESTA_401]);

    let error = drenar_cola(&connection, &contexto(&base_url), 10)
        .expect_err("el 401 tiene que subir para que la app renueve el token");

    assert!(error.token_dispositivo_vencido(), "{error}");
    let pedidos_primera = pedidos.lock().unwrap().clone();
    assert_eq!(
        pedidos_primera.len(),
        1,
        "la siguiente salida no se adelanta"
    );
    assert!(pedidos_primera[0].starts_with("PATCH /rest/v1/ingresos?id=eq.uuid-melania"));
    assert_eq!(
        estado_e_intentos(&connection, 10),
        ("pendiente".to_string(), 0),
        "sin intento ni espera: lista para el reintento inmediato"
    );
    assert_eq!(
        estado_e_intentos(&connection, 11),
        ("pendiente".to_string(), 0)
    );

    // El reintento de la app, ya con el token renovado.
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![RESPUESTA_204, RESPUESTA_204]);
    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!(resumen.enviados, 2);
    let pedidos = pedidos.lock().unwrap().clone();
    assert!(
        pedidos[0].starts_with("PATCH /rest/v1/ingresos?id=eq.uuid-melania"),
        "{pedidos:?}"
    );
    assert!(
        pedidos[1].starts_with("PATCH /rest/v1/ingresos?id=eq.uuid-otro"),
        "{pedidos:?}"
    );
    assert_eq!(estado_e_intentos(&connection, 10).0, "enviado");
    assert_eq!(estado_e_intentos(&connection, 11).0, "enviado");
}

/// Sin red (la nube ni contesta): tampoco es culpa de la salida. Queda
/// intacta para el pulso siguiente, en el mismo orden.
#[test]
fn una_salida_sin_red_no_se_castiga_y_conserva_el_orden() {
    let connection = conexion_con_dos_salidas_en_cola();

    let error = drenar_cola(&connection, &contexto("http://127.0.0.1:1"), 10)
        .expect_err("sin red la pasada se corta");

    assert!(error.es_falta_de_conexion(), "{error}");
    assert_eq!(
        estado_e_intentos(&connection, 10),
        ("pendiente".to_string(), 0)
    );
    assert_eq!(
        estado_e_intentos(&connection, 11),
        ("pendiente".to_string(), 0)
    );
}

/// Un rechazo de verdad de la nube (`400`: el dato no le sirve) sí sigue el
/// camino de siempre: suma un intento, espera, y la siguiente sale igual.
#[test]
fn un_rechazo_del_dato_sigue_con_su_espera_y_no_frena_a_las_demas() {
    let connection = conexion_con_dos_salidas_en_cola();
    let (base_url, _) = servidor_que_anota_pedidos(vec![
        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
         {\"message\":\"dato invalido\"}",
        RESPUESTA_204,
    ]);

    let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

    assert_eq!((resumen.enviados, resumen.fallidos), (1, 1));
    assert_eq!(
        estado_e_intentos(&connection, 10),
        ("pendiente".to_string(), 1)
    );
    assert_eq!(estado_e_intentos(&connection, 11).0, "enviado");
}

/// Un lote que vuelve con `401` tampoco cae a probar fila por fila (todas
/// fallarían igual, sumando intentos): se corta y queda intacto.
#[test]
fn un_lote_con_el_token_vencido_no_prueba_fila_por_fila() {
    let connection = conexion_con_tres_empresas();
    let (base_url, pedidos) = servidor_que_anota_pedidos(vec![RESPUESTA_401]);

    let error =
        drenar_cola(&connection, &contexto(&base_url), 10).expect_err("el 401 del lote sube");

    assert!(error.token_dispositivo_vencido(), "{error}");
    assert_eq!(pedidos.lock().unwrap().len(), 1, "un solo pedido: el lote");
    let intentos: i64 = connection
        .query_row("SELECT SUM(intentos) FROM cola_salida", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(intentos, 0);
}

/// Lo que lee el aviso "registros sin subir": cuenta lo pendiente, toma el
/// más viejo y separa los fallos permanentes; lo enviado no cuenta.
#[test]
fn el_estado_de_la_bandeja_cuenta_pendientes_el_mas_viejo_y_los_fallidos() {
    let connection = conexion_con_dos_salidas_en_cola();
    assert_eq!(
        estado_bandeja_salida(&connection).unwrap(),
        EstadoBandejaSalida {
            pendientes: 2,
            pendiente_mas_antiguo: Some("2026-10-30T04:00:38Z".to_string()),
            fallidos: 0,
        }
    );

    connection
        .execute(
            "UPDATE cola_salida SET estado = 'fallido' WHERE id = 10",
            [],
        )
        .unwrap();
    assert_eq!(
        estado_bandeja_salida(&connection).unwrap(),
        EstadoBandejaSalida {
            pendientes: 1,
            pendiente_mas_antiguo: Some("2026-10-30T04:04:13Z".to_string()),
            fallidos: 1,
        }
    );

    connection.execute("DELETE FROM cola_salida", []).unwrap();
    assert_eq!(
        estado_bandeja_salida(&connection).unwrap(),
        EstadoBandejaSalida::default()
    );
}

//! Drena la bandeja de salida (`cola_salida`, ver
//! `docs/plan-persistencia-nube.md`) hacia el receptor: por cada fila
//! pendiente, arma el pedido HTTP correspondiente y la marca `enviado` o
//! `fallido` según la respuesta. Una fila fallida no detiene a las demás --
//! se reintenta en la próxima llamada, no bloquea el resto de la cola.

use rusqlite::{Connection, params};
use serde_json::json;

use super::cliente::NubeError;

/// Todo lo que hace falta para hablar con el receptor en nombre de este
/// dispositivo. `apikey` es la clave publicable del proyecto (no un
/// secreto -- ver `get_publishable_keys`), separada del `token` (el JWT que
/// ya identifica a este dispositivo y a su sitio).
pub struct ContextoSincronizacion<'a> {
    pub base_url: &'a str,
    pub apikey: &'a str,
    pub token: &'a str,
    pub dispositivo_id: &'a str,
    pub sitio_id: &'a str,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResumenDrenado {
    pub enviados: u32,
    pub fallidos: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum SincronizacionError {
    #[error("Error de base de datos local: {0}")]
    BaseLocal(#[from] rusqlite::Error),
    #[error(transparent)]
    Red(#[from] NubeError),
    #[error("El receptor respondió {status}: {cuerpo}")]
    RespuestaInesperada { status: u16, cuerpo: String },
}

struct FilaCola {
    id: i64,
    entidad: String,
    entidad_uuid: String,
    operacion: String,
}

/// Envía hasta `limite` filas pendientes, en orden de creación. Nunca
/// devuelve error por una fila individual fallida -- eso queda registrado
/// en la propia fila (`estado = 'fallido'`, `ultimo_error`); sólo devuelve
/// error si no se pudo ni siquiera leer/actualizar la cola local.
pub fn drenar_cola(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    limite: u32,
) -> Result<ResumenDrenado, SincronizacionError> {
    let cliente = reqwest::blocking::Client::new();
    let mut resumen = ResumenDrenado::default();

    for fila in pendientes(connection, limite)? {
        let resultado = match (fila.entidad.as_str(), fila.operacion.as_str()) {
            ("contratista", _) => {
                enviar_contratista(&cliente, connection, contexto, &fila.entidad_uuid)
            }
            ("ingreso", "cerrar") => {
                enviar_cierre_ingreso(&cliente, connection, contexto, &fila.entidad_uuid)
            }
            ("ingreso", _) => enviar_ingreso(&cliente, connection, contexto, &fila.entidad_uuid),
            _ => Ok(()),
        };

        match resultado {
            Ok(()) => {
                marcar(connection, fila.id, "enviado", None)?;
                resumen.enviados += 1;
            }
            Err(error) => {
                marcar(connection, fila.id, "fallido", Some(&error.to_string()))?;
                resumen.fallidos += 1;
            }
        }
    }

    Ok(resumen)
}

fn pendientes(connection: &Connection, limite: u32) -> Result<Vec<FilaCola>, SincronizacionError> {
    let mut statement = connection.prepare(
        "
        SELECT id, entidad, entidad_uuid, operacion FROM cola_salida
        WHERE estado = 'pendiente'
        ORDER BY creado_en
        LIMIT ?1
        ",
    )?;
    let filas = statement
        .query_map(params![limite], |row| {
            Ok(FilaCola {
                id: row.get(0)?,
                entidad: row.get(1)?,
                entidad_uuid: row.get(2)?,
                operacion: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(filas)
}

fn marcar(
    connection: &Connection,
    id: i64,
    estado: &str,
    error: Option<&str>,
) -> Result<(), SincronizacionError> {
    connection.execute(
        "
        UPDATE cola_salida
        SET
            estado = ?1,
            intentos = intentos + 1,
            ultimo_error = ?2,
            actualizado_en = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        WHERE id = ?3
        ",
        params![estado, error, id],
    )?;
    Ok(())
}

fn exigir_2xx(respuesta: reqwest::blocking::Response) -> Result<(), SincronizacionError> {
    if respuesta.status().is_success() {
        return Ok(());
    }
    let status = respuesta.status().as_u16();
    let cuerpo = respuesta.text().unwrap_or_default();
    Err(SincronizacionError::RespuestaInesperada { status, cuerpo })
}

/// Contratistas (espejo): crear y actualizar se resuelven igual -- un
/// `upsert` (`Prefer: resolution=merge-duplicates`) es idempotente y la
/// versión más nueva siempre termina ganando, así que no hace falta
/// distinguir la operación.
///
/// `empresa_id` (la referencia real a la tabla `empresas` de la nube) queda
/// sin mandar a propósito -- esa tabla espejo todavía no tiene su propio
/// UUID sincronizado (ver nota en `docs/plan-persistencia-nube.md`). Alcanza
/// con el nombre de la empresa como texto, que es lo único que necesita hoy
/// el panel de auditoría.
fn enviar_contratista(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (cedula, nombre, tiene_acceso, empresa_nombre): (String, String, i64, String) = connection
        .query_row(
            "
            SELECT c.cedula, c.nombre, c.tiene_acceso, e.nombre
            FROM contratistas c
            JOIN empresas e ON e.id = c.empresa_id
            WHERE c.uuid = ?1
            ",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

    let cuerpo = json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "nombre": nombre,
        "identificacion": cedula,
        "empresa_nombre": empresa_nombre,
        "activo": tiene_acceso != 0,
    });

    let respuesta = cliente
        .post(format!("{}/rest/v1/contratistas", contexto.base_url))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Ingresos (cola), apertura: mismo criterio de `upsert` que contratistas
/// -- reintentar un envío ya recibido no duplica nada.
fn enviar_ingreso(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (contratista_id_local, contratista_nombre, fecha_hora_ingreso, usuario_ingreso_nombre): (
        i64,
        String,
        String,
        String,
    ) = connection.query_row(
        "
        SELECT contratista_id, contratista_nombre, fecha_hora_ingreso, usuario_ingreso_nombre
        FROM registro_ingresos
        WHERE uuid = ?1
        ",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let contratista_uuid: String = connection.query_row(
        "SELECT uuid FROM contratistas WHERE id = ?1",
        params![contratista_id_local],
        |row| row.get(0),
    )?;

    let cuerpo = json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_entrada_id": contexto.dispositivo_id,
        "contratista_id": contratista_uuid,
        "contratista_nombre": contratista_nombre,
        "hora_entrada": fecha_hora_ingreso,
        "usuario_entrada_nombre": usuario_ingreso_nombre,
    });

    let respuesta = cliente
        .post(format!("{}/rest/v1/ingresos", contexto.base_url))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Ingresos (cola), cierre: `PATCH .../ingresos?id=eq.<uuid>&hora_salida=is.null`
/// es la regla de "primero en llegar gana" completa -- si el otro
/// dispositivo del mismo sitio ya cerró este ingreso, el filtro no
/// encuentra fila para actualizar (0 filas afectadas, no un error). El
/// objetivo ("que quede cerrado en la nube") ya se cumplió de todas formas,
/// así que no hace falta distinguir ese caso.
fn enviar_cierre_ingreso(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (fecha_hora_salida, usuario_salida_nombre): (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT fecha_hora_salida, usuario_salida_nombre FROM registro_ingresos WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

    let cuerpo = json!({
        "hora_salida": fecha_hora_salida,
        "dispositivo_salida_id": contexto.dispositivo_id,
        "usuario_salida_nombre": usuario_salida_nombre,
    });

    let url = format!(
        "{}/rest/v1/ingresos?id=eq.{uuid}&hora_salida=is.null",
        contexto.base_url
    );

    let respuesta = cliente
        .patch(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Fila cacheada localmente de un ingreso todavía abierto, creado por el
/// otro dispositivo de este mismo sitio -- ver `ingresos_remotos` en
/// `database::schema` sobre por qué esto no es una fila de
/// `registro_ingresos`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoRemoto {
    pub uuid: String,
    pub contratista_nombre: String,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: Option<String>,
}

#[derive(serde::Deserialize)]
struct FilaIngresoRemoto {
    id: String,
    contratista_nombre: String,
    hora_entrada: String,
    usuario_entrada_nombre: Option<String>,
    dispositivo_entrada_id: String,
}

/// Refresca la caché local `ingresos_remotos` con lo que hay abierto ahora
/// mismo en la nube para este sitio, creado por *otro* dispositivo
/// (`dispositivo_entrada_id=neq.<el mío>`). Reemplaza el contenido entero
/// de la tabla en una sola transacción -- más simple que llevar la cuenta
/// de qué cambió, y la tabla es chica (sólo lo que está abierto ahora).
pub fn recibir_ingresos_abiertos(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<IngresoRemoto>, SincronizacionError> {
    let cliente = reqwest::blocking::Client::new();
    let url = format!(
        "{}/rest/v1/ingresos?sitio_id=eq.{}&dispositivo_entrada_id=neq.{}&hora_salida=is.null\
         &select=id,contratista_nombre,hora_entrada,usuario_entrada_nombre,dispositivo_entrada_id",
        contexto.base_url, contexto.sitio_id, contexto.dispositivo_id,
    );
    let respuesta = cliente
        .get(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .send()
        .map_err(NubeError::Red)?;

    if !respuesta.status().is_success() {
        let status = respuesta.status().as_u16();
        let cuerpo = respuesta.text().unwrap_or_default();
        return Err(SincronizacionError::RespuestaInesperada { status, cuerpo });
    }
    let filas: Vec<FilaIngresoRemoto> = respuesta.json().map_err(NubeError::Red)?;

    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "DELETE FROM ingresos_remotos WHERE sitio_id = ?1",
        params![contexto.sitio_id],
    )?;
    for fila in &filas {
        transaction.execute(
            "
            INSERT INTO ingresos_remotos (
                uuid, sitio_id, contratista_nombre, hora_entrada,
                usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            ",
            params![
                fila.id,
                contexto.sitio_id,
                fila.contratista_nombre,
                fila.hora_entrada,
                fila.usuario_entrada_nombre,
                fila.dispositivo_entrada_id,
            ],
        )?;
    }
    transaction.commit()?;

    Ok(filas
        .into_iter()
        .map(|fila| IngresoRemoto {
            uuid: fila.id,
            contratista_nombre: fila.contratista_nombre,
            hora_entrada: fila.hora_entrada,
            usuario_entrada_nombre: fila.usuario_entrada_nombre,
        })
        .collect())
}

/// Cierra, directo contra la nube, un ingreso que abrió el otro
/// dispositivo del mismo sitio (mismo `PATCH` condicional que
/// `enviar_cierre_ingreso` -- "primero en llegar gana"). Nunca toca
/// `registro_ingresos` local, sólo la caché `ingresos_remotos`: este
/// ingreso no es -- y nunca fue -- del historial de este dispositivo.
pub fn cerrar_ingreso_remoto(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
    usuario_salida_nombre: &str,
) -> Result<(), SincronizacionError> {
    let cliente = reqwest::blocking::Client::new();
    let cuerpo = json!({
        "hora_salida": crate::tiempo::serializar_utc(chrono::Utc::now()),
        "dispositivo_salida_id": contexto.dispositivo_id,
        "usuario_salida_nombre": usuario_salida_nombre,
    });

    let url = format!(
        "{}/rest/v1/ingresos?id=eq.{uuid}&hora_salida=is.null",
        contexto.base_url
    );
    let respuesta = cliente
        .patch(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)?;

    connection.execute("DELETE FROM ingresos_remotos WHERE uuid = ?1", params![uuid])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

    use crate::database::schema::initialize_database;

    use super::*;

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
    fn envia_un_contratista_pendiente_y_lo_marca_enviado() {
        let (connection, _uuid) = conexion_con_contratista();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(resumen, ResumenDrenado { enviados: 1, fallidos: 0 });
        let estado: String = connection
            .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
            .unwrap();
        assert_eq!(estado, "enviado");
    }

    #[test]
    fn error_del_receptor_marca_la_fila_fallida_con_el_motivo() {
        let (connection, _uuid) = conexion_con_contratista();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"boom\"}",
        );

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(resumen, ResumenDrenado { enviados: 0, fallidos: 1 });
        let (estado, intentos, ultimo_error): (String, i64, Option<String>) = connection
            .query_row(
                "SELECT estado, intentos, ultimo_error FROM cola_salida",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(estado, "fallido");
        assert_eq!(intentos, 1);
        assert!(ultimo_error.unwrap().contains("500"));
    }

    #[test]
    fn envia_la_apertura_de_un_ingreso() {
        let (connection, contratista_uuid) = conexion_con_contratista();
        connection
            .execute(
                "DELETE FROM cola_salida",
                [],
            )
            .unwrap();
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

        assert_eq!(resumen, ResumenDrenado { enviados: 1, fallidos: 0 });
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
            .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| row.get(0))
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
            .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| row.get(0))
            .unwrap();
        assert_eq!(cacheados, 0, "lo que ya no viene en la respuesta se borra de la caché");
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

        cerrar_ingreso_remoto(&connection, &contexto(&base_url), "uuid-remoto", "Op Celular")
            .unwrap();

        let cacheados: i64 = connection
            .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| row.get(0))
            .unwrap();
        assert_eq!(cacheados, 0);
    }
}

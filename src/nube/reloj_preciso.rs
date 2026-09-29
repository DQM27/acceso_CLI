//! Desfase del reloj de este equipo con precisión de milisegundos, al
//! estilo NTP.
//!
//! El header HTTP `Date` (ver `cliente::autenticar_dispositivo`) trae
//! segundos enteros: el desfase medido con él puede errar hasta 1 s. Acá se
//! le pide la hora al servidor en milisegundos (`public.hora_servidor_ms`,
//! migración `20260929075809_hora_servidor_ms.sql`) varias veces y se usa
//! la respuesta que tardó MENOS en ir y volver: su error es, como mucho, la
//! mitad de ese viaje (unas decenas de ms en 4G). La primera respuesta suele
//! ser la peor (arma la conexión TLS), por eso no basta con una.
//!
//! Si el servidor no tiene la función (un proyecto sin la migración) o la
//! red falla, devuelve `None` y quien llama se queda con el desfase del
//! header `Date`: nunca empeora lo que había.

use std::time::{Duration, Instant};

use super::cliente::cliente_http;

/// Cuántas veces se pregunta la hora. Con 4, la más rápida casi siempre
/// cae en un viaje sin demoras de la red.
const MUESTRAS: usize = 4;
/// Tope por consulta: una respuesta más lenta que esto ya no sirve para
/// medir milisegundos.
const TIMEOUT_MUESTRA: Duration = Duration::from_secs(3);
/// Si ni la mejor muestra bajó de esto, la red está demasiado lenta para
/// mejorar al header `Date`.
const VIAJE_MAXIMO_UTIL: Duration = Duration::from_millis(1_500);

/// Reloj local MENOS reloj del servidor, en ms (misma convención que
/// `TokenDispositivo::desfase_reloj_ms`: positivo si el equipo va
/// adelantado). `None` si no se pudo medir.
pub fn medir_desfase_ms(base_url: &str, apikey: &str, access_token: &str) -> Option<i64> {
    let url = format!("{base_url}/rest/v1/rpc/hora_servidor_ms");
    let cliente = cliente_http();
    let mut mejor: Option<(Duration, i64)> = None;
    for _ in 0..MUESTRAS {
        let inicio_pared = chrono::Utc::now();
        let inicio = Instant::now();
        let respuesta = cliente
            .post(&url)
            .header("apikey", apikey)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .body("{}")
            .timeout(TIMEOUT_MUESTRA)
            .send()
            .ok()?;
        let viaje = inicio.elapsed();
        // Sin la función (404) o sin permiso: no tiene sentido insistir.
        if !respuesta.status().is_success() {
            return None;
        }
        let Ok(hora_servidor_ms) = respuesta.text().ok()?.trim().parse::<i64>() else {
            return None;
        };
        // El servidor leyó su reloj, en promedio, a mitad del viaje. Se
        // suma la mitad medida con el reloj monotónico (no con la hora de
        // pared, que podría saltar entre medio).
        let mitad = chrono::Duration::from_std(viaje / 2).ok()?;
        let local_ms = (inicio_pared + mitad).timestamp_millis();
        let desfase = local_ms - hora_servidor_ms;
        if mejor.is_none_or(|(viaje_mejor, _)| viaje < viaje_mejor) {
            mejor = Some((viaje, desfase));
        }
    }
    mejor
        .filter(|(viaje, _)| *viaje <= VIAJE_MAXIMO_UTIL)
        .map(|(_, desfase)| desfase)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use super::*;

    /// Lee un pedido HTTP completo (headers y cuerpo según
    /// `Content-Length`) sin esperar a que el cliente cierre.
    fn leer_pedido(conexion: &mut std::net::TcpStream) {
        let mut leido = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let Ok(n) = conexion.read(&mut buffer) else {
                return;
            };
            if n == 0 {
                return;
            }
            leido.extend_from_slice(&buffer[..n]);
            let texto = String::from_utf8_lossy(&leido);
            if let Some(fin_headers) = texto.find("\r\n\r\n") {
                let largo = texto[..fin_headers]
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                    })
                    .unwrap_or(0);
                if leido.len() >= fin_headers + 4 + largo {
                    return;
                }
            }
        }
    }

    /// Servidor HTTP mínimo: una conexión por respuesta (`Connection:
    /// close`). `responder` arma la respuesta EN EL MOMENTO de contestar,
    /// apenas llegó el pedido (así la hora que devuelve cae a mitad del
    /// viaje, como en un servidor real).
    fn servidor(veces: usize, responder: impl Fn() -> String + Send + 'static) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
        let direccion = listener.local_addr().expect("dirección local");
        thread::spawn(move || {
            for _ in 0..veces {
                let Ok((mut conexion, _)) = listener.accept() else {
                    return;
                };
                conexion
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("set_read_timeout");
                leer_pedido(&mut conexion);
                let _ = conexion.write_all(responder().as_bytes());
                let _ = conexion.flush();
            }
        });
        format!("http://{direccion}")
    }

    fn respuesta(cuerpo: &str, estado: &str) -> String {
        format!(
            "HTTP/1.1 {estado}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
    }

    #[test]
    fn mide_el_desfase_en_milisegundos() {
        // El servidor va 2,345 s atrasado respecto de este equipo.
        let base_url = servidor(MUESTRAS, || {
            respuesta(
                &(chrono::Utc::now().timestamp_millis() - 2_345).to_string(),
                "200 OK",
            )
        });

        let desfase = medir_desfase_ms(&base_url, "apikey", "token").expect("se mide");

        // En localhost el viaje dura milisegundos: la medición cae muy
        // cerca de 2345 (margen amplio por si la máquina está cargada).
        assert!(
            (2_245..=2_445).contains(&desfase),
            "desfase medido fuera de rango: {desfase} ms"
        );
    }

    #[test]
    fn sin_la_funcion_en_el_servidor_no_hay_medicion() {
        let base_url = servidor(1, || respuesta("", "404 Not Found"));
        assert_eq!(medir_desfase_ms(&base_url, "apikey", "token"), None);
    }

    #[test]
    fn una_respuesta_que_no_es_un_numero_no_se_usa() {
        let base_url = servidor(1, || respuesta("{}", "200 OK"));
        assert_eq!(medir_desfase_ms(&base_url, "apikey", "token"), None);
    }
}

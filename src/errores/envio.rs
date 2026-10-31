//! Manda los errores en disco al receptor, en lotes. Bloqueante: sólo desde
//! un hilo propio, nunca desde un runtime de tokio.

use std::time::Duration;

use reqwest::StatusCode;

use super::cola::Cola;
use super::destino::Destino;

/// Errores por petición.
pub const POR_LOTE: usize = 10;
const ESPERA_MAXIMA: Duration = Duration::from_secs(10);

/// Manda todo lo pendiente y borra lo que el receptor recibió. Devuelve
/// cuántos errores salieron. Si el receptor no está disponible (red,
/// límite, 5xx), corta y deja el resto para la próxima vuelta.
///
/// Un lote que el receptor rechaza por mal formado (400) también se borra:
/// reintentarlo daría siempre lo mismo.
pub fn enviar(destino: &Destino, cola: &Cola) -> Result<usize, String> {
    let pendientes = cola.pendientes();
    if pendientes.is_empty() {
        return Ok(0);
    }
    let cliente = reqwest::blocking::Client::builder()
        .timeout(ESPERA_MAXIMA)
        .build()
        .map_err(|error| error.to_string())?;
    let mut enviados = 0;
    for lote in pendientes.chunks(POR_LOTE) {
        let mut reportes = Vec::with_capacity(lote.len());
        let mut rutas = Vec::with_capacity(lote.len());
        for ruta in lote {
            let leido = std::fs::read(ruta)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
            if let Some(reporte) = leido {
                reportes.push(reporte);
                rutas.push(ruta);
            } else {
                // Dañado: nunca va a servir.
                Cola::borrar(ruta);
            }
        }
        if reportes.is_empty() {
            continue;
        }
        let estado = cliente
            .post(&destino.url)
            .header("x-clave-errores", &destino.clave)
            .json(&serde_json::json!({ "reportes": reportes }))
            .send()
            .map_err(|error| error.to_string())?
            .status();
        if !estado.is_success() && estado != StatusCode::BAD_REQUEST {
            return Err(format!("receptor-errores respondió {estado}"));
        }
        if estado == StatusCode::BAD_REQUEST {
            log::warn!("receptor-errores rechazó un lote de errores por mal formado: se descarta");
        } else {
            enviados += rutas.len();
        }
        for ruta in rutas {
            Cola::borrar(ruta);
        }
    }
    Ok(enviados)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    use super::{POR_LOTE, enviar};
    use crate::errores::cola::Cola;
    use crate::errores::destino::Destino;
    use crate::errores::formato::{ESQUEMA, Reporte};

    fn reporte(i: usize) -> Reporte {
        Reporte {
            esquema: ESQUEMA,
            id: format!("id-{i:03}"),
            ocurrido_en: format!("2026-10-31T12:00:{i:02}.000Z"),
            plataforma: "movil".into(),
            version_app: "1.8.12 (57)".into(),
            origen: "excepcion".into(),
            titulo: "IllegalStateException".into(),
            traza: Vec::new(),
            pantalla: Some("Ingreso".into()),
            operaciones: Vec::new(),
            contexto: BTreeMap::new(),
        }
    }

    /// Un receptor de prueba: responde `estados` en orden, una petición por
    /// estado, y pasa lo que recibió (clave y cuerpo).
    fn receptor(estados: Vec<u16>) -> (String, mpsc::Receiver<(String, String)>) {
        let escucha = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1/errores", escucha.local_addr().unwrap());
        let (enviar, recibir) = mpsc::channel();
        std::thread::spawn(move || {
            for estado in estados {
                let (conexion, _) = escucha.accept().unwrap();
                let mut lector = BufReader::new(conexion);
                let mut clave = String::new();
                let mut largo = 0;
                loop {
                    let mut linea = String::new();
                    lector.read_line(&mut linea).unwrap();
                    let linea = linea.trim_end().to_owned();
                    if linea.is_empty() {
                        break;
                    }
                    let (nombre, valor) = linea.split_once(": ").unwrap_or_default();
                    match nombre.to_ascii_lowercase().as_str() {
                        "x-clave-errores" => valor.clone_into(&mut clave),
                        "content-length" => largo = valor.parse().unwrap(),
                        _ => {}
                    }
                }
                let mut cuerpo = vec![0; largo];
                lector.read_exact(&mut cuerpo).unwrap();
                enviar
                    .send((clave, String::from_utf8(cuerpo).unwrap()))
                    .unwrap();
                write!(
                    lector.get_mut(),
                    "HTTP/1.1 {estado} X\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{{}}"
                )
                .unwrap();
            }
        });
        (url, recibir)
    }

    #[test]
    fn manda_en_lotes_con_la_clave_y_borra_lo_recibido() {
        let dir = tempfile::tempdir().unwrap();
        let cola = Cola::en(dir.path());
        for i in 0..POR_LOTE + 2 {
            cola.guardar(&reporte(i)).unwrap();
        }
        let (url, recibido) = receptor(vec![200, 200]);
        let destino = Destino {
            url,
            clave: "clave-1".into(),
        };

        assert_eq!(enviar(&destino, &cola), Ok(POR_LOTE + 2));
        assert!(cola.pendientes().is_empty());
        let (clave, cuerpo) = recibido.recv().unwrap();
        assert_eq!(clave, "clave-1");
        let cuerpo: serde_json::Value = serde_json::from_str(&cuerpo).unwrap();
        assert_eq!(cuerpo["reportes"].as_array().unwrap().len(), POR_LOTE);
        assert_eq!(cuerpo["reportes"][0]["id"], "id-000", "del más viejo");
        assert_eq!(cuerpo["reportes"][0]["pantalla"], "Ingreso");
    }

    #[test]
    fn si_el_receptor_no_esta_disponible_deja_todo_para_despues() {
        let dir = tempfile::tempdir().unwrap();
        let cola = Cola::en(dir.path());
        cola.guardar(&reporte(1)).unwrap();
        let (url, _recibido) = receptor(vec![503]);
        let destino = Destino {
            url,
            clave: "c".into(),
        };

        assert!(enviar(&destino, &cola).is_err());
        assert_eq!(cola.pendientes().len(), 1);
    }

    #[test]
    fn un_lote_mal_formado_se_descarta_y_uno_danado_ni_se_manda() {
        let dir = tempfile::tempdir().unwrap();
        let cola = Cola::en(dir.path());
        cola.guardar(&reporte(1)).unwrap();
        std::fs::write(
            dir.path().join("errores_pendientes").join("0-roto.json"),
            "{no",
        )
        .unwrap();
        let (url, _recibido) = receptor(vec![400]);
        let destino = Destino {
            url,
            clave: "c".into(),
        };

        assert_eq!(enviar(&destino, &cola), Ok(0));
        assert!(cola.pendientes().is_empty());
    }

    #[test]
    fn sin_pendientes_no_hace_ninguna_peticion() {
        let dir = tempfile::tempdir().unwrap();
        let destino = Destino {
            url: "http://127.0.0.1:9/nadie".into(),
            clave: "c".into(),
        };
        assert_eq!(enviar(&destino, &Cola::en(dir.path())), Ok(0));
    }
}

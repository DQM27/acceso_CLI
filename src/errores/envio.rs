//! Manda los errores en disco a la receptora `observabilidad` de la nube
//! (`/functions/v1/observabilidad/errores`), en lotes comprimidos con brotli y
//! con el token del equipo, como la telemetría
//! (`docs/features-futuras/plan-observabilidad-en-produccion.md`). La
//! receptora limpia otra vez el texto, calcula la huella y llama a
//! `registrar_errores` con ese mismo token: la nube pone el equipo y la
//! unidad.
//!
//! Bloqueante: sólo desde un hilo propio, nunca desde un runtime de tokio.

use reqwest::StatusCode;

use super::cola::Cola;
use crate::telemetria::lote;

/// Ruta de la receptora, relativa a la URL base del proyecto.
pub const RUTA_RECEPTORA: &str = "/functions/v1/observabilidad/errores";
/// Errores por petición.
pub const POR_LOTE: usize = 10;

/// A dónde y con qué se manda.
pub struct Destino<'a> {
    pub cliente: &'a reqwest::blocking::Client,
    pub base_url: &'a str,
    pub apikey: &'a str,
    pub token: &'a str,
}

/// Manda todo lo pendiente y borra lo que la nube recibió. Devuelve cuántos
/// errores salieron. Si la nube no puede recibir (red, token, ritmo, 5xx),
/// corta y deja el resto para la próxima vuelta.
///
/// Un lote que la nube rechaza por mal formado (400, 413, 422) también se
/// borra: reintentarlo daría siempre lo mismo.
pub fn enviar(destino: &Destino<'_>, cola: &Cola) -> Result<usize, String> {
    let pendientes = cola.pendientes();
    let mut enviados = 0;
    for lote_rutas in pendientes.chunks(POR_LOTE) {
        let mut reportes = Vec::with_capacity(lote_rutas.len());
        let mut rutas = Vec::with_capacity(lote_rutas.len());
        for ruta in lote_rutas {
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
        let cuerpo = lote::comprimir(&serde_json::json!({ "reportes": reportes }));
        let estado = destino
            .cliente
            .post(format!("{}{RUTA_RECEPTORA}", destino.base_url))
            .header("apikey", destino.apikey)
            .bearer_auth(destino.token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::CONTENT_ENCODING, "br")
            .body(cuerpo)
            .send()
            .map_err(|error| error.to_string())?
            .status();
        match estado {
            _ if estado.is_success() => enviados += rutas.len(),
            StatusCode::BAD_REQUEST
            | StatusCode::PAYLOAD_TOO_LARGE
            | StatusCode::UNPROCESSABLE_ENTITY => {
                log::warn!("la nube rechazó un lote de errores ({estado}): se descarta");
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err("sin autorización para mandar errores".to_owned());
            }
            _ => return Err(format!("la receptora de errores respondió {estado}")),
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

    use super::{Destino, POR_LOTE, RUTA_RECEPTORA, enviar};
    use crate::errores::cola::Cola;
    use crate::errores::formato::{ESQUEMA, Reporte};
    use crate::telemetria::lote;

    fn reporte(i: usize) -> Reporte {
        Reporte {
            esquema: ESQUEMA,
            id: format!("id-{i:03}-de-prueba"),
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

    /// Lo que la receptora de prueba vio en una petición.
    struct Peticion {
        ruta: String,
        cabeceras: Vec<(String, String)>,
        cuerpo: Vec<u8>,
    }

    impl Peticion {
        fn cabecera(&self, nombre: &str) -> Option<&str> {
            self.cabeceras
                .iter()
                .find(|(n, _)| n == nombre)
                .map(|(_, v)| v.as_str())
        }

        fn reportes(&self) -> serde_json::Value {
            serde_json::from_slice::<serde_json::Value>(
                &lote::descomprimir(&self.cuerpo, 1 << 20).unwrap(),
            )
            .unwrap()["reportes"]
                .clone()
        }
    }

    /// Una receptora de prueba: responde `estados` en orden, una petición por
    /// estado, y pasa lo que recibió.
    fn receptora(estados: Vec<u16>) -> (String, mpsc::Receiver<Peticion>) {
        let escucha = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", escucha.local_addr().unwrap());
        let (enviar, recibir) = mpsc::channel();
        std::thread::spawn(move || {
            for estado in estados {
                let Ok((conexion, _)) = escucha.accept() else {
                    return;
                };
                let mut lector = BufReader::new(conexion);
                let mut primera = String::new();
                lector.read_line(&mut primera).unwrap();
                let ruta = primera
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned();
                let mut cabeceras = Vec::new();
                let mut largo = 0;
                loop {
                    let mut linea = String::new();
                    lector.read_line(&mut linea).unwrap();
                    let linea = linea.trim_end().to_owned();
                    if linea.is_empty() {
                        break;
                    }
                    let (nombre, valor) = linea.split_once(": ").unwrap_or_default();
                    let nombre = nombre.to_ascii_lowercase();
                    if nombre == "content-length" {
                        largo = valor.parse().unwrap();
                    }
                    cabeceras.push((nombre, valor.to_owned()));
                }
                let mut cuerpo = vec![0; largo];
                lector.read_exact(&mut cuerpo).unwrap();
                enviar
                    .send(Peticion {
                        ruta,
                        cabeceras,
                        cuerpo,
                    })
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

    fn destino<'a>(cliente: &'a reqwest::blocking::Client, url: &'a str) -> Destino<'a> {
        Destino {
            cliente,
            base_url: url,
            apikey: "clave-publica",
            token: "jwt-del-equipo",
        }
    }

    #[test]
    fn manda_en_lotes_brotli_con_el_token_y_borra_lo_recibido() {
        let dir = tempfile::tempdir().unwrap();
        let cola = Cola::en(dir.path());
        for i in 0..POR_LOTE + 2 {
            cola.guardar(&reporte(i)).unwrap();
        }
        let (url, peticiones) = receptora(vec![200, 200]);
        let cliente = reqwest::blocking::Client::new();

        assert_eq!(enviar(&destino(&cliente, &url), &cola), Ok(POR_LOTE + 2));
        assert!(cola.pendientes().is_empty());

        let peticion = peticiones.recv().unwrap();
        assert_eq!(peticion.ruta, RUTA_RECEPTORA);
        assert_eq!(
            peticion.cabecera("authorization"),
            Some("Bearer jwt-del-equipo")
        );
        assert_eq!(peticion.cabecera("apikey"), Some("clave-publica"));
        assert_eq!(peticion.cabecera("content-encoding"), Some("br"));
        let reportes = peticion.reportes();
        assert_eq!(reportes.as_array().unwrap().len(), POR_LOTE);
        assert_eq!(reportes[0]["id"], "id-000-de-prueba", "del más viejo");
        assert_eq!(reportes[0]["pantalla"], "Ingreso");
        assert_eq!(peticiones.recv().unwrap().reportes().as_array().unwrap().len(), 2);
    }

    #[test]
    fn si_la_nube_no_esta_disponible_deja_todo_para_despues() {
        let dir = tempfile::tempdir().unwrap();
        let cola = Cola::en(dir.path());
        cola.guardar(&reporte(1)).unwrap();
        let (url, _peticiones) = receptora(vec![503]);
        let cliente = reqwest::blocking::Client::new();

        assert!(enviar(&destino(&cliente, &url), &cola).is_err());
        assert_eq!(cola.pendientes().len(), 1);
    }

    #[test]
    fn sin_autorizacion_corta_y_deja_todo() {
        let dir = tempfile::tempdir().unwrap();
        let cola = Cola::en(dir.path());
        cola.guardar(&reporte(1)).unwrap();
        let (url, _peticiones) = receptora(vec![401]);
        let cliente = reqwest::blocking::Client::new();

        assert_eq!(
            enviar(&destino(&cliente, &url), &cola),
            Err("sin autorización para mandar errores".to_owned())
        );
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
        let (url, peticiones) = receptora(vec![400]);
        let cliente = reqwest::blocking::Client::new();

        assert_eq!(enviar(&destino(&cliente, &url), &cola), Ok(0));
        assert!(cola.pendientes().is_empty());
        assert_eq!(
            peticiones.recv().unwrap().reportes().as_array().unwrap().len(),
            1,
            "el dañado no viajó"
        );
    }

    #[test]
    fn sin_pendientes_no_hace_ninguna_peticion() {
        let dir = tempfile::tempdir().unwrap();
        let cliente = reqwest::blocking::Client::new();
        let destino = Destino {
            cliente: &cliente,
            base_url: "http://127.0.0.1:9",
            apikey: "k",
            token: "t",
        };
        assert_eq!(enviar(&destino, &Cola::en(dir.path())), Ok(0));
    }
}

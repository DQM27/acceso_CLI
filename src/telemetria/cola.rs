//! Los eventos de telemetría que todavía no llegaron a la nube, en disco
//! (`docs/features-futuras/plan-observabilidad-en-produccion.md`). Sobreviven
//! a un cierre, un apagón o días sin red, y ningún evento se pierde ni se
//! manda dos veces aunque el proceso muera en cualquier punto.
//!
//! Tres archivos en `telemetria_cola/`:
//!
//! - `eventos.jsonl`: lo que se va midiendo, una línea por evento. Sólo se
//!   le agregan líneas.
//! - `tomando.jsonl`: `eventos.jsonl` renombrado al empezar a armar un lote.
//!   Renombrar es atómico: lo nuevo que se mida mientras tanto va a un
//!   `eventos.jsonl` nuevo y no se mezcla.
//! - `en_vuelo.json`: el lote armado (con su `lote_id`) y lo que sobró para
//!   el próximo. Se escribe entero en un temporal y se renombra (atómico).
//!   Mientras exista, se reintenta ESE lote con el mismo `lote_id`, y la nube
//!   responde `duplicado` si ya lo había recibido.
//!
//! Cada paso deja el disco en un estado del que se puede seguir:
//! `tomando.jsonl` sin `en_vuelo.json` se vuelve a armar; con `en_vuelo.json`
//! ya quedó capturado y se borra.

use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::lote::{self, EventoTelemetria};

/// Carpeta de la cola, dentro del directorio de datos de la app.
pub const CARPETA: &str = "telemetria_cola";

/// Tope de `eventos.jsonl`. Sin red por días, lo nuevo se descarta (y se
/// cuenta) en vez de llenar el disco del teléfono. ~8 MB son unos 20.000
/// eventos: varios días de un equipo con estudios encendidos.
pub const TOPE_BYTES: u64 = 8 * 1024 * 1024;

/// Filas por lote. La nube acepta hasta 1.000 (`observabilidad.config`).
pub const FILAS_POR_LOTE: usize = 500;

pub struct ColaTelemetria {
    carpeta: PathBuf,
}

/// Lo que se guarda en `en_vuelo.json`.
struct EnVuelo {
    lote: Value,
    resto: Vec<EventoTelemetria>,
}

impl ColaTelemetria {
    pub fn en(base: &Path) -> Self {
        Self {
            carpeta: base.join(CARPETA),
        }
    }

    fn ruta(&self, nombre: &str) -> PathBuf {
        self.carpeta.join(nombre)
    }

    /// Agrega eventos al final. Devuelve cuántos se descartaron por el tope.
    pub fn agregar(&self, eventos: &[EventoTelemetria]) -> io::Result<usize> {
        if eventos.is_empty() {
            return Ok(0);
        }
        fs::create_dir_all(&self.carpeta)?;
        let ruta = self.ruta("eventos.jsonl");
        let ocupado = fs::metadata(&ruta).map_or(0, |m| m.len());
        let mut lineas = String::new();
        let mut descartados = 0;
        for evento in eventos {
            let linea = evento.a_linea();
            if ocupado + (lineas.len() + linea.len()) as u64 > TOPE_BYTES {
                descartados += 1;
                continue;
            }
            lineas.push_str(&linea);
            lineas.push('\n');
        }
        if !lineas.is_empty() {
            // Si un corte dejó la última línea a medias, lo nuevo empieza en
            // una línea aparte: se pierde sólo la línea cortada.
            if ocupado > 0 && !termina_en_salto(&ruta)? {
                lineas.insert(0, '\n');
            }
            let mut archivo = OpenOptions::new().create(true).append(true).open(&ruta)?;
            archivo.write_all(lineas.as_bytes())?;
        }
        Ok(descartados)
    }

    /// Cuántos eventos esperan, aproximado (por el tamaño de los archivos, sin
    /// leerlos): alcanza para decidir si conviene no esperar más
    /// (`disparador`). Unos 150 bytes por evento.
    pub fn pendientes_aprox(&self) -> usize {
        let bytes: u64 = ["eventos.jsonl", "tomando.jsonl", "en_vuelo.json"]
            .iter()
            .filter_map(|nombre| fs::metadata(self.ruta(nombre)).ok())
            .map(|m| m.len())
            .sum();
        usize::try_from(bytes / 150).unwrap_or(usize::MAX)
    }

    /// `true` si hay algo para mandar (un lote en vuelo o eventos nuevos).
    pub fn hay_pendientes(&self) -> bool {
        self.ruta("en_vuelo.json").exists()
            || self.ruta("tomando.jsonl").exists()
            || fs::metadata(self.ruta("eventos.jsonl")).is_ok_and(|m| m.len() > 0)
    }

    /// El lote que toca mandar: el que está en vuelo (reintento, mismo
    /// `lote_id`) o uno nuevo con los eventos esperando. `None` si no hay
    /// nada. `nuevo_id` sólo se llama si hace falta armar uno.
    pub fn lote_a_enviar(
        &self,
        sesion: &Value,
        nuevo_id: impl Fn() -> String,
    ) -> io::Result<Option<Value>> {
        if let Some(en_vuelo) = self.leer_en_vuelo()? {
            // Quedó de antes de un corte: ya está capturado.
            let _ = fs::remove_file(self.ruta("tomando.jsonl"));
            return Ok(Some(en_vuelo.lote));
        }
        let tomando = self.ruta("tomando.jsonl");
        if !tomando.exists() {
            match fs::rename(self.ruta("eventos.jsonl"), &tomando) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(error),
            }
        }
        let eventos = leer_lineas(&tomando)?;
        if eventos.is_empty() {
            let _ = fs::remove_file(&tomando);
            return Ok(None);
        }
        let lote = self.guardar_siguiente(sesion, eventos, &nuevo_id)?;
        let _ = fs::remove_file(&tomando);
        Ok(lote)
    }

    /// El lote en vuelo ya llegó (`recibido` o `duplicado`) o la nube lo
    /// rechazó por mal formado (no tiene arreglo): se pasa al siguiente con lo
    /// que había sobrado, o se termina.
    pub fn confirmar(&self, sesion: &Value, nuevo_id: impl Fn() -> String) -> io::Result<()> {
        let Some(en_vuelo) = self.leer_en_vuelo()? else {
            return Ok(());
        };
        if en_vuelo.resto.is_empty() {
            return borrar_si_existe(&self.ruta("en_vuelo.json"));
        }
        self.guardar_siguiente(sesion, en_vuelo.resto, &nuevo_id)?;
        Ok(())
    }

    /// Arma un lote con los primeros [`FILAS_POR_LOTE`] eventos y guarda
    /// `en_vuelo.json` (lote + resto) de una vez.
    fn guardar_siguiente(
        &self,
        sesion: &Value,
        mut eventos: Vec<EventoTelemetria>,
        nuevo_id: &impl Fn() -> String,
    ) -> io::Result<Option<Value>> {
        if eventos.is_empty() {
            borrar_si_existe(&self.ruta("en_vuelo.json"))?;
            return Ok(None);
        }
        let resto = eventos.split_off(eventos.len().min(FILAS_POR_LOTE));
        let lote = lote::armar(&nuevo_id(), sesion, &eventos);
        let contenido = json!({
            "lote": lote,
            "resto": resto.iter().map(EventoTelemetria::a_valor).collect::<Vec<_>>(),
        });
        escribir_atomico(
            &self.ruta("en_vuelo.json"),
            &serde_json::to_vec(&contenido)?,
        )?;
        Ok(Some(lote))
    }

    fn leer_en_vuelo(&self) -> io::Result<Option<EnVuelo>> {
        let ruta = self.ruta("en_vuelo.json");
        let bytes = match fs::read(&ruta) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let Ok(valor) = serde_json::from_slice::<Value>(&bytes) else {
            // Dañado (no debería pasar: se escribe con renombre atómico).
            log::warn!("telemetría: en_vuelo.json dañado, se descarta");
            borrar_si_existe(&ruta)?;
            return Ok(None);
        };
        let resto = valor["resto"]
            .as_array()
            .map(|eventos| {
                eventos
                    .iter()
                    .filter_map(EventoTelemetria::desde_valor)
                    .collect()
            })
            .unwrap_or_default();
        Ok(Some(EnVuelo {
            lote: valor["lote"].clone(),
            resto,
        }))
    }
}

/// Las líneas que se pueden leer; una dañada (corte a mitad de escritura) se
/// salta.
fn leer_lineas(ruta: &Path) -> io::Result<Vec<EventoTelemetria>> {
    let archivo = fs::File::open(ruta)?;
    Ok(BufReader::new(archivo)
        .lines()
        .map_while(Result::ok)
        .filter_map(|linea| serde_json::from_str::<Value>(&linea).ok())
        .filter_map(|valor| EventoTelemetria::desde_valor(&valor))
        .collect())
}

fn termina_en_salto(ruta: &Path) -> io::Result<bool> {
    use std::io::{Read, Seek, SeekFrom};
    let mut archivo = fs::File::open(ruta)?;
    archivo.seek(SeekFrom::End(-1))?;
    let mut ultimo = [0_u8; 1];
    archivo.read_exact(&mut ultimo)?;
    Ok(ultimo[0] == b'\n')
}

fn escribir_atomico(ruta: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporal = ruta.with_extension("tmp");
    {
        let mut archivo = fs::File::create(&temporal)?;
        archivo.write_all(bytes)?;
        archivo.sync_all()?;
    }
    fs::rename(temporal, ruta)
}

fn borrar_si_existe(ruta: &Path) -> io::Result<()> {
    match fs::remove_file(ruta) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn evento(i: i64) -> EventoTelemetria {
        EventoTelemetria {
            ocurrido_ms: 1_793_470_000_000 + i,
            tipo: "reloj".to_owned(),
            datos: json!({ "i": i }).as_object().cloned().unwrap(),
        }
    }

    fn sesion() -> Value {
        json!({ "id": "dddddddd-0000-0000-0000-000000000001" })
    }

    fn ids() -> impl Fn() -> String {
        let siguiente = Cell::new(0);
        move || {
            siguiente.set(siguiente.get() + 1);
            format!("lote-{}", siguiente.get())
        }
    }

    fn eventos_del(lote: &Value) -> Vec<i64> {
        lote::reconstruir(lote)
            .unwrap()
            .iter()
            .map(|e| e.datos["i"].as_i64().unwrap())
            .collect()
    }

    #[test]
    fn sin_nada_no_hay_lote() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        assert!(!cola.hay_pendientes());
        assert_eq!(cola.lote_a_enviar(&sesion(), ids()).unwrap(), None);
    }

    #[test]
    fn el_reintento_manda_el_mismo_lote_con_el_mismo_id() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        cola.agregar(&[evento(1), evento(2)]).unwrap();
        let nuevo_id = ids();
        let primero = cola.lote_a_enviar(&sesion(), &nuevo_id).unwrap().unwrap();
        // Llega un evento nuevo mientras el lote está en vuelo.
        cola.agregar(&[evento(3)]).unwrap();
        let reintento = cola.lote_a_enviar(&sesion(), &nuevo_id).unwrap().unwrap();
        assert_eq!(primero, reintento);
        assert_eq!(reintento["lote_id"], "lote-1");
        assert_eq!(eventos_del(&reintento), vec![1, 2]);
    }

    #[test]
    fn confirmar_pasa_al_siguiente_sin_perder_ni_repetir() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        let total = i64::try_from(FILAS_POR_LOTE).unwrap() * 2 + 7;
        cola.agregar(&(0..total).map(evento).collect::<Vec<_>>())
            .unwrap();
        let nuevo_id = ids();
        let mut vistos = Vec::new();
        let mut lote_ids = Vec::new();
        while let Some(lote) = cola.lote_a_enviar(&sesion(), &nuevo_id).unwrap() {
            lote_ids.push(lote["lote_id"].as_str().unwrap().to_owned());
            vistos.extend(eventos_del(&lote));
            cola.confirmar(&sesion(), &nuevo_id).unwrap();
        }
        assert_eq!(vistos, (0..total).collect::<Vec<_>>());
        assert_eq!(lote_ids, ["lote-1", "lote-2", "lote-3"]);
        assert!(!cola.hay_pendientes());
    }

    #[test]
    fn un_corte_despues_de_renombrar_no_pierde_eventos() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        cola.agregar(&[evento(1), evento(2)]).unwrap();
        // El proceso murió justo después de renombrar, antes de armar el lote.
        fs::rename(
            dir.path().join(CARPETA).join("eventos.jsonl"),
            dir.path().join(CARPETA).join("tomando.jsonl"),
        )
        .unwrap();
        cola.agregar(&[evento(3)]).unwrap();
        let nuevo_id = ids();
        let lote = cola.lote_a_enviar(&sesion(), &nuevo_id).unwrap().unwrap();
        assert_eq!(eventos_del(&lote), vec![1, 2]);
        cola.confirmar(&sesion(), &nuevo_id).unwrap();
        let siguiente = cola.lote_a_enviar(&sesion(), &nuevo_id).unwrap().unwrap();
        assert_eq!(eventos_del(&siguiente), vec![3]);
    }

    #[test]
    fn un_corte_despues_de_guardar_el_lote_no_duplica() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        cola.agregar(&[evento(1)]).unwrap();
        let nuevo_id = ids();
        let lote = cola.lote_a_enviar(&sesion(), &nuevo_id).unwrap().unwrap();
        // Simula que murió antes de borrar `tomando.jsonl`.
        fs::write(
            dir.path().join(CARPETA).join("tomando.jsonl"),
            format!("{}\n", evento(1).a_linea()),
        )
        .unwrap();
        let reintento = cola.lote_a_enviar(&sesion(), &nuevo_id).unwrap().unwrap();
        assert_eq!(reintento, lote, "se reintenta el mismo lote");
        cola.confirmar(&sesion(), &nuevo_id).unwrap();
        assert_eq!(
            cola.lote_a_enviar(&sesion(), &nuevo_id).unwrap(),
            None,
            "el evento no vuelve a salir"
        );
    }

    #[test]
    fn una_linea_danada_se_salta() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        cola.agregar(&[evento(1)]).unwrap();
        let ruta = dir.path().join(CARPETA).join("eventos.jsonl");
        let mut archivo = OpenOptions::new().append(true).open(&ruta).unwrap();
        archivo.write_all(b"{\"t\": 17934700").unwrap(); // cortada a la mitad
        drop(archivo);
        cola.agregar(&[evento(2)]).unwrap();
        let lote = cola.lote_a_enviar(&sesion(), ids()).unwrap().unwrap();
        // Se pierde sólo la línea cortada; lo que vino después, no.
        assert_eq!(eventos_del(&lote), vec![1, 2]);
    }

    #[test]
    fn con_el_tope_lleno_lo_nuevo_se_descarta_y_se_cuenta() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        fs::create_dir_all(dir.path().join(CARPETA)).unwrap();
        let relleno = vec![b' '; usize::try_from(TOPE_BYTES).unwrap() - 10];
        fs::write(dir.path().join(CARPETA).join("eventos.jsonl"), relleno).unwrap();
        assert_eq!(cola.agregar(&[evento(1), evento(2)]).unwrap(), 2);
    }
}

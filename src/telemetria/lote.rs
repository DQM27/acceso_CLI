//! El lote de telemetría que viaja a la nube: formato columnar + brotli
//! (`docs/features-futuras/plan-observabilidad-en-produccion.md`, "Formato del
//! lote"). La nube lo decodifica en `observabilidad.registrar_telemetria`
//! (migración `20261101120000_observabilidad_telemetria`); [`reconstruir`] es
//! la misma regla en Rust y sirve de especificación ejecutable.
//!
//! ```json
//! { "lote_id": "<uuid>", "esquema": 3, "base_ms": 1793470000000,
//!   "sesion": { ... },
//!   "bloques": [ { "tipo": "llamada_nucleo",
//!                  "cols": ["errores", "llamadas", "nombre", ...],
//!                  "filas": [[0, 1, 12, "registrar_ingreso", ...], ...] } ] }
//! ```
//!
//! - Los eventos se ordenan por hora; `base_ms` es la del primero.
//! - Un bloque agrupa los eventos de un mismo tipo con exactamente las mismas
//!   claves: las columnas van una vez y cada fila es `[dt, valores...]`, con
//!   `dt` en ms desde la fila anterior del bloque (la primera, desde
//!   `base_ms`). Como todo va ordenado, `dt` nunca es negativo.
//! - Un campo ausente y uno `null` se distinguen: un evento sin la clave
//!   cae en otro bloque que uno con la clave en `null`.
//! - Medido con un lote real de 200 filas del móvil: 75.463 B en JSON por
//!   fila, 11.479 B columnar y 2.932 B columnar con brotli 11.

use std::collections::HashMap;
use std::io::{Read, Write};

use serde_json::{Map, Value, json};

/// Versión del contrato de telemetría que inaugura este formato.
pub const ESQUEMA_LOTE: u32 = 3;

/// Calidad de brotli: la máxima. Medido: 22 ms para comprimir un lote de
/// 200 filas en una PC, una vez por envío y en un hilo de fondo.
const CALIDAD_BROTLI: u32 = 11;

/// Ventana de brotli (2^22 = 4 MB): de sobra para un lote, que la nube corta
/// en 1 MB descomprimido.
const VENTANA_BROTLI: u32 = 22;

/// Un evento tal como lo produce un punto de medición.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventoTelemetria {
    /// Hora del evento, en ms desde la época (reloj confiable del núcleo).
    pub ocurrido_ms: i64,
    pub tipo: String,
    /// Los campos propios del evento (sin `esquema` ni `plataforma`, que van
    /// en la cabecera de la sesión).
    pub datos: Map<String, Value>,
}

/// Un lote que no se puede leer: el equipo lo descarta, no lo reintenta.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ErrorLote {
    #[error("lote mal formado: {0}")]
    MalFormado(&'static str),
    #[error("el lote descomprimido supera {0} bytes")]
    DemasiadoGrande(usize),
    #[error("el lote no es brotli válido")]
    BrotliInvalido,
}

/// Un bloque en armado: un tipo con un conjunto fijo de claves.
struct Bloque {
    tipo: String,
    cols: Vec<String>,
    filas: Vec<Value>,
    ultimo_ms: i64,
}

/// Arma el lote columnar. `sesion` es la cabecera de la sesión de la app
/// (id, plataforma, versión, commit, ...), que la nube guarda una vez.
pub fn armar(lote_id: &str, sesion: &Value, eventos: &[EventoTelemetria]) -> Value {
    let mut ordenados: Vec<&EventoTelemetria> = eventos.iter().collect();
    // Estable: dos eventos con la misma hora conservan su orden.
    ordenados.sort_by_key(|evento| evento.ocurrido_ms);
    let base_ms = ordenados.first().map_or(0, |evento| evento.ocurrido_ms);

    let mut bloques: Vec<Bloque> = Vec::new();
    let mut indice: HashMap<(String, Vec<String>), usize> = HashMap::new();
    for evento in ordenados {
        // `serde_json::Map` ordena las claves, así que el mismo conjunto de
        // claves da siempre las mismas columnas en el mismo orden.
        let cols: Vec<String> = evento.datos.keys().cloned().collect();
        let posicion = *indice
            .entry((evento.tipo.clone(), cols.clone()))
            .or_insert_with(|| {
                bloques.push(Bloque {
                    tipo: evento.tipo.clone(),
                    cols,
                    filas: Vec::new(),
                    ultimo_ms: base_ms,
                });
                bloques.len() - 1
            });
        let bloque = &mut bloques[posicion];
        let mut fila = Vec::with_capacity(bloque.cols.len() + 1);
        fila.push(json!(evento.ocurrido_ms - bloque.ultimo_ms));
        fila.extend(bloque.cols.iter().map(|col| evento.datos[col].clone()));
        bloque.filas.push(Value::Array(fila));
        bloque.ultimo_ms = evento.ocurrido_ms;
    }

    json!({
        "lote_id": lote_id,
        "esquema": ESQUEMA_LOTE,
        "base_ms": base_ms,
        "sesion": sesion,
        "bloques": bloques
            .into_iter()
            .map(|bloque| json!({ "tipo": bloque.tipo, "cols": bloque.cols, "filas": bloque.filas }))
            .collect::<Vec<_>>(),
    })
}

/// La regla inversa de [`armar`], la misma que aplica la nube: los eventos del
/// lote, ordenados por hora.
pub fn reconstruir(lote: &Value) -> Result<Vec<EventoTelemetria>, ErrorLote> {
    let base_ms = lote["base_ms"]
        .as_i64()
        .ok_or(ErrorLote::MalFormado("base_ms"))?;
    let bloques = lote["bloques"]
        .as_array()
        .ok_or(ErrorLote::MalFormado("bloques"))?;

    let mut eventos = Vec::new();
    for bloque in bloques {
        let tipo = bloque["tipo"]
            .as_str()
            .ok_or(ErrorLote::MalFormado("tipo"))?;
        let cols = bloque["cols"]
            .as_array()
            .ok_or(ErrorLote::MalFormado("cols"))?
            .iter()
            .map(|col| col.as_str().ok_or(ErrorLote::MalFormado("columna")))
            .collect::<Result<Vec<_>, _>>()?;
        let filas = bloque["filas"]
            .as_array()
            .ok_or(ErrorLote::MalFormado("filas"))?;

        let mut ultimo_ms = base_ms;
        for fila in filas {
            let fila = fila.as_array().ok_or(ErrorLote::MalFormado("fila"))?;
            if fila.len() != cols.len() + 1 {
                return Err(ErrorLote::MalFormado("largo de fila"));
            }
            let dt = fila[0]
                .as_i64()
                .filter(|dt| *dt >= 0)
                .ok_or(ErrorLote::MalFormado("diferencia de hora"))?;
            ultimo_ms += dt;
            let datos = cols
                .iter()
                .zip(&fila[1..])
                .map(|(col, valor)| ((*col).to_owned(), valor.clone()))
                .collect();
            eventos.push(EventoTelemetria {
                ocurrido_ms: ultimo_ms,
                tipo: tipo.to_owned(),
                datos,
            });
        }
    }
    eventos.sort_by_key(|evento| evento.ocurrido_ms);
    Ok(eventos)
}

/// El lote serializado y comprimido, listo para el cuerpo de la petición
/// (`Content-Encoding: br`).
pub fn comprimir(lote: &Value) -> Vec<u8> {
    let json = serde_json::to_vec(lote).unwrap_or_default();
    let mut salida = Vec::with_capacity(json.len() / 8 + 64);
    {
        let mut compresor =
            brotli::CompressorWriter::new(&mut salida, 4096, CALIDAD_BROTLI, VENTANA_BROTLI);
        // Escribir en un `Vec` en memoria no falla.
        let _ = compresor.write_all(&json);
    }
    salida
}

/// Descomprime sin pasar de `tope` bytes: un cuerpo que lo pase se rechaza
/// entero en vez de llenar la memoria (bomba de compresión). Es lo que hace el
/// receptor; acá sirve para las pruebas de ida y vuelta.
pub fn descomprimir(comprimido: &[u8], tope: usize) -> Result<Vec<u8>, ErrorLote> {
    let mut salida = Vec::new();
    let leidos = brotli::Decompressor::new(comprimido, 4096)
        .take(tope as u64 + 1)
        .read_to_end(&mut salida)
        .map_err(|_| ErrorLote::BrotliInvalido)?;
    if leidos > tope {
        return Err(ErrorLote::DemasiadoGrande(tope));
    }
    Ok(salida)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evento(ocurrido_ms: i64, tipo: &str, datos: Value) -> EventoTelemetria {
        EventoTelemetria {
            ocurrido_ms,
            tipo: tipo.to_owned(),
            datos: datos.as_object().cloned().unwrap(),
        }
    }

    fn sesion() -> Value {
        json!({ "id": "dddddddd-0000-0000-0000-000000000001", "plataforma": "movil",
                "version_app": "1.8.14", "version_codigo": 59, "commit": "abc1234" })
    }

    /// Un lote parecido a uno real del móvil: llamadas del núcleo, frames y
    /// reloj intercalados, con valores que cambian.
    fn eventos_realistas(cantidad: i64) -> Vec<EventoTelemetria> {
        let base = 1_793_470_000_000;
        (0..cantidad)
            .map(|i| match i % 4 {
                0 | 1 => evento(base + i * 1_500, "llamada_nucleo", json!({
                    "nombre": if i % 8 == 0 { "registrar_ingreso" } else { "listar_ingresos_activos" },
                    "llamadas": 1 + i % 13, "errores": i % 5 / 4, "p50_ms": 3.2 + f64::from(u32::try_from(i % 7).unwrap()),
                    "p90_ms": 7 + i % 11, "max_ms": 12 + i % 17, "total_ms": 40 + i % 23,
                    "red_tipo": if i % 3 == 0 { "wifi" } else { "datos" }, "errores_por_tipo": {}
                })),
                2 => evento(base + i * 1_500 + 7, "frames", json!({
                    "pantalla": "activos", "frames": 290 + i % 30, "p50_ms": 8.1, "p90_ms": 12 + i % 4,
                    "trabados": i % 3, "congelados": 0
                })),
                _ => evento(base + i * 1_500 + 11, "reloj", json!({
                    "confiable": true, "margen_ms": 40 + i % 9, "diferencia_equipo_ms": -120 + i % 5
                })),
            })
            .collect()
    }

    #[test]
    fn ida_y_vuelta_sin_perder_nada() {
        let mut originales = eventos_realistas(200);
        let lote = armar(
            "eeeeeeee-0000-0000-0000-000000000001",
            &sesion(),
            &originales,
        );
        let reconstruidos = reconstruir(&lote).unwrap();
        originales.sort_by_key(|evento| evento.ocurrido_ms);
        assert_eq!(reconstruidos, originales);
    }

    #[test]
    fn null_y_ausente_no_se_confunden() {
        let originales = vec![
            evento(
                1_000,
                "muestra_sistema",
                json!({ "carga_sistema_1m": null, "hilos": 40 }),
            ),
            evento(2_000, "muestra_sistema", json!({ "hilos": 41 })),
        ];
        let lote = armar("l", &sesion(), &originales);
        assert_eq!(
            lote["bloques"].as_array().unwrap().len(),
            2,
            "un bloque por conjunto de claves"
        );
        let reconstruidos = reconstruir(&lote).unwrap();
        assert!(reconstruidos[0].datos.contains_key("carga_sistema_1m"));
        assert_eq!(reconstruidos[0].datos["carga_sistema_1m"], Value::Null);
        assert!(!reconstruidos[1].datos.contains_key("carga_sistema_1m"));
    }

    #[test]
    fn las_horas_van_en_diferencias_no_negativas_desde_el_primer_evento() {
        // Desordenados a propósito: el lote los ordena.
        let originales = vec![
            evento(5_000, "reloj", json!({ "confiable": true })),
            evento(1_000, "reloj", json!({ "confiable": false })),
            evento(5_000, "frames", json!({ "frames": 1 })),
        ];
        let lote = armar("l", &sesion(), &originales);
        assert_eq!(lote["base_ms"], 1_000);
        let reloj = &lote["bloques"][0];
        assert_eq!(reloj["tipo"], "reloj");
        assert_eq!(reloj["filas"], json!([[0, false], [4_000, true]]));
        assert_eq!(lote["bloques"][1]["filas"], json!([[4_000, 1]]));
    }

    #[test]
    fn un_lote_vacio_es_valido() {
        let lote = armar("l", &sesion(), &[]);
        assert_eq!(lote["bloques"], json!([]));
        assert_eq!(reconstruir(&lote).unwrap(), Vec::new());
    }

    #[test]
    fn lleva_cabecera_y_esquema() {
        let lote = armar("eeeeeeee-0000-0000-0000-000000000009", &sesion(), &[]);
        assert_eq!(lote["lote_id"], "eeeeeeee-0000-0000-0000-000000000009");
        assert_eq!(lote["esquema"], ESQUEMA_LOTE);
        assert_eq!(lote["sesion"], sesion());
    }

    #[test]
    fn brotli_ida_y_vuelta_y_mucho_mas_chico_que_json_por_fila() {
        let originales = eventos_realistas(200);
        let lote = armar("l", &sesion(), &originales);
        let comprimido = comprimir(&lote);
        let descomprimido = descomprimir(&comprimido, 1 << 20).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&descomprimido).unwrap(),
            lote
        );

        // El formato de hoy: una fila JSON por evento, con todo repetido.
        let por_fila: Vec<Value> = originales
            .iter()
            .map(|e| json!({ "ocurrido_ms": e.ocurrido_ms, "tipo": e.tipo, "sesion": sesion(), "datos": e.datos }))
            .collect();
        let crudo = serde_json::to_vec(&por_fila).unwrap().len();
        assert!(
            comprimido.len() * 10 < crudo,
            "columnar + brotli ({} B) debería ser al menos 10 veces menor que JSON por fila ({crudo} B)",
            comprimido.len()
        );
    }

    #[test]
    fn descomprimir_corta_en_el_tope() {
        // 1 MB de ceros comprime a unos pocos bytes: una bomba en miniatura.
        let ceros = Value::String("0".repeat(1 << 20));
        let comprimido = comprimir(&ceros);
        assert!(comprimido.len() < 1_000);
        assert_eq!(
            descomprimir(&comprimido, 64 * 1024),
            Err(ErrorLote::DemasiadoGrande(64 * 1024))
        );
        assert_eq!(
            descomprimir(b"esto no es brotli", 1024),
            Err(ErrorLote::BrotliInvalido)
        );
    }

    #[test]
    fn reconstruir_rechaza_lotes_mal_formados() {
        let casos = [
            json!({ "bloques": [] }),
            json!({ "base_ms": 0, "bloques": [{ "tipo": "frames", "cols": ["a", "b"], "filas": [[0, 1]] }] }),
            json!({ "base_ms": 0, "bloques": [{ "tipo": "frames", "cols": ["a"], "filas": [[-5, 1]] }] }),
            json!({ "base_ms": 0, "bloques": [{ "tipo": "frames", "cols": ["a"], "filas": [[1.5, 1]] }] }),
        ];
        for caso in casos {
            assert!(reconstruir(&caso).is_err(), "debió rechazar {caso}");
        }
    }

    /// El catálogo de la nube (la migración) y el del núcleo tienen que ser el
    /// mismo: un evento que falte en la nube se descartaría en silencio.
    #[test]
    fn el_catalogo_de_la_nube_es_el_del_nucleo() {
        let migracion =
            include_str!("../../supabase/migrations/20261101120000_observabilidad_telemetria.sql");
        let mut en_la_nube = 0;
        for (tipo, grupo) in super::super::CATALOGO {
            let fila = format!("('{tipo}', '{}')", grupo.nombre());
            assert!(
                migracion.contains(&fila),
                "falta {fila} en el catálogo de la nube"
            );
            en_la_nube += 1;
        }
        let filas_en_migracion = migracion
            .lines()
            .filter(|linea| {
                let linea = linea.trim_start();
                linea.starts_with("('")
                    && super::super::CATALOGO
                        .iter()
                        .any(|(tipo, _)| linea.starts_with(&format!("('{tipo}'")))
            })
            .count();
        assert_eq!(filas_en_migracion, en_la_nube);
    }
}

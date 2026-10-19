//! Avisos push de la administración y su bandeja (la campanita de la app).
//!
//! Todas las reglas viven acá; Kotlin sólo recibe el mensaje de Firebase,
//! guarda en disco el texto que devuelve [`BandejaAvisos::serializar`] y
//! dibuja [`EstadoBandeja`].
//!
//! La bandeja vive sólo en el teléfono (decisión 2026-10-18): se llena con
//! cada push que llega. No usa la base `SQLite` del [`crate::Nucleo`] a
//! propósito: el push llega también con la app cerrada, cuando la base
//! (cifrada, con la clave en el Keystore) todavía no está abierta. Un aviso
//! que FCM no llegó a entregar no aparece; eso se ve en el historial del
//! panel web (`avisos_enviados.no_recibieron`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Datelike, Utc};
use control_acceso::tiempo::{a_costa_rica, fecha_costa_rica, parsear_utc};
use serde::{Deserialize, Serialize};

/// Lo que se conserva: 30 días y, aunque lleguen muchos, los últimos 100.
pub const DIAS_EN_BANDEJA: i64 = 30;
pub const MAXIMO_EN_BANDEJA: usize = 100;
const MS_POR_DIA: i64 = 24 * 60 * 60 * 1000;
/// Título cuando el mensaje no trae ninguno.
const TITULO_POR_DEFECTO: &str = "Lattis";

/// Un aviso recién llegado por Firebase Cloud Messaging.
///
/// `emergente`: va por el canal "Emergencias" (sonido de alarma, vibración
/// larga) en vez de "Avisos". `id` y `enviado_en_ms` los manda el servidor
/// desde que existe el historial `avisos_enviados` (ver
/// `supabase/functions/_shared/fcm.ts`); `None` en un mensaje de prueba de la
/// consola de Firebase o de un servidor viejo.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AvisoPush {
    pub titulo: String,
    pub cuerpo: String,
    pub emergente: bool,
    pub id: Option<String>,
    pub enviado_en_ms: Option<i64>,
}

/// Arma el aviso a partir de un mensaje de FCM.
///
/// El servidor manda mensajes de **datos** (`titulo`, `cuerpo`, `tipo`, `id`,
/// `enviado_en`): con prioridad alta llegan aunque la app esté cerrada y la
/// notificación la arma la app. Un mensaje con bloque `notification` (una
/// prueba desde la consola de Firebase) también se acepta: su título y
/// cuerpo son el respaldo.
///
/// `None` si no hay nada que mostrar (mensaje silencioso, p. ej. sólo para
/// despertar una sincronización).
// UniFFI exporta el `HashMap` con el hasher por defecto: no admite otro.
#[allow(clippy::implicit_hasher)]
#[uniffi::export]
pub fn aviso_de_mensaje(
    datos: HashMap<String, String>,
    titulo_notificacion: Option<String>,
    cuerpo_notificacion: Option<String>,
) -> Option<AvisoPush> {
    let con_texto =
        |valor: Option<&String>| valor.filter(|texto| !texto.trim().is_empty()).cloned();
    let titulo = con_texto(datos.get("titulo")).or_else(|| con_texto(titulo_notificacion.as_ref()));
    let cuerpo = con_texto(datos.get("cuerpo")).or_else(|| con_texto(cuerpo_notificacion.as_ref()));
    if titulo.is_none() && cuerpo.is_none() {
        return None;
    }
    Some(AvisoPush {
        titulo: titulo.unwrap_or_else(|| TITULO_POR_DEFECTO.to_owned()),
        cuerpo: cuerpo.unwrap_or_default(),
        emergente: datos.get("tipo").is_some_and(|tipo| tipo == "emergente"),
        id: con_texto(datos.get("id")),
        enviado_en_ms: datos
            .get("enviado_en")
            .and_then(|iso| parsear_utc(iso).ok())
            .map(|instante| instante.timestamp_millis()),
    })
}

/// Grupo por día, en la hora de Costa Rica. En el orden en que se muestran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, uniffi::Enum)]
pub enum GrupoBandeja {
    Hoy,
    Ayer,
    Anteriores,
}

/// Un aviso de la bandeja, listo para dibujar.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AvisoBandeja {
    pub id: String,
    pub titulo: String,
    pub cuerpo: String,
    pub emergente: bool,
    pub leido: bool,
    /// "Ahora", "hace 5 min", "14:32", "17/10 14:32" o "17/10/2025 14:32".
    pub momento: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GrupoAvisos {
    pub grupo: GrupoBandeja,
    /// Más nuevo primero.
    pub avisos: Vec<AvisoBandeja>,
}

/// Todo lo que la campanita y la pantalla de avisos necesitan.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EstadoBandeja {
    /// Hoy / Ayer / Anteriores, sin grupos vacíos.
    pub grupos: Vec<GrupoAvisos>,
    pub no_leidos: u32,
    /// El globito de la campanita: `None` en 0 (un contador siempre prendido
    /// deja de mirarse) y "9+" de 10 en adelante (el botón es chico).
    pub contador: Option<String>,
    /// La emergencia sin leer más reciente: mientras exista, la pantalla
    /// principal muestra la franja roja y la campanita cambia de ícono.
    pub emergencia_sin_leer: Option<AvisoBandeja>,
}

/// Lo que se guarda en disco por cada aviso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AvisoGuardado {
    id: String,
    titulo: String,
    cuerpo: String,
    emergente: bool,
    enviado_en_ms: i64,
    leido: bool,
}

/// Formato en disco, con versión por si algún día cambia.
#[derive(Serialize, Deserialize)]
struct Guardado {
    version: u32,
    avisos: Vec<AvisoGuardado>,
}
const VERSION_GUARDADO: u32 = 1;

/// Para el `id` de un aviso que no lo trae (prueba desde la consola de
/// Firebase): sólo tiene que no chocar con otro del mismo proceso.
static SECUENCIA_LOCAL: AtomicU64 = AtomicU64::new(0);

/// La bandeja de avisos de este teléfono. Kotlin guarda una sola instancia
/// por proceso: la usan el servicio de Firebase y la interfaz a la vez, por
/// eso el `Mutex`.
#[derive(uniffi::Object)]
pub struct BandejaAvisos {
    avisos: Mutex<Vec<AvisoGuardado>>,
}

#[uniffi::export]
impl BandejaAvisos {
    /// Arma la bandeja desde lo que Kotlin guardó con [`Self::serializar`]
    /// (`None` la primera vez), sin lo vencido. Lo que no se entiende
    /// (archivo roto o de otra versión) se descarta: perder la bandeja es
    /// mejor que una app que no abre.
    #[uniffi::constructor]
    pub fn desde_guardado(guardado: Option<String>, ahora_ms: i64) -> Arc<Self> {
        let avisos = guardado
            .and_then(|texto| serde_json::from_str::<Guardado>(&texto).ok())
            .filter(|guardado| guardado.version == VERSION_GUARDADO)
            .map(|guardado| guardado.avisos)
            .unwrap_or_default();
        Arc::new(Self {
            avisos: Mutex::new(depurar(avisos, ahora_ms)),
        })
    }

    /// Agrega un aviso recién llegado. Sin hora de envío usa la de llegada.
    /// FCM puede entregar el mismo mensaje dos veces: si el `id` ya está no
    /// se agrega de nuevo (y no se pierde si ya se había leído). `true` si
    /// la bandeja cambió y hay que guardarla.
    pub fn agregar(&self, aviso: AvisoPush, recibido_en_ms: i64) -> bool {
        let mut avisos = self.bloquear();
        if aviso
            .id
            .as_ref()
            .is_some_and(|id| avisos.iter().any(|guardado| &guardado.id == id))
        {
            return false;
        }
        let id = aviso.id.unwrap_or_else(|| {
            format!(
                "local-{recibido_en_ms}-{}",
                SECUENCIA_LOCAL.fetch_add(1, Ordering::Relaxed)
            )
        });
        let mut nuevos = std::mem::take(&mut *avisos);
        nuevos.push(AvisoGuardado {
            id,
            titulo: aviso.titulo,
            cuerpo: aviso.cuerpo,
            emergente: aviso.emergente,
            enviado_en_ms: aviso.enviado_en_ms.unwrap_or(recibido_en_ms),
            leido: false,
        });
        *avisos = depurar(nuevos, recibido_en_ms);
        true
    }

    /// `true` si cambió algo (el aviso existía y no estaba leído).
    pub fn leer(&self, id: String) -> bool {
        self.bloquear()
            .iter_mut()
            .find(|aviso| aviso.id == id && !aviso.leido)
            .map(|aviso| aviso.leido = true)
            .is_some()
    }

    /// `true` si había alguno sin leer.
    pub fn leer_todos(&self) -> bool {
        let mut cambio = false;
        for aviso in self.bloquear().iter_mut().filter(|aviso| !aviso.leido) {
            aviso.leido = true;
            cambio = true;
        }
        cambio
    }

    /// El estado para dibujar a la hora `ahora_ms`: los grupos por día y el
    /// texto "hace 5 min" dependen de la hora, por eso no se guardan.
    pub fn estado(&self, ahora_ms: i64) -> EstadoBandeja {
        let avisos = depurar(self.bloquear().clone(), ahora_ms);
        let hoy = fecha_de(ahora_ms);
        let ayer = hoy.and_then(|dia| dia.pred_opt());
        let mut grupos: Vec<GrupoAvisos> = Vec::new();
        for aviso in &avisos {
            let dia = fecha_de(aviso.enviado_en_ms);
            // Uno "del futuro" (reloj del teléfono atrasado) va a Hoy.
            let grupo = if aviso.enviado_en_ms > ahora_ms || (dia.is_some() && dia == hoy) {
                GrupoBandeja::Hoy
            } else if dia.is_some() && dia == ayer {
                GrupoBandeja::Ayer
            } else {
                GrupoBandeja::Anteriores
            };
            let fila = para_dibujar(aviso, ahora_ms);
            match grupos.iter_mut().find(|existente| existente.grupo == grupo) {
                Some(existente) => existente.avisos.push(fila),
                None => grupos.push(GrupoAvisos {
                    grupo,
                    avisos: vec![fila],
                }),
            }
        }
        grupos.sort_by_key(|grupo| grupo.grupo);

        let no_leidos =
            u32::try_from(avisos.iter().filter(|aviso| !aviso.leido).count()).unwrap_or(u32::MAX);
        EstadoBandeja {
            grupos,
            no_leidos,
            contador: texto_contador(no_leidos),
            emergencia_sin_leer: avisos
                .iter()
                .find(|aviso| aviso.emergente && !aviso.leido)
                .map(|aviso| para_dibujar(aviso, ahora_ms)),
        }
    }

    /// Texto para que Kotlin lo guarde en disco y se lo devuelva a
    /// [`Self::desde_guardado`] la próxima vez.
    pub fn serializar(&self) -> String {
        let guardado = Guardado {
            version: VERSION_GUARDADO,
            avisos: self.bloquear().clone(),
        };
        serde_json::to_string(&guardado).unwrap_or_default()
    }
}

impl BandejaAvisos {
    /// Un pánico con el candado tomado no deja la bandeja a medias (cada
    /// cambio se arma completo antes de asignarse): se sigue usando.
    fn bloquear(&self) -> MutexGuard<'_, Vec<AvisoGuardado>> {
        self.avisos
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Más nuevo primero, sin los vencidos ni los que pasan del tope.
fn depurar(mut avisos: Vec<AvisoGuardado>, ahora_ms: i64) -> Vec<AvisoGuardado> {
    let limite = ahora_ms.saturating_sub(DIAS_EN_BANDEJA * MS_POR_DIA);
    avisos.retain(|aviso| aviso.enviado_en_ms >= limite);
    avisos.sort_by_key(|aviso| std::cmp::Reverse(aviso.enviado_en_ms));
    avisos.truncate(MAXIMO_EN_BANDEJA);
    avisos
}

fn texto_contador(no_leidos: u32) -> Option<String> {
    match no_leidos {
        0 => None,
        1..=9 => Some(no_leidos.to_string()),
        _ => Some("9+".to_owned()),
    }
}

fn para_dibujar(aviso: &AvisoGuardado, ahora_ms: i64) -> AvisoBandeja {
    AvisoBandeja {
        id: aviso.id.clone(),
        titulo: aviso.titulo.clone(),
        cuerpo: aviso.cuerpo.clone(),
        emergente: aviso.emergente,
        leido: aviso.leido,
        momento: texto_momento(aviso.enviado_en_ms, ahora_ms),
    }
}

fn instante(ms: i64) -> Option<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp_millis(ms)
}

fn fecha_de(ms: i64) -> Option<chrono::NaiveDate> {
    instante(ms).map(fecha_costa_rica)
}

/// "Ahora", "hace 5 min" en la primera hora, "14:32" hoy, "17/10 14:32" en
/// el año y "17/10/2025 14:32" antes. 24 h, como el resto de la app.
fn texto_momento(enviado_en_ms: i64, ahora_ms: i64) -> String {
    let minutos = ahora_ms.saturating_sub(enviado_en_ms) / 60_000;
    if minutos < 1 {
        return "Ahora".to_owned();
    }
    if minutos < 60 {
        return format!("hace {minutos} min");
    }
    let (Some(momento), Some(ahora)) = (instante(enviado_en_ms), instante(ahora_ms)) else {
        return String::new();
    };
    let (momento, ahora) = (a_costa_rica(momento), a_costa_rica(ahora));
    if momento.date_naive() == ahora.date_naive() {
        momento.format("%H:%M").to_string()
    } else if momento.year() == ahora.year() {
        momento.format("%d/%m %H:%M").to_string()
    } else {
        momento.format("%d/%m/%Y %H:%M").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use control_acceso::tiempo::ZONA_APLICACION;

    /// `dia`/`mes`/`anio` a las `hora`:`minuto` en Costa Rica, en ms.
    fn ms(anio: i32, mes: u32, dia: u32, hora: u32, minuto: u32) -> i64 {
        ZONA_APLICACION
            .with_ymd_and_hms(anio, mes, dia, hora, minuto, 0)
            .unwrap()
            .timestamp_millis()
    }

    /// 18/10/2026 15:00 en Costa Rica.
    fn ahora() -> i64 {
        ms(2026, 10, 18, 15, 0)
    }

    fn datos(pares: &[(&str, &str)]) -> HashMap<String, String> {
        pares
            .iter()
            .map(|(clave, valor)| ((*clave).to_owned(), (*valor).to_owned()))
            .collect()
    }

    fn push(id: &str, enviado_en_ms: i64, emergente: bool) -> AvisoPush {
        AvisoPush {
            titulo: format!("Título {id}"),
            cuerpo: format!("Cuerpo {id}"),
            emergente,
            id: Some(id.to_owned()),
            enviado_en_ms: Some(enviado_en_ms),
        }
    }

    fn ids(estado: &EstadoBandeja) -> Vec<Vec<String>> {
        estado
            .grupos
            .iter()
            .map(|grupo| grupo.avisos.iter().map(|aviso| aviso.id.clone()).collect())
            .collect()
    }

    // --- aviso_de_mensaje ---

    #[test]
    fn mensaje_completo_del_servidor() {
        let aviso = aviso_de_mensaje(
            datos(&[
                ("titulo", "Evacuación"),
                ("cuerpo", "Salir por el portón norte"),
                ("tipo", "emergente"),
                ("id", "abc-123"),
                ("enviado_en", "2026-10-18T15:00:00.000Z"),
            ]),
            None,
            None,
        );
        assert_eq!(
            aviso,
            Some(AvisoPush {
                titulo: "Evacuación".to_owned(),
                cuerpo: "Salir por el portón norte".to_owned(),
                emergente: true,
                id: Some("abc-123".to_owned()),
                enviado_en_ms: Some(1_792_335_600_000),
            })
        );
    }

    #[test]
    fn hora_con_desfase_como_la_devuelve_postgres() {
        let aviso = aviso_de_mensaje(
            datos(&[("titulo", "T"), ("enviado_en", "2026-10-18T15:00:00+00:00")]),
            None,
            None,
        );
        assert_eq!(
            aviso.and_then(|aviso| aviso.enviado_en_ms),
            Some(1_792_335_600_000)
        );
    }

    #[test]
    fn los_datos_ganan_sobre_el_bloque_notification() {
        let aviso = aviso_de_mensaje(
            datos(&[("titulo", "De datos")]),
            Some("De notification".to_owned()),
            Some("Cuerpo notification".to_owned()),
        )
        .unwrap();
        assert_eq!(
            (aviso.titulo.as_str(), aviso.cuerpo.as_str()),
            ("De datos", "Cuerpo notification")
        );
    }

    #[test]
    fn mensaje_de_prueba_de_la_consola() {
        let aviso = aviso_de_mensaje(
            HashMap::new(),
            Some("Prueba".to_owned()),
            Some("Hola".to_owned()),
        )
        .unwrap();
        assert_eq!(
            (aviso.titulo.as_str(), aviso.cuerpo.as_str()),
            ("Prueba", "Hola")
        );
        assert!(!aviso.emergente);
        assert_eq!((aviso.id, aviso.enviado_en_ms), (None, None));
    }

    #[test]
    fn sin_titulo_usa_el_nombre_de_la_app() {
        let aviso = aviso_de_mensaje(datos(&[("cuerpo", "Sólo cuerpo")]), None, None).unwrap();
        assert_eq!(aviso.titulo, "Lattis");
    }

    #[test]
    fn mensaje_silencioso_no_muestra_nada() {
        assert_eq!(
            aviso_de_mensaje(datos(&[("tipo", "sincronizar")]), None, None),
            None
        );
        assert_eq!(
            aviso_de_mensaje(
                datos(&[("titulo", "  "), ("cuerpo", "")]),
                Some(" ".to_owned()),
                None
            ),
            None
        );
    }

    #[test]
    fn id_y_hora_invalidos_quedan_en_none() {
        let aviso = aviso_de_mensaje(
            datos(&[("titulo", "T"), ("id", " "), ("enviado_en", "ayer")]),
            None,
            None,
        )
        .unwrap();
        assert_eq!((aviso.id, aviso.enviado_en_ms), (None, None));
    }

    #[test]
    fn solo_tipo_emergente_es_emergencia() {
        assert!(
            !aviso_de_mensaje(datos(&[("titulo", "A"), ("tipo", "normal")]), None, None)
                .unwrap()
                .emergente
        );
        assert!(
            !aviso_de_mensaje(datos(&[("titulo", "A")]), None, None)
                .unwrap()
                .emergente
        );
    }

    // --- BandejaAvisos ---

    #[test]
    fn agrega_el_nuevo_primero_y_no_duplica() {
        let bandeja = BandejaAvisos::desde_guardado(None, ahora());
        assert!(bandeja.agregar(push("a", ms(2026, 10, 18, 9, 0), false), ahora()));
        assert!(bandeja.agregar(push("b", ms(2026, 10, 18, 14, 0), false), ahora()));
        assert_eq!(ids(&bandeja.estado(ahora())), vec![vec!["b", "a"]]);

        // FCM lo entrega dos veces: queda uno, y si ya estaba leído sigue leído.
        assert!(bandeja.leer("b".to_owned()));
        assert!(!bandeja.agregar(push("b", ms(2026, 10, 18, 14, 0), false), ahora()));
        let estado = bandeja.estado(ahora());
        assert_eq!(estado.no_leidos, 1);
        assert!(estado.grupos[0].avisos[0].leido);
    }

    #[test]
    fn sin_id_ni_hora_usa_respaldos() {
        let bandeja = BandejaAvisos::desde_guardado(None, ahora());
        let sin_datos = AvisoPush {
            id: None,
            enviado_en_ms: None,
            ..push("x", 0, false)
        };
        assert!(bandeja.agregar(sin_datos.clone(), ahora()));
        assert!(bandeja.agregar(sin_datos, ahora()));
        let estado = bandeja.estado(ahora());
        let avisos = &estado.grupos[0].avisos;
        assert_eq!(
            avisos.len(),
            2,
            "dos pruebas de consola son dos avisos distintos"
        );
        assert_ne!(avisos[0].id, avisos[1].id);
        assert_eq!(avisos[0].momento, "Ahora");
    }

    #[test]
    fn tira_los_vencidos_y_respeta_el_tope() {
        let bandeja = BandejaAvisos::desde_guardado(None, ahora());
        bandeja.agregar(
            push("viejo", ahora() - (DIAS_EN_BANDEJA + 1) * MS_POR_DIA, false),
            ahora(),
        );
        bandeja.agregar(push("nuevo", ahora() - 60_000, false), ahora());
        assert_eq!(ids(&bandeja.estado(ahora())), vec![vec!["nuevo"]]);

        let llena = BandejaAvisos::desde_guardado(None, ahora());
        for n in 1..=i64::try_from(MAXIMO_EN_BANDEJA).unwrap() + 5 {
            llena.agregar(push(&format!("n{n}"), ahora() - n * 1_000, false), ahora());
        }
        let estado = llena.estado(ahora());
        assert_eq!(estado.grupos[0].avisos.len(), MAXIMO_EN_BANDEJA);
        assert_eq!(estado.grupos[0].avisos[0].id, "n1");
    }

    #[test]
    fn un_aviso_vence_aunque_nadie_agregue_otro() {
        let bandeja = BandejaAvisos::desde_guardado(None, ahora());
        bandeja.agregar(push("a", ahora(), false), ahora());
        let dentro_de_31_dias = ahora() + (DIAS_EN_BANDEJA + 1) * MS_POR_DIA;
        assert!(bandeja.estado(dentro_de_31_dias).grupos.is_empty());
    }

    #[test]
    fn leer_y_contador() {
        let bandeja = BandejaAvisos::desde_guardado(None, ahora());
        assert_eq!(bandeja.estado(ahora()).contador, None);
        for n in 0..12 {
            bandeja.agregar(push(&format!("a{n}"), ahora() - n * 60_000, false), ahora());
        }
        assert_eq!(bandeja.estado(ahora()).contador.as_deref(), Some("9+"));
        assert!(bandeja.leer("a0".to_owned()));
        assert!(
            !bandeja.leer("a0".to_owned()),
            "leer dos veces no cambia nada"
        );
        assert!(!bandeja.leer("no-existe".to_owned()));
        assert!(bandeja.leer_todos());
        assert!(!bandeja.leer_todos());
        let estado = bandeja.estado(ahora());
        assert_eq!((estado.no_leidos, estado.contador), (0, None));

        bandeja.agregar(push("b", ahora(), false), ahora());
        assert_eq!(bandeja.estado(ahora()).contador.as_deref(), Some("1"));
    }

    #[test]
    fn emergencia_sin_leer_mas_reciente() {
        let bandeja = BandejaAvisos::desde_guardado(None, ahora());
        bandeja.agregar(push("normal", ms(2026, 10, 18, 14, 0), false), ahora());
        bandeja.agregar(push("vieja", ms(2026, 10, 18, 9, 0), true), ahora());
        bandeja.agregar(push("nueva", ms(2026, 10, 18, 12, 0), true), ahora());
        assert_eq!(
            bandeja
                .estado(ahora())
                .emergencia_sin_leer
                .map(|aviso| aviso.id)
                .as_deref(),
            Some("nueva")
        );
        bandeja.leer("nueva".to_owned());
        assert_eq!(
            bandeja
                .estado(ahora())
                .emergencia_sin_leer
                .map(|aviso| aviso.id)
                .as_deref(),
            Some("vieja")
        );
        bandeja.leer_todos();
        assert_eq!(bandeja.estado(ahora()).emergencia_sin_leer, None);
    }

    #[test]
    fn agrupa_por_dia_en_costa_rica() {
        let bandeja = BandejaAvisos::desde_guardado(None, ahora());
        bandeja.agregar(
            push("hoy-temprano", ms(2026, 10, 18, 0, 30), false),
            ahora(),
        );
        bandeja.agregar(push("ayer", ms(2026, 10, 17, 23, 59), false), ahora());
        bandeja.agregar(push("hoy-tarde", ms(2026, 10, 18, 14, 0), false), ahora());
        bandeja.agregar(
            push("semana-pasada", ms(2026, 10, 11, 10, 0), false),
            ahora(),
        );
        // Reloj del teléfono atrasado: el aviso "es del futuro".
        bandeja.agregar(push("futuro", ms(2026, 10, 19, 8, 0), false), ahora());

        let estado = bandeja.estado(ahora());
        let grupos: Vec<GrupoBandeja> = estado.grupos.iter().map(|grupo| grupo.grupo).collect();
        assert_eq!(
            grupos,
            vec![
                GrupoBandeja::Hoy,
                GrupoBandeja::Ayer,
                GrupoBandeja::Anteriores
            ]
        );
        assert_eq!(
            ids(&estado),
            vec![
                vec!["futuro", "hoy-tarde", "hoy-temprano"],
                vec!["ayer"],
                vec!["semana-pasada"]
            ]
        );

        assert!(
            BandejaAvisos::desde_guardado(None, ahora())
                .estado(ahora())
                .grupos
                .is_empty()
        );
    }

    #[test]
    fn momento_del_aviso() {
        let ahora = ahora();
        assert_eq!(texto_momento(ahora - 30_000, ahora), "Ahora");
        assert_eq!(texto_momento(ahora + 120_000, ahora), "Ahora");
        assert_eq!(texto_momento(ahora - 5 * 60_000, ahora), "hace 5 min");
        assert_eq!(texto_momento(ahora - 59 * 60_000, ahora), "hace 59 min");
        assert_eq!(texto_momento(ms(2026, 10, 18, 9, 5), ahora), "09:05");
        assert_eq!(
            texto_momento(ms(2026, 10, 17, 23, 59), ahora),
            "17/10 23:59"
        );
        assert_eq!(
            texto_momento(ms(2025, 12, 24, 8, 0), ahora),
            "24/12/2025 08:00"
        );
    }

    #[test]
    fn guarda_y_vuelve_a_leer() {
        let bandeja = BandejaAvisos::desde_guardado(None, ahora());
        bandeja.agregar(
            AvisoPush {
                titulo: "Corte\tde \"agua\"".to_owned(),
                cuerpo: "Línea 1\nLínea 2 \\ruta".to_owned(),
                ..push("a", ahora() - 60_000, true)
            },
            ahora(),
        );
        bandeja.agregar(push("b", ahora() - 120_000, false), ahora());
        bandeja.leer("b".to_owned());

        let leida = BandejaAvisos::desde_guardado(Some(bandeja.serializar()), ahora());
        assert_eq!(leida.estado(ahora()), bandeja.estado(ahora()));
        assert_eq!(leida.serializar(), bandeja.serializar());
    }

    #[test]
    fn al_leer_del_disco_tira_lo_vencido() {
        let bandeja = BandejaAvisos::desde_guardado(None, ahora());
        bandeja.agregar(push("a", ahora(), false), ahora());
        let mas_tarde = ahora() + (DIAS_EN_BANDEJA + 1) * MS_POR_DIA;
        let leida = BandejaAvisos::desde_guardado(Some(bandeja.serializar()), mas_tarde);
        assert_eq!(leida.serializar(), r#"{"version":1,"avisos":[]}"#);
    }

    #[test]
    fn lo_que_no_se_entiende_se_descarta_sin_reventar() {
        for roto in [
            "",
            "no es json",
            r#"{"version":2,"avisos":[]}"#,
            r#"{"version":1,"avisos":[{"id":1}]}"#,
        ] {
            let bandeja = BandejaAvisos::desde_guardado(Some(roto.to_owned()), ahora());
            assert!(bandeja.estado(ahora()).grupos.is_empty(), "{roto:?}");
        }
    }
}

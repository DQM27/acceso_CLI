use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::{America::Costa_Rica, Tz};

pub const ZONA_APLICACION_NOMBRE: &str = "America/Costa_Rica";
pub const ZONA_APLICACION: Tz = Costa_Rica;
const FORMATO_UTC: &str = "%Y-%m-%dT%H:%M:%SZ";

pub trait Reloj: Send + Sync {
    fn ahora_utc(&self) -> DateTime<Utc>;

    /// No-op por defecto -- sólo [`RelojCorregido`] hace algo con esto.
    /// Que viva en el trait (en vez de exigir un downcast) permite que
    /// quien recibe un `Arc<dyn Reloj>` cualquiera (`RelojSistema` en
    /// CLI/TUI, `RelojCorregido` en escritorio/móvil) llame esto sin saber
    /// qué implementación hay detrás. Lo llama quien acaba de MEDIR el
    /// desfase contra el servidor (también fija el ancla).
    fn actualizar_desfase_ms(&self, _desfase_ms: i64) {}

    /// Al arrancar: lo último guardado en la base (ver `AppCore::con_reloj`).
    /// A diferencia de [`Self::actualizar_desfase_ms`], no fija un ancla
    /// nueva con el reloj de ahora (que pudo cambiar con la app cerrada):
    /// sólo reusa el ancla guardada si sigue siendo del mismo arranque.
    fn restaurar(&self, _desfase_ms: Option<i64>, _ancla: Option<Ancla>) {}

    /// El ancla vigente, para guardarla. `None` si este reloj no tiene.
    fn ancla(&self) -> Option<Ancla> {
        None
    }

    /// La hora con su margen de error en milisegundos; `None` = no hay una
    /// hora confiable (sin ancla, el reloj del equipo pudo haberse movido).
    fn ahora_con_margen(&self) -> HoraConMargen {
        HoraConMargen {
            instante: self.ahora_utc(),
            margen_ms: None,
        }
    }
}

/// Una hora y cuánto puede estar equivocada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoraConMargen {
    pub instante: DateTime<Utc>,
    /// `None` = hora no confiable.
    pub margen_ms: Option<u64>,
}

/// Punto de referencia del reloj confiable: la hora del servidor en un
/// instante dado del contador de arranque ([`crate::reloj_arranque`]). Desde
/// ahí, la hora es `servidor_ms + (contador_ahora - arranque_ms)`, sin pasar
/// por el reloj del equipo. Mismo esquema que `Kronos` (Lyft) y `TrustedTime`
/// (Google).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ancla {
    /// Hora del servidor (ms desde 1970) al fijar el ancla.
    pub servidor_ms: i64,
    /// Contador de arranque en ese mismo instante.
    pub arranque_ms: u64,
    /// Hora del arranque según el reloj del equipo (`reloj - contador`) al
    /// fijar el ancla. Sólo sirve para reconocer, al reabrir la app, si el
    /// equipo se reinició: entonces el contador volvió a cero y el ancla ya
    /// no vale.
    pub epoca_arranque_ms: i64,
}

#[derive(Debug, Default)]
pub struct RelojSistema;

impl Reloj for RelojSistema {
    fn ahora_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RelojFijo {
    instante: DateTime<Utc>,
}

impl RelojFijo {
    pub fn new(instante: DateTime<Utc>) -> Self {
        Self { instante }
    }
}

impl Reloj for RelojFijo {
    fn ahora_utc(&self) -> DateTime<Utc> {
        self.instante
    }
}

/// Reloj confiable para escritorio y móvil: sella ingresos, salidas,
/// auditoría y sesiones con la hora del SERVIDOR, aunque el reloj del
/// equipo esté mal o alguien lo cambie sin conexión.
///
/// Historia: un reloj adelantado ~11 minutos (equipos sin pila/CMOS o sin
/// permiso para corregir la hora) hacía que la marca de agua del sync
/// incremental perdiera movimientos en silencio; eso se corrigió con el
/// `updated_at` del servidor, y el resto de la app pasó a restar un
/// desfase medido. Pero el desfase se aplica sobre el reloj del equipo: si
/// alguien lo movía estando sin conexión, todos los registros quedaban mal.
///
/// Ahora, con el patrón probado de `Kronos` (Lyft) y `TrustedTime` (Google):
/// cada medición contra el servidor fija un [`Ancla`] (hora del servidor +
/// contador de arranque), y la hora sale de `ancla + tiempo transcurrido en
/// el contador`, que nadie puede mover. Sin ancla válida (equipo reiniciado
/// sin red) se vuelve al desfase sobre el reloj del equipo, y
/// [`Reloj::ahora_con_margen`] lo informa como hora no confiable.
pub struct RelojCorregido {
    desfase_ms: std::sync::atomic::AtomicI64,
    ancla: std::sync::Mutex<Option<Ancla>>,
    /// Inyectable para las pruebas; en uso real es
    /// [`crate::reloj_arranque::ms_desde_arranque`].
    contador: fn() -> u64,
}

/// Error fijo del ancla: la medición contra el servidor (header `Date`,
/// al segundo, o la medición fina con la mitad del viaje de ida y vuelta).
const MARGEN_ANCLA_MS: u64 = 2_000;

/// Deriva máxima del contador de arranque: 100 ppm (0,01 %), holgado para
/// los cristales de PCs y teléfonos (típicamente 20-50 ppm).
const DERIVA_POR_MS: u64 = 10_000;

/// Diferencia tolerada entre la hora de arranque guardada en el ancla y la
/// de ahora para considerar que es el mismo arranque (correcciones normales
/// del reloj de Windows/Android entre una apertura y otra de la app).
const TOLERANCIA_MISMO_ARRANQUE_MS: i64 = 2 * 60 * 1000;

impl std::fmt::Debug for RelojCorregido {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelojCorregido")
            .field("desfase_ms", &self.desfase_ms)
            .field("ancla", &self.ancla_vigente())
            .finish_non_exhaustive()
    }
}

impl Default for RelojCorregido {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl RelojCorregido {
    pub fn nuevo() -> Self {
        Self::con_contador(crate::reloj_arranque::ms_desde_arranque)
    }

    fn con_contador(contador: fn() -> u64) -> Self {
        Self {
            desfase_ms: std::sync::atomic::AtomicI64::new(0),
            ancla: std::sync::Mutex::new(None),
            contador,
        }
    }

    fn ancla_vigente(&self) -> Option<Ancla> {
        *self
            .ancla
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Hora según el ancla y milisegundos transcurridos desde ella. `None`
    /// sin ancla, o si el contador quedó por detrás de ella (el equipo se
    /// reinició mientras la app seguía abierta: no debería pasar).
    fn segun_ancla(&self) -> Option<(DateTime<Utc>, u64)> {
        let ancla = self.ancla_vigente()?;
        let transcurrido = (self.contador)().checked_sub(ancla.arranque_ms)?;
        let ms = ancla
            .servidor_ms
            .checked_add(i64::try_from(transcurrido).ok()?)?;
        Some((DateTime::from_timestamp_millis(ms)?, transcurrido))
    }

    fn segun_desfase(&self) -> DateTime<Utc> {
        let desfase_ms = self.desfase_ms.load(std::sync::atomic::Ordering::Relaxed);
        Utc::now() - chrono::Duration::milliseconds(desfase_ms)
    }
}

impl Reloj for RelojCorregido {
    fn ahora_utc(&self) -> DateTime<Utc> {
        self.segun_ancla()
            .map_or_else(|| self.segun_desfase(), |(instante, _)| instante)
    }

    fn actualizar_desfase_ms(&self, desfase_ms: i64) {
        self.desfase_ms
            .store(desfase_ms, std::sync::atomic::Ordering::Relaxed);
        let contador = (self.contador)();
        let reloj_equipo = Utc::now().timestamp_millis();
        let ancla = i64::try_from(contador).ok().map(|contador_ms| Ancla {
            servidor_ms: reloj_equipo - desfase_ms,
            arranque_ms: contador,
            epoca_arranque_ms: reloj_equipo - contador_ms,
        });
        *self
            .ancla
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = ancla;
    }

    fn restaurar(&self, desfase_ms: Option<i64>, ancla: Option<Ancla>) {
        if let Some(desfase_ms) = desfase_ms {
            self.desfase_ms
                .store(desfase_ms, std::sync::atomic::Ordering::Relaxed);
        }
        let Some(ancla) = ancla else { return };
        let contador = (self.contador)();
        let mismo_arranque = contador >= ancla.arranque_ms
            && i64::try_from(contador).is_ok_and(|contador_ms| {
                let epoca = Utc::now().timestamp_millis() - contador_ms;
                (epoca - ancla.epoca_arranque_ms).abs() <= TOLERANCIA_MISMO_ARRANQUE_MS
            });
        if mismo_arranque {
            *self
                .ancla
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(ancla);
        } else {
            // Reinicio (o el reloj del equipo se movió mucho con la app
            // cerrada): hasta la próxima medición, hora no confiable.
            log::info!("el ancla de hora guardada no es de este arranque; se descarta");
        }
    }

    fn ancla(&self) -> Option<Ancla> {
        self.ancla_vigente()
    }

    fn ahora_con_margen(&self) -> HoraConMargen {
        match self.segun_ancla() {
            Some((instante, transcurrido)) => HoraConMargen {
                instante,
                margen_ms: Some(MARGEN_ANCLA_MS + transcurrido / DERIVA_POR_MS),
            },
            None => HoraConMargen {
                instante: self.segun_desfase(),
                margen_ms: None,
            },
        }
    }
}

pub fn ahora_costa_rica() -> DateTime<Tz> {
    Utc::now().with_timezone(&ZONA_APLICACION)
}

pub fn fecha_costa_rica(instante: DateTime<Utc>) -> NaiveDate {
    instante.with_timezone(&ZONA_APLICACION).date_naive()
}

pub fn a_costa_rica(instante: DateTime<Utc>) -> DateTime<Tz> {
    instante.with_timezone(&ZONA_APLICACION)
}

pub fn local_costa_rica_a_utc(fecha_hora: NaiveDateTime) -> Result<DateTime<Utc>, TiempoError> {
    match ZONA_APLICACION.from_local_datetime(&fecha_hora) {
        LocalResult::Single(instante) => Ok(instante.with_timezone(&Utc)),
        LocalResult::Ambiguous(_, _) => Err(TiempoError::HoraLocalAmbigua),
        LocalResult::None => Err(TiempoError::HoraLocalInexistente),
    }
}

pub fn inicio_dia_costa_rica_utc(fecha: NaiveDate) -> Result<DateTime<Utc>, TiempoError> {
    let inicio = fecha
        .and_hms_opt(0, 0, 0)
        .ok_or(TiempoError::HoraLocalInexistente)?;
    local_costa_rica_a_utc(inicio)
}

pub fn serializar_utc(instante: DateTime<Utc>) -> String {
    instante.format(FORMATO_UTC).to_string()
}

/// Marca de agua de una sincronización incremental (`updated_at=gt.<marca>`),
/// con microsegundos -- la misma precisión que `timestamptz` en Postgres.
/// Con [`serializar_utc`] (al segundo) la marca quedaba por DEBAJO del
/// `updated_at` real de la última fila recibida, y `gt.` volvía a traer
/// todas las filas de ese segundo en cada sincronización, para siempre:
/// visto en producción con los 1.438 encargados de ruta, cargados todos en
/// el mismo segundo y re-descargados (3 páginas) en cada pulso y cada aviso.
pub fn serializar_marca_utc(instante: DateTime<Utc>) -> String {
    instante.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

pub fn parsear_utc(valor: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(valor).map(|instante| instante.with_timezone(&Utc))
}

pub fn hora_actual_texto() -> String {
    ahora_costa_rica().format("%H:%M").to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TiempoError {
    #[error("La hora local es ambigua en la zona {ZONA_APLICACION_NOMBRE}")]
    HoraLocalAmbigua,
    #[error("La hora local no existe en la zona {ZONA_APLICACION_NOMBRE}")]
    HoraLocalInexistente,
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Timelike, Utc};

    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{
        Ancla, MARGEN_ANCLA_MS, Reloj, RelojCorregido, a_costa_rica, fecha_costa_rica,
        inicio_dia_costa_rica_utc, local_costa_rica_a_utc, parsear_utc, serializar_marca_utc,
        serializar_utc,
    };

    #[test]
    fn la_marca_de_agua_conserva_los_microsegundos_de_postgres() {
        // `updated_at` tal cual lo devuelve PostgREST para los encargados de
        // ruta cargados en lote (todos en el mismo segundo).
        let ultima = parsear_utc("2026-09-15T19:18:26.954576+00:00").unwrap();

        let marca = serializar_marca_utc(ultima);

        assert_eq!(marca, "2026-09-15T19:18:26.954576Z");
        assert_eq!(
            parsear_utc(&marca).unwrap(),
            ultima,
            "ida y vuelta sin pérdida"
        );
        // Al segundo (el formato viejo) la marca queda por debajo de la fila,
        // y `updated_at=gt.<marca>` la vuelve a traer para siempre.
        assert!(parsear_utc(&serializar_utc(ultima)).unwrap() < ultima);
    }

    #[test]
    fn reloj_corregido_sin_desfase_medido_todavia_se_comporta_como_el_sistema() {
        let reloj = RelojCorregido::nuevo();
        let diferencia = (reloj.ahora_utc() - Utc::now()).num_milliseconds().abs();
        assert!(diferencia < 1000, "sin desfase aplicado debería ser ~ahora");
    }

    #[test]
    fn reloj_corregido_resta_el_desfase_medido() {
        let reloj = RelojCorregido::nuevo();
        // Reloj local "adelantado" 5 minutos respecto al confiable -- mismo
        // caso reproducido en producción (ver doc-comment del tipo).
        reloj.actualizar_desfase_ms(5 * 60 * 1000);
        let diferencia = (Utc::now() - reloj.ahora_utc()).num_milliseconds();
        assert!(
            (299_000..301_000).contains(&diferencia),
            "debería quedar ~5 minutos detrás del reloj del sistema: {diferencia}ms"
        );
    }

    #[test]
    fn sin_ancla_la_hora_no_es_confiable() {
        let reloj = RelojCorregido::nuevo();
        reloj.restaurar(Some(60_000), None);
        let hora = reloj.ahora_con_margen();
        assert_eq!(hora.margen_ms, None);
        let diferencia = (Utc::now() - hora.instante).num_milliseconds();
        assert!((59_000..61_000).contains(&diferencia), "{diferencia}ms");
    }

    #[test]
    fn con_ancla_la_hora_avanza_con_el_contador_y_no_con_el_reloj_del_equipo() {
        static CONTADOR: AtomicU64 = AtomicU64::new(10_000_000);
        fn contador() -> u64 {
            CONTADOR.load(Ordering::Relaxed)
        }
        let reloj = RelojCorregido::con_contador(contador);
        // Medición: el equipo va 5 minutos adelantado respecto al servidor.
        reloj.actualizar_desfase_ms(5 * 60 * 1000);
        let anclada = reloj.ahora_utc();

        // Pasa una hora según el contador (el reloj del equipo, en esta
        // prueba, apenas se movió: es como si alguien lo hubiera atrasado).
        CONTADOR.fetch_add(60 * 60 * 1000, Ordering::Relaxed);
        let hora = reloj.ahora_con_margen();

        let avance = (hora.instante - anclada).num_milliseconds();
        assert!(
            (3_599_900..3_600_100).contains(&avance),
            "avanzó {avance}ms"
        );
        assert_eq!(hora.margen_ms, Some(MARGEN_ANCLA_MS + 360));
    }

    #[test]
    fn un_ancla_de_otro_arranque_se_descarta_y_la_del_mismo_se_reusa() {
        static CONTADOR: AtomicU64 = AtomicU64::new(5_000);
        fn contador() -> u64 {
            CONTADOR.load(Ordering::Relaxed)
        }
        let ahora = Utc::now().timestamp_millis();
        // Guardada cuando el contador iba por 9.000: el equipo se reinició.
        let de_otro_arranque = Ancla {
            servidor_ms: ahora,
            arranque_ms: 9_000,
            epoca_arranque_ms: ahora - 9_000,
        };
        let reloj = RelojCorregido::con_contador(contador);
        reloj.restaurar(None, Some(de_otro_arranque));
        assert_eq!(reloj.ancla(), None);
        assert_eq!(reloj.ahora_con_margen().margen_ms, None);

        // Guardada hace 4 segundos en este mismo arranque.
        let del_mismo = Ancla {
            servidor_ms: ahora - 4_000,
            arranque_ms: 1_000,
            epoca_arranque_ms: ahora - 5_000,
        };
        reloj.restaurar(None, Some(del_mismo));
        assert_eq!(reloj.ancla(), Some(del_mismo));
        let diferencia = (reloj.ahora_utc().timestamp_millis() - ahora).abs();
        assert!(diferencia < 1_000, "{diferencia}ms");
    }

    #[test]
    fn un_ancla_cuyo_arranque_no_coincide_con_el_reloj_se_descarta() {
        static CONTADOR: AtomicU64 = AtomicU64::new(50_000);
        fn contador() -> u64 {
            CONTADOR.load(Ordering::Relaxed)
        }
        let ahora = Utc::now().timestamp_millis();
        // El contador no retrocedió, pero el arranque calculado difiere en
        // una hora: reinicio con más tiempo encendido, o reloj movido con la
        // app cerrada. En la duda, no se usa.
        let dudosa = Ancla {
            servidor_ms: ahora,
            arranque_ms: 10_000,
            epoca_arranque_ms: ahora - 10_000 - 3_600_000,
        };
        let reloj = RelojCorregido::con_contador(contador);
        reloj.restaurar(None, Some(dudosa));
        assert_eq!(reloj.ancla(), None);
    }

    #[test]
    fn costa_rica_define_hoy_sin_depender_de_la_zona_del_sistema() {
        let instante = Utc.with_ymd_and_hms(2026, 8, 16, 3, 30, 0).unwrap();
        assert_eq!(
            fecha_costa_rica(instante),
            NaiveDate::from_ymd_opt(2026, 8, 15).unwrap()
        );
        assert_eq!(a_costa_rica(instante).hour(), 21);
    }

    #[test]
    fn medianoche_de_costa_rica_se_convierte_a_utc() {
        let fecha = NaiveDate::from_ymd_opt(2026, 8, 15).unwrap();
        assert_eq!(
            inicio_dia_costa_rica_utc(fecha),
            Ok(Utc.with_ymd_and_hms(2026, 8, 15, 6, 0, 0).unwrap())
        );
        assert_eq!(
            local_costa_rica_a_utc(fecha.and_hms_opt(21, 30, 0).unwrap()).unwrap(),
            Utc.with_ymd_and_hms(2026, 8, 16, 3, 30, 0).unwrap()
        );
    }

    #[test]
    fn persistencia_utc_tiene_un_formato_unico_y_reversible() {
        let instante = Utc.with_ymd_and_hms(2026, 8, 16, 3, 30, 45).unwrap();
        let texto = serializar_utc(instante);
        assert_eq!(texto, "2026-08-16T03:30:45Z");
        assert_eq!(parsear_utc(&texto), Ok(instante));
    }
}

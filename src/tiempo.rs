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
    /// qué implementación hay detrás.
    fn actualizar_desfase_ms(&self, _desfase_ms: i64) {}
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

/// Reloj del sistema corregido por un desfase medido contra una hora
/// confiable externa (el header `Date` de cualquier respuesta HTTPS del
/// receptor en la nube, ver `nube::cliente::autenticar_dispositivo`) --
/// pensado para equipos cuyo reloj de Windows no se puede corregir
/// (permisos, política corporativa, hardware sin pila/CMOS confiable).
/// Reproducido en producción: un reloj adelantado ~11 minutos sin
/// sincronizar hacía que la marca de agua del sync incremental
/// (`nube::sincronizacion`) se calculara mal y perdiera movimientos/altas
/// en silencio -- eso ya se corrigió calculando esa marca a partir del
/// `updated_at` real del servidor, pero el resto de la app (horas de
/// entrada/salida, auditoría) seguía dependiendo del reloj de Windows tal
/// cual. `AtomicI64` en vez de un `Mutex`: `ahora_utc()` se llama en cada
/// registro de ingreso/salida, no vale la pena bloquear por una lectura.
#[derive(Debug, Default)]
pub struct RelojCorregido {
    desfase_ms: std::sync::atomic::AtomicI64,
}

impl RelojCorregido {
    pub fn nuevo() -> Self {
        Self::default()
    }
}

impl Reloj for RelojCorregido {
    fn ahora_utc(&self) -> DateTime<Utc> {
        let desfase_ms = self.desfase_ms.load(std::sync::atomic::Ordering::Relaxed);
        Utc::now() - chrono::Duration::milliseconds(desfase_ms)
    }

    fn actualizar_desfase_ms(&self, desfase_ms: i64) {
        self.desfase_ms
            .store(desfase_ms, std::sync::atomic::Ordering::Relaxed);
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

    use super::{
        Reloj, RelojCorregido, a_costa_rica, fecha_costa_rica, inicio_dia_costa_rica_utc,
        local_costa_rica_a_utc, parsear_utc, serializar_utc,
    };

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

use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::{America::Costa_Rica, Tz};

pub const ZONA_APLICACION_NOMBRE: &str = "America/Costa_Rica";
pub const ZONA_APLICACION: Tz = Costa_Rica;
const FORMATO_UTC: &str = "%Y-%m-%dT%H:%M:%SZ";

pub trait Reloj: Send + Sync {
    fn ahora_utc(&self) -> DateTime<Utc>;
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
    ahora_costa_rica().format("%H:%M:%S").to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TiempoError {
    HoraLocalAmbigua,
    HoraLocalInexistente,
}

impl std::fmt::Display for TiempoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HoraLocalAmbigua => write!(
                formatter,
                "La hora local es ambigua en la zona {ZONA_APLICACION_NOMBRE}"
            ),
            Self::HoraLocalInexistente => write!(
                formatter,
                "La hora local no existe en la zona {ZONA_APLICACION_NOMBRE}"
            ),
        }
    }
}

impl std::error::Error for TiempoError {}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Timelike, Utc};

    use super::{
        a_costa_rica, fecha_costa_rica, inicio_dia_costa_rica_utc, local_costa_rica_a_utc,
        parsear_utc, serializar_utc,
    };

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

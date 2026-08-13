use crate::{
    database::queries::ingresos::{EstadoMovimiento, FiltroHistorial},
    models::{empresa::Empresa, tipo_ingreso::TipoIngreso},
};
use chrono::{Datelike, Duration, Local, NaiveDate};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltrosHistorial {
    pub desde: String,
    pub hasta: String,
    pub nombre_cedula: String,
    pub empresa_id: Option<i64>,
    pub tipo: Option<TipoIngreso>,
    pub gafete: String,
    pub estado: EstadoMovimiento,
}
impl Default for FiltrosHistorial {
    fn default() -> Self {
        let h = Local::now().date_naive();
        let d = NaiveDate::from_ymd_opt(h.year(), h.month(), 1).unwrap();
        Self {
            desde: d.format("%d/%m/%Y").to_string(),
            hasta: h.format("%d/%m/%Y").to_string(),
            nombre_cedula: String::new(),
            empresa_id: None,
            tipo: None,
            gafete: String::new(),
            estado: EstadoMovimiento::Todos,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CampoFiltro {
    Desde,
    Hasta,
    NombreCedula,
    Empresa,
    Tipo,
    Gafete,
    Estado,
}
impl CampoFiltro {
    pub(super) const TODOS: [Self; 7] = [
        Self::Desde,
        Self::Hasta,
        Self::NombreCedula,
        Self::Empresa,
        Self::Tipo,
        Self::Gafete,
        Self::Estado,
    ];
}
pub(super) fn fecha(v: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(v, "%d/%m/%Y").ok()
}
pub(super) fn construir(
    f: &FiltrosHistorial,
    busqueda: &str,
    limit: usize,
    offset: usize,
) -> Result<FiltroHistorial, String> {
    let d = fecha(&f.desde).ok_or("Fecha Desde inválida. Use DD/MM/YYYY")?;
    let visual = fecha(&f.hasta).ok_or("Fecha Hasta inválida. Use DD/MM/YYYY")?;
    let hasta = visual
        .checked_add_signed(Duration::days(1))
        .ok_or("El rango de fechas no es válido")?;
    let desde = d.and_hms_opt(0, 0, 0).unwrap();
    let hasta = hasta.and_hms_opt(0, 0, 0).unwrap();
    if desde >= hasta {
        return Err("El rango de fechas no es válido".into());
    }
    let gafete = if f.gafete.trim().is_empty() {
        None
    } else {
        Some(
            f.gafete
                .trim()
                .parse::<i64>()
                .map_err(|_| "Ingrese un número de gafete válido")?,
        )
    };
    let texto = if busqueda.trim().is_empty() {
        f.nombre_cedula.trim()
    } else {
        busqueda.trim()
    };
    Ok(FiltroHistorial {
        desde,
        hasta,
        texto_persona: (!texto.is_empty()).then(|| texto.to_owned()),
        empresa_id: f.empresa_id,
        tipo_ingreso: f.tipo,
        gafete_numero: gafete,
        estado: f.estado,
        limite: limit,
        offset,
    })
}
pub(super) fn estado_texto(e: EstadoMovimiento) -> &'static str {
    match e {
        EstadoMovimiento::Todos => "Todos",
        EstadoMovimiento::Activos => "Activos",
        EstadoMovimiento::Cerrados => "Cerrados",
    }
}
pub(super) fn empresa_texto(id: Option<i64>, empresas: &[Empresa]) -> String {
    id.and_then(|x| empresas.iter().find(|e| e.id == x))
        .map_or("Todas".into(), |e| e.nombre.clone())
}
pub(super) fn tipo_texto(t: Option<TipoIngreso>) -> &'static str {
    match t {
        None => "Todos",
        Some(TipoIngreso::Praind) => "PRAIND",
        Some(TipoIngreso::InHouse) => "IN HOUSE",
        Some(TipoIngreso::PorCorreo) => "POR CORREO",
        Some(TipoIngreso::Swat) => "SWAT",
    }
}

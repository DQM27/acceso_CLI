use crate::{
    database::queries::ingresos::{EstadoMovimiento, FiltroHistorial},
    models::{empresa::Empresa, tipo_ingreso::TipoIngreso},
    tiempo::{ahora_costa_rica, inicio_dia_costa_rica_utc},
};
use chrono::{Datelike, Duration, NaiveDate};
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
        let h = ahora_costa_rica().date_naive();
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
pub(super) fn fecha(v: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(v, "%d/%m/%Y").ok()
}
pub(super) fn construir(
    f: &FiltrosHistorial,
    busqueda: &str,
    limit: usize,
    offset: usize,
    corte_id: Option<i64>,
) -> Result<FiltroHistorial, String> {
    let d = fecha(&f.desde).ok_or("Fecha Desde inválida. Use DD/MM/YYYY")?;
    let visual = fecha(&f.hasta).ok_or("Fecha Hasta inválida. Use DD/MM/YYYY")?;
    let hasta = visual
        .checked_add_signed(Duration::days(1))
        .ok_or("El rango de fechas no es válido")?;
    let desde = inicio_dia_costa_rica_utc(d)
        .map_err(|_| "La fecha Desde no existe en la zona de Costa Rica")?;
    let hasta = inicio_dia_costa_rica_utc(hasta)
        .map_err(|_| "La fecha Hasta no existe en la zona de Costa Rica")?;
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
        corte_id,
    })
}
/// Separa la consulta en tokens respetando comillas, para que
/// `empresa:"Brisas del Oeste"` sea un solo token en vez de partirse en el
/// espacio.
fn tokenizar(consulta: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut actual = String::new();
    let mut entre_comillas = false;
    for c in consulta.chars() {
        match c {
            '"' => entre_comillas = !entre_comillas,
            c if c.is_whitespace() && !entre_comillas => {
                if !actual.is_empty() {
                    tokens.push(std::mem::take(&mut actual));
                }
            }
            c => actual.push(c),
        }
    }
    if !actual.is_empty() {
        tokens.push(actual);
    }
    tokens
}

fn tipo_desde_texto(v: &str) -> Option<TipoIngreso> {
    match v.to_lowercase().as_str() {
        "praind" => Some(TipoIngreso::Praind),
        "inhouse" | "in-house" | "in_house" => Some(TipoIngreso::InHouse),
        "correo" | "porcorreo" => Some(TipoIngreso::PorCorreo),
        "swat" => Some(TipoIngreso::Swat),
        _ => None,
    }
}

fn estado_desde_texto(v: &str) -> Option<EstadoMovimiento> {
    match v.to_lowercase().as_str() {
        "activos" | "activo" | "dentro" => Some(EstadoMovimiento::Activos),
        "cerrados" | "cerrado" | "salieron" | "salio" | "salió" => Some(EstadoMovimiento::Cerrados),
        "todos" => Some(EstadoMovimiento::Todos),
        _ => None,
    }
}

/// Interpreta el campo de búsqueda libre con sintaxis `clave:valor`
/// (`empresa:`, `tipo:`, `estado:`, `gafete:`, `desde:`, `hasta:`) sobre los
/// filtros ya aplicados (`base`), y deja el texto no reconocido para que se
/// use como nombre/cédula. No valida ni construye la consulta SQL — eso lo
/// sigue haciendo `construir` sobre el resultado, para no duplicar esa
/// lógica.
pub(super) fn parsear_consulta(
    base: &FiltrosHistorial,
    texto: &str,
    empresas: &[Empresa],
) -> (FiltrosHistorial, String) {
    let mut filtros = base.clone();
    let mut libres = Vec::new();
    for token in tokenizar(texto) {
        let Some((clave, valor)) = token.split_once(':') else {
            libres.push(token);
            continue;
        };
        if valor.is_empty() {
            libres.push(token);
            continue;
        }
        match clave.to_lowercase().as_str() {
            "empresa" => {
                match empresas
                    .iter()
                    .find(|e| e.nombre.to_lowercase().contains(&valor.to_lowercase()))
                {
                    Some(e) => filtros.empresa_id = Some(e.id),
                    None => libres.push(token),
                }
            }
            "tipo" => match tipo_desde_texto(valor) {
                Some(t) => filtros.tipo = Some(t),
                None => libres.push(token),
            },
            "estado" => match estado_desde_texto(valor) {
                Some(e) => filtros.estado = e,
                None => libres.push(token),
            },
            "gafete" => filtros.gafete = valor.to_owned(),
            "desde" => filtros.desde = valor.to_owned(),
            "hasta" => filtros.hasta = valor.to_owned(),
            _ => libres.push(token),
        }
    }
    (filtros, libres.join(" "))
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

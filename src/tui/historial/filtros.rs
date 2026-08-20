use crate::{
    database::queries::ingresos::{EstadoMovimiento, FiltroHistorial},
    models::{empresa::Empresa, tipo_ingreso::TipoIngreso},
    tiempo::{ahora_costa_rica, inicio_dia_costa_rica_utc},
    tui::ui_kit::{Term, plegar_diacriticos, resolver_terminos, valores},
};
use chrono::{Datelike, Duration, NaiveDate};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltrosHistorial {
    pub desde: String,
    pub hasta: String,
    pub nombre_cedula: String,
    pub empresa_id: Option<i64>,
    /// `None` = todos los tipos; `Some(vec)` = cualquiera de los listados
    /// (`tipo:praind,swat`, o el complemento si viene negado `-tipo:swat`).
    pub tipos: Option<Vec<TipoIngreso>>,
    pub gafete: String,
    pub estado: EstadoMovimiento,
    /// Nombre (parcial) de quien registró el ingreso.
    pub usuario_ingreso: String,
    /// Nombre (parcial) de quien registró la salida.
    pub usuario_salida: String,
}
impl Default for FiltrosHistorial {
    fn default() -> Self {
        let h = ahora_costa_rica().date_naive();
        // Día 1 de cualquier mes siempre es válido, pero se evita el unwrap
        // igual: si algún día cambia el día usado aquí, cae a `h` en vez de
        // arriesgar un panic.
        let d = NaiveDate::from_ymd_opt(h.year(), h.month(), 1).unwrap_or(h);
        Self {
            desde: d.format("%d/%m/%Y").to_string(),
            hasta: h.format("%d/%m/%Y").to_string(),
            nombre_cedula: String::new(),
            empresa_id: None,
            tipos: None,
            gafete: String::new(),
            estado: EstadoMovimiento::Todos,
            usuario_ingreso: String::new(),
            usuario_salida: String::new(),
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
        tipos_incluidos: f.tipos.clone(),
        gafete_numero: gafete,
        estado: f.estado,
        usuario_ingreso: (!f.usuario_ingreso.trim().is_empty()).then(|| f.usuario_ingreso.trim().to_owned()),
        usuario_salida: (!f.usuario_salida.trim().is_empty()).then(|| f.usuario_salida.trim().to_owned()),
        limite: limit,
        offset,
        corte_id,
    })
}

fn estado_desde_texto(v: &str) -> Option<EstadoMovimiento> {
    match v.to_lowercase().as_str() {
        "activos" | "activo" | "dentro" => Some(EstadoMovimiento::Activos),
        "cerrados" | "cerrado" | "salieron" | "salio" | "salió" => Some(EstadoMovimiento::Cerrados),
        "todos" => Some(EstadoMovimiento::Todos),
        _ => None,
    }
}

/// Interpreta el campo de búsqueda libre con el lenguaje `clave:valor`
/// genérico de `ui_kit::query_lang` (negación `-clave:valor`, listas
/// `clave:a,b,c`, comillas), sobre los filtros ya aplicados (`base`). Cada
/// clave reconocida (`empresa`, `tipo`, `estado`, `gafete`, `desde`,
/// `hasta`, `ingreso` — quién registró el ingreso, `salida` — quién
/// registró la salida) la resuelve `aplicar_clave`; lo no reconocido, y lo
/// que no calza con lo que esa clave admite (p. ej. `gafete:1,2` o
/// `-desde:...`), se deja como texto libre para usarse como nombre/cédula.
/// No valida ni construye la consulta SQL — eso lo sigue haciendo
/// `construir` sobre el resultado.
pub(super) fn parsear_consulta(
    base: &FiltrosHistorial,
    texto: &str,
    empresas: &[Empresa],
) -> (FiltrosHistorial, String) {
    let mut filtros = base.clone();
    let libres = resolver_terminos(texto, &mut filtros, |f, term| aplicar_clave(f, term, empresas));
    (filtros, libres)
}

/// Aplica un término `clave:valor` ya interpretado sobre `f`. Devuelve
/// `false` cuando la clave no se reconoce o trae una combinación
/// (negada/lista) que esa clave no admite, para que el llamador la deje
/// como texto libre en vez de aplicarla a medias.
fn aplicar_clave(f: &mut FiltrosHistorial, term: &Term, empresas: &[Empresa]) -> bool {
    let clave = term.key.as_deref().unwrap_or_default().to_lowercase();
    let valores = valores(term);
    match clave.as_str() {
        "empresa" if !term.negated && valores.len() == 1 => {
            let buscado = plegar_diacriticos(&valores[0].to_lowercase());
            match empresas
                .iter()
                .find(|e| plegar_diacriticos(&e.nombre.to_lowercase()).contains(&buscado))
            {
                Some(e) => {
                    f.empresa_id = Some(e.id);
                    true
                }
                None => false,
            }
        }
        "tipo" => {
            let Some(reconocidos) = valores
                .iter()
                .map(|v| TipoIngreso::from_str_filtro(v))
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            if reconocidos.is_empty() {
                return false;
            }
            f.tipos = Some(if term.negated {
                TipoIngreso::ALL
                    .into_iter()
                    .filter(|t| !reconocidos.contains(t))
                    .collect()
            } else {
                reconocidos
            });
            true
        }
        "estado" => {
            let Some(reconocidos) = valores
                .iter()
                .map(|v| estado_desde_texto(v))
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            if reconocidos.is_empty() {
                return false;
            }
            let incluye_activos = reconocidos
                .iter()
                .any(|e| matches!(e, EstadoMovimiento::Activos | EstadoMovimiento::Todos));
            let incluye_cerrados = reconocidos
                .iter()
                .any(|e| matches!(e, EstadoMovimiento::Cerrados | EstadoMovimiento::Todos));
            let (activos, cerrados) = if term.negated {
                (!incluye_activos, !incluye_cerrados)
            } else {
                (incluye_activos, incluye_cerrados)
            };
            f.estado = match (activos, cerrados) {
                (true, true) => EstadoMovimiento::Todos,
                (true, false) => EstadoMovimiento::Activos,
                (false, true) => EstadoMovimiento::Cerrados,
                (false, false) => return false,
            };
            true
        }
        // Sólo acepta un valor numérico, igual que Activos: un valor no
        // numérico cae a texto libre en silencio en vez de escribirse en
        // `f.gafete` y hacer que `construir()` rechace toda la búsqueda con
        // "Ingrese un número de gafete válido" — ese mensaje queda para
        // cuando el operador lo escribe directo en el campo del panel.
        "gafete" if !term.negated && valores.len() == 1 && valores[0].trim().parse::<i64>().is_ok() => {
            f.gafete = valores[0].clone();
            true
        }
        "desde" if !term.negated && valores.len() == 1 => {
            f.desde = valores[0].clone();
            true
        }
        "hasta" if !term.negated && valores.len() == 1 => {
            f.hasta = valores[0].clone();
            true
        }
        "ingreso" if !term.negated && valores.len() == 1 => {
            f.usuario_ingreso = valores[0].clone();
            true
        }
        "salida" if !term.negated && valores.len() == 1 => {
            f.usuario_salida = valores[0].clone();
            true
        }
        _ => false,
    }
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
pub(super) fn tipo_texto(t: TipoIngreso) -> &'static str {
    match t {
        TipoIngreso::Praind => "PRAIND",
        TipoIngreso::InHouse => "IN HOUSE",
        TipoIngreso::PorCorreo => "POR CORREO",
        TipoIngreso::Swat => "SWAT",
    }
}
/// Resume el filtro de tipos aplicado para mostrarlo en la etiqueta de
/// búsqueda: "Todos" si no hay filtro, o los tipos unidos con " o ".
pub(super) fn tipos_texto(tipos: Option<&[TipoIngreso]>) -> String {
    match tipos {
        None => "Todos".into(),
        Some(tipos) => tipos
            .iter()
            .map(|t| tipo_texto(*t))
            .collect::<Vec<_>>()
            .join(" o "),
    }
}

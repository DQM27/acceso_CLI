//! Modelo puro de la Surface de Historial (`/historial`, alias `/h`) —
//! DEC-023/024 de `docs/lenguaje-visual-mutaciones.md`. Sin IO, sin
//! `AppCore`: interpreta el texto `clave:valor` y arma el `FiltroHistorial`
//! listo para consultar; la consulta en sí (`AppCore::buscar_historial`) es
//! responsabilidad de `historial_controller.rs`.
//!
//! Claves reconocidas, mismo vocabulario que `tui::historial::filtros`
//! (reescrito, no importado — DEC-002/DEC-014): `empresa`, `tipo`,
//! `estado`, `gafete`, `ingreso` (quién registró el ingreso), `salida`
//! (quién registró la salida), todas con negación (`-clave:valor`);
//! `desde`/`hasta` son la única excepción — no admiten negación porque no
//! tiene un significado obvio para el límite de un rango de fechas.

use chrono::{Datelike, NaiveDate};

use crate::database::queries::Igualdad;
use crate::database::queries::ingresos::{EstadoMovimiento, FiltroHistorial, PaginaHistorial};
use crate::models::empresa::Empresa;
use crate::models::tipo_ingreso::TipoIngreso;
use crate::texto::plegar_para_busqueda;
use crate::tiempo::{ahora_costa_rica, inicio_dia_costa_rica_utc};

use super::query_lang::{Term, resolver_terminos, valores};

#[derive(Debug, Clone)]
pub struct HistorialState {
    /// Filtro vigente: fechas + lo que ya se resolvió de la última consulta
    /// aplicada con Enter (base sobre la que se reinterpreta el texto cada
    /// vez que se vuelve a aplicar).
    pub filtro: FiltroHistorial,
    /// `Some` sólo tras aplicar con Enter — Historial no filtra en vivo
    /// (DEC-024): mientras se edita el texto esto es `None`.
    pub resultado: Option<PaginaHistorial>,
    pub seleccion: usize,
    /// Términos `clave:valor` de la última consulta aplicada que no se
    /// reconocieron — aviso, no error: el resto del filtro sí se aplicó.
    pub no_reconocidos: Vec<String>,
    /// Catálogo cargado al abrir Historial, para resolver `empresa:nombre`.
    pub empresas: Vec<Empresa>,
    /// Ruta de exportación en edición (`F5`), o `None` si no se está
    /// exportando. Input propio, no comparte `app.input` — así no pisa el
    /// texto del filtro que sigue mostrándose congelado detrás mientras se
    /// exporta (DEC-024: Esc a la exportación no debe perder la consulta).
    pub exportacion_destino: Option<tui_input::Input>,
}

impl HistorialState {
    /// Mismo rango por defecto que la TUI clásica: desde el día 1 del mes
    /// actual hasta hoy.
    pub fn nuevo(empresas: Vec<Empresa>) -> Self {
        let hoy = ahora_costa_rica().date_naive();
        let inicio_mes = NaiveDate::from_ymd_opt(hoy.year(), hoy.month(), 1).unwrap_or(hoy);
        let manana = hoy.succ_opt().unwrap_or(hoy);
        let ahora_utc = ahora_costa_rica().to_utc();
        let desde = inicio_dia_costa_rica_utc(inicio_mes).unwrap_or(ahora_utc);
        let hasta = inicio_dia_costa_rica_utc(manana).unwrap_or(ahora_utc);
        Self {
            filtro: FiltroHistorial::nuevo(desde, hasta),
            resultado: None,
            seleccion: 0,
            no_reconocidos: Vec::new(),
            empresas,
            exportacion_destino: None,
        }
    }

    /// Interpreta `texto` sobre el filtro vigente (fechas incluidas: un
    /// `desde:`/`hasta:` en el texto reemplaza el rango). No consulta SQLite
    /// — sólo deja el `FiltroHistorial` listo; `historial_controller.rs`
    /// hace la consulta con el resultado. Devuelve también las claves que no
    /// se reconocieron, para mostrarlas como aviso sin bloquear el resto.
    pub fn resolver_filtro(&self, texto: &str) -> (FiltroHistorial, Vec<String>) {
        let mut filtro = self.filtro.clone();
        let empresas = &self.empresas;
        let resolucion = resolver_terminos(texto, &mut filtro, |f, term| {
            aplicar_clave(f, term, empresas)
        });
        let libre = resolucion.texto_libre.trim();
        filtro.texto_persona = (!libre.is_empty()).then(|| libre.to_string());
        (filtro, resolucion.no_reconocidos)
    }
}

/// Nombre de archivo sugerido para la exportación — la ruta completa
/// (carpeta por defecto, si existe) la resuelve `historial_controller.rs`,
/// que es quien conoce el entorno (`%USERPROFILE%`).
pub fn nombre_exportacion_predeterminado() -> String {
    format!(
        "historial_{}.xlsx",
        ahora_costa_rica().format("%Y-%m-%d_%H%M")
    )
}

fn fecha(valor: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let fecha = NaiveDate::parse_from_str(valor, "%d/%m/%Y").ok()?;
    inicio_dia_costa_rica_utc(fecha).ok()
}

/// `hasta` es el límite exclusivo del rango (inicio del día *siguiente* al
/// último que se quiere incluir) — quien escribe `hasta:31/01/2026` espera
/// que el 31 sí cuente, así que se suma un día antes de convertir. Mismo
/// criterio que `tui::historial::filtros::construir`.
fn fecha_hasta(valor: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let dia = NaiveDate::parse_from_str(valor, "%d/%m/%Y").ok()?;
    let siguiente = dia.succ_opt()?;
    inicio_dia_costa_rica_utc(siguiente).ok()
}

fn estado_desde_texto(valor: &str) -> Option<EstadoMovimiento> {
    match valor.to_lowercase().as_str() {
        "activos" | "activo" | "dentro" => Some(EstadoMovimiento::Activos),
        "cerrados" | "cerrado" | "salieron" | "salio" | "salió" => {
            Some(EstadoMovimiento::Cerrados)
        }
        "todos" => Some(EstadoMovimiento::Todos),
        _ => None,
    }
}

/// Aplica un término `clave:valor` ya interpretado sobre `f`. Devuelve
/// `false` cuando la clave no se reconoce o trae una combinación
/// (negada/lista) que esa clave no admite, para que el llamador la deje
/// como texto libre en vez de aplicarla a medias.
fn aplicar_clave(f: &mut FiltroHistorial, term: &Term, empresas: &[Empresa]) -> bool {
    let clave = term.key.as_deref().unwrap_or_default().to_lowercase();
    let valores = valores(term);
    match clave.as_str() {
        "empresa" if valores.len() == 1 => {
            let buscado = plegar_para_busqueda(&valores[0]);
            match empresas
                .iter()
                .find(|e| plegar_para_busqueda(&e.nombre).contains(&buscado))
            {
                Some(empresa) => {
                    f.empresa_id = Some(if term.negated {
                        Igualdad::Excluye(empresa.id)
                    } else {
                        Igualdad::Incluye(empresa.id)
                    });
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
            f.tipos_incluidos = Some(if term.negated {
                TipoIngreso::ALL
                    .into_iter()
                    .filter(|t| !reconocidos.contains(t))
                    .collect()
            } else {
                reconocidos
            });
            true
        }
        "estado" if valores.len() == 1 => {
            let Some(estado) = estado_desde_texto(&valores[0]) else {
                return false;
            };
            f.estado = if term.negated {
                match estado {
                    EstadoMovimiento::Activos => EstadoMovimiento::Cerrados,
                    EstadoMovimiento::Cerrados => EstadoMovimiento::Activos,
                    EstadoMovimiento::Todos => return false,
                }
            } else {
                estado
            };
            true
        }
        "gafete" if valores.len() == 1 && valores[0].trim().parse::<i64>().is_ok() => {
            let numero: i64 = valores[0].trim().parse().unwrap_or_default();
            f.gafete_numero = Some(if term.negated {
                Igualdad::Excluye(numero)
            } else {
                Igualdad::Incluye(numero)
            });
            true
        }
        // desde/hasta no admiten negación: -desde:X no tiene un significado
        // obvio para el límite de un rango de fechas.
        "desde" if !term.negated && valores.len() == 1 => match fecha(&valores[0]) {
            Some(f_desde) => {
                f.desde = f_desde;
                true
            }
            None => false,
        },
        "hasta" if !term.negated && valores.len() == 1 => match fecha_hasta(&valores[0]) {
            Some(f_hasta) => {
                f.hasta = f_hasta;
                true
            }
            None => false,
        },
        "ingreso" if valores.len() == 1 => {
            f.usuario_ingreso = Some(valores[0].clone());
            f.usuario_ingreso_negado = term.negated;
            true
        }
        "salida" if valores.len() == 1 => {
            f.usuario_salida = Some(valores[0].clone());
            f.usuario_salida_negado = term.negated;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empresas() -> Vec<Empresa> {
        vec![
            Empresa {
                id: 1,
                nombre: "Constructora Pérez".to_string(),
                activo: true,
            },
            Empresa {
                id: 2,
                nombre: "Eléctrica Quesada".to_string(),
                activo: true,
            },
        ]
    }

    #[test]
    fn rango_por_defecto_es_desde_a_hoy() {
        let estado = HistorialState::nuevo(empresas());
        assert!(estado.filtro.desde <= estado.filtro.hasta);
        assert_eq!(estado.filtro.estado, EstadoMovimiento::Todos);
    }

    #[test]
    fn texto_libre_se_vuelve_texto_persona() {
        let estado = HistorialState::nuevo(empresas());
        let (filtro, no_reconocidos) = estado.resolver_filtro("Carlos Pérez");
        assert_eq!(filtro.texto_persona, Some("Carlos Pérez".to_string()));
        assert!(no_reconocidos.is_empty());
    }

    #[test]
    fn empresa_por_nombre_parcial_pliega_tildes() {
        let estado = HistorialState::nuevo(empresas());
        let (filtro, _) = estado.resolver_filtro("empresa:electrica");
        assert_eq!(filtro.empresa_id, Some(Igualdad::Incluye(2)));
    }

    #[test]
    fn tipo_admite_lista_y_negacion() {
        let estado = HistorialState::nuevo(empresas());
        let (filtro, _) = estado.resolver_filtro("tipo:praind,swat");
        assert_eq!(
            filtro.tipos_incluidos,
            Some(vec![TipoIngreso::Praind, TipoIngreso::Swat])
        );

        let (filtro_neg, _) = estado.resolver_filtro("-tipo:swat");
        let incluidos = filtro_neg.tipos_incluidos.unwrap();
        assert!(!incluidos.contains(&TipoIngreso::Swat));
        assert!(incluidos.contains(&TipoIngreso::Praind));
    }

    #[test]
    fn estado_reconoce_sinonimos() {
        let estado = HistorialState::nuevo(empresas());
        let (filtro, _) = estado.resolver_filtro("estado:activos");
        assert_eq!(filtro.estado, EstadoMovimiento::Activos);
    }

    #[test]
    fn desde_hasta_parsean_fecha_dd_mm_aaaa() {
        let estado = HistorialState::nuevo(empresas());
        let (filtro, no_reconocidos) = estado.resolver_filtro("desde:01/01/2026 hasta:31/01/2026");
        assert!(no_reconocidos.is_empty());
        assert!(filtro.desde < filtro.hasta);
    }

    #[test]
    fn fecha_invalida_cae_a_no_reconocido() {
        let estado = HistorialState::nuevo(empresas());
        let (_, no_reconocidos) = estado.resolver_filtro("desde:31/02/2026");
        assert_eq!(no_reconocidos, vec!["desde:31/02/2026"]);
    }

    #[test]
    fn desde_negado_no_se_reconoce() {
        let estado = HistorialState::nuevo(empresas());
        let (_, no_reconocidos) = estado.resolver_filtro("-desde:01/01/2026");
        assert_eq!(no_reconocidos, vec!["-desde:01/01/2026"]);
    }

    #[test]
    fn gafete_no_numerico_cae_a_texto_libre() {
        let estado = HistorialState::nuevo(empresas());
        let (filtro, no_reconocidos) = estado.resolver_filtro("gafete:abc");
        assert_eq!(filtro.gafete_numero, None);
        assert_eq!(no_reconocidos, vec!["gafete:abc"]);
    }

    #[test]
    fn ingreso_y_salida_admiten_negacion() {
        let estado = HistorialState::nuevo(empresas());
        let (filtro, _) = estado.resolver_filtro("ingreso:daniel -salida:ana");
        assert_eq!(filtro.usuario_ingreso, Some("daniel".to_string()));
        assert!(!filtro.usuario_ingreso_negado);
        assert_eq!(filtro.usuario_salida, Some("ana".to_string()));
        assert!(filtro.usuario_salida_negado);
    }

    #[test]
    fn clave_desconocida_cae_a_texto_libre() {
        let estado = HistorialState::nuevo(empresas());
        let (filtro, no_reconocidos) = estado.resolver_filtro("Ana clave:invalida");
        assert_eq!(filtro.texto_persona, Some("Ana clave:invalida".to_string()));
        assert_eq!(no_reconocidos, vec!["clave:invalida"]);
    }
}

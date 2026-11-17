//! Reglas del catálogo de gafetes. Dos estados: `Disponible` (se puede
//! asignar) y `Perdido` (no se puede: el gafete no está físicamente). Tres
//! reportes, cada uno a nombre de quien lo hace:
//!
//! - reportar perdido: `Disponible -> Perdido`, con o sin responsable;
//! - registrar pago: se le quita la marca al responsable, el gafete sigue
//!   `Perdido` hasta que se reimprima;
//! - recuperar: `Perdido -> Disponible` (apareció o se reimprimió).
//!
//! Ningún paso es definitivo: un gafete siempre se puede reimprimir.

use std::collections::BTreeSet;

use crate::models::gafete::{EstadoGafete, Gafete};

/// Largo máximo del detalle de un reporte (mismo tope que el `CHECK` de
/// `gafetes_incidentes.detalle`).
pub const LARGO_MAXIMO_DETALLE: usize = 1000;

fn esta_disponible(estado: EstadoGafete) -> bool {
    estado == EstadoGafete::Disponible
}

/// Sólo un gafete `Disponible` puede reportarse perdido.
pub fn puede_reportarse_perdido(estado: EstadoGafete) -> bool {
    esta_disponible(estado)
}

/// Sólo se registra el pago de un gafete perdido que tiene responsable.
pub fn puede_registrarse_pago(gafete: &Gafete) -> bool {
    gafete.estado == EstadoGafete::Perdido && gafete.portador().is_some()
}

/// Sólo un gafete `Perdido` puede recuperarse.
pub fn puede_recuperarse(estado: EstadoGafete) -> bool {
    estado == EstadoGafete::Perdido
}

/// El detalle de un reporte pasa de [`LARGO_MAXIMO_DETALLE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetalleMuyLargo;

/// Detalle de un reporte ya limpio: sin espacios a los costados y `None` si
/// queda vacío.
pub fn normalizar_detalle(detalle: Option<&str>) -> Result<Option<String>, DetalleMuyLargo> {
    let Some(texto) = detalle.map(str::trim).filter(|texto| !texto.is_empty()) else {
        return Ok(None);
    };
    if texto.chars().count() > LARGO_MAXIMO_DETALLE {
        return Err(DetalleMuyLargo);
    }
    Ok(Some(texto.to_string()))
}

/// Resultado de validar un número de gafete contra el catálogo antes de
/// asignarlo a un ingreso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidacionAsignacion {
    Asignable,
    NoRegistrado,
    NoDisponible(EstadoGafete),
}

/// Regla de negocio (`docs/plan-gafetes.md`): catálogo primero (¿existe y
/// está disponible?). La ocupación (¿está en uso ahora mismo por un ingreso
/// activo?) es un chequeo aparte porque depende de `registro_ingresos`, no
/// del catálogo — vive en el servicio, que sí tiene acceso a ese repositorio.
pub fn validar_para_asignar(gafete: Option<&Gafete>) -> ValidacionAsignacion {
    match gafete {
        None => ValidacionAsignacion::NoRegistrado,
        Some(gafete) if !esta_disponible(gafete.estado) => {
            ValidacionAsignacion::NoDisponible(gafete.estado)
        }
        Some(_) => ValidacionAsignacion::Asignable,
    }
}

/// Un gafete del catálogo tal como lo necesita el conteo físico: su número,
/// su estado y, si alguien que está adentro lo tiene puesto, quién.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GafeteParaConteo {
    pub numero: i64,
    pub estado: EstadoGafete,
    pub en_uso_por: Option<String>,
}

/// Qué es cada número escrito en el conteo, en el momento de escribirlo.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "tipo", rename_all = "snake_case"))]
pub enum LecturaConteo {
    /// Está en el catálogo, disponible y nadie lo tiene puesto: es lo esperado.
    Esperado,
    /// No existe en el catálogo de este tipo.
    NoExiste,
    /// Figura perdido y sin embargo se tiene en la mano.
    Perdido,
    /// Figura puesto en alguien que está adentro: hay una salida sin registrar.
    EnUso { nombre: String },
}

/// Clasifica un número escrito en el conteo contra el catálogo.
pub fn leer_numero_conteo(catalogo: &[GafeteParaConteo], numero: i64) -> LecturaConteo {
    match catalogo.iter().find(|gafete| gafete.numero == numero) {
        None => LecturaConteo::NoExiste,
        Some(gafete) if gafete.estado == EstadoGafete::Perdido => LecturaConteo::Perdido,
        Some(GafeteParaConteo {
            en_uso_por: Some(nombre),
            ..
        }) => LecturaConteo::EnUso {
            nombre: nombre.clone(),
        },
        Some(_) => LecturaConteo::Esperado,
    }
}

/// Un gafete que tiene puesto alguien que está adentro.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GafeteEnUso {
    pub numero: i64,
    pub nombre: String,
}

/// Lo que dice el botón "Comprobar": lo escrito contra lo que debería estar.
/// Todas las listas van ordenadas por número.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ResultadoConteo {
    /// Cuántos se esperaba contar: disponibles que nadie tiene puestos.
    pub esperados: usize,
    /// Esperados que sí se contaron.
    pub presentes: Vec<i64>,
    /// Esperados que no se contaron: deberían estar y no están.
    pub faltantes: Vec<i64>,
    /// Los tiene puestos alguien que está adentro y no se contaron (lo
    /// normal: no se esperan).
    pub en_uso: Vec<GafeteEnUso>,
    /// Figuran puestos en alguien y sin embargo se contaron: o se digitó mal
    /// el número, o falta registrar una salida.
    pub en_uso_contados: Vec<GafeteEnUso>,
    /// Contados que no existen en el catálogo de este tipo.
    pub no_existen: Vec<i64>,
    /// Figuran perdidos y sin embargo se contaron: o se digitó mal el
    /// número, o apareció y hay que recuperarlo.
    pub perdidos_contados: Vec<i64>,
    /// Perdidos que no se contaron (lo normal: no están).
    pub perdidos: Vec<i64>,
}

/// Compara los números escritos en el conteo con el catálogo. Los repetidos
/// cuentan una sola vez (la pantalla ya avisa al escribirlos).
pub fn comprobar_conteo(catalogo: &[GafeteParaConteo], numeros: &[i64]) -> ResultadoConteo {
    let contados: BTreeSet<i64> = numeros.iter().copied().collect();
    let mut ordenado: Vec<&GafeteParaConteo> = catalogo.iter().collect();
    ordenado.sort_by_key(|gafete| gafete.numero);

    let mut resultado = ResultadoConteo::default();
    for gafete in ordenado {
        let contado = contados.contains(&gafete.numero);
        match (&gafete.estado, &gafete.en_uso_por) {
            (EstadoGafete::Perdido, _) => {
                if contado {
                    resultado.perdidos_contados.push(gafete.numero);
                } else {
                    resultado.perdidos.push(gafete.numero);
                }
            }
            (EstadoGafete::Disponible, Some(nombre)) => {
                let en_uso = GafeteEnUso {
                    numero: gafete.numero,
                    nombre: nombre.clone(),
                };
                if contado {
                    resultado.en_uso_contados.push(en_uso);
                } else {
                    resultado.en_uso.push(en_uso);
                }
            }
            (EstadoGafete::Disponible, None) => {
                resultado.esperados += 1;
                if contado {
                    resultado.presentes.push(gafete.numero);
                } else {
                    resultado.faltantes.push(gafete.numero);
                }
            }
        }
    }
    resultado.no_existen = contados
        .into_iter()
        .filter(|numero| !catalogo.iter().any(|gafete| gafete.numero == *numero))
        .collect();
    resultado
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gafete(numero: i64, estado: EstadoGafete, en_uso_por: Option<&str>) -> GafeteParaConteo {
        GafeteParaConteo {
            numero,
            estado,
            en_uso_por: en_uso_por.map(str::to_string),
        }
    }

    fn catalogo() -> Vec<GafeteParaConteo> {
        vec![
            gafete(3, EstadoGafete::Disponible, None),
            gafete(1, EstadoGafete::Disponible, None),
            gafete(2, EstadoGafete::Disponible, None),
            gafete(4, EstadoGafete::Disponible, Some("Juan Pérez")),
            gafete(5, EstadoGafete::Perdido, None),
            gafete(6, EstadoGafete::Perdido, None),
            gafete(7, EstadoGafete::Disponible, Some("Ana Mora")),
        ]
    }

    #[test]
    fn el_conteo_separa_faltantes_en_uso_inexistentes_y_perdidos() {
        let resultado = comprobar_conteo(&catalogo(), &[3, 1, 99, 6, 7, 1]);

        assert_eq!(resultado.esperados, 3);
        assert_eq!(resultado.presentes, vec![1, 3]);
        assert_eq!(resultado.faltantes, vec![2]);
        // Cada número sale en un solo lugar: el 7 se contó, el 4 no.
        assert_eq!(
            resultado.en_uso,
            vec![GafeteEnUso {
                numero: 4,
                nombre: "Juan Pérez".to_string()
            }]
        );
        assert_eq!(
            resultado.en_uso_contados,
            vec![GafeteEnUso {
                numero: 7,
                nombre: "Ana Mora".to_string()
            }]
        );
        assert_eq!(resultado.no_existen, vec![99]);
        assert_eq!(resultado.perdidos_contados, vec![6]);
        assert_eq!(resultado.perdidos, vec![5]);
    }

    #[test]
    fn un_conteo_completo_no_tiene_faltantes() {
        let resultado = comprobar_conteo(&catalogo(), &[2, 3, 1]);
        assert!(resultado.faltantes.is_empty());
        assert_eq!(resultado.presentes, vec![1, 2, 3]);
    }

    #[test]
    fn cada_numero_escrito_se_clasifica_al_momento() {
        let catalogo = catalogo();
        assert_eq!(leer_numero_conteo(&catalogo, 1), LecturaConteo::Esperado);
        assert_eq!(leer_numero_conteo(&catalogo, 99), LecturaConteo::NoExiste);
        assert_eq!(leer_numero_conteo(&catalogo, 5), LecturaConteo::Perdido);
        assert_eq!(
            leer_numero_conteo(&catalogo, 4),
            LecturaConteo::EnUso {
                nombre: "Juan Pérez".to_string()
            }
        );
    }

    #[test]
    fn el_detalle_se_limpia_y_tiene_tope() {
        assert_eq!(normalizar_detalle(None), Ok(None));
        assert_eq!(normalizar_detalle(Some("   ")), Ok(None));
        assert_eq!(
            normalizar_detalle(Some("  recibí sin el 07 ")),
            Ok(Some("recibí sin el 07".to_string()))
        );
        let largo = "x".repeat(LARGO_MAXIMO_DETALLE + 1);
        assert_eq!(normalizar_detalle(Some(&largo)), Err(DetalleMuyLargo));
    }
}

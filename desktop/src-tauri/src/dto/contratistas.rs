use chrono::NaiveDate;

use control_acceso::database::queries::contratistas::FiltroContratistas;
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::services::contratista_service::{
    DatosActualizacionContratista, DatosContratista,
};

/// Sólo texto: la grilla de Contratistas ya no manda filtro (carga el
/// universo completo y filtra del lado del cliente con AG Grid — ver
/// `desktop/src/pantallas/Contratistas.tsx`), pero el buscador en vivo de
/// los modales (`NuevoIngresoModal`, `GestionGafeteModal`) sigue
/// necesitando una búsqueda de texto contra el servidor. `limite` siempre
/// pide el máximo (`FiltroContratistas::default()` lo clampea contra
/// `LIMITE_LISTADO_MAXIMO_CARGA_COMPLETA` en
/// `src/database/queries/contratistas.rs`) — ninguno de los dos casos de
/// uso quiere paginar.
#[derive(serde::Deserialize, Default)]
pub struct FiltroContratistasEntrada {
    pub texto: Option<String>,
}

impl FiltroContratistasEntrada {
    pub fn construir(self) -> FiltroContratistas {
        FiltroContratistas {
            texto: self
                .texto
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty()),
            limite: usize::MAX,
            ..Default::default()
        }
    }
}

/// Espejo de `DatosContratista`/`DatosActualizacionContratista` — el core ya
/// modela crear y editar con dos structs idénticos en forma (distinto nombre
/// nada más), así que del lado del webview alcanza con uno solo. Cubre tanto
/// el formulario completo (crear/editar) como el toggle rápido de
/// "es de ruta"/"tiene acceso" desde la grilla (la fila ya tiene todos los
/// demás campos, se mandan de vuelta sin cambios).
#[derive(serde::Deserialize)]
pub struct DatosContratistaEntrada {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

impl From<DatosContratistaEntrada> for DatosContratista {
    fn from(entrada: DatosContratistaEntrada) -> Self {
        Self {
            cedula: entrada.cedula,
            nombre: entrada.nombre,
            empresa_id: entrada.empresa_id,
            tipo_ingreso: entrada.tipo_ingreso,
            fecha_vencimiento_praind: entrada.fecha_vencimiento_praind,
            es_personal_ruta: entrada.es_personal_ruta,
            tiene_acceso: entrada.tiene_acceso,
        }
    }
}

impl From<DatosContratistaEntrada> for DatosActualizacionContratista {
    fn from(entrada: DatosContratistaEntrada) -> Self {
        Self {
            cedula: entrada.cedula,
            nombre: entrada.nombre,
            empresa_id: entrada.empresa_id,
            tipo_ingreso: entrada.tipo_ingreso,
            fecha_vencimiento_praind: entrada.fecha_vencimiento_praind,
            es_personal_ruta: entrada.es_personal_ruta,
            tiene_acceso: entrada.tiene_acceso,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_texto_en_blanco_se_convierte_en_ninguno() {
        let filtro = FiltroContratistasEntrada {
            texto: Some("   ".to_owned()),
        }
        .construir();

        assert_eq!(filtro.texto, None);
    }

    #[test]
    fn el_texto_se_recorta_de_espacios() {
        let filtro = FiltroContratistasEntrada {
            texto: Some("  ana  ".to_owned()),
        }
        .construir();

        assert_eq!(filtro.texto, Some("ana".to_owned()));
    }

    #[test]
    fn sin_texto_pide_el_limite_maximo_sin_paginar() {
        let filtro = FiltroContratistasEntrada::default().construir();

        assert_eq!(filtro.texto, None);
        assert_eq!(filtro.offset, 0);
    }
}

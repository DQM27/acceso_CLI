use crate::domain::resultado_salida_ruta::ResultadoSalidaRuta;

/// Vínculo N a N entre un tramo (`salidas_ruta`) y un documento
/// (`documentos_ruta`) -- ver
/// `docs/planes-implementados/plan-control-rutas.md`, sección "Rediseño
/// del núcleo de rutas -- documento/tramo/viaje". El mismo mecanismo
/// cubre dos casos reales a la vez: una salida con varios documentos
/// simultáneos (varias filas para el mismo `salida_id`), y un documento
/// repartido en 2+ salidas -- una recarga que termina de despachar lo
/// que no cupo en el primer tramo (varias filas para el mismo
/// `documento_id`).
///
/// `resultado`/`motivo_resultado` viven ACÁ, no en `DocumentoRuta` --
/// hallazgo clave de esta ronda: un documento puede reusarse en un tramo
/// de OTRO día ("sacar una ruta del día anterior"), así que el veredicto
/// de fecha vencida se evalúa por cada vínculo, contra la fecha de ESE
/// tramo, no una vez fijo en el documento.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SalidaRutaDocumento {
    pub salida_id: i64,
    pub documento_id: i64,
    pub resultado: ResultadoSalidaRuta,
}

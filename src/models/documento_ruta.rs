use chrono::NaiveDate;

/// El comprobante real que ampara una carga (ver
/// `docs/planes-implementados/plan-control-rutas.md`, sección "Rediseño
/// del núcleo de rutas -- documento/tramo/viaje"). Vive independiente
/// del vehículo/tramo -- puede repartirse entre varias salidas físicas,
/// incluso de camiones distintos.
///
/// `ruta_id` es `Option` a propósito, al revés de como era antes en
/// `salidas_ruta` (donde `ruta_id` era obligatorio): el número de ruta es
/// propiedad del documento, no del camión -- va impreso en cada
/// comprobante individual. Un camión tercerizado puede llevar documentos
/// de varias rutas distintas a la vez, o de ninguna (carga ajena a la
/// operación normal de Brisas/KOF) -- en ese caso `ruta_id` queda `None`,
/// pero `numero_documento` sigue siendo obligatorio.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DocumentoRuta {
    pub id: i64,
    pub numero_documento: String,
    pub ruta_id: Option<i64>,
    pub sub_numero: i64,
    pub fecha_documento: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct NuevoDocumentoRuta {
    pub numero_documento: String,
    pub ruta_id: Option<i64>,
    pub sub_numero: i64,
    pub fecha_documento: NaiveDate,
}

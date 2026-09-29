//! Lectura de documentos por OCR: todo lo que decide QUÉ dice el texto que
//! entrega ML Kit (auditoría OCR 2026-09-28, punto A-1). En Kotlin quedan
//! sólo la cámara, ML Kit, el recorte de la imagen y la pantalla.
//!
//! - [`renglones_visuales`]: el texto del frame ordenado como lo ve una
//!   persona, a partir de las cajas de ML Kit (punto E-3).
//! - [`identidad`] y [`campos_identidad`]: cédula, DIMEX, licencia, carnet
//!   PRAIND, gafetes y carnets de empresa.
//! - [`mrz_texto`]: la zona MRZ dentro del texto (su lectura, en `mrz.rs`).
//! - [`vehiculo`], [`comprobante`] y [`carnet_kof`]: los lectores de rutas.
//! - [`estabilizador`]: decide, frame a frame, cuándo una lectura de
//!   documento está confirmada.
//!
//! Cada lector acepta una o varias versiones del texto del mismo frame
//! (ver [`renglones_visuales::textos_de_frame`]) y se queda con la primera
//! que lee.

mod campos_identidad;
mod carnet_kof;
mod cedula;
mod comprobante;
mod estabilizador;
mod identidad;
mod mrz_texto;
mod renglones_visuales;
mod texto;
mod vehiculo;

pub use carnet_kof::{CarnetKofDetectado, es_carnet_kof, extraer_carnet_kof};
pub use cedula::extraer_cedula_de_texto;
pub use comprobante::{
    ComprobanteRutaDetectado, es_comprobante_carga_ruta, extraer_comprobante_ruta,
};
pub use estabilizador::{
    ConfiguracionEstabilizador, EstabilizadorDocumento, EstadoEscaneo, ModoEscaneoDocumento,
    OrientacionEncuadre, ResultadoEstabilizacion, consenso_alcanza, es_otra_lectura,
    orientacion_de_texto_documento, orientacion_encuadre_de_tipo,
};
pub use identidad::{
    DocumentoLeido, FuenteDatos, TipoDocumento, clasificar_tipo_documento, documento_desde_mrz,
    leer_documento_de_texto, nombre_legible_tipo_documento, parece_reverso_cedula_anterior,
    reclasificar_por_edad,
};
pub use mrz_texto::{LecturaMrz, leer_lectura_mrz};
pub use renglones_visuales::{
    LineaOcr, PalabraOcr, confianza_minima_palabra, reconstruir_texto_visual, textos_de_frame,
};
pub use texto::FechaOcr;
pub use vehiculo::{
    FormatoVehiculo, LecturaVehiculo, TipoVehiculoDetectado, VehiculoRutaDetectado,
    extraer_vehiculo, extraer_vehiculo_con_detalle, placa_como_se_imprime,
};

package com.brisas.controlacceso

// "Comprobante de Carga de Ruta" (Coca-Cola FEMSA): número de ruta,
// sub-número (1 = principal, 2+ = recargas) y "Transporte" (número de
// documento). La lectura vive en el núcleo Rust
// (`lectura_documentos/comprobante.rs`).

typealias ComprobanteRutaDetectado = uniffi.control_acceso_mobile.ComprobanteRutaDetectado

/// ¿Es un Comprobante de Carga de Ruta? Separado de la extracción para que
/// la pantalla siga buscando sin tratar cada frame parcial como fallo.
fun esComprobanteCargaRuta(texto: String): Boolean = uniffi.control_acceso_mobile.esComprobanteCargaRuta(texto)

/// Ruta y documento son obligatorios: sin alguno no hay nada que registrar.
fun extraerComprobanteRuta(texto: String): ComprobanteRutaDetectado? =
    uniffi.control_acceso_mobile.extraerComprobanteRuta(texto)

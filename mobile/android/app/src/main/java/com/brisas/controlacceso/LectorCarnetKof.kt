package com.brisas.controlacceso

// Carnet KOF (personal interno de Coca-Cola FEMSA) del paso "Gafete KOF"
// del checklist de rutas: nombre y código de empleado. La lectura vive en
// el núcleo Rust (`lectura_documentos/carnet_kof.rs`).

typealias CarnetKofDetectado = uniffi.control_acceso_mobile.CarnetKofDetectado

/// ¿El texto viene de un carnet KOF (frente o reverso)? Excluye el
/// comprobante de carga, que imprime la misma marca.
fun esCarnetKof(texto: String): Boolean = uniffi.control_acceso_mobile.esCarnetKof(texto)

/// Nombre y código de empleado son independientes (frente y reverso se
/// escanean por separado): devuelve lo que haya, al menos uno.
fun extraerCarnetKof(texto: String): CarnetKofDetectado? = uniffi.control_acceso_mobile.extraerCarnetKof(texto)

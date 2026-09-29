package com.brisas.controlacceso

// Placa o número de unidad del paso "Placa o número de unidad" del
// checklist de rutas (y del registro de proveedores). La lectura --placa de
// carga, particular y de moto, número de unidad, y la corrección de
// confusiones letra/dígito según la posición-- vive en el núcleo Rust
// (`lectura_documentos/vehiculo.rs`); acá quedan el punto de entrada de
// siempre y la clave con que se vota entre frames.

typealias VehiculoRutaDetectado = uniffi.control_acceso_mobile.VehiculoRutaDetectado

typealias TipoVehiculoDetectado = uniffi.control_acceso_mobile.TipoVehiculoDetectado

/// Clave estable de un vehículo leído (lo que se vota entre frames, ver
/// `EstabilizadorPorRepeticion`) y su inversa.
fun VehiculoRutaDetectado.claveVotacion(): String = "${tipo.name}:$valor"

fun vehiculoDesdeClave(clave: String): VehiculoRutaDetectado? {
    val (tipo, valor) = clave.split(':', limit = 2).takeIf { it.size == 2 } ?: return null
    val tipoDetectado = TipoVehiculoDetectado.entries.firstOrNull { it.name == tipo } ?: return null
    return VehiculoRutaDetectado(valor, tipoDetectado).takeIf { valor.isNotBlank() }
}

/// Placa (lo más específico) o, si no hay, número de unidad.
fun extraerVehiculo(texto: String): VehiculoRutaDetectado? = uniffi.control_acceso_mobile.extraerVehiculo(texto)

/// Igual que [extraerVehiculo], con cómo se obtuvo (formato, correcciones,
/// "CL" restituida) para la telemetría de diagnóstico.
fun extraerVehiculoConDetalle(texto: String): LecturaVehiculo? =
    uniffi.control_acceso_mobile.extraerVehiculoConDetalle(texto)

typealias LecturaVehiculo = uniffi.control_acceso_mobile.LecturaVehiculo

/// La placa como va impresa (`CL 371931`, `BPH-485`), para mostrarla y
/// llenar el formulario. El valor sin formato es el que se vota y se busca
/// en el catálogo de vehículos (Rutas), que lo compara exacto.
fun placaComoSeImprime(valor: String): String = uniffi.control_acceso_mobile.placaComoSeImprime(valor)

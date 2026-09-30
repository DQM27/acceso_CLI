package com.brisas.controlacceso

import uniffi.control_acceso_mobile.NucleoException

/// Errores ESPERADOS de una operación contra el núcleo (`NucleoException`,
/// con mensaje ya listo para mostrar; incluye los fallos de la clave del
/// teléfono en Android Keystore). Devuelve su mensaje; cualquier otra excepción se relanza tal cual -- incluida
/// `CancellationException`, para no romper la cancelación de corrutinas.
///
/// Reemplaza el bloque de 3 `catch` idénticos que se repetía en casi todos
/// los ViewModels (auditoría móvil, punto M3):
///
/// ```
/// } catch (excepcion: Exception) {
///     error = excepcion.mensajeDeErrorEsperado()
/// }
/// ```
fun Exception.mensajeDeErrorEsperado(): String? = when (this) {
    is NucleoException -> message
    else -> throw this
}

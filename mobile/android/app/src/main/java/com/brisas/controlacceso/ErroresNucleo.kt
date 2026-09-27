package com.brisas.controlacceso

import uniffi.control_acceso_mobile.NucleoException

/// Errores ESPERADOS de una operación contra el núcleo: el propio núcleo
/// (`NucleoException`, con mensaje ya listo para mostrar) o el almacén del
/// secreto del dispositivo (no está, o no se pudo leer). Devuelve su
/// mensaje; cualquier otra excepción se relanza tal cual -- incluida
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
    is NucleoException,
    is SecretoDispositivoNoEncontradoException,
    is SecretoDispositivoStoreException,
    -> message
    else -> throw this
}

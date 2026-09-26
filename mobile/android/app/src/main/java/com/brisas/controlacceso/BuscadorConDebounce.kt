package com.brisas.controlacceso

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/// MV-10 (auditoría 2026-09-24): antes de esto, `RutasViewModel` (x3,
/// encargado/ruta/vehículo), `ProveedoresViewModel` (empresa) y
/// `GafetesProvisionalesViewModel` (encargado) tenían cada uno su propia
/// copia idéntica de este patrón (`Job?` propio, cancelar, `delay(150)`,
/// relanzar) -- 6 copias reducidas a 1.
///
/// `ActivosViewModel.buscar()` queda fuera a propósito: comparte el mismo
/// `Job` con un contador de versión (`versionBusqueda`) y el flag
/// `cargando` para varios modos de búsqueda a la vez -- forzarlo a esta
/// forma genérica exigiría exponer esos dos conceptos acá también,
/// convirtiendo el helper en algo específico de esa pantalla en vez de un
/// utilitario simple.
///
/// Una instancia por buscador -- campo privado del ViewModel dueño, nunca
/// compartida entre dos buscadores distintos del mismo ViewModel (mismo
/// criterio que ya exigía el `Job?` que reemplaza).
class BuscadorConDebounce(
    private val scope: CoroutineScope,
    private val debounceMs: Long = 150L,
) {
    private var trabajo: Job? = null

    /// Cancela cualquier búsqueda en curso y lanza `accion`, tras esperar
    /// `debounceMs` -- salvo `inmediato = true`, para cuando el texto viene
    /// de un escaneo OCR (`usarEncargadoEscaneado`/`usarVehiculoEscaneado`/
    /// `usarNumeroRutaEscaneado` en `RutasViewModel`) en vez de tipeo: ahí no
    /// hay nada que debouncear, la búsqueda debe salir ya.
    fun buscar(inmediato: Boolean = false, accion: suspend () -> Unit) {
        trabajo?.cancel()
        trabajo = scope.launch {
            if (!inmediato) delay(debounceMs)
            accion()
        }
    }

    /// Sólo cancela, sin relanzar -- para cuando el texto queda en blanco o
    /// se elige un resultado directamente (sin esperar el debounce).
    fun cancelar() {
        trabajo?.cancel()
    }
}

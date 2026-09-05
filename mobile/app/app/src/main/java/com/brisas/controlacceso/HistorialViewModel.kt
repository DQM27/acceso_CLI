package com.brisas.controlacceso

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.MovimientoHistorial
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException

/// Dueño del estado de [PantallaHistorial] y de la llamada a
/// `Nucleo.buscarHistorial` — ver mobile/app/ARQUITECTURA.md. A diferencia
/// de `PantallaConfirmarIngreso` (que a propósito no tiene ViewModel),
/// Historial es una pestaña que persiste mientras el usuario navega
/// (mismo rol que `ActivosViewModel` para su pestaña), así que sí aplica
/// el patrón completo acá.
class HistorialViewModel(
    private val nucleo: Nucleo,
    // Ver el mismo parámetro en ActivosViewModel — permite tests con
    // tiempo controlado en vez de hilos reales.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.Default,
) : ViewModel() {
    var texto by mutableStateOf("")
        private set
    var movimientos by mutableStateOf<List<MovimientoHistorial>>(emptyList())
        private set
    var error by mutableStateOf<String?>(null)
        private set

    // Cancela la búsqueda anterior si `texto` cambió antes de que
    // terminara — mismo criterio que `ActivosViewModel.buscar`.
    private var trabajoBusqueda: Job? = null

    init {
        buscar()
    }

    fun cambiarTexto(nuevo: String) {
        texto = nuevo
        buscar()
    }

    private fun buscar() {
        trabajoBusqueda?.cancel()
        trabajoBusqueda = viewModelScope.launch {
            try {
                movimientos = withContext(dispatcherIO) { nucleo.buscarHistorial(texto) }
                error = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    fun refrescar() = buscar()

    companion object {
        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { HistorialViewModel(nucleo) }
        }
    }
}

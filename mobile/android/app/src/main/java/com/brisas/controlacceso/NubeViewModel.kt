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
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.ResumenSincronizacion

/// Dueño del estado del botón manual "Sincronizar" (ver
/// docs/planes-implementados/plan-persistencia-nube.md y mobile/android/arquitectura.md) — el
/// Composable que lo use sólo dibuja lo que expone acá y le reporta
/// eventos, nunca llama a [Nucleo] directamente.
///
/// Antes también manejaba pegar/consultar el secreto del dispositivo
/// (`Operacion::GestionarNube`, sólo Root) desde una pantalla "Nube"
/// propia -- se sacó junto con esa pestaña (la app móvil se limita a
/// registros rápidos + historial; configurar un dispositivo de cero sigue
/// siendo el primer arranque, `PantallaPrimerArranque.kt`). Lo que queda
/// acá es `sincronizar`, disponible para cualquier rol
/// (`Operacion::UsarNube`).
class NubeViewModel(
    private val nucleo: Nucleo,
    // Misma reacción ante `sesionExpulsada` que el pulso periódico
    // (`SincronizacionPeriodica`, ver `PantallaPrincipal.kt`).
    private val onSesionExpulsada: (enOtraUnidad: Boolean) -> Unit = {},
    // Ver el mismo parámetro en ActivosViewModel/RutasViewModel —
    // permite tests con tiempo controlado en vez de hilos reales.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var sincronizando by mutableStateOf(false)
        private set
    var ultimoResumen by mutableStateOf<ResumenSincronizacion?>(null)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    /// Autentica este dispositivo, drena la bandeja de salida pendiente y
    /// trae lo que el otro dispositivo del sitio tiene abierto ahora
    /// mismo. Es una llamada de red real (cientos de milisegundos o más,
    /// ver doc-comment de `sincronizar_con_nube` en
    /// `src/application/nube.rs`) -- `sincronizando` es lo que la pantalla
    /// usa para deshabilitar el botón mientras tanto. Los ingresos
    /// abiertos por el otro dispositivo (`ingresos_remotos`, ya
    /// actualizada por esta misma llamada) se leen y se cierran desde
    /// Activos, no desde acá -- ver `ActivosViewModel.FilaActiva`.
    fun sincronizar() {
        if (sincronizando) return
        error = null
        sincronizando = true
        viewModelScope.launch {
            try {
                val resumen = withContext(dispatcherIO) {
                    medirNucleo("sincronizarConNube") { nucleo.sincronizarConNube() }
                }
                ultimoResumen = resumen
                if (resumen.sesionExpulsada) onSesionExpulsada(resumen.sesionEnOtraUnidad)
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                sincronizando = false
            }
        }
    }

    companion object {
        fun factory(
            nucleo: Nucleo,
            onSesionExpulsada: (enOtraUnidad: Boolean) -> Unit = {},
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { NubeViewModel(nucleo, onSesionExpulsada) }
        }
    }
}

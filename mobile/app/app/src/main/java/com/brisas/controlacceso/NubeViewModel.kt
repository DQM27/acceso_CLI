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
import uniffi.control_acceso_mobile.IngresoRemoto
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.ResumenSincronizacion

/// Dueño del estado de la sincronización con la nube (ver
/// docs/plan-persistencia-nube.md y mobile/app/ARQUITECTURA.md) — el
/// Composable que lo use sólo dibuja lo que expone acá y le reporta
/// eventos, nunca llama a [Nucleo] directamente.
///
/// `guardarSecreto`/`actualizarEstadoSecreto` exigen Root del lado de Rust
/// (`Operacion::GestionarNube`, ver `src/application/nube.rs`) —
/// `sincronizar`/`cerrarIngresoRemoto` los puede llamar cualquier rol
/// (`Operacion::UsarNube`). Esta clase no repite esas reglas: si alguien
/// sin permiso llama un método exclusivo de Root, `Nucleo` tira
/// [NucleoException] y acá sólo se refleja como `error` — el gateo de UI
/// (qué ve un Operador) es responsabilidad de quien arme la pantalla, no
/// de este ViewModel.
///
/// A propósito **no** llama a nada en `init` — a diferencia de
/// `ActivosViewModel`/`HistorialViewModel` (que sí cargan datos apenas se
/// crean), acá hasta comprobar si ya hay un secreto guardado es una
/// operación exclusiva de Root; auto-dispararla para cualquier usuario
/// que entre a la pantalla generaría un error para todo el que no sea
/// Root sin que haya pedido nada.
class NubeViewModel(
    private val nucleo: Nucleo,
    private val directorio: String,
    // Ver el mismo parámetro en ActivosViewModel/HistorialViewModel —
    // permite tests con tiempo controlado en vez de hilos reales.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.Default,
) : ViewModel() {
    var secretoGuardado by mutableStateOf(false)
        private set
    var sincronizando by mutableStateOf(false)
        private set
    var ultimoResumen by mutableStateOf<ResumenSincronizacion?>(null)
        private set
    var ingresosRemotos by mutableStateOf<List<IngresoRemoto>>(emptyList())
        private set
    var error by mutableStateOf<String?>(null)
        private set

    /// Sólo Root — ver el doc-comment de la clase. Síncrono a propósito:
    /// es una lectura de archivo local, sin red de por medio (mismo
    /// criterio que `LoginViewModel.autenticar`).
    fun actualizarEstadoSecreto() {
        try {
            secretoGuardado = nucleo.secretoDispositivoGuardado(directorio)
            error = null
        } catch (excepcion: NucleoException) {
            error = excepcion.message
        }
    }

    /// Sólo Root — ver el doc-comment de la clase. Síncrono, mismo motivo
    /// que [actualizarEstadoSecreto]: guardar el secreto es escribir un
    /// archivo, no hablar con la nube.
    fun guardarSecreto(secreto: String) {
        error = null
        try {
            nucleo.guardarSecretoDispositivo(directorio, secreto)
            secretoGuardado = true
        } catch (excepcion: NucleoException) {
            error = excepcion.message
        }
    }

    /// Cualquier rol — autentica este dispositivo, drena la bandeja de
    /// salida pendiente y trae lo que el otro dispositivo del sitio tiene
    /// abierto ahora mismo. Es una llamada de red real (cientos de
    /// milisegundos o más, ver doc-comment de `sincronizar_con_nube` en
    /// `src/application/nube.rs`) — `sincronizando` es lo que la pantalla
    /// usa para deshabilitar el botón mientras tanto.
    fun sincronizar() {
        error = null
        sincronizando = true
        viewModelScope.launch {
            try {
                ultimoResumen = withContext(dispatcherIO) { nucleo.sincronizarConNube(directorio) }
                // La propia sincronización ya llenó la caché local
                // ingresos_remotos — esta lectura es local, no vuelve a
                // pegarle a la red (ver doc-comment de
                // `listar_ingresos_remotos` en src/application/nube.rs).
                ingresosRemotos = withContext(dispatcherIO) { nucleo.listarIngresosRemotos() }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                sincronizando = false
            }
        }
    }

    /// Cualquier rol — cierra, contra la nube, un ingreso que abrió el
    /// otro dispositivo del sitio. Nunca toca el historial local de este
    /// teléfono.
    fun cerrarIngresoRemoto(uuid: String) {
        error = null
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) { nucleo.cerrarIngresoRemoto(directorio, uuid) }
                ingresosRemotos = withContext(dispatcherIO) { nucleo.listarIngresosRemotos() }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    companion object {
        /// `directorio` es el mismo que ya usa [MainActivity] para abrir la
        /// base `SQLite` (`filesDir.absolutePath`) — no un archivo, la
        /// carpeta; Android no tiene `%LOCALAPPDATA%`, así que a diferencia
        /// de escritorio acá siempre hay que pasarlo explícito.
        fun factory(nucleo: Nucleo, directorio: String): ViewModelProvider.Factory = viewModelFactory {
            initializer { NubeViewModel(nucleo, directorio) }
        }
    }
}

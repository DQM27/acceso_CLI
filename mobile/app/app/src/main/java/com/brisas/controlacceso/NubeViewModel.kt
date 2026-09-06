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
    // `Settings.Secure.ANDROID_ID` (resuelto en `MainActivity`) -- cifra el
    // secreto de dispositivo en disco, ver `guardarSecreto` más abajo.
    private val identificadorDispositivo: String,
    // Ver `PantallaPrincipal.kt` / `SincronizacionPeriodica` -- misma
    // reacción ante `sesionExpulsada` que el pulso periódico, para el botón
    // manual "Sincronizar" de esta pantalla.
    private val onSesionExpulsada: () -> Unit = {},
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
    var error by mutableStateOf<String?>(null)
        private set

    /// Sólo Root — ver el doc-comment de la clase. Síncrono a propósito:
    /// es una lectura de archivo local, sin red de por medio (mismo
    /// criterio que `LoginViewModel.autenticar`).
    fun actualizarEstadoSecreto() {
        try {
            secretoGuardado = nucleo.secretoDispositivoGuardado(directorio, identificadorDispositivo)
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
            nucleo.guardarSecretoDispositivo(directorio, identificadorDispositivo, secreto)
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
    /// usa para deshabilitar el botón mientras tanto. Los ingresos abiertos
    /// por el otro dispositivo (`ingresos_remotos`, ya actualizada por esta
    /// misma llamada) se leen y se cierran desde Activos, no desde acá --
    /// ver `ActivosViewModel.FilaActiva`.
    fun sincronizar() {
        error = null
        sincronizando = true
        viewModelScope.launch {
            try {
                val resumen = withContext(dispatcherIO) { nucleo.sincronizarConNube(directorio) }
                ultimoResumen = resumen
                if (resumen.sesionExpulsada) onSesionExpulsada()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                sincronizando = false
            }
        }
    }

    companion object {
        /// `directorio` es el mismo que ya usa [MainActivity] para abrir la
        /// base `SQLite` (`filesDir.absolutePath`) — no un archivo, la
        /// carpeta; Android no tiene `%LOCALAPPDATA%`, así que a diferencia
        /// de escritorio acá siempre hay que pasarlo explícito.
        fun factory(
            nucleo: Nucleo,
            directorio: String,
            identificadorDispositivo: String,
            onSesionExpulsada: () -> Unit = {},
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { NubeViewModel(nucleo, directorio, identificadorDispositivo, onSesionExpulsada) }
        }
    }
}

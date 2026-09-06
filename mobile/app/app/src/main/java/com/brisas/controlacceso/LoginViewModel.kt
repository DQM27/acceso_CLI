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
import uniffi.control_acceso_mobile.UsuarioSesion

/// Dueño del estado de [PantallaLogin] y de las llamadas a [Nucleo] para
/// autenticar/cerrar sesión — ver mobile/app/ARQUITECTURA.md.
///
/// `Nucleo.autenticar` intenta una sincronización corta contra la nube
/// antes de confirmar el login (ver su doc-comment en Rust: trae una
/// baja/desactivación reciente si hay señal, sigue con lo local si no) —
/// por eso, a diferencia de antes, sí hace falta despachar a
/// `Dispatchers.Default` como cualquier otra llamada con red de por medio
/// (mismo criterio que `ActivosViewModel`), en vez de llamarlo directo
/// desde el hilo de UI.
class LoginViewModel(
    private val nucleo: Nucleo,
    private val directorio: String,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.Default,
) : ViewModel() {
    var cedula by mutableStateOf("")
        private set
    var password by mutableStateOf("")
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var autenticando by mutableStateOf(false)
        private set
    var sesion by mutableStateOf<UsuarioSesion?>(null)
        private set

    /// Cédula que necesita fijar contraseña en este teléfono por primera
    /// vez (`NucleoException.SinPasswordLocal`) -- `null` es el estado
    /// normal (formulario de login); con esto puesto, [PantallaLogin]
    /// muestra el formulario de "fijar contraseña" en su lugar.
    var cedulaSinPassword by mutableStateOf<String?>(null)
        private set

    fun cambiarCedula(nueva: String) {
        cedula = nueva
    }

    fun cambiarPassword(nueva: String) {
        password = nueva
    }

    fun autenticar() {
        error = null
        autenticando = true
        viewModelScope.launch {
            try {
                sesion = withContext(dispatcherIO) { nucleo.autenticar(cedula, password, directorio) }
            } catch (excepcion: NucleoException.SinPasswordLocal) {
                cedulaSinPassword = cedula
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                autenticando = false
            }
        }
    }

    /// Completa el alta de contraseña tras `cedulaSinPassword` -- sin
    /// contraseña anterior a propósito, nunca existió una en este
    /// teléfono (ver `Nucleo.fijarPasswordInicial`).
    fun fijarPasswordInicial(nuevaPassword: String) {
        val cedulaObjetivo = cedulaSinPassword ?: return
        error = null
        autenticando = true
        viewModelScope.launch {
            try {
                sesion = withContext(dispatcherIO) {
                    nucleo.fijarPasswordInicial(cedulaObjetivo, nuevaPassword)
                }
                cedulaSinPassword = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                autenticando = false
            }
        }
    }

    fun cancelarFijarPassword() {
        cedulaSinPassword = null
        error = null
    }

    /// Sólo olvida el actor en memoria — el `Nucleo`/la conexión SQLite del
    /// teléfono se quedan abiertos (son de la Activity, no de la sesión) —
    /// mismo criterio que `Nucleo::cerrar_sesion` del lado de Rust.
    fun cerrarSesion() {
        nucleo.cerrarSesion()
        sesion = null
        cedula = ""
        password = ""
    }

    companion object {
        fun factory(nucleo: Nucleo, directorio: String): ViewModelProvider.Factory = viewModelFactory {
            initializer { LoginViewModel(nucleo, directorio) }
        }
    }
}

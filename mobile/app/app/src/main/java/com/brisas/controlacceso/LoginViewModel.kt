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
/// `Nucleo.autenticar` confirma en vivo (una consulta puntual, no una
/// sincronización completa -- ver su doc-comment en Rust) que la cuenta
/// sigue activa antes de dejar entrar, así que sigue con red de por medio y
/// hace falta despachar a `Dispatchers.Default` (mismo criterio que
/// `ActivosViewModel`) en vez de llamarlo directo desde el hilo de UI.
/// La sincronización completa (cola, catálogo, historial...) ya no la
/// dispara `autenticar` -- se lanza acá, aparte, sin que el login la
/// espere (`lanzarSincronizacionDeFondo`): antes retenerla adentro del
/// login era la causa real del retraso de "un par de segundos" al entrar.
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
                lanzarSincronizacionDeFondo()
            } catch (excepcion: NucleoException.SinPasswordLocal) {
                cedulaSinPassword = cedula
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                autenticando = false
            }
        }
    }

    /// Sincronización completa (cola de salida, catálogo, historial...)
    /// disparada al loguearse, sin que `autenticar()` la espere -- ver el
    /// doc-comment de la clase. Fire-and-forget real: ni actualiza estado
    /// propio ni reporta error acá (el pulso periódico de
    /// `SincronizacionPeriodica`, que arranca apenas se monta la pantalla
    /// principal, retoma la sincronización normal enseguida de todos
    /// modos).
    private fun lanzarSincronizacionDeFondo() {
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) { nucleo.sincronizarConNube(directorio) }
            } catch (_: NucleoException) {
                // Sin red, o sin secreto configurado todavía -- no es un
                // error que el login deba mostrar, el pulso periódico
                // reintenta solo.
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

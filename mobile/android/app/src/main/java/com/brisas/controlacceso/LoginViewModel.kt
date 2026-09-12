package com.brisas.controlacceso

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.ResultadoLogin
import uniffi.control_acceso_mobile.UsuarioSesion

/// Dueño del estado de [PantallaLogin] y de las llamadas a [Nucleo] para
/// autenticar/cerrar sesión — ver mobile/android/ARQUITECTURA.md.
///
/// `Nucleo.autenticar` confirma en vivo (una consulta puntual, no una
/// sincronización completa -- ver su doc-comment en Rust) que la cuenta
/// sigue activa antes de dejar entrar, así que sigue con red de por medio y
/// hace falta despachar a `Dispatchers.IO` (mismo criterio que
/// `ActivosViewModel`) en vez de llamarlo directo desde el hilo de UI.
/// La sincronización completa (cola, catálogo, historial...) ya no la
/// dispara `autenticar` -- se lanza acá, aparte, sin que el login la
/// espere (`lanzarSincronizacionDeFondo`): antes retenerla adentro del
/// login era la causa real del retraso de "un par de segundos" al entrar.
class LoginViewModel(
    private val nucleo: Nucleo,
    private val secretoStore: SecretoDispositivoStore,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
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
    var propietarioSesion by mutableStateOf<PropietarioSesion?>(null)
        private set

    /// Sesión recién autenticada contra Supabase Auth con
    /// `debe_cambiar_password = true` (contraseña temporal de un solo uso,
    /// ver docs/planes-implementados/plan-autenticacion-supabase-auth.md) -- `null` es el estado
    /// normal; con esto puesto, [PantallaLogin] muestra el paso de cambio
    /// obligatorio en vez de dejar entrar. `passwordActual` es la temporal
    /// que recién tipeó, hace falta para que `cambiarPasswordSupabase`
    /// revalide del lado del backend antes de aceptar la nueva.
    var cambioObligatorio by mutableStateOf<Pair<UsuarioSesion, String>?>(null)
        private set

    fun cambiarCedula(nueva: String) {
        cedula = nueva
    }

    fun cambiarPassword(nueva: String) {
        password = nueva
    }

    fun autenticar() {
        if (autenticando || cedula.isBlank() || password.isBlank()) return
        error = null
        autenticando = true
        val passwordTipeada = password
        viewModelScope.launch {
            try {
                val resultado: ResultadoLogin = withContext(dispatcherIO) {
                    val secreto = secretoStore.cargar().orEmpty()
                    nucleo.autenticarConSecreto(cedula, passwordTipeada, secreto)
                }
                if (resultado.debeCambiarPassword) {
                    cambioObligatorio = resultado.sesion to passwordTipeada
                    return@launch
                }
                abrirSesion(resultado.sesion)
                lanzarSincronizacionDeFondo()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoStoreException) {
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
                withContext(dispatcherIO) {
                    val secreto = secretoStore.cargar() ?: return@withContext
                    nucleo.sincronizarConNubeConSecreto(secreto)
                }
            } catch (_: NucleoException) {
                // Sin red, o sin secreto configurado todavía -- no es un
                // error que el login deba mostrar, el pulso periódico
                // reintenta solo.
            }
        }
    }

    /// Completa el cambio obligatorio tras `cambioObligatorio` --
    /// `cambiarPasswordSupabase` ya revalida la temporal contra Supabase
    /// antes de aceptar la nueva, la sesión local ya está abierta desde
    /// `autenticar()` (esto no vuelve a autenticar, sólo cambia la
    /// contraseña).
    fun completarCambioObligatorio(passwordNueva: String) {
        if (autenticando) return
        val (sesionPendiente, passwordActual) = cambioObligatorio ?: return
        error = null
        autenticando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    nucleo.cambiarPasswordSupabase(passwordActual, passwordNueva)
                }
                abrirSesion(sesionPendiente)
                cambioObligatorio = null
                lanzarSincronizacionDeFondo()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                autenticando = false
            }
        }
    }

    fun cancelarCambioObligatorio() {
        cambioObligatorio = null
        error = null
    }

    /// Sólo olvida el actor en memoria — el `Nucleo`/la conexión SQLite del
    /// teléfono se quedan abiertos (son de la Activity, no de la sesión) —
    /// mismo criterio que `Nucleo::cerrar_sesion` del lado de Rust.
    fun cerrarSesion() {
        propietarioSesion?.cerrar()
        propietarioSesion = null
        nucleo.cerrarSesion()
        sesion = null
        cedula = ""
        password = ""
    }

    private fun abrirSesion(sesionNueva: UsuarioSesion) {
        propietarioSesion?.cerrar()
        propietarioSesion = PropietarioSesion()
        sesion = sesionNueva
        password = ""
    }

    override fun onCleared() {
        propietarioSesion?.cerrar()
        super.onCleared()
    }

    companion object {
        fun factory(
            nucleo: Nucleo,
            secretoStore: SecretoDispositivoStore,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { LoginViewModel(nucleo, secretoStore) }
        }
    }
}

class PropietarioSesion : ViewModelStoreOwner {
    override val viewModelStore = ViewModelStore()
    fun cerrar() = viewModelStore.clear()
}

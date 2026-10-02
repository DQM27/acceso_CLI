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
import uniffi.control_acceso_mobile.UsuarioSesion

/// Dueño del estado de [PantallaLogin] y de las llamadas a [Nucleo] para
/// autenticar/cerrar sesión — ver mobile/android/arquitectura.md.
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
                // Una contraseña temporal se rechaza en el núcleo con un
                // mensaje que manda a cambiarla en escritorio: el celular no
                // cambia contraseñas.
                val sesionNueva = withContext(dispatcherIO) {
                    medirNucleo("autenticar") { nucleo.autenticar(cedula, passwordTipeada) }
                }
                abrirSesion(sesionNueva)
                lanzarSincronizacionDeFondo()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
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
                    // Sin vincular: nada que sincronizar.
                    if (!nucleo.nubeConfigurada()) return@withContext
                    val resumen = medirNucleo("sincronizarConNube") { nucleo.sincronizarConNube() }
                    informarDiagnosticoSincronizacion(nucleo, resumen)
                }
            } catch (_: NucleoException) {
                // Sin red, o sin vincular todavía -- no es un
                // error que el login deba mostrar, el pulso periódico
                // reintenta solo.
            }
        }
    }

    /// Sólo olvida el actor en memoria — el `Nucleo`/la conexión SQLite del
    /// teléfono se quedan abiertos (son de la Activity, no de la sesión) —
    /// mismo criterio que `Nucleo::cerrar_sesion` del lado de Rust.
    fun cerrarSesion() {
        val cedulaSaliente = sesion?.cedula
        propietarioSesion?.cerrar()
        propietarioSesion = null
        medirNucleo("cerrarSesion") { nucleo.cerrarSesion() }
        // Avisa a la nube (sesión única por unidad y bitácora del panel) sin
        // demorar la salida: hace red, así que va en segundo plano y nunca
        // falla hacia la pantalla.
        if (cedulaSaliente != null) {
            viewModelScope.launch(dispatcherIO) {
                runCatching { nucleo.cerrarSesionEnLaNube(cedulaSaliente) }
            }
        }
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
        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { LoginViewModel(nucleo) }
        }
    }
}

class PropietarioSesion : ViewModelStoreOwner {
    override val viewModelStore = ViewModelStore()
    fun cerrar() = viewModelStore.clear()
}

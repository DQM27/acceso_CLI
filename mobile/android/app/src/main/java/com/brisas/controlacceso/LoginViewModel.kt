package com.brisas.controlacceso

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
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
import uniffi.control_acceso_mobile.RevisionPasswordNueva
import uniffi.control_acceso_mobile.UsuarioSesion
import uniffi.control_acceso_mobile.revisarPasswordNueva

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
    /// Error que no es de un campo (sin red, cuenta inactiva...): va en el
    /// aviso de arriba del formulario.
    var error by mutableStateOf<String?>(null)
        private set
    /// Cédula o contraseña incorrecta: va debajo del campo de contraseña.
    var errorCredenciales by mutableStateOf<String?>(null)
        private set
    /// Sube con cada intento rechazado: la pantalla sacude el formulario.
    var intentosFallidos by mutableIntStateOf(0)
        private set
    var autenticando by mutableStateOf(false)
        private set
    var sesion by mutableStateOf<UsuarioSesion?>(null)
        private set
    var propietarioSesion by mutableStateOf<PropietarioSesion?>(null)
        private set

    /// Paso de la contraseña temporal: la que tipeó era la del alta y antes
    /// de entrar tiene que elegir una nueva (`Nucleo.cambiarPasswordTemporal`).
    var cambiandoTemporal by mutableStateOf(false)
        private set
    var passwordNueva by mutableStateOf("")
        private set
    var confirmacion by mutableStateOf("")
        private set
    var revision by mutableStateOf(revisar("", "", ""))
        private set
    var guardandoNueva by mutableStateOf(false)
        private set
    var errorCambio by mutableStateOf<String?>(null)
        private set
    /// La temporal recién tipeada: el núcleo la revalida contra Supabase
    /// antes de aceptar la nueva. Sólo en memoria, y se borra al salir del paso.
    private var passwordTemporal = ""

    fun cambiarCedula(nueva: String) {
        cedula = nueva
        errorCredenciales = null
    }

    fun cambiarPassword(nueva: String) {
        password = nueva
        errorCredenciales = null
    }

    fun autenticar() {
        if (autenticando || cedula.isBlank() || password.isBlank()) return
        error = null
        errorCredenciales = null
        autenticando = true
        val passwordTipeada = password
        viewModelScope.launch {
            try {
                val sesionNueva = withContext(dispatcherIO) {
                    medirNucleo("autenticar") { nucleo.autenticar(cedula, passwordTipeada) }
                }
                abrirSesion(sesionNueva)
                lanzarSincronizacionDeFondo()
            } catch (_: NucleoException.PasswordTemporal) {
                empezarCambioTemporal(passwordTipeada)
            } catch (excepcion: NucleoException.CredencialesInvalidas) {
                errorCredenciales = excepcion.message
                intentosFallidos++
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
                intentosFallidos++
            } finally {
                autenticando = false
            }
        }
    }

    private fun empezarCambioTemporal(temporal: String) {
        passwordTemporal = temporal
        password = ""
        passwordNueva = ""
        confirmacion = ""
        errorCambio = null
        revision = revisar(temporal, "", "")
        cambiandoTemporal = true
    }

    fun cambiarPasswordNueva(nueva: String) {
        passwordNueva = nueva
        errorCambio = null
        revision = revisar(passwordTemporal, passwordNueva, confirmacion)
    }

    fun cambiarConfirmacion(nueva: String) {
        confirmacion = nueva
        errorCambio = null
        revision = revisar(passwordTemporal, passwordNueva, confirmacion)
    }

    /// Cambia la temporal por la nueva y entra directo, sin volver a pedir
    /// la contraseña.
    fun guardarPasswordNueva() {
        if (guardandoNueva || !revision.valida) return
        errorCambio = null
        guardandoNueva = true
        val nueva = passwordNueva
        val repetida = confirmacion
        viewModelScope.launch {
            try {
                val sesionNueva = withContext(dispatcherIO) {
                    medirNucleo("cambiarPasswordTemporal") {
                        nucleo.cambiarPasswordTemporal(cedula, passwordTemporal, nueva, repetida)
                    }
                }
                salirDelCambio()
                abrirSesion(sesionNueva)
                lanzarSincronizacionDeFondo()
            } catch (excepcion: Exception) {
                errorCambio = excepcion.mensajeDeErrorEsperado()
            } finally {
                guardandoNueva = false
            }
        }
    }

    /// "Volver" desde el paso de la temporal: queda en el login con la cédula
    /// ya puesta.
    fun cancelarCambioTemporal() {
        if (guardandoNueva) return
        salirDelCambio()
    }

    private fun salirDelCambio() {
        cambiandoTemporal = false
        passwordTemporal = ""
        passwordNueva = ""
        confirmacion = ""
        errorCambio = null
        revision = revisar("", "", "")
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
        error = null
        errorCredenciales = null
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

/// Los tres requisitos a la vez (mismo atajo que `RevisionPasswordNueva::valida`
/// en el núcleo, que no cruza la frontera FFI por ser un método).
val RevisionPasswordNueva.valida: Boolean
    get() = largoSuficiente && distintaDeLaTemporal && coinciden

/// Las reglas de la contraseña nueva las decide el núcleo.
private fun revisar(temporal: String, nueva: String, confirmacion: String): RevisionPasswordNueva =
    revisarPasswordNueva(temporal, nueva, confirmacion)

class PropietarioSesion : ViewModelStoreOwner {
    override val viewModelStore = ViewModelStore()
    fun cerrar() = viewModelStore.clear()
}

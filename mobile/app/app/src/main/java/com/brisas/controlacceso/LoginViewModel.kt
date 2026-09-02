package com.brisas.controlacceso

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.UsuarioSesion

/// Dueño del estado de [PantallaLogin] y de las llamadas a [Nucleo] para
/// autenticar/cerrar sesión — ver mobile/app/ARQUITECTURA.md.
///
/// Sin corrutina propia a propósito: `Nucleo.autenticar`/`cerrarSesion` son
/// llamadas síncronas a SQLite local (sin red de por medio, ver
/// docs/plan-app-movil.md), así que no hace falta `viewModelScope.launch`
/// para esto — a diferencia de `ActivosViewModel`, que sí despacha a
/// `Dispatchers.Default` porque encadena varias consultas más pesadas.
class LoginViewModel(private val nucleo: Nucleo) : ViewModel() {
    var cedula by mutableStateOf("")
        private set
    var password by mutableStateOf("")
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var sesion by mutableStateOf<UsuarioSesion?>(null)
        private set

    fun cambiarCedula(nueva: String) {
        cedula = nueva
    }

    fun cambiarPassword(nueva: String) {
        password = nueva
    }

    fun autenticar() {
        error = null
        try {
            sesion = nucleo.autenticar(cedula, password)
        } catch (excepcion: NucleoException) {
            error = excepcion.message
        }
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
        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { LoginViewModel(nucleo) }
        }
    }
}

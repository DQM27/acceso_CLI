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

/// Dueño del estado de [PantallaPrimerArranque] -- ver el doc-comment de esa
/// pantalla. Un solo intento a la vez, sin reintento automático: si el
/// código es inválido o no hay red, la persona lo ve y decide si reintenta.
///
/// No guarda nada: el código es de un solo uso y no es una credencial. Al
/// canjearlo, el núcleo genera la clave del teléfono en Android Keystore
/// (ver `AlmacenClaveKeystore`) y sólo manda la pública.
class PrimerArranqueViewModel(
    private val nucleo: Nucleo,
    private val metadata: MetadatosDispositivoLocal,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var vinculando by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    /// Bajando los datos de la unidad de un teléfono que ya estaba
    /// vinculado (ver [prepararSiYaEstaVinculado]).
    var preparando by mutableStateOf(false)
        private set

    /// Un teléfono que YA está vinculado (su clave sigue en Android
    /// Keystore) no necesita código: pasa al cambiar de unidad, cuando el
    /// núcleo borró la base de la anterior al arrancar. Baja los datos de
    /// su unidad solo; si falla, queda el formulario del código con el
    /// motivo.
    fun prepararSiYaEstaVinculado(onListo: () -> Unit) {
        if (preparando || vinculando || !nucleo.nubeConfigurada()) return
        error = null
        preparando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    medirNucleo("prepararEquipoVinculado") { nucleo.prepararEquipoVinculado() }
                }
                onListo()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                preparando = false
            }
        }
    }

    fun vincular(codigo: String, onListo: () -> Unit) {
        if (vinculando || !CodigoVinculacion.completo(codigo)) return
        error = null
        vinculando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    medirNucleo("vincularDispositivoInicial") {
                        nucleo.vincularDispositivoInicial(
                            codigo = CodigoVinculacion.normalizar(codigo),
                            identificadorHardware = metadata.identificadorHardware,
                            nombreDispositivo = metadata.nombreDispositivo,
                            plataforma = metadata.plataforma,
                            versionBuild = metadata.versionBuild,
                            appVersion = metadata.appVersion,
                        )
                    }
                }
                onListo()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                vinculando = false
            }
        }
    }

    companion object {
        fun factory(
            nucleo: Nucleo,
            metadata: MetadatosDispositivoLocal,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { PrimerArranqueViewModel(nucleo, metadata) }
        }
    }
}

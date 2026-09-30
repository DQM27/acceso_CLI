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

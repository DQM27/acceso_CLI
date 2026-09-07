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

/// Dueño del estado de [PantallaPrimerArranque] -- ver el doc-comment de esa
/// pantalla. Un solo intento a la vez, sin reintento automático: si el
/// secreto es inválido o no hay red, la persona lo ve y decide si reintenta.
class PrimerArranqueViewModel(
    private val nucleo: Nucleo,
    private val directorio: String,
    private val identificadorDispositivo: String,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var conectando by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    fun conectar(secreto: String, onListo: () -> Unit) {
        if (secreto.isBlank()) return
        error = null
        conectando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    nucleo.configurarDispositivoInicial(directorio, identificadorDispositivo, secreto)
                }
                onListo()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                conectando = false
            }
        }
    }

    companion object {
        fun factory(
            nucleo: Nucleo,
            directorio: String,
            identificadorDispositivo: String,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { PrimerArranqueViewModel(nucleo, directorio, identificadorDispositivo) }
        }
    }
}

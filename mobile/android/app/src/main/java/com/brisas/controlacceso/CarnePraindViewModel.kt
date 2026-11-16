package com.brisas.controlacceso

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.CarnePraind
import uniffi.control_acceso_mobile.EstadoCarnePraind
import uniffi.control_acceso_mobile.Nucleo

/// Lo que el ViewModel necesita del núcleo (en las pruebas, un doble).
interface FuenteCarnes {
    fun estado(contratistaId: Long): EstadoCarnePraind
    fun ver(contratistaId: Long): CarnePraind
}

private class FuenteCarnesNucleo(private val nucleo: Nucleo) : FuenteCarnes {
    override fun estado(contratistaId: Long) = nucleo.estadoCarnePraind(contratistaId)
    override fun ver(contratistaId: Long) = nucleo.verCarnePraind(contratistaId)
}

sealed interface VistaCarne {
    data object Cerrada : VistaCarne
    data class Cargando(val nombre: String) : VistaCarne
    /// Los bytes de la imagen tal cual (PNG o JPEG); la pantalla los decodifica.
    data class Imagen(val nombre: String, val bytes: ByteArray) : VistaCarne {
        override fun equals(other: Any?) = other is Imagen && other.nombre == nombre && other.bytes.contentEquals(bytes)
        override fun hashCode() = 31 * nombre.hashCode() + bytes.contentHashCode()
    }
    data class Error(val nombre: String, val mensaje: String) : VistaCarne
}

/// Carné PRAIND del contratista que se está por registrar (fase 6 de
/// docs/features-futuras/plan-fotos-praind.md). El carné se baja recién al
/// pedirlo y queda guardado en el teléfono: la segunda vez sale al instante
/// y sin internet. Sin internet y sin haberlo visto antes, el núcleo devuelve
/// el aviso de que hace falta conexión.
class CarnePraindViewModel(
    private val fuente: FuenteCarnes,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    /// Si el contratista de la hoja abierta tiene carné (muestra "Ver carné").
    private val _disponible = MutableStateFlow(false)
    val disponible: StateFlow<Boolean> = _disponible.asStateFlow()

    private val _vista = MutableStateFlow<VistaCarne>(VistaCarne.Cerrada)
    val vista: StateFlow<VistaCarne> = _vista.asStateFlow()

    private var consulta: Job? = null
    private var carga: Job? = null

    /// Consulta local (sin red) al abrir la hoja de un contratista.
    fun consultar(contratistaId: Long?) {
        consulta?.cancel()
        _disponible.value = false
        if (contratistaId == null) return
        consulta = viewModelScope.launch {
            _disponible.value = withContext(dispatcherIO) {
                try {
                    fuente.estado(contratistaId).tieneCarne
                } catch (excepcion: Exception) {
                    excepcion.mensajeDeErrorEsperado()
                    false
                }
            }
        }
    }

    fun abrir(contratistaId: Long, nombre: String) {
        carga?.cancel()
        _vista.value = VistaCarne.Cargando(nombre)
        carga = viewModelScope.launch {
            _vista.value = withContext(dispatcherIO) {
                try {
                    VistaCarne.Imagen(nombre, fuente.ver(contratistaId).bytes)
                } catch (excepcion: Exception) {
                    VistaCarne.Error(nombre, excepcion.mensajeDeErrorEsperado() ?: "No se pudo mostrar el carné.")
                }
            }
        }
    }

    fun cerrar() {
        carga?.cancel()
        _vista.value = VistaCarne.Cerrada
    }

    companion object {
        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { CarnePraindViewModel(FuenteCarnesNucleo(nucleo)) }
        }
    }
}

package com.brisas.controlacceso

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.EstadoBandeja

/// Dueño del estado de la campanita y de PantallaAvisos. Lo que se muestra
/// lo decide el núcleo (`BandejaAvisos`); AlmacenBandejaAvisos lo guarda y
/// lo publica. Las llamadas que pueden tocar el disco van en `dispatcherIO`.
class AvisosViewModel(
    private val contexto: Context,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    /// `null` mientras se lee el disco por primera vez.
    val estado: StateFlow<EstadoBandeja?> = AlmacenBandejaAvisos.estado

    /// Tocaron la notificación de un aviso: hay que abrir la bandeja.
    val aperturaSolicitada: StateFlow<Boolean> = AlmacenBandejaAvisos.aperturaSolicitada

    init {
        viewModelScope.launch(dispatcherIO) { AlmacenBandejaAvisos.cargar(contexto) }
    }

    fun leer(id: String) {
        viewModelScope.launch(dispatcherIO) { AlmacenBandejaAvisos.leerAviso(contexto, id) }
    }

    fun leerTodos() {
        viewModelScope.launch(dispatcherIO) { AlmacenBandejaAvisos.leerTodos(contexto) }
    }

    /// Mientras la bandeja está a la vista: que "hace 5 min" avance.
    fun refrescar() {
        AlmacenBandejaAvisos.recalcular()
    }

    fun aperturaAtendida() {
        AlmacenBandejaAvisos.aperturaAtendida()
    }

    companion object {
        fun factory(contexto: Context): ViewModelProvider.Factory = viewModelFactory {
            initializer { AvisosViewModel(contexto.applicationContext) }
        }
    }
}

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
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import uniffi.control_acceso_mobile.EstadoBandeja
import uniffi.control_acceso_mobile.Nucleo

/// Dueño del estado de la campanita y de PantallaAvisos. Lo que se muestra y
/// qué se puede archivar o borrar lo decide el núcleo (`BandejaAvisos`);
/// AlmacenBandejaAvisos lo guarda y lo publica. Las llamadas que tocan el
/// disco o la red van en `dispatcherIO`.
///
/// Archivar y borrar se cuentan al servidor para que el panel muestre quién
/// fue. Si no hay red queda pendiente en el núcleo y se reintenta al abrir la
/// pantalla principal y con la próxima acción.
class AvisosViewModel(
    private val contexto: Context,
    private val nucleo: Nucleo,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    /// `null` mientras se lee el disco por primera vez.
    val estado: StateFlow<EstadoBandeja?> = AlmacenBandejaAvisos.estado

    /// Tocaron la notificación de un aviso: hay que abrir la bandeja.
    val aperturaSolicitada: StateFlow<Boolean> = AlmacenBandejaAvisos.aperturaSolicitada

    /// Una sola ronda de envío a la vez; reenviar sería inofensivo (el
    /// servidor ignora lo repetido) pero es tráfico de más.
    private val envio = Mutex()

    init {
        viewModelScope.launch(dispatcherIO) {
            AlmacenBandejaAvisos.cargar(contexto)
            enviarPendientes()
        }
    }

    fun leer(id: String) {
        viewModelScope.launch(dispatcherIO) { AlmacenBandejaAvisos.leerAviso(contexto, id) }
    }

    fun leerTodos() {
        viewModelScope.launch(dispatcherIO) { AlmacenBandejaAvisos.leerTodos(contexto) }
    }

    /// `usuario`: quien tiene la sesión abierta ahora. Se pide en cada llamada
    /// y no al crear el ViewModel, que vive más que una sesión.
    fun archivar(id: String, usuario: String) {
        viewModelScope.launch(dispatcherIO) {
            AlmacenBandejaAvisos.archivar(contexto, id, usuario)
            enviarPendientes()
        }
    }

    fun borrar(id: String, usuario: String) {
        viewModelScope.launch(dispatcherIO) {
            AlmacenBandejaAvisos.borrar(contexto, id, usuario)
            enviarPendientes()
        }
    }

    /// Mientras la bandeja está a la vista: que "hace 5 min" avance.
    fun refrescar() {
        AlmacenBandejaAvisos.recalcular()
    }

    fun aperturaAtendida() {
        AlmacenBandejaAvisos.aperturaAtendida()
    }

    private suspend fun enviarPendientes() = envio.withLock {
        val pendientes = AlmacenBandejaAvisos.accionesPendientes(contexto)
        if (pendientes.isEmpty()) return@withLock
        val enviadas = NotificacionesPush.enviarAcciones(nucleo, pendientes)
        if (enviadas.isNotEmpty()) AlmacenBandejaAvisos.confirmarAcciones(contexto, enviadas)
    }

    companion object {
        fun factory(contexto: Context, nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { AvisosViewModel(contexto.applicationContext, nucleo) }
        }
    }
}

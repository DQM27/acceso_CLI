package com.brisas.controlacceso

import android.content.Context
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import uniffi.control_acceso_mobile.AvisoPush
import uniffi.control_acceso_mobile.BandejaAvisos
import uniffi.control_acceso_mobile.EstadoBandeja

/// Dónde vive la bandeja de avisos de este teléfono. Las reglas (no
/// duplicar, vencer, contar, agrupar) son del núcleo
/// (`BandejaAvisos`, mobile/rust-core/src/avisos.rs); acá sólo se guarda en
/// disco el texto que el núcleo devuelve y se publica su estado.
///
/// Un solo objeto por proceso, compartido por ServicioMensajesPush (el push
/// llega también con la app cerrada, sin el `Nucleo` abierto) y
/// AvisosViewModel: por eso cada cambio va dentro de `synchronized`.
///
/// Un SharedPreferences privado de la app: el texto de los avisos queda sin
/// cifrar, igual que en la notificación que Android ya muestra en la barra.
object AlmacenBandejaAvisos {
    private const val PREFERENCIAS = "bandeja_avisos"
    private const val CLAVE_AVISOS = "avisos"

    private var bandeja: BandejaAvisos? = null

    /// `null` hasta la primera lectura del disco.
    private val estadoActual = MutableStateFlow<EstadoBandeja?>(null)
    val estado: StateFlow<EstadoBandeja?> = estadoActual.asStateFlow()

    /// Tocar la notificación pide abrir la bandeja (MainActivity). Queda
    /// pendiente hasta que alguien la atiende: si la sesión estaba cerrada,
    /// se abre después de iniciarla.
    private val apertura = MutableStateFlow(false)
    val aperturaSolicitada: StateFlow<Boolean> = apertura.asStateFlow()

    /// Lee el disco la primera vez. Llamar fuera del hilo principal.
    fun cargar(context: Context) {
        synchronized(this) { bandeja(context) }
    }

    fun guardar(context: Context, aviso: AvisoPush) = cambiar(context) { it.agregar(aviso, System.currentTimeMillis()) }

    fun leerAviso(context: Context, id: String) = cambiar(context) { it.leer(id) }

    fun leerTodos(context: Context) = cambiar(context) { it.leerTodos() }

    /// Vuelve a calcular el estado a la hora actual ("hace 5 min", Hoy/Ayer,
    /// lo vencido) aunque no haya cambiado nada.
    fun recalcular() {
        synchronized(this) { bandeja?.let(::publicar) }
    }

    fun solicitarApertura() {
        apertura.value = true
    }

    fun aperturaAtendida() {
        apertura.value = false
    }

    private fun cambiar(context: Context, cambio: (BandejaAvisos) -> Boolean) {
        synchronized(this) {
            val actual = bandeja(context)
            if (!cambio(actual)) return
            // `apply`: escribe en segundo plano y no traba la interfaz.
            preferencias(context).edit().putString(CLAVE_AVISOS, actual.serializar()).apply()
            publicar(actual)
        }
    }

    /// Sólo dentro de `synchronized`.
    private fun bandeja(context: Context): BandejaAvisos =
        bandeja ?: BandejaAvisos.desdeGuardado(
            preferencias(context).getString(CLAVE_AVISOS, null),
            System.currentTimeMillis(),
        ).also {
            bandeja = it
            publicar(it)
        }

    private fun publicar(bandeja: BandejaAvisos) {
        estadoActual.value = bandeja.estado(System.currentTimeMillis())
    }

    private fun preferencias(context: Context) =
        context.applicationContext.getSharedPreferences(PREFERENCIAS, Context.MODE_PRIVATE)
}

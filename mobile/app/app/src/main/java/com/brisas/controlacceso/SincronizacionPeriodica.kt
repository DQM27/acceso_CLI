package com.brisas.controlacceso

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.ResumenSincronizacion

/**
 * Sincronización pasiva de fondo mientras la app está abierta -- mismo
 * criterio que el disparador automático de escritorio
 * (`crate::iniciar_sincronizacion_automatica`, cada 2 minutos): sin esto,
 * el celular sólo sincroniza cuando alguien toca "Sincronizar" a mano.
 *
 * También atiende [CambiosNube] al guardar datos o recibir un Broadcast.
 * Serializa las ejecuciones y conserva un aviso pendiente si llega mientras
 * hay una sincronización en curso. El timer es respaldo ante desconexiones.
 */
class SincronizacionPeriodica(
    private val nucleo: Nucleo,
    private val directorio: String,
    private val scope: CoroutineScope,
    private val onSincronizado: (ResumenSincronizacion) -> Unit = {},
) {
    private var trabajo: Job? = null

    fun iniciar() {
        if (trabajo?.isActive == true) return
        trabajo = scope.launch {
            coroutineScope {
                val pendientes = Channel<Unit>(Channel.CONFLATED)
                launch { CambiosNube.cambios.collect { pendientes.trySend(Unit) } }
                withTimeoutOrNull(ESPERA_INICIAL_MS) { pendientes.receive() }
                while (true) {
                    delay(600)
                    try {
                        val resumen = withContext(Dispatchers.IO) { nucleo.sincronizarConNube(directorio) }
                        onSincronizado(resumen)
                    } catch (cancelacion: CancellationException) {
                        throw cancelacion
                    } catch (_: Throwable) {
                        // La cola local conserva lo pendiente hasta recuperar la conexión.
                    }
                    withTimeoutOrNull(INTERVALO_MS) { pendientes.receive() }
                }
            }
        }
    }

    fun detener() {
        trabajo?.cancel()
        trabajo = null
    }

    private companion object {
        const val ESPERA_INICIAL_MS = 10_000L
        const val INTERVALO_MS = 2 * 60_000L
    }
}

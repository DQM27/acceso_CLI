package com.brisas.controlacceso

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.ResumenSincronizacion

/**
 * Sincronización pasiva de fondo mientras la app está abierta -- mismo
 * criterio que el disparador automático de escritorio
 * (`crate::iniciar_sincronizacion_automatica`, cada 2 minutos): sin esto,
 * el celular sólo sincroniza cuando alguien toca "Sincronizar" a mano.
 *
 * Reemplaza a [NubeRealtime] (ese archivo queda sin usar, no se borra): el
 * canal privado de Supabase Realtime no logra autorizarse -- bug de la
 * plataforma con el sistema nuevo de JWT Signing Keys, no de este código
 * (ver docs/migracion-supabase-realtime-broadcast.sql). Reactivar
 * [NubeRealtime] en cuanto Supabase resuelva el bug (o encontremos un
 * workaround), en vez de esta clase.
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
            delay(ESPERA_INICIAL_MS)
            while (true) {
                try {
                    val resumen = nucleo.sincronizarConNube(directorio)
                    onSincronizado(resumen)
                } catch (cancelacion: CancellationException) {
                    throw cancelacion
                } catch (_: Throwable) {
                    // Sin conexión, o el secreto de este dispositivo todavía
                    // no se configuró -- se reintenta solo en la próxima
                    // vuelta, sin interrumpir a quien esté usando la app.
                }
                delay(INTERVALO_MS)
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

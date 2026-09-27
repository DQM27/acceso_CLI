package com.brisas.controlacceso

import android.util.Log

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
 *
 * Un aviso en vivo que trae su tabla corre sólo esa parte
 * (`sincronizarCambiosConSecreto`, ver `AlcanceSincronizacion` en el
 * núcleo); el pulso, un registro local y la reconexión del canal corren la
 * completa. Antes cada aviso corría la completa (~12 consultas a la nube
 * por un solo cambio) -- el aviso llegaba al instante, lo que tardaba era
 * lo que se hacía al recibirlo.
 */
class SincronizacionPeriodica(
    private val nucleo: Nucleo,
    private val secretoStore: SecretoDispositivoStore,
    private val scope: CoroutineScope,
    private val onSincronizado: (ResumenSincronizacion) -> Unit = {},
) {
    private var trabajo: Job? = null

    fun iniciar() {
        if (trabajo?.isActive == true) return
        trabajo = scope.launch {
            coroutineScope {
                val pendientes = Channel<Unit>(Channel.CONFLATED)
                val porSincronizar = PendientesSincronizacion()
                launch {
                    CambiosNube.cambios.collect { tabla ->
                        porSincronizar.anotar(tabla)
                        pendientes.trySend(Unit)
                    }
                }
                withTimeoutOrNull(ESPERA_INICIAL_MS) { pendientes.receive() }
                // La primera corrida siempre es completa.
                porSincronizar.anotar(null)
                while (true) {
                    delay(600)
                    val tablas = porSincronizar.tomar()
                    try {
                        val resumen = withContext(Dispatchers.IO) {
                            val secreto = secretoStore.cargar()
                                ?: throw SecretoDispositivoNoEncontradoException()
                            if (tablas == null) {
                                nucleo.sincronizarConNubeConSecreto(secreto)
                            } else {
                                nucleo.sincronizarCambiosConSecreto(secreto, tablas)
                            }
                        }
                        Log.i("SincronizacionNube", "Recibidos: gafetes=${resumen.gafetesRecibidos}, historial=${resumen.movimientosHistorialRecibidos}, abiertos=${resumen.remotosAbiertos}")
                        onSincronizado(resumen)
                    } catch (cancelacion: CancellationException) {
                        throw cancelacion
                    } catch (error: Throwable) {
                        Log.w("SincronizacionNube", "No se completó la sincronización: ${error.javaClass.simpleName}")
                        // La cola local conserva lo pendiente hasta recuperar la conexión.
                    }
                    // Sin avisos durante el intervalo = pulso periódico:
                    // siempre completo (es la red de seguridad para
                    // cualquier aviso perdido o sincronización parcial que
                    // haya fallado).
                    val llegoAviso = withTimeoutOrNull(INTERVALO_MS) { pendientes.receive() } != null
                    if (!llegoAviso) porSincronizar.anotar(null)
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

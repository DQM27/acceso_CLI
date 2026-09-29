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
 * núcleo); un registro local sólo sube lo pendiente
 * (`enviarCambiosConSecreto`, igual que escritorio); el pulso y la
 * reconexión del canal corren la completa. Antes cada aviso corría la
 * completa (~12 consultas a la nube por un solo cambio) -- el aviso llegaba
 * al instante, lo que tardaba era lo que se hacía al recibirlo. Lo mismo
 * pasaba al guardar: el cambio se subía al final de una completa.
 */
class SincronizacionPeriodica(
    private val nucleo: Nucleo,
    private val secretoStore: SecretoDispositivoStore,
    private val scope: CoroutineScope,
    private val onSincronizado: (ResumenSincronizacion) -> Unit = {},
) {
    private var trabajo: Job? = null

    /**
     * `inmediata`: al volver a primer plano (no en el primer arranque de la
     * sesión, donde el login ya lanzó su propia sincronización) la primera
     * corrida sale enseguida, en paralelo con la reconexión de Realtime, en
     * vez de esperar el aviso de "canal suscrito" (~0,7 s) más la pausa de
     * agrupación. Medido con telemetría: al desbloquear el teléfono el dato
     * de otro dispositivo tardaba ~3 s en verse; la corrida que dispara la
     * suscripción sigue ocurriendo después y cubre lo que cambie en el
     * medio.
     */
    fun iniciar(inmediata: Boolean = false) {
        if (trabajo?.isActive == true) return
        trabajo = scope.launch {
            coroutineScope {
                val pendientes = Channel<Unit>(Channel.CONFLATED)
                val porSincronizar = PendientesSincronizacion()
                launch {
                    CambiosNube.cambios.collect { solicitud ->
                        when (solicitud) {
                            is SolicitudNube.Remota -> porSincronizar.anotar(solicitud.tabla)
                            SolicitudNube.CambioLocal -> porSincronizar.anotarCambioLocal()
                        }
                        pendientes.trySend(Unit)
                    }
                }
                if (!inmediata) {
                    withTimeoutOrNull(ESPERA_INICIAL_MS) { pendientes.receive() }
                }
                // La primera corrida siempre es completa.
                porSincronizar.anotar(null)
                var agrupar = !inmediata
                while (true) {
                    // La pausa junta avisos remotos que llegan casi a la vez.
                    // Un cambio hecho acá sale enseguida: esperar sólo
                    // demoraba ~0,6 s que el otro equipo lo viera (lo que
                    // llegue mientras tanto va en la corrida siguiente).
                    if (agrupar && !porSincronizar.soloEnvio()) delay(PAUSA_AGRUPACION_MS)
                    agrupar = true
                    val alcance = porSincronizar.tomar()
                    try {
                        val resumen = withContext(Dispatchers.IO) {
                            val secreto = secretoStore.cargar()
                                ?: throw SecretoDispositivoNoEncontradoException()
                            when (alcance) {
                                AlcancePendiente.Completa ->
                                    medirNucleo("sincronizarConNubeConSecreto") { nucleo.sincronizarConNubeConSecreto(secreto) }
                                is AlcancePendiente.Tablas ->
                                    medirNucleo("sincronizarCambiosConSecreto") { nucleo.sincronizarCambiosConSecreto(secreto, alcance.tablas) }
                                AlcancePendiente.SoloEnvio ->
                                    medirNucleo("enviarCambiosConSecreto") { nucleo.enviarCambiosConSecreto(secreto) }
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
        /** Junta avisos que llegan casi a la vez en una sola corrida. */
        const val PAUSA_AGRUPACION_MS = 600L
        const val INTERVALO_MS = 2 * 60_000L
    }
}

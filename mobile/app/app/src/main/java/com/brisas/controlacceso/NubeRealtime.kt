package com.brisas.controlacceso

import io.github.jan.supabase.createSupabaseClient
import io.github.jan.supabase.realtime.Realtime
import io.github.jan.supabase.realtime.broadcastFlow
import io.github.jan.supabase.realtime.channel
import io.github.jan.supabase.realtime.realtime
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.launchIn
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonObject
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.ResumenSincronizacion

class NubeRealtime(
    private val nucleo: Nucleo,
    private val directorio: String,
    private val scope: CoroutineScope,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.Default,
    private val onSincronizado: (ResumenSincronizacion) -> Unit = {},
) {
    private var trabajo: Job? = null

    fun iniciar() {
        if (trabajo?.isActive == true) return
        trabajo = scope.launch {
            while (isActive) {
                val esperaTrasError = try {
                    conectarHastaRenovar()
                    2_000L
                } catch (cancelacion: CancellationException) {
                    throw cancelacion
                } catch (_: NucleoException) {
                    30_000L
                } catch (_: Throwable) {
                    30_000L
                }
                delay(esperaTrasError)
            }
        }
    }

    fun detener() {
        trabajo?.cancel()
        trabajo = null
    }

    private suspend fun conectarHastaRenovar() {
        val sesion = withContext(dispatcherIO) { nucleo.sesionRealtimeNube(directorio) }
        val token = sesion.accessToken
        val supabase = createSupabaseClient(sesion.baseUrl, sesion.apikey) {
            install(Realtime) {
                accessToken = { token }
            }
        }
        val canal = supabase.channel(sesion.topic) {
            isPrivate = true
        }

        try {
            coroutineScope {
                var sincronizacionPendiente: Job? = null
                canal.broadcastFlow<JsonObject>("cambio_nube")
                    .onEach {
                        sincronizacionPendiente?.cancel()
                        sincronizacionPendiente = launch {
                            delay(600)
                            val resumen = withContext(dispatcherIO) {
                                nucleo.sincronizarConNube(directorio)
                            }
                            onSincronizado(resumen)
                        }
                    }
                    .launchIn(this)

                canal.subscribe(blockUntilSubscribed = true)
                delay(milisegundosHastaRenovar(sesion.expiresIn))
            }
        } finally {
            withContext(NonCancellable) {
                supabase.realtime.removeChannel(canal)
                supabase.realtime.disconnect()
            }
        }
    }

    private fun milisegundosHastaRenovar(expiresIn: ULong): Long {
        val segundos = expiresIn.toLong().coerceAtLeast(60L)
        return (segundos - 60L).coerceAtLeast(60L) * 1_000L
    }
}

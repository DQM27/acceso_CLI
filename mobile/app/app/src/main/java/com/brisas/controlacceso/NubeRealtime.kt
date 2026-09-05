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
import kotlinx.serialization.json.jsonPrimitive
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException

class NubeRealtime(
    private val nucleo: Nucleo,
    private val directorio: String,
    private val scope: CoroutineScope,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.Default,
    private val onCambio: () -> Unit = { CambiosNube.solicitar() },
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
                val avisos = canal.broadcastFlow<JsonObject>("cambio_nube")
                    .onEach { payload ->
                        if (payload["dispositivo_id"]?.jsonPrimitive?.content != sesion.dispositivoId) onCambio()
                    }
                    .launchIn(this)
                try {
                    canal.subscribe(blockUntilSubscribed = true)
                    onCambio()
                    delay(milisegundosHastaRenovar(sesion.expiresIn))
                } finally {
                    // El colector infinito debe terminar para poder renovar el JWT.
                    avisos.cancel()
                }
            }
        } finally {
            withContext(NonCancellable) {
                supabase.realtime.removeChannel(canal)
                supabase.close()
            }
        }
    }

    private fun milisegundosHastaRenovar(expiresIn: ULong): Long {
        val segundos = expiresIn.toLong().coerceAtLeast(60L)
        return (segundos - 60L).coerceAtLeast(60L) * 1_000L
    }
}

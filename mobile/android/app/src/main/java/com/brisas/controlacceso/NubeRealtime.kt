package com.brisas.controlacceso

import android.util.Log

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
import kotlinx.coroutines.withTimeout
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException

class NubeRealtime(
    private val nucleo: Nucleo,
    private val secretoStore: SecretoDispositivoStore,
    private val scope: CoroutineScope,
    // Quién tiene la sesión abierta en este teléfono ahora -- viaja en el
    // mismo `track()` que ya marca el dispositivo como presente, para que
    // el panel pueda mostrar "usuarios en línea y desde dónde" sin abrir
    // una conexión nueva (ver docs/features-futuras/plan-sesion-unica-dispositivos.md). Se
    // manda la cédula, no el id local (`UsuarioSesion.id` es el rowid de
    // ESTE SQLite, no el id global de la tabla `usuarios` de Supabase que
    // ve el panel -- la cédula es la única clave que de verdad coincide en
    // los dos lados).
    private val usuarioCedula: String,
    private val usuarioNombre: String,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
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
                } catch (excepcion: NucleoException) {
                    Log.w("SincronizacionNube", "No se pudo autenticar para Realtime", excepcion)
                    30_000L
                } catch (excepcion: Throwable) {
                    // Antes esto se descartaba en silencio -- si Realtime nunca
                    // conectaba, no había ni un solo log que explicara por qué
                    // (el pulso periódico de `SincronizacionPeriodica` disimulaba
                    // el problema, todo seguía funcionando pero sin la parte en
                    // vivo).
                    Log.w("SincronizacionNube", "Fallo conectando el canal de Realtime", excepcion)
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
        val sesion = withContext(dispatcherIO) {
            val secreto = secretoStore.cargar() ?: throw SecretoDispositivoNoEncontradoException()
            nucleo.sesionRealtimeNubeConSecreto(secreto)
        }
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
                    .onEach {
                        // El dispositivo del payload es el origen del registro, no
                        // necesariamente quien lo modificó desde el panel web.
                        Log.i("SincronizacionNube", "Aviso remoto recibido; solicitando descarga")
                        onCambio()
                    }
                    .launchIn(this)
                try {
                    canal.subscribe(blockUntilSubscribed = true)
                    Log.i("SincronizacionNube", "Canal de avisos suscrito")
                    // Presencia (docs/features-futuras/plan-sesion-unica-dispositivos.md,
                    // "Panel de presencia en tiempo real"): marca este
                    // dispositivo como conectado mientras dure la
                    // suscripción -- no hace falta "untrack" explícito, al
                    // cerrar el canal/socket (ver el `finally` de más abajo,
                    // o simplemente `ON_STOP` del ciclo de vida) el propio
                    // servidor de Realtime lo saca de la lista de
                    // presentes. Costo de red/batería: cero extra, viaja
                    // sobre esta misma conexión.
                    canal.track(
                        buildJsonObject {
                            put("dispositivo_id", sesion.dispositivoId)
                            put("usuario_cedula", usuarioCedula)
                            put("usuario_nombre", usuarioNombre)
                        },
                    )
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

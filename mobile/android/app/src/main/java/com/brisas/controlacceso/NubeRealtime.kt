package com.brisas.controlacceso

import android.os.SystemClock
import android.util.Log

import io.github.jan.supabase.createSupabaseClient
import io.github.jan.supabase.realtime.Realtime
import io.github.jan.supabase.realtime.RealtimeChannel
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
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.launchIn
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
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
    // `tabla` = la que cambió según el aviso (`payload.table`), o `null` =
    // sincronizar todo (al (re)suscribirse, para recuperar lo perdido).
    private val onCambio: (tabla: String?) -> Unit = { tabla -> CambiosNube.solicitar(tabla) },
    // Se guardó en la base local la fila que trajo un aviso en vivo: la
    // pantalla puede refrescarse sin esperar la sincronización.
    private val onCambioAplicado: () -> Unit = {},
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
                    Telemetria.realtime?.error(excepcion.javaClass.simpleName)
                    30_000L
                } catch (excepcion: Throwable) {
                    // Antes esto se descartaba en silencio -- si Realtime nunca
                    // conectaba, no había ni un solo log que explicara por qué
                    // (el pulso periódico de `SincronizacionPeriodica` disimulaba
                    // el problema, todo seguía funcionando pero sin la parte en
                    // vivo).
                    Log.w("SincronizacionNube", "Fallo conectando el canal de Realtime", excepcion)
                    Telemetria.realtime?.error(excepcion.javaClass.simpleName)
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
        val inicioIntento = SystemClock.elapsedRealtime()
        Telemetria.realtime?.conectando()
        val sesion = withContext(dispatcherIO) {
            val secreto = secretoStore.cargar() ?: throw SecretoDispositivoNoEncontradoException()
            medirNucleo("sesionRealtimeNubeConSecreto") { nucleo.sesionRealtimeNubeConSecreto(secreto) }
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
                    .onEach { aviso ->
                        val ecoPropio = aviso.texto("dispositivo_id") == sesion.dispositivoId
                        Telemetria.realtime?.aviso(
                            tabla = aviso.texto("table"),
                            bytesAviso = aviso.toString().length,
                            ecoPropio = ecoPropio,
                            latenciaMs = aviso.texto("changed_at")?.let(::msDesde),
                        )
                        // `dispositivo_id` es quien hizo ESTE cambio (el
                        // `sub` de su JWT, ver la migración
                        // `avisa_cambio_nube_segun_quien_escribe_no_quien_creo_la_fila`):
                        // el eco de un cambio propio ya está en la base
                        // local, no hay nada que bajar -- mismo filtro que
                        // `nubeRealtime.ts` en escritorio.
                        if (ecoPropio) return@onEach
                        val tabla = aviso.texto("table")
                        // El aviso trae la fila: se guarda al instante (el
                        // núcleo decide qué hacer con ella). La descarga por
                        // tabla corre igual detrás, como red de seguridad.
                        if (aviso["registro"] is JsonObject || aviso.texto("operation") == "DELETE") {
                            val inicioAplicar = System.nanoTime()
                            val aplicado = try {
                                withContext(dispatcherIO) { medirNucleo("aplicarCambioNube") { nucleo.aplicarCambioNube(aviso.toString()) } }
                            } catch (excepcion: NucleoException) {
                                Log.w("SincronizacionNube", "No se pudo aplicar el cambio en vivo", excepcion)
                                false
                            }
                            Telemetria.realtime?.aplicado(aplicado, System.nanoTime() - inicioAplicar)
                            if (aplicado) onCambioAplicado()
                        }
                        Log.i("SincronizacionNube", "Aviso remoto recibido (${tabla ?: "sin tabla"}); solicitando descarga")
                        onCambio(tabla)
                    }
                    .launchIn(this)
                try {
                    // Con límite: si el `phx_join` falla (token vencido,
                    // reconexión sin red), `realtime-kt` 3.2.2 NO avisa --
                    // `subscribe(blockUntilSubscribed = true)` se quedaba
                    // esperando para siempre y el celular quedaba sin
                    // avisos en vivo hasta pasar a segundo plano y volver.
                    // `withTimeoutOrNull`, no `withTimeout`: su excepción es
                    // una `CancellationException` y el bucle de `iniciar`
                    // la re-lanzaría en vez de reintentar.
                    withTimeoutOrNull(ESPERA_SUSCRIPCION_MS) { canal.subscribe(blockUntilSubscribed = true) }
                        ?: throw IllegalStateException("El canal de avisos no confirmó la suscripción")
                    Log.i("SincronizacionNube", "Canal de avisos suscrito")
                    val suscritoDesde = SystemClock.elapsedRealtime()
                    Telemetria.realtime?.suscrito(suscritoDesde - inicioIntento)
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
                    onCambio(null)
                    // Espera hasta la renovación del token O hasta que el
                    // canal deje de estar suscrito, lo que pase primero.
                    // `realtime-kt` no reacciona a un cierre del canal por
                    // parte del servidor (token vencido, reinicio del nodo:
                    // `phx_close` con el socket todavía vivo) ni a una
                    // re-suscripción fallida tras perder la red -- el estado
                    // pasa a UNSUBSCRIBED/SUBSCRIBING y ahí se queda. Antes
                    // esto era un `delay` ciego de casi 12 h: el canal podía
                    // estar muerto casi todo ese tiempo y sólo andaba el
                    // pulso de 2 minutos. Ahora se reconecta con sesión y
                    // token nuevos.
                    val seCayo = withTimeoutOrNull(milisegundosHastaRenovar(sesion.expiresIn)) {
                        canal.status.first { it != RealtimeChannel.Status.SUBSCRIBED }
                    }
                    if (seCayo != null) {
                        Log.w("SincronizacionNube", "El canal de avisos se cayó ($seCayo); reconectando")
                    }
                    Telemetria.realtime?.terminado(
                        motivo = seCayo?.name ?: "renovacion",
                        msConectadoAhora = SystemClock.elapsedRealtime() - suscritoDesde,
                    )
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

    /// Milisegundos desde `instanteIso` (hora del servidor) hasta ahora (hora
    /// del teléfono). `null` si no se puede leer la fecha.
    private fun msDesde(instanteIso: String): Long? = try {
        System.currentTimeMillis() - java.time.OffsetDateTime.parse(instanteIso).toInstant().toEpochMilli()
    } catch (e: java.time.format.DateTimeParseException) {
        null
    }

    private fun JsonObject.texto(clave: String): String? =
        (this[clave] as? JsonPrimitive)?.takeIf { it.isString }?.content

    private companion object {
        const val ESPERA_SUSCRIPCION_MS = 15_000L
    }

    private fun milisegundosHastaRenovar(expiresIn: ULong): Long {
        val segundos = expiresIn.toLong().coerceAtLeast(60L)
        return (segundos - 60L).coerceAtLeast(60L) * 1_000L
    }
}

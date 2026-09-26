package com.brisas.controlacceso

import android.util.Log

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.ObservadorRealtimeNube
import uniffi.control_acceso_mobile.ProveedorTokenRealtimeNube
import uniffi.control_acceso_mobile.TareaRealtimeNube
import uniffi.control_acceso_mobile.TokenRealtimeNube
import uniffi.control_acceso_mobile.iniciarRealtimeNube as iniciarRealtimeNubeFfi

/**
 * Canal privado real de Supabase Realtime -- desde 2026-09-26, ya no abre
 * su propio cliente `io.github.jan.supabase.realtime`: sólo arranca/para
 * el cliente Rust (`mobile/rust-core/src/realtime_nube.rs`, cliente
 * Phoenix Channels escrito a medida: reconexión con backoff, JWT de
 * dispositivo con renovación sin reconectar, Presence -- ver
 * `benchmarks/realtime-rust/HANDOFF.md`) y traduce su único evento
 * (`ObservadorRealtimeNube.enCambioRemoto`) al mismo `onCambio` de
 * siempre. `SincronizacionPeriodica` sigue siendo quien de verdad
 * descarga los cambios (debounce + serialización ya viven ahí, ver
 * [CambiosNube]) -- este archivo nunca cambió esa parte.
 */
class NubeRealtime(
    private val nucleo: Nucleo,
    private val secretoStore: SecretoDispositivoStore,
    private val scope: CoroutineScope,
    // Quién tiene la sesión abierta en este teléfono ahora -- viaja como
    // Presence (ver `realtime_nube::iniciar_realtime_nube`), para que el
    // panel pueda mostrar "usuarios en línea y desde dónde" sin abrir una
    // conexión nueva (ver docs/features-futuras/plan-sesion-unica-dispositivos.md).
    private val usuarioCedula: String,
    private val usuarioNombre: String,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
    private val onCambio: () -> Unit = { CambiosNube.solicitar() },
) {
    private var tarea: TareaRealtimeNube? = null

    fun iniciar() {
        if (tarea != null) return
        scope.launch {
            val sesionInicial = withContext(dispatcherIO) {
                try {
                    val secreto = secretoStore.cargar() ?: throw SecretoDispositivoNoEncontradoException()
                    nucleo.sesionRealtimeNubeConSecreto(secreto)
                } catch (excepcion: NucleoException) {
                    Log.w("SincronizacionNube", "No se pudo autenticar para Realtime", excepcion)
                    null
                } catch (excepcion: SecretoDispositivoNoEncontradoException) {
                    null
                }
            } ?: return@launch
            if (tarea != null) return@launch

            // Llamado desde el hilo propio del cliente Rust (nunca el
            // hilo principal de Android) -- bloquea mientras dura la
            // llamada de red, inofensivo: ese hilo es exclusivo de esta
            // conexión (ver el doc-comment de `TareaRealtimeNube`).
            val proveedorToken =
                object : ProveedorTokenRealtimeNube {
                    override fun tokenFresco(): TokenRealtimeNube? =
                        try {
                            val secreto = secretoStore.cargar() ?: return null
                            val sesion = nucleo.sesionRealtimeNubeConSecreto(secreto)
                            TokenRealtimeNube(sesion.accessToken, sesion.sitioId, sesion.dispositivoId)
                        } catch (excepcion: Throwable) {
                            Log.w("SincronizacionNube", "No se pudo renovar el token de Realtime", excepcion)
                            null
                        }
                }
            val observador =
                object : ObservadorRealtimeNube {
                    override fun enCambioRemoto() {
                        Log.i("SincronizacionNube", "Aviso remoto recibido; solicitando descarga")
                        onCambio()
                    }
                }

            tarea =
                iniciarRealtimeNubeFfi(
                    sesionInicial.baseUrl,
                    sesionInicial.apikey,
                    usuarioCedula,
                    usuarioNombre,
                    proveedorToken,
                    observador,
                )
        }
    }

    fun detener() {
        tarea?.detener()
        tarea?.destroy()
        tarea = null
    }
}

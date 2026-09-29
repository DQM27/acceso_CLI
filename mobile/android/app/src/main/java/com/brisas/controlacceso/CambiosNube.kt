package com.brisas.controlacceso

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/** Qué pide un aviso a `SincronizacionPeriodica`. */
sealed interface SolicitudNube {
    /**
     * Algo cambió en la nube: `tabla` es la del aviso en vivo
     * (`payload.table` de `cambio_nube`, ver `NubeRealtime`), o `null` =
     * "sincronizar todo" (la reconexión del canal).
     */
    data class Remota(val tabla: String?) : SolicitudNube

    /**
     * Se guardó algo en ESTE teléfono: basta con subirlo, no hay nada que
     * bajar (mismo criterio que `enviarCambiosNube` en escritorio).
     */
    data object CambioLocal : SolicitudNube
}

/**
 * Avisos locales y remotos que el sincronizador agrupa en una sola ejecución
 * -- ver `PendientesSincronizacion`.
 */
object CambiosNube {
    // Buffer holgado: con tablas en juego, descartar un aviso ya no es
    // inofensivo (antes todos eran `Unit`, perder uno daba igual). El
    // colector de `SincronizacionPeriodica` sólo los anota, así que en la
    // práctica nunca se llena.
    private val solicitudes = MutableSharedFlow<SolicitudNube>(extraBufferCapacity = 64, onBufferOverflow = BufferOverflow.DROP_OLDEST)
    val cambios = solicitudes.asSharedFlow()

    /** Aviso remoto: sólo `tabla`, o todo si es `null`. */
    fun solicitar(tabla: String? = null) {
        solicitudes.tryEmit(SolicitudNube.Remota(tabla))
    }

    /**
     * Registro, salida o cualquier dato guardado en este teléfono. Antes
     * pedía la sincronización completa: el cambio recién se subía al
     * terminarla (telemetría de staging: ~1,6 s de promedio y hasta 5 s) y
     * el otro equipo lo veía tarde.
     */
    fun cambioLocal() {
        solicitudes.tryEmit(SolicitudNube.CambioLocal)
    }
}

/** Qué corre la próxima sincronización. */
sealed interface AlcancePendiente {
    data object Completa : AlcancePendiente

    /** Sólo esas tablas (la bandeja de salida se vacía igual). */
    data class Tablas(val tablas: List<String>) : AlcancePendiente

    /** Sólo vaciar la bandeja de salida. */
    data object SoloEnvio : AlcancePendiente
}

/**
 * Lo que quedó pendiente de sincronizar entre una corrida y la siguiente:
 * un conjunto de tablas, o "todo". Puro (sin coroutines ni Android) para
 * poder probarlo; thread-safe porque se anota desde el colector de
 * [CambiosNube] y se toma desde el bucle de `SincronizacionPeriodica`.
 */
class PendientesSincronizacion {
    private val tablas = linkedSetOf<String>()
    private var completa = false
    private var envio = false

    /** `null` = sincronización completa. */
    @Synchronized
    fun anotar(tabla: String?) {
        if (tabla.isNullOrBlank()) completa = true else tablas.add(tabla)
    }

    /** Un cambio guardado en este teléfono: alcanza con subirlo. */
    @Synchronized
    fun anotarCambioLocal() {
        envio = true
    }

    /**
     * Lo pendiente, y lo deja vacío. Gana lo más amplio: la completa si se
     * pidió; si no, las tablas en el orden en que llegaron (también suben lo
     * local); si no, sólo el envío. Sin nada anotado (el pulso periódico, la
     * primera corrida) es la completa.
     */
    @Synchronized
    fun tomar(): AlcancePendiente {
        val resultado = when {
            completa -> AlcancePendiente.Completa
            tablas.isNotEmpty() -> AlcancePendiente.Tablas(tablas.toList())
            envio -> AlcancePendiente.SoloEnvio
            else -> AlcancePendiente.Completa
        }
        completa = false
        envio = false
        tablas.clear()
        return resultado
    }
}

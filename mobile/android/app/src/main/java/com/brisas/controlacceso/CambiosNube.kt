package com.brisas.controlacceso

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/**
 * Avisos locales y remotos que el sincronizador agrupa en una sola ejecución.
 *
 * Cada aviso lleva la tabla que cambió (`payload.table` del aviso en vivo
 * `cambio_nube`, ver `NubeRealtime`) o `null` = "sincronizar todo" (un
 * registro local, la reconexión del canal). Con la tabla,
 * `SincronizacionPeriodica` corre sólo esa parte de la sincronización en
 * vez de la completa -- ver `PendientesSincronizacion`.
 */
object CambiosNube {
    // Buffer holgado: con tablas en juego, descartar un aviso ya no es
    // inofensivo (antes todos eran `Unit`, perder uno daba igual). El
    // colector de `SincronizacionPeriodica` sólo los anota, así que en la
    // práctica nunca se llena.
    private val solicitudes = MutableSharedFlow<String?>(extraBufferCapacity = 64, onBufferOverflow = BufferOverflow.DROP_OLDEST)
    val cambios = solicitudes.asSharedFlow()

    fun solicitar(tabla: String? = null) {
        solicitudes.tryEmit(tabla)
    }
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

    /** `null` = sincronización completa. */
    @Synchronized
    fun anotar(tabla: String?) {
        if (tabla.isNullOrBlank()) completa = true else tablas.add(tabla)
    }

    /**
     * Lo pendiente, y lo deja vacío: `null` si hace falta la sincronización
     * completa (se pidió explícitamente, o no hay ninguna tabla anotada --
     * ej. el pulso periódico), si no, las tablas en el orden en que
     * llegaron.
     */
    @Synchronized
    fun tomar(): List<String>? {
        val resultado = if (completa || tablas.isEmpty()) null else tablas.toList()
        completa = false
        tablas.clear()
        return resultado
    }
}

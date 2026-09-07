package com.brisas.controlacceso

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/** Avisos locales y remotos que el sincronizador agrupa en una sola ejecución. */
object CambiosNube {
    private val solicitudes = MutableSharedFlow<Unit>(extraBufferCapacity = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST)
    val cambios = solicitudes.asSharedFlow()

    fun solicitar() {
        solicitudes.tryEmit(Unit)
    }
}

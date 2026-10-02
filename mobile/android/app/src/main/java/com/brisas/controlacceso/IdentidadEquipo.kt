package com.brisas.controlacceso

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.Nucleo

/// Texto con la unidad y la etiqueta con que el panel registró este
/// teléfono (mismo formato que escritorio, `IdentidadEquipo.logica.ts`), o
/// `null` si todavía no llegó ninguna de las dos: mejor no mostrar nada que
/// un dato inventado. Sirve para notar un equipo registrado en la unidad
/// equivocada antes de que la sesión única empiece a cerrar sesiones.
fun textoIdentidadEquipo(unidad: String?, etiqueta: String?): String? {
    val unidadLimpia = unidad?.trim()?.takeIf { it.isNotEmpty() }
    val etiquetaLimpia = etiqueta?.trim()?.takeIf { it.isNotEmpty() }
    return when {
        unidadLimpia != null && etiquetaLimpia != null -> "Unidad: $unidadLimpia · $etiquetaLimpia"
        unidadLimpia != null -> "Unidad: $unidadLimpia"
        etiquetaLimpia != null -> "Equipo: $etiquetaLimpia"
        else -> null
    }
}

/// Lee la identidad del equipo fuera del hilo de UI (el núcleo puede estar
/// ocupado con una sincronización) y la vuelve a leer cada vez que cambia
/// `recargar` -- por ejemplo al terminar una sincronización, que es cuando
/// puede llegar un token con datos nuevos.
@Composable
fun rememberTextoIdentidadEquipo(nucleo: Nucleo, recargar: Any? = null): String? {
    val texto by produceState<String?>(initialValue = null, nucleo, recargar) {
        value = withContext(Dispatchers.IO) {
            val identidad = nucleo.identidadEquipo()
            textoIdentidadEquipo(identidad.unidad, identidad.etiqueta)
        }
    }
    return texto
}

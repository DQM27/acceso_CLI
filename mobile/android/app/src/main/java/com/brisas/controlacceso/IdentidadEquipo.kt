package com.brisas.controlacceso

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import java.util.Locale
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

/// Sólo la unidad operativa, en mayúsculas (`BRISAS`), para la pantalla
/// principal (mismo criterio que `textoUnidadOperativa` de escritorio);
/// `null` si todavía no llegó.
fun textoUnidadOperativa(unidad: String?): String? =
    unidad?.trim()?.takeIf { it.isNotEmpty() }?.uppercase(Locale("es"))

/// Lee la identidad del equipo fuera del hilo de UI (el núcleo puede estar
/// ocupado con una sincronización) y la vuelve a leer cada vez que cambia
/// `recargar` -- por ejemplo al terminar una sincronización, que es cuando
/// puede llegar un token con datos nuevos.
@Composable
private fun rememberIdentidadEquipo(
    nucleo: Nucleo,
    recargar: Any?,
    formato: (unidad: String?, etiqueta: String?) -> String?,
): String? {
    val texto by produceState<String?>(initialValue = null, nucleo, recargar) {
        value = withContext(Dispatchers.IO) {
            val identidad = nucleo.identidadEquipo()
            formato(identidad.unidad, identidad.etiqueta)
        }
    }
    return texto
}

/// Unidad y etiqueta completas (pantalla de login).
@Composable
fun rememberTextoIdentidadEquipo(nucleo: Nucleo, recargar: Any? = null): String? =
    rememberIdentidadEquipo(nucleo, recargar, ::textoIdentidadEquipo)

/// Sólo la unidad, en mayúsculas (pantalla principal).
@Composable
fun rememberTextoUnidadOperativa(nucleo: Nucleo, recargar: Any? = null): String? =
    rememberIdentidadEquipo(nucleo, recargar) { unidad, _ -> textoUnidadOperativa(unidad) }

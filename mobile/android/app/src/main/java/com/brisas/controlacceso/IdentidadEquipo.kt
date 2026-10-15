package com.brisas.controlacceso

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.Nucleo

/// Sólo la unidad operativa, en mayúsculas (`BRISAS`), para el login y la
/// pantalla principal (mismo criterio que `textoUnidadOperativa` de
/// escritorio); `null` si todavía no llegó: mejor no mostrar nada que un dato
/// inventado. Sirve para notar un equipo registrado en la unidad equivocada
/// antes de que la sesión única empiece a cerrar sesiones.
fun textoUnidadOperativa(unidad: String?): String? =
    unidad?.trim()?.takeIf { it.isNotEmpty() }?.uppercase(Locale("es"))

/// Lee la unidad del equipo fuera del hilo de UI (el núcleo puede estar
/// ocupado con una sincronización) y la vuelve a leer cada vez que cambia
/// `recargar` -- por ejemplo al terminar una sincronización, que es cuando
/// puede llegar un token con datos nuevos.
@Composable
fun rememberTextoUnidadOperativa(nucleo: Nucleo, recargar: Any? = null): String? {
    val texto by produceState<String?>(initialValue = null, nucleo, recargar) {
        value = withContext(Dispatchers.IO) {
            textoUnidadOperativa(nucleo.identidadEquipo().unidad)
        }
    }
    return texto
}

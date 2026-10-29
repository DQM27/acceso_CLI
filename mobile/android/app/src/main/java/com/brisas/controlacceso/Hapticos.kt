package com.brisas.controlacceso

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback

/// Los toques de vibración de la interfaz, como los de iOS: uno seco al
/// cambiar de pestaña o de opción, uno de confirmación cuando un registro sale
/// bien y uno de rechazo ante un error de lo que se acaba de hacer. Son los
/// del sistema (`HapticFeedbackType`), así que respetan la vibración al tocar
/// apagada en los ajustes del teléfono.
///
/// Aparte de las alarmas del escáner (`vibrarConfirmacion`/`vibrarError` en
/// `EscaneoCompartido.kt`), que tienen que sentirse con el teléfono en la
/// mano y una cédula en la otra.
internal enum class Haptico { SELECCION, CONFIRMAR, RECHAZAR, ACTIVAR, DESACTIVAR }

internal fun tipoHaptico(haptico: Haptico): HapticFeedbackType = when (haptico) {
    // El "clic" de las teclas del sistema. Era `SegmentTick`, el más tenue de
    // Android: en el Samsung A25 casi no se sentía al cambiar de sección
    // (pruebas del usuario 2026-10-29).
    Haptico.SELECCION -> HapticFeedbackType.VirtualKey
    Haptico.CONFIRMAR -> HapticFeedbackType.Confirm
    Haptico.RECHAZAR -> HapticFeedbackType.Reject
    Haptico.ACTIVAR -> HapticFeedbackType.ToggleOn
    Haptico.DESACTIVAR -> HapticFeedbackType.ToggleOff
}

@Composable
internal fun rememberHapticos(): (Haptico) -> Unit {
    val retroalimentacion = LocalHapticFeedback.current
    return remember(retroalimentacion) { { haptico -> retroalimentacion.performHapticFeedback(tipoHaptico(haptico)) } }
}

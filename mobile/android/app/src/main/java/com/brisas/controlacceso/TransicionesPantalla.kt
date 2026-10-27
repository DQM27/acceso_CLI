package com.brisas.controlacceso

import androidx.compose.animation.ContentTransform
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically

/// Transiciones entre pantallas y pestañas (etapa 2 del plan de animaciones
/// del móvil). Cortas y baratas: sólo desplazamiento y transparencia, nunca
/// el tamaño (no obligan a volver a medir el diseño), y sin animaciones
/// infinitas. Android las respeta si la persona quita las animaciones en los
/// ajustes del teléfono.
///
/// Las pantallas con cámara NO se animan (`SinTransicion`): abrir el escáner
/// debe sentirse instantáneo y esas pantallas ya van justas de tiempo por
/// cuadro (telemetría de staging, 2026-10-19).

internal const val MS_ENTRADA = 240
internal const val MS_SALIDA = 120
internal const val MS_FUNDIDO_ENTRADA = 180

/// Curva "emphasized decelerate" de Material 3: arranca rápido y frena suave.
internal val CurvaEntrada = CubicBezierEasing(0.05f, 0.7f, 0.1f, 1f)

/// Curva "emphasized accelerate": la pantalla que se va sale rápido.
internal val CurvaSalida = CubicBezierEasing(0.3f, 0f, 0.8f, 0.15f)

/// Sin animación: el cambio es inmediato.
internal val SinTransicion = ContentTransform(
    targetContentEnter = EnterTransition.None,
    initialContentExit = ExitTransition.None,
    sizeTransform = null,
)

/// Cambio de pestaña: el contenido nuevo entra desde el lado donde está su
/// pestaña, con un desplazamiento corto (una décima del ancho) y un fundido,
/// y el anterior se va hacia el lado contrario.
internal fun transicionDePestana(desde: Int, hacia: Int): ContentTransform {
    if (desde == hacia) return SinTransicion
    val sentido = if (hacia > desde) 1 else -1
    return ContentTransform(
        targetContentEnter = fadeIn(tween(MS_FUNDIDO_ENTRADA, delayMillis = MS_SALIDA / 2, easing = CurvaEntrada)) +
            slideInHorizontally(tween(MS_ENTRADA, easing = CurvaEntrada)) { ancho -> sentido * ancho / 10 },
        initialContentExit = fadeOut(tween(MS_SALIDA, easing = CurvaSalida)) +
            slideOutHorizontally(tween(MS_SALIDA, easing = CurvaSalida)) { ancho -> -sentido * ancho / 10 },
        sizeTransform = null,
    )
}

/// Una pantalla que se abre encima de otra (`avanza`) o que se cierra para
/// volver a la anterior. Al avanzar entra desde abajo (poco recorrido) con
/// fundido y la de atrás casi no se mueve; al volver, al revés.
internal fun transicionDePantalla(avanza: Boolean): ContentTransform =
    if (avanza) {
        ContentTransform(
            targetContentEnter = fadeIn(tween(MS_FUNDIDO_ENTRADA, easing = CurvaEntrada)) +
                slideInVertically(tween(MS_ENTRADA, easing = CurvaEntrada)) { alto -> alto / 12 },
            initialContentExit = fadeOut(tween(MS_SALIDA, easing = CurvaSalida)),
            targetContentZIndex = 1f,
            sizeTransform = null,
        )
    } else {
        ContentTransform(
            targetContentEnter = fadeIn(tween(MS_FUNDIDO_ENTRADA, easing = CurvaEntrada)) +
                scaleIn(tween(MS_ENTRADA, easing = CurvaEntrada), initialScale = 0.96f),
            initialContentExit = fadeOut(tween(MS_SALIDA, easing = CurvaSalida)) +
                slideOutVertically(tween(MS_SALIDA, easing = CurvaSalida)) { alto -> alto / 12 },
            targetContentZIndex = -1f,
            sizeTransform = null,
        )
    }

/// Los paneles de una pantalla con lista, un formulario aparte y dos
/// escáneres de cámara (Proveedores y Por correo).
internal enum class PanelConFormulario(val conCamara: Boolean) {
    LISTA(conCamara = false),
    FORMULARIO(conCamara = false),
    ESCANER_CEDULA(conCamara = true),
    ESCANER_PLACA(conCamara = true),
}

/// Qué panel ocupa la pantalla. Mismo orden de prioridad de siempre: los
/// escáneres tapan al formulario y éste a la lista.
internal fun panelConFormulario(
    mostrandoFormulario: Boolean,
    escaneandoCedula: Boolean,
    escaneandoPlaca: Boolean,
): PanelConFormulario = when {
    escaneandoCedula -> PanelConFormulario.ESCANER_CEDULA
    escaneandoPlaca -> PanelConFormulario.ESCANER_PLACA
    mostrandoFormulario -> PanelConFormulario.FORMULARIO
    else -> PanelConFormulario.LISTA
}

/// El formulario se abre encima de la lista y se cierra de vuelta; si uno de
/// los dos paneles es de cámara, el cambio es inmediato.
internal fun transicionDePanel(desde: PanelConFormulario, hacia: PanelConFormulario): ContentTransform =
    if (desde.conCamara || hacia.conCamara) SinTransicion else transicionDePantalla(avanza = hacia != PanelConFormulario.LISTA)

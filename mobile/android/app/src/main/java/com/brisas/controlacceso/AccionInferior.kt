package com.brisas.controlacceso

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

// El botón redondo junto a la barra de secciones y el buscador que se abre
// desde abajo, como la app Teléfono de iOS 26 (pedido del usuario 2026-10-21).

/// Lo que hace el botón redondo de abajo: lo pone la pantalla a la vista
/// (cámara en Ingreso, lupa en Salida y KOF, "+" en Proveedores y Correo).
internal class AccionInferior(
    val icono: ImageVector,
    val descripcion: String,
    private val alTocar: State<() -> Unit>,
) {
    fun tocar() = alTocar.value()
}

/// Las acciones registradas, la última manda. Es una pila y no un único
/// valor porque en una transición la pantalla que entra y la que sale
/// conviven un instante: al irse la vieja, queda la nueva.
internal class AccionesInferiores {
    private val pila = mutableStateListOf<AccionInferior>()
    val actual: AccionInferior? get() = pila.lastOrNull()

    fun poner(accion: AccionInferior) {
        pila.add(accion)
    }

    fun quitar(accion: AccionInferior) {
        pila.remove(accion)
    }
}

internal val LocalAccionesInferiores = staticCompositionLocalOf<AccionesInferiores?> { null }

/// La pantalla que la llama pone su acción en el botón redondo mientras esté a
/// la vista (y [visible]). Fuera de las pestañas no hace nada.
@Composable
internal fun AccionPrincipal(icono: ImageVector, descripcion: String, onClick: () -> Unit, visible: Boolean = true) {
    val acciones = LocalAccionesInferiores.current ?: return
    if (!visible) return
    val alTocar = rememberUpdatedState(onClick)
    DisposableEffect(acciones, icono, descripcion) {
        val accion = AccionInferior(icono, descripcion, alTocar)
        acciones.poner(accion)
        onDispose { acciones.quitar(accion) }
    }
}

/// Botón redondo de vidrio junto a la barra de secciones, del mismo alto y el
/// mismo vidrio que ella. Al cambiar de acción, el ícono nuevo entra con
/// rebote.
@Composable
internal fun BotonRedondoAccion(accion: AccionInferior, tamano: Dp, modifier: Modifier = Modifier) {
    BotonRedondoVidrio(tamano = tamano, descripcion = accion.descripcion, onClick = accion::tocar, modifier = modifier) {
        AnimatedContent(
            targetState = accion.icono,
            transitionSpec = {
                (fadeIn(tween(MS_FUNDIDO_ENTRADA)) + scaleIn(ResorteBoton, initialScale = 0.4f)) togetherWith
                    (fadeOut(tween(MS_SALIDA)) + scaleOut(tween(MS_SALIDA), targetScale = 0.4f)) using null
            },
            label = "iconoAccion",
        ) { icono ->
            Icon(icono, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(24.dp))
        }
    }
}

/// Opacidad del fondo de la barra de secciones y del botón redondo: casi
/// opaco, para que el texto se lea sin desenfocar lo de atrás (ver
/// `BarraSecciones`).
internal const val ALFA_VELO = 0.94f

/// Círculo con el mismo velo que la barra de secciones ([ALFA_VELO]).
@Composable
internal fun BotonRedondoVidrio(
    tamano: Dp,
    descripcion: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    contenido: @Composable () -> Unit,
) {
    val superficie = MaterialTheme.colorScheme.surface
    Box(
        modifier = modifier
            .size(tamano)
            .clip(CircleShape)
            .background(superficie.copy(alpha = ALFA_VELO))
            .border(0.5.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.25f), CircleShape)
            .clickable(onClick = onClick)
            .semantics {
                contentDescription = descripcion
                role = Role.Button
            },
        contentAlignment = Alignment.Center,
    ) {
        contenido()
    }
}

/// Buscador que se abre desde abajo, como el de la app Teléfono de iOS: al
/// abrirse sale el teclado y el campo queda pegado encima de él, con una X
/// redonda al lado para cerrar; los resultados ([resultados]) ocupan el resto
/// de la pantalla, encima. La barra de secciones se esconde mientras tanto.
/// Atrás del sistema también lo cierra.
@Composable
internal fun BuscadorInferior(
    texto: String,
    onCambiar: (String) -> Unit,
    ayuda: String,
    onCerrar: () -> Unit,
    modifier: Modifier = Modifier,
    resultados: @Composable () -> Unit,
) {
    OcultarBarraSecciones()
    BackHandler(onBack = onCerrar)
    val foco = remember { FocusRequester() }
    val teclado = LocalSoftwareKeyboardController.current
    LaunchedEffect(Unit) {
        foco.requestFocus()
        teclado?.show()
    }
    Column(modifier = modifier.fillMaxSize().imePadding()) {
        Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
            resultados()
        }
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CampoBusquedaBrisas(
                valor = texto,
                onCambiar = onCambiar,
                ayuda = ayuda,
                modifier = Modifier.weight(1f).focusRequester(foco),
            )
            Box(
                modifier = Modifier
                    .size(AlturaBusquedaBrisas)
                    .clip(CircleShape)
                    .border(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.4f), CircleShape)
                    .clickable(onClick = onCerrar)
                    .semantics {
                        contentDescription = "Cerrar búsqueda"
                        role = Role.Button
                    },
                contentAlignment = Alignment.Center,
            ) {
                Icon(Icons.Default.Close, contentDescription = null)
            }
        }
    }
}

private val ResorteBoton = spring<Float>(
    dampingRatio = Spring.DampingRatioMediumBouncy,
    stiffness = Spring.StiffnessMediumLow,
)

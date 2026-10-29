package com.brisas.controlacceso

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
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
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
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

/// Lo que busca la píldora de búsqueda de abajo ([PildoraBusqueda]): el texto
/// de ayuda y qué pasa al tocarla (la pantalla abre su [BuscadorInferior]).
internal class BusquedaInferior(
    val ayuda: String,
    private val alAbrir: State<() -> Unit>,
) {
    fun abrir() = alAbrir.value()
}

/// Las acciones y las búsquedas registradas, la última manda. Son pilas y no
/// un único valor porque en una transición la pantalla que entra y la que
/// sale conviven un instante: al irse la vieja, queda la nueva.
internal class AccionesInferiores {
    private val pila = mutableStateListOf<AccionInferior>()
    val actual: AccionInferior? get() = pila.lastOrNull()

    private val pilaBusquedas = mutableStateListOf<BusquedaInferior>()
    val busqueda: BusquedaInferior? get() = pilaBusquedas.lastOrNull()

    fun poner(accion: AccionInferior) {
        pila.add(accion)
    }

    fun quitar(accion: AccionInferior) {
        pila.remove(accion)
    }

    fun ponerBusqueda(busqueda: BusquedaInferior) {
        pilaBusquedas.add(busqueda)
    }

    fun quitarBusqueda(busqueda: BusquedaInferior) {
        pilaBusquedas.remove(busqueda)
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

/// La pantalla que la llama pone su búsqueda en la píldora de abajo mientras
/// esté a la vista. Misma mecánica que [AccionPrincipal]. Con `visible` en
/// `false` no hay píldora: en KOF, Proveedores y Correo, con la lista vacía
/// no hay nada que buscar, y una búsqueda de más se confundía con la del
/// "+" (en KOF, la de encargados).
@Composable
internal fun BusquedaPrincipal(ayuda: String, onAbrir: () -> Unit, visible: Boolean = true) {
    if (!visible) return
    val acciones = LocalAccionesInferiores.current ?: return
    val alAbrir = rememberUpdatedState(onAbrir)
    DisposableEffect(acciones, ayuda) {
        val busqueda = BusquedaInferior(ayuda, alAbrir)
        acciones.ponerBusqueda(busqueda)
        onDispose { acciones.quitarBusqueda(busqueda) }
    }
}

/// La píldora de búsqueda de abajo, encima de la barra de secciones: dónde se
/// busca en todas las pestañas, al alcance del pulgar (como el buscador de
/// abajo de iOS 26). Tocarla abre el [BuscadorInferior] de la pantalla, cuyo
/// campo tiene la misma forma: parece la misma pieza que sube con el teclado.
@Composable
internal fun PildoraBusqueda(busqueda: BusquedaInferior, modifier: Modifier = Modifier) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .height(ALTO_PILDORA_BUSQUEDA)
            .clip(FormaBusquedaBrisas)
            .background(MaterialTheme.colorScheme.surface.copy(alpha = ALFA_VELO))
            .border(ANCHO_BORDE_VIDRIO, paletaActual.bordeVidrio, FormaBusquedaBrisas)
            .clickable(onClick = busqueda::abrir)
            .semantics {
                contentDescription = busqueda.ayuda
                role = Role.Button
            }
            .padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            Icons.Default.Search,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.size(20.dp),
        )
        Spacer(Modifier.width(10.dp))
        Text(
            busqueda.ayuda,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

/// Alto de [PildoraBusqueda].
internal val ALTO_PILDORA_BUSQUEDA = 46.dp

/// Botón redondo de vidrio junto a la barra de secciones, del mismo alto y el
/// mismo vidrio que ella. Al cambiar de acción, el ícono nuevo entra con
/// rebote.
@Composable
internal fun BotonRedondoAccion(accion: AccionInferior, tamano: Dp, modifier: Modifier = Modifier) {
    // Redondo, no la mitad de una pieza (pedido del usuario 2026-10-29): la
    // barra sí queda cortada de su lado, y esa asimetría se ve mejor.
    BotonRedondoVidrio(
        tamano = tamano,
        descripcion = accion.descripcion,
        onClick = accion::tocar,
        modifier = modifier,
        forma = CircleShape,
    ) {
        // Por acción y no por ícono: rebota en cada cambio de sección, también
        // entre dos con el mismo ícono (Proveedores y Correo).
        AnimatedContent(
            targetState = accion,
            transitionSpec = {
                (fadeIn(tween(MS_FUNDIDO_ENTRADA)) + scaleIn(ResorteBoton, initialScale = 0.4f)) togetherWith
                    (fadeOut(tween(MS_SALIDA)) + scaleOut(tween(MS_SALIDA), targetScale = 0.4f)) using null
            },
            label = "iconoAccion",
        ) { mostrada ->
            Icon(mostrada.icono, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(24.dp))
        }
    }
}

/// Opacidad del fondo de la barra de secciones y del botón redondo: casi
/// opaco, para que el texto se lea sin desenfocar lo de atrás (ver
/// `BarraSecciones`).
internal const val ALFA_VELO = 0.94f

/// Botón cuadrado con el mismo velo que la barra de secciones ([ALFA_VELO]);
/// [forma] dice de qué lado va el corte cuando forma pieza con otra.
@Composable
internal fun BotonRedondoVidrio(
    tamano: Dp,
    descripcion: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    forma: Shape = FormaPiezaEntera,
    contenido: @Composable () -> Unit,
) {
    val superficie = MaterialTheme.colorScheme.surface
    Box(
        modifier = modifier
            .size(tamano)
            .clip(forma)
            .background(superficie.copy(alpha = ALFA_VELO))
            .border(ANCHO_BORDE_VIDRIO, paletaActual.bordeVidrio, forma)
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

/// Buscador que se abre desde abajo, como el de iOS 26: al abrirse sube con
/// el teclado y el campo queda pegado encima de él, con una X redonda al lado
/// para cerrar; los resultados ([resultados]) ocupan el resto de la pantalla,
/// encima, con lo buscado resaltado ([LocalConsultaResaltada]). La barra de
/// secciones se esconde mientras tanto. Atrás del sistema también lo cierra.
///
/// - Deslizar los resultados esconde el teclado (y el campo baja con él),
///   para verlos enteros; la tecla "Buscar" hace lo mismo.
/// - [encabezado]: lo que va justo encima del campo, como la barra de
///   alcances de iOS (Ingreso/Salida/Gafete en Activos) o un título que dice
///   para qué se busca.
/// - Entra y sale con un deslizamiento corto y un fundido: antes aparecía de
///   golpe mientras la barra sí se escondía animada.
@Composable
internal fun BuscadorInferior(
    visible: Boolean,
    texto: String,
    onCambiar: (String) -> Unit,
    ayuda: String,
    onCerrar: () -> Unit,
    modifier: Modifier = Modifier,
    teclado: KeyboardOptions = KeyboardOptions.Default,
    encabezado: (@Composable () -> Unit)? = null,
    resultados: @Composable () -> Unit,
) {
    AnimatedVisibility(
        visible = visible,
        modifier = modifier,
        enter = fadeIn(tween(MS_FUNDIDO_ENTRADA)) + slideInVertically(tween(MS_ENTRADA)) { alto -> alto / 10 },
        exit = fadeOut(tween(MS_SALIDA)) + slideOutVertically(tween(MS_SALIDA)) { alto -> alto / 10 },
        label = "buscadorInferior",
    ) {
        OcultarBarraSecciones()
        BackHandler(enabled = visible, onBack = onCerrar)
        val foco = remember { FocusRequester() }
        val tecladoSistema = LocalSoftwareKeyboardController.current
        val focos = LocalFocusManager.current
        LaunchedEffect(Unit) {
            foco.requestFocus()
            tecladoSistema?.show()
        }
        // Al arrastrar la lista (no al desplazarla sola), fuera el teclado. Una
        // sola vez, mientras el campo tiene el foco: antes se le pedía al
        // sistema esconderlo en cada evento del arrastre (decenas por
        // segundo), aunque ya estuviera cerrado. No es estado de Compose:
        // sólo lo leen el arrastre y el foco, nunca el dibujo.
        val campoEnfocado = remember { booleanArrayOf(false) }
        val esconderTeclado = remember(tecladoSistema, focos) {
            object : NestedScrollConnection {
                override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
                    if (campoEnfocado[0] && source == NestedScrollSource.UserInput && available.y != 0f) {
                        campoEnfocado[0] = false
                        tecladoSistema?.hide()
                        focos.clearFocus()
                    }
                    return Offset.Zero
                }
            }
        }
        Column(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background)
                .imePadding(),
        ) {
            Box(modifier = Modifier.weight(1f).fillMaxWidth().nestedScroll(esconderTeclado)) {
                CompositionLocalProvider(LocalConsultaResaltada provides texto) {
                    resultados()
                }
            }
            if (encabezado != null) {
                Box(modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 6.dp)) {
                    encabezado()
                }
            }
            // El campo y la X, una sola pieza con un corte, como la barra de
            // secciones y su botón; con el margen de las tarjetas.
            Row(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(ESPACIO_CORTE),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                CampoBusquedaBrisas(
                    valor = texto,
                    onCambiar = onCambiar,
                    ayuda = ayuda,
                    keyboardOptions = teclado,
                    modifier = Modifier
                        .weight(1f)
                        .focusRequester(foco)
                        .onFocusChanged { campoEnfocado[0] = it.hasFocus },
                    forma = FormaPiezaInicio,
                )
                BotonRedondoVidrio(
                    tamano = AlturaBusquedaBrisas,
                    descripcion = "Cerrar búsqueda",
                    onClick = onCerrar,
                    forma = FormaPiezaFin,
                ) {
                    Icon(Icons.Default.Close, contentDescription = null)
                }
            }
        }
    }
}

private val ResorteBoton = spring<Float>(
    dampingRatio = Spring.DampingRatioMediumBouncy,
    stiffness = Spring.StiffnessMediumLow,
)

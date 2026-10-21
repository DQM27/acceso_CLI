package com.brisas.controlacceso

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Surface
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupProperties
import kotlinx.coroutines.delay
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.RolUsuario
import uniffi.control_acceso_mobile.UsuarioSesion

/// Menú de la cuenta de quien tiene la sesión, mismo lugar y misma marca que el
/// menú de usuario del escritorio (`desktop/src/componentes/MenuUsuario.tsx`):
/// la inicial dentro de un engranaje ([EngranajeConInicial]) arriba a la
/// derecha. Queda arriba y no en la barra de secciones a propósito: abajo es lo
/// operativo y rápido, y ahí se tocaría por error (decisión del usuario
/// 2026-10-21).
///
/// Al tocarlo brota un panel chico, a la manera de los menús de iOS 26
/// (rediseño 2026-10-21): el nombre, el tema (sol/luna), la actualización con
/// la versión, y "Cerrar sesión" en rojo, apartado del resto por un separador
/// grueso para no tocarlo por error. Es un panel propio y no un
/// `DropdownMenu`: el de Material no deja cambiar cómo aparece.
///
/// Si hay una versión nueva, un puntito late unos segundos sobre el engranaje y
/// luego queda fijo; el ítem de actualización cambia de "Buscar" a "Actualizar".
///
/// Sólo UI: el tema lo guarda [GestorTema], el estado de la actualización es de
/// [ActualizacionViewModel] y cerrar sesión lo decide quien llama.
@Composable
fun MenuCuenta(
    sesion: UsuarioSesion,
    tema: TemaApp,
    onAlternarTema: () -> Unit,
    onCerrarSesion: () -> Unit,
    actualizador: ActualizacionViewModel,
) {
    var abierto by remember { mutableStateOf(false) }
    // Mantiene el panel en pantalla mientras se encoge al cerrarse.
    val visible = remember { MutableTransitionState(false) }
    visible.targetState = abierto
    val giro = remember { Animatable(0f) }
    val alcance = rememberCoroutineScope()
    val estadoActualizacion by actualizador.estado.collectAsState()
    val busqueda by actualizador.busqueda.collectAsState()
    val hayPendiente = estadoActualizacion is EstadoActualizacion.ConNovedad

    val contexto = LocalContext.current
    val densidad = LocalDensity.current

    fun cerrar() {
        abierto = false
        actualizador.olvidarBusqueda()
    }

    // La respuesta de "Buscar actualizaciones" sale como un aviso corto, sin
    // dejar el menú abierto; si hay versión nueva no hay aviso: aparece la
    // tarjeta flotante y el puntito en el engranaje.
    LaunchedEffect(busqueda) {
        val aviso = avisoDeBusqueda(busqueda) ?: return@LaunchedEffect
        Toast.makeText(contexto, aviso, Toast.LENGTH_SHORT).show()
        actualizador.olvidarBusqueda()
    }

    Box {
        BotonIconoCuadradoBrisas(onClick = {
            abierto = !abierto
            // La rueda da media vuelta con rebote al tocarla (pedido del
            // usuario 2026-10-21); cada toque suma otra media vuelta.
            alcance.launch { giro.animateTo(giro.targetValue + 180f, ResorteRebote) }
        }) {
            // Mientras busca, el círculo girando dice que algo está pasando.
            if (busqueda == EstadoBusqueda.BUSCANDO) {
                CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
            } else {
                EngranajeConInicial(
                    inicialDe(sesion.nombre),
                    color = MaterialTheme.colorScheme.onSurface,
                    tamano = 28.dp,
                    modifier = Modifier.semantics { contentDescription = "Cuenta de ${sesion.nombre}" },
                    giro = { giro.value },
                )
            }
        }
        if (hayPendiente) PuntoPendiente(Modifier.align(Alignment.TopEnd).offset(x = 3.dp, y = (-3).dp))

        if (visible.currentState || visible.targetState) {
            // Debajo del botón, alineado a su borde derecho.
            val separacion = with(densidad) { (ALTO_BOTON_CUENTA + 8.dp).roundToPx() }
            Popup(
                alignment = Alignment.TopEnd,
                offset = IntOffset(0, separacion),
                onDismissRequest = ::cerrar,
                properties = PopupProperties(focusable = true),
            ) {
                // Como los menús de iOS 26: el panel brota del botón (crece
                // desde su esquina, con un resorte que se pasa un poco) y al
                // cerrarse se encoge de vuelta hacia él.
                AnimatedVisibility(
                    visibleState = visible,
                    enter = fadeIn(tween(MS_SALIDA)) +
                        scaleIn(ResorteMenu, initialScale = 0.25f, transformOrigin = OrigenEsquina),
                    exit = fadeOut(tween(MS_SALIDA)) +
                        scaleOut(tween(MS_SALIDA + 40), targetScale = 0.25f, transformOrigin = OrigenEsquina),
                    label = "panelCuenta",
                ) {
                    PanelCuenta(
                        sesion = sesion,
                        tema = tema,
                        onAlternarTema = onAlternarTema,
                        actualizacion = if (actualizador.activo) opcionActualizacion(estadoActualizacion, busqueda) else null,
                        onActualizacion = { accion ->
                            when (accion) {
                                // Cierra el menú: la respuesta llega como aviso corto.
                                AccionActualizacion.BUSCAR -> {
                                    cerrar()
                                    actualizador.buscarAhora()
                                }
                                AccionActualizacion.ACTUALIZAR -> {
                                    cerrar()
                                    actualizador.actualizar()
                                }
                                AccionActualizacion.NINGUNA -> Unit
                            }
                        },
                        onCerrarSesion = {
                            cerrar()
                            onCerrarSesion()
                        },
                    )
                }
            }
        }
    }
}

/// El contenido del menú, a la manera de iOS: tarjeta de esquinas grandes con
/// sombra suave; en cada opción el texto a la izquierda y el ícono a la
/// derecha; un separador grueso aparta "Cerrar sesión", en rojo, del resto.
/// Las filas entran escalonadas, una tras otra.
@Composable
private fun PanelCuenta(
    sesion: UsuarioSesion,
    tema: TemaApp,
    onAlternarTema: () -> Unit,
    actualizacion: OpcionActualizacion?,
    onActualizacion: (AccionActualizacion) -> Unit,
    onCerrarSesion: () -> Unit,
) {
    Surface(
        shape = RoundedCornerShape(26.dp),
        color = MaterialTheme.colorScheme.surface,
        shadowElevation = 16.dp,
        // Angosto a prop�sito: a 260dp tapaba media pantalla.
        modifier = Modifier.width(IntrinsicSize.Max).widthIn(min = 180.dp, max = 230.dp),
    ) {
        Column(modifier = Modifier.padding(vertical = 6.dp)) {
            FilaEscalonada(indice = 0) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    EngranajeConInicial(inicialDe(sesion.nombre), color = MaterialTheme.colorScheme.primary, tamano = 34.dp)
                    Spacer(Modifier.width(12.dp))
                    Text(
                        sesion.nombre,
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.SemiBold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            SeparadorFino()
            FilaEscalonada(indice = 1) {
                // El ícono dice cuál está puesto (como en el escritorio);
                // tocar la fila cambia al otro. El menú queda abierto: se ve el
                // cambio de tema en el acto.
                OpcionMenu(
                    texto = if (tema == TemaApp.TOKYO_NIGHT) "Tema oscuro" else "Tema claro",
                    onClick = onAlternarTema,
                ) {
                    IconoTema(tema)
                }
            }
            SeparadorFino()
            FilaEscalonada(indice = 2) {
                if (actualizacion != null) {
                    OpcionMenu(
                        texto = actualizacion.texto,
                        detalle = "Versión ${BuildConfig.VERSION_NAME}",
                        resaltada = actualizacion.resaltada,
                        enabled = actualizacion.accion != AccionActualizacion.NINGUNA,
                        onClick = { onActualizacion(actualizacion.accion) },
                    ) {
                        Icon(
                            if (actualizacion.accion == AccionActualizacion.ACTUALIZAR) Icons.Default.Download else Icons.Default.Refresh,
                            contentDescription = null,
                            tint = if (actualizacion.resaltada) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                } else {
                    // Builds de prueba: sin actualizador, sólo la versión.
                    OpcionMenu(texto = "Versión ${BuildConfig.VERSION_NAME}", enabled = false, onClick = {}) {}
                }
            }
            // Separador grueso, como el que aparta las acciones destructivas
            // en los menús de iOS.
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 4.dp)
                    .height(6.dp)
                    .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f)),
            )
            FilaEscalonada(indice = 3) {
                OpcionMenu(texto = "Cerrar sesión", color = MaterialTheme.colorScheme.error, onClick = onCerrarSesion) {
                    Icon(Icons.AutoMirrored.Filled.ExitToApp, contentDescription = null, tint = MaterialTheme.colorScheme.error)
                }
            }
        }
    }
}

/// Una opción del menú: texto (y un detalle chico debajo) a la izquierda,
/// ícono a la derecha, toda la fila tocable.
@Composable
private fun OpcionMenu(
    texto: String,
    onClick: () -> Unit,
    detalle: String? = null,
    color: Color = MaterialTheme.colorScheme.onSurface,
    resaltada: Boolean = false,
    enabled: Boolean = true,
    icono: @Composable () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                texto,
                style = MaterialTheme.typography.bodyLarge,
                color = when {
                    resaltada -> MaterialTheme.colorScheme.primary
                    !enabled -> MaterialTheme.colorScheme.onSurfaceVariant
                    else -> color
                },
                fontWeight = if (resaltada) FontWeight.SemiBold else FontWeight.Normal,
            )
            if (detalle != null) {
                Text(detalle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        Spacer(Modifier.width(12.dp))
        icono()
    }
}

@Composable
private fun SeparadorFino() {
    HorizontalDivider(
        modifier = Modifier.padding(start = 16.dp),
        color = MaterialTheme.colorScheme.outline.copy(alpha = 0.25f),
    )
}

/// Cada fila entra un poco después que la anterior, subiendo apenas y
/// fundiéndose: el menú se "arma" en vez de aparecer de golpe.
@Composable
private fun FilaEscalonada(indice: Int, contenido: @Composable () -> Unit) {
    val avance = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        delay(indice * MS_ESCALON)
        avance.animateTo(1f, tween(MS_FUNDIDO_ENTRADA))
    }
    Box(
        modifier = Modifier.graphicsLayer {
            alpha = avance.value
            translationY = (1f - avance.value) * 8.dp.toPx()
        },
    ) {
        contenido()
    }
}

/// Sol (claro) o luna (oscuro), los íconos del escritorio ([IconosLucide]).
/// Al cambiar, con rebote (pedido del usuario 2026-10-21): el nuevo entra
/// creciendo y girando, se pasa un poco y vuelve; el que se va se encoge con
/// el mismo resorte.
@Composable
private fun IconoTema(tema: TemaApp) {
    AnimatedContent(
        targetState = tema == TemaApp.TOKYO_NIGHT,
        transitionSpec = {
            (fadeIn(tween(MS_FUNDIDO_ENTRADA)) + scaleIn(ResorteRebote, initialScale = 0.3f)) togetherWith
                (fadeOut(tween(MS_SALIDA)) + scaleOut(ResorteRebote, targetScale = 0.3f)) using null
        },
        label = "iconoTema",
    ) { esOscuro ->
        // Media vuelta al aparecer (el sentido cambia con el tema), con el
        // mismo resorte: se pasa del derecho y vuelve.
        val giro = remember { Animatable(if (esOscuro) -120f else 120f) }
        LaunchedEffect(Unit) { giro.animateTo(0f, spring(Spring.DampingRatioMediumBouncy, Spring.StiffnessMediumLow)) }
        Icon(
            if (esOscuro) IconosLucide.Luna else IconosLucide.Sol,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
            modifier = Modifier.graphicsLayer { rotationZ = giro.value },
        )
    }
}

/// Alto de [BotonIconoCuadradoBrisas]: el panel se abre justo debajo.
private val ALTO_BOTON_CUENTA = 40.dp

/// El panel crece desde la esquina superior derecha, donde está el botón.
private val OrigenEsquina = TransformOrigin(1f, 0f)

private const val MS_ESCALON = 35L

/// Resorte del panel al brotar: rápido y con un rebote corto.
private val ResorteMenu = spring<Float>(
    dampingRatio = 0.7f,
    stiffness = Spring.StiffnessMedium,
)

/// Puntito sobre el engranaje: late unas veces al aparecer (llama la atención
/// una vez) y queda fijo. Si el sistema tiene las animaciones apagadas, las
/// animaciones de Compose terminan al instante y sólo queda el punto.
@Composable
private fun PuntoPendiente(modifier: Modifier = Modifier) {
    val escala = remember { Animatable(1f) }
    LaunchedEffect(Unit) {
        repeat(LATIDOS) {
            escala.animateTo(1.7f, tween(450))
            escala.animateTo(1f, tween(450))
        }
    }
    Box(
        modifier = modifier
            .size(12.dp)
            .graphicsLayer {
                scaleX = escala.value
                scaleY = escala.value
            }
            .background(MaterialTheme.colorScheme.primary, CircleShape)
            .border(2.dp, MaterialTheme.colorScheme.background, CircleShape),
    )
}

private const val LATIDOS = 4

/// Resorte con rebote visible del ícono de tema ([BotonTema]).
private val ResorteRebote = spring<Float>(
    dampingRatio = Spring.DampingRatioMediumBouncy,
    stiffness = Spring.StiffnessMediumLow,
)

enum class AccionActualizacion { BUSCAR, ACTUALIZAR, NINGUNA }

/// Lo que dice y hace el ítem de actualización del menú. Un solo botón que
/// cambia: sin versión confirmada busca; con una pendiente actualiza.
internal data class OpcionActualizacion(
    val texto: String,
    val accion: AccionActualizacion,
    val resaltada: Boolean,
)

internal fun opcionActualizacion(estado: EstadoActualizacion, busqueda: EstadoBusqueda): OpcionActualizacion =
    when (estado) {
        is EstadoActualizacion.Descargando ->
            OpcionActualizacion("Descargando ${estado.porcentaje} %", AccionActualizacion.NINGUNA, resaltada = true)
        is EstadoActualizacion.Instalando ->
            OpcionActualizacion("Instalando…", AccionActualizacion.NINGUNA, resaltada = true)
        is EstadoActualizacion.Fallo ->
            OpcionActualizacion(
                "Reintentar ${estado.novedad.manifiesto.versionName}",
                AccionActualizacion.ACTUALIZAR,
                resaltada = true,
            )
        // Con una versión confirmada (aviso nuevo, pospuesto o a falta de un
        // permiso) el ítem deja de buscar y pasa a actualizar.
        is EstadoActualizacion.ConNovedad ->
            OpcionActualizacion(
                "Actualizar a ${estado.novedad.manifiesto.versionName}",
                AccionActualizacion.ACTUALIZAR,
                resaltada = true,
            )
        // El "Estás al día" y el "No se pudo revisar" salen como aviso corto
        // (ver [avisoDeBusqueda]); el ítem sólo vuelve a ofrecer buscar.
        EstadoActualizacion.SinAviso ->
            if (busqueda == EstadoBusqueda.BUSCANDO) {
                OpcionActualizacion("Buscando…", AccionActualizacion.NINGUNA, resaltada = false)
            } else {
                OpcionActualizacion("Buscar actualizaciones", AccionActualizacion.BUSCAR, resaltada = false)
            }
    }

/// El aviso corto con la respuesta de una búsqueda, o `null` si no hay nada
/// que decir (todavía buscando, o ya apareció la versión nueva).
internal fun avisoDeBusqueda(busqueda: EstadoBusqueda): String? = when (busqueda) {
    EstadoBusqueda.AL_DIA -> "Estás al día"
    EstadoBusqueda.SIN_RESPUESTA -> "No se pudo revisar"
    EstadoBusqueda.INACTIVA, EstadoBusqueda.BUSCANDO -> null
}

/// La inicial del nombre para el botón (vacío no se espera: toda cuenta tiene
/// nombre; "?" evita un botón sin nada si alguna vez llegara vacío).
internal fun inicialDe(nombre: String): String =
    nombre.trim().firstOrNull()?.uppercase() ?: "?"

internal fun etiquetaRol(rol: RolUsuario): String = when (rol) {
    RolUsuario.ROOT -> "Root"
    RolUsuario.ADMINISTRADOR -> "Administrador"
    RolUsuario.OPERADOR -> "Operador"
}

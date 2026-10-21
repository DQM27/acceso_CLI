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
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.IconButton
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
/// Al tocarlo se despliega un panel chico (rediseño 2026-10-21, achicado tras
/// verlo en el teléfono: rol, unidad y conexión no se usaban ahí): el nombre
/// con el botón de tema (sol/luna) al lado, la actualización con la versión, y
/// "Cerrar sesión" en rojo, separado del resto para no tocarlo por error.
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
    val estadoActualizacion by actualizador.estado.collectAsState()
    val busqueda by actualizador.busqueda.collectAsState()
    val hayPendiente = estadoActualizacion is EstadoActualizacion.ConNovedad

    val contexto = LocalContext.current

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
        BotonIconoCuadradoBrisas(onClick = { abierto = true }) {
            // Mientras busca, el círculo girando dice que algo está pasando.
            if (busqueda == EstadoBusqueda.BUSCANDO) {
                CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
            } else {
                EngranajeConInicial(
                    inicialDe(sesion.nombre),
                    color = MaterialTheme.colorScheme.onSurface,
                    tamano = 28.dp,
                    modifier = Modifier.semantics { contentDescription = "Cuenta de ${sesion.nombre}" },
                )
            }
        }
        if (hayPendiente) PuntoPendiente(Modifier.align(Alignment.TopEnd).offset(x = 3.dp, y = (-3).dp))
        DropdownMenu(
            expanded = abierto,
            onDismissRequest = ::cerrar,
            shape = RoundedCornerShape(16.dp),
            containerColor = MaterialTheme.colorScheme.surface,
        ) {
            // Quién tiene la sesión y, a la derecha, el tema: el ícono dice
            // cuál está puesto (como en el escritorio) y tocarlo cambia al otro.
            Row(
                modifier = Modifier.padding(start = 16.dp, end = 8.dp, top = 4.dp, bottom = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    sesion.nombre,
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.widthIn(max = 160.dp),
                )
                Spacer(Modifier.width(12.dp))
                BotonTema(tema, onAlternarTema)
            }

            if (actualizador.activo) {
                val opcion = opcionActualizacion(estadoActualizacion, busqueda)
                DropdownMenuItem(
                    text = {
                        Text(
                            opcion.texto,
                            fontWeight = if (opcion.resaltada) FontWeight.SemiBold else null,
                            color = if (opcion.resaltada) MaterialTheme.colorScheme.primary else Color.Unspecified,
                        )
                    },
                    leadingIcon = {
                        Icon(
                            if (opcion.accion == AccionActualizacion.ACTUALIZAR) Icons.Default.Download else Icons.Default.Refresh,
                            contentDescription = null,
                            tint = if (opcion.resaltada) MaterialTheme.colorScheme.primary else LocalContentColor.current,
                        )
                    },
                    // La versión instalada, chica, a la derecha de la acción:
                    // no necesita una fila propia.
                    trailingIcon = {
                        Text(
                            BuildConfig.VERSION_NAME,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    },
                    enabled = opcion.accion != AccionActualizacion.NINGUNA,
                    onClick = {
                        when (opcion.accion) {
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
                )
            } else {
                // Builds de prueba: sin actualizador, sólo la versión.
                Text(
                    "Versión ${BuildConfig.VERSION_NAME}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                )
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp))
            // En rojo y separado del resto, para no tocarlo por error.
            DropdownMenuItem(
                text = { Text("Cerrar sesión", color = MaterialTheme.colorScheme.error, fontWeight = FontWeight.Medium) },
                leadingIcon = {
                    Icon(
                        Icons.AutoMirrored.Filled.ExitToApp,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.error,
                    )
                },
                onClick = {
                    cerrar()
                    onCerrarSesion()
                },
            )
        }
    }
}

/// Sol (claro) o luna (oscuro), los íconos del escritorio ([IconosLucide]).
/// Al cambiar, con rebote (pedido del usuario 2026-10-21): el nuevo entra
/// creciendo y girando, se pasa un poco y vuelve; el que se va se encoge con
/// el mismo resorte. El menú queda abierto: se ve el cambio de tema en el acto.
@Composable
private fun BotonTema(tema: TemaApp, onAlternarTema: () -> Unit) {
    val oscuro = tema == TemaApp.TOKYO_NIGHT
    IconButton(onClick = onAlternarTema) {
        AnimatedContent(
            targetState = oscuro,
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
                contentDescription = if (esOscuro) "Tema oscuro; tocar para pasar a claro" else "Tema claro; tocar para pasar a oscuro",
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.graphicsLayer { rotationZ = giro.value },
            )
        }
    }
}

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

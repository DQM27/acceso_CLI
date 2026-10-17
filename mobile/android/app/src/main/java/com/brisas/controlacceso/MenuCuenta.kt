package com.brisas.controlacceso

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.filled.DarkMode
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.LightMode
import androidx.compose.material.icons.filled.Refresh
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.RolUsuario
import uniffi.control_acceso_mobile.UsuarioSesion

/// Menú de la cuenta de quien tiene la sesión, mismo lugar que el menú de
/// usuario del escritorio (`desktop/src/componentes/MenuUsuario.tsx`): un
/// botón con la inicial del nombre que abre quién es, el tema, la
/// actualización de la app, la versión instalada y cerrar sesión. Todo lo que
/// es de la persona y no del trabajo del puesto vive acá, para que la barra
/// superior quede con lo operativo (nuevo contratista y sincronizar).
///
/// Si hay una versión nueva, un puntito late unos segundos sobre la inicial y
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

    fun cerrar() {
        abierto = false
        actualizador.olvidarBusqueda()
    }

    Box {
        BotonIconoCuadradoBrisas(onClick = { abierto = true }) {
            Text(
                inicialDe(sesion.nombre),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
            )
        }
        if (hayPendiente) PuntoPendiente(Modifier.align(Alignment.TopEnd).offset(x = 3.dp, y = (-3).dp))
        DropdownMenu(expanded = abierto, onDismissRequest = ::cerrar) {
            Column(modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
                Text(
                    sesion.nombre,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    etiquetaRol(sesion.rol),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            HorizontalDivider()
            // Alterna Tokyo Day / Tokyo Night, como el escritorio; el ítem
            // nombra a cuál pasa con el próximo toque.
            val siguiente = tema.siguiente()
            DropdownMenuItem(
                text = { Text(textoCambioTema(siguiente)) },
                leadingIcon = { Icon(iconoTema(siguiente), contentDescription = null) },
                onClick = {
                    cerrar()
                    onAlternarTema()
                },
            )
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
                    enabled = opcion.accion != AccionActualizacion.NINGUNA,
                    onClick = {
                        when (opcion.accion) {
                            // La búsqueda deja el menú abierto para ver la respuesta.
                            AccionActualizacion.BUSCAR -> actualizador.buscarAhora()
                            AccionActualizacion.ACTUALIZAR -> {
                                cerrar()
                                actualizador.actualizar()
                            }
                            AccionActualizacion.NINGUNA -> Unit
                        }
                    },
                )
            }
            DropdownMenuItem(
                text = {
                    Text(
                        "Versión ${BuildConfig.VERSION_NAME}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                },
                enabled = false,
                onClick = {},
            )
            HorizontalDivider()
            DropdownMenuItem(
                text = { Text("Cerrar sesión") },
                leadingIcon = { Icon(Icons.AutoMirrored.Filled.ExitToApp, contentDescription = null) },
                onClick = {
                    cerrar()
                    onCerrarSesion()
                },
            )
        }
    }
}

/// Puntito sobre la inicial: late unas veces al aparecer (llama la atención
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
        EstadoActualizacion.SinAviso -> when (busqueda) {
            EstadoBusqueda.INACTIVA ->
                OpcionActualizacion("Buscar actualizaciones", AccionActualizacion.BUSCAR, resaltada = false)
            EstadoBusqueda.BUSCANDO ->
                OpcionActualizacion("Buscando…", AccionActualizacion.NINGUNA, resaltada = false)
            EstadoBusqueda.AL_DIA ->
                OpcionActualizacion("Estás al día", AccionActualizacion.NINGUNA, resaltada = false)
            EstadoBusqueda.SIN_RESPUESTA ->
                OpcionActualizacion("Sin respuesta, reintentar", AccionActualizacion.BUSCAR, resaltada = false)
        }
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

private fun textoCambioTema(tema: TemaApp): String = when (tema) {
    TemaApp.TOKYO_DAY -> "Tokyo Day"
    TemaApp.TOKYO_NIGHT -> "Tokyo Night"
}

private fun iconoTema(tema: TemaApp) = when (tema) {
    TemaApp.TOKYO_DAY -> Icons.Default.LightMode
    TemaApp.TOKYO_NIGHT -> Icons.Default.DarkMode
}

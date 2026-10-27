package com.brisas.controlacceso

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.ErrorOutline
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

/// Qué tipo de mensaje lleva una [FranjaEstado] (o un [AvisoBreve]): define
/// el color y el ícono.
internal enum class TipoEstado { ERROR, AVISO, EXITO, INFO, CARGANDO }

/// Un mensaje de estado dentro de una pantalla: franja redondeada con ícono,
/// del mismo estilo que la de conflictos y la de emergencia. Reemplaza los
/// textos rojos o verdes sueltos, el "⚠" escrito a mano y el "Verificando…"
/// gris que había en cada pantalla, cada uno a su manera.
///
/// [vibrar]: un toque de rechazo al aparecer un error, sólo para los que
/// responden a algo que la persona acaba de hacer (no a una sincronización de
/// fondo que falla sola cada cinco minutos).
@Composable
internal fun FranjaEstado(texto: String, tipo: TipoEstado, modifier: Modifier = Modifier, vibrar: Boolean = false) {
    val hapticos = rememberHapticos()
    if (vibrar && tipo == TipoEstado.ERROR) {
        LaunchedEffect(texto) { hapticos(Haptico.RECHAZAR) }
    }
    val (fondo, contenido) = coloresEstado(tipo)
    Surface(
        shape = RoundedCornerShape(12.dp),
        color = fondo,
        contentColor = contenido,
        modifier = modifier.fillMaxWidth(),
    ) {
        Row(modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
            IconoEstado(tipo)
            Spacer(Modifier.width(10.dp))
            Text(
                texto,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = if (tipo == TipoEstado.ERROR || tipo == TipoEstado.AVISO) FontWeight.SemiBold else FontWeight.Normal,
            )
        }
    }
}

@Composable
private fun IconoEstado(tipo: TipoEstado) {
    if (tipo == TipoEstado.CARGANDO) {
        CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp, color = LocalContentColor.current)
        return
    }
    Icon(
        when (tipo) {
            TipoEstado.ERROR -> Icons.Default.ErrorOutline
            TipoEstado.AVISO -> Icons.Default.Warning
            TipoEstado.EXITO -> Icons.Default.CheckCircle
            else -> Icons.Default.Info
        },
        contentDescription = null,
        modifier = Modifier.size(20.dp),
    )
}

@Composable
private fun coloresEstado(tipo: TipoEstado): Pair<Color, Color> {
    val esquema = MaterialTheme.colorScheme
    return when (tipo) {
        TipoEstado.ERROR, TipoEstado.AVISO -> esquema.errorContainer to esquema.onErrorContainer
        TipoEstado.EXITO -> lerp(esquema.surface, ColorExitoBrisas, 0.16f) to ColorExitoBrisas
        TipoEstado.INFO, TipoEstado.CARGANDO -> esquema.surface to esquema.onSurfaceVariant
    }
}

/// Un aviso corto que baja desde arriba y se va solo ("Está al día",
/// "Teléfono reconectado…"), en vez del Toast de Android, que salía abajo
/// tapando la barra de secciones.
internal data class AvisoBreve(val texto: String, val tipo: TipoEstado, val id: Long)

internal class AvisosBreves {
    var actual by mutableStateOf<AvisoBreve?>(null)
        private set
    private var siguienteId = 0L

    fun mostrar(texto: String, tipo: TipoEstado = TipoEstado.INFO) {
        actual = AvisoBreve(texto, tipo, ++siguienteId)
    }

    /// Sólo si sigue a la vista el mismo: uno nuevo no se borra con el tiempo
    /// del anterior.
    fun ocultar(id: Long) {
        if (actual?.id == id) actual = null
    }
}

/// `null` fuera de la pantalla principal (login, arranque): ahí quien avisa
/// usa un Toast.
internal val LocalAvisosBreves = staticCompositionLocalOf<AvisosBreves?> { null }

/// Dibuja el [AvisoBreve] a la vista, arriba de todo. Tocarlo lo cierra.
@Composable
internal fun HostAvisosBreves(avisos: AvisosBreves, modifier: Modifier = Modifier) {
    val actual = avisos.actual
    var mostrado by remember { mutableStateOf<AvisoBreve?>(null) }
    if (actual != null) mostrado = actual
    LaunchedEffect(actual?.id) {
        val id = actual?.id ?: return@LaunchedEffect
        delay(MS_AVISO_BREVE_VISIBLE)
        avisos.ocultar(id)
    }
    AnimatedVisibility(
        visible = actual != null,
        modifier = modifier,
        enter = fadeIn(tween(MS_FUNDIDO_ENTRADA)) + slideInVertically(tween(MS_ENTRADA)) { alto -> -alto },
        exit = fadeOut(tween(MS_SALIDA)) + slideOutVertically(tween(MS_SALIDA)) { alto -> -alto },
        label = "avisoBreve",
    ) {
        val aviso = mostrado ?: return@AnimatedVisibility
        val paleta = paletaActual
        Surface(
            shape = RoundedCornerShape(percent = 50),
            // Lleno y con texto blanco, en el color de su tipo: verde éxito,
            // azul información, rojo error (pedido del usuario 2026-10-27:
            // antes sólo el éxito tenía color y los demás se confundían con
            // la superficie). Sin sombra (ver la nota de 1.6.3 sobre sombras y
            // desenfoque). Los tonos profundos, no los de texto: en oscuro los
            // claros quedaban fluorescentes (ver `PaletaLattis.kt`).
            color = when (aviso.tipo) {
                TipoEstado.EXITO -> paleta.rellenoExito
                TipoEstado.ERROR, TipoEstado.AVISO -> paleta.rellenoError
                TipoEstado.INFO, TipoEstado.CARGANDO -> paleta.rellenoInfo
            },
            contentColor = paleta.sobreRelleno,
            modifier = Modifier.clickable { avisos.ocultar(aviso.id) },
        ) {
            Row(modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                IconoEstado(aviso.tipo)
                Spacer(Modifier.width(10.dp))
                Text(
                    aviso.texto,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

internal const val MS_AVISO_BREVE_VISIBLE = 2_500L

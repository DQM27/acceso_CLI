package com.brisas.controlacceso

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Groups
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

// Lista de quién está adentro, con sus estados (sacada de
// `ContenidoActivos.kt` para compartirla con Proveedores, Por correo y KOF).

/// Lo que muestra una lista de quién está adentro: la propia lista,
/// marcadores de posición mientras carga por primera vez, o un mensaje si no
/// hay nadie.
private enum class EstadoLista { MARCADORES, VACIA, LISTA }

/// Lista de gente adentro (contratistas, proveedores, visitas por correo,
/// gafetes provisionales KOF): mismo comportamiento en todas.
///
/// - Las filas se acomodan con una animación cuando alguien entra o sale
///   (`animateItem`), en vez de saltar.
/// - Tirar hacia abajo sincroniza con la nube, igual que el botón de arriba.
/// - Sin filas: marcadores mientras carga y un mensaje (no un hueco en
///   blanco) cuando de verdad no hay nadie. Los marcadores sólo salen si no
///   hay nada que mostrar: recargar con la lista llena no la tapa.
/// - Tocar una fila abre la confirmación de salida de cada pantalla: registrar
///   una salida no se hace con un gesto sin confirmar (se probó deslizar con
///   "Deshacer" y se descartó, 2026-10-19).
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun <T> ListaConEstados(
    filas: List<T>,
    clave: (T) -> Any,
    cargando: Boolean,
    tituloVacio: String,
    detalleVacio: String,
    sincronizando: Boolean,
    onSincronizar: () -> Unit,
    // Abajo, lugar para lo que flota sobre la barra (ver [EspacioPieLista]).
    contentPadding: PaddingValues = PaddingValues(top = 5.dp, bottom = EspacioPieLista),
    // El del mensaje sin filas: el de la sección (cajas en Proveedores, sobre
    // en Correo, insignia en KOF), como el ícono de su pestaña.
    iconoVacio: ImageVector = Icons.Default.Groups,
    contenidoFila: @Composable (T) -> Unit,
) {
    val estado = when {
        filas.isNotEmpty() -> EstadoLista.LISTA
        cargando -> EstadoLista.MARCADORES
        else -> EstadoLista.VACIA
    }
    PullToRefreshBox(
        isRefreshing = sincronizando,
        onRefresh = onSincronizar,
        modifier = Modifier.fillMaxSize(),
    ) {
        AnimatedContent(
            targetState = estado,
            modifier = Modifier.fillMaxSize(),
            transitionSpec = {
                (fadeIn(tween(MS_FUNDIDO_ENTRADA)) togetherWith fadeOut(tween(MS_SALIDA))) using null
            },
            label = "estadoListaActivos",
        ) { mostrado ->
            when (mostrado) {
                EstadoLista.LISTA -> LazyColumn(
                    contentPadding = contentPadding,
                    verticalArrangement = Arrangement.spacedBy(5.dp),
                ) {
                    items(filas, key = { fila -> clave(fila) }) { fila ->
                        Box(modifier = Modifier.animateItem()) {
                            contenidoFila(fila)
                        }
                    }
                }
                EstadoLista.MARCADORES -> FilasMarcadoras(contentPadding.calculateTopPadding())
                EstadoLista.VACIA -> EstadoVacio(tituloVacio, detalleVacio, iconoVacio)
            }
        }
    }
}

/// Cuatro tarjetas grises con la forma de una fila, quietas (sin brillo que
/// recorra la pantalla: eso sería una animación infinita sólo para esperar).
@Composable
private fun FilasMarcadoras(arriba: androidx.compose.ui.unit.Dp) {
    val descripcion = "Cargando"
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = arriba)
            .semantics { contentDescription = descripcion },
        verticalArrangement = Arrangement.spacedBy(5.dp),
    ) {
        repeat(4) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(MaterialTheme.shapes.medium)
                    .background(MaterialTheme.colorScheme.surface)
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                BarraMarcadora(ancho = 0.5f, alto = 14.dp)
                BarraMarcadora(ancho = 0.8f, alto = 10.dp)
                BarraMarcadora(ancho = 0.6f, alto = 10.dp)
            }
        }
    }
}

@Composable
private fun BarraMarcadora(ancho: Float, alto: androidx.compose.ui.unit.Dp) {
    Box(
        modifier = Modifier
            .fillMaxWidth(ancho)
            .height(alto)
            .clip(MaterialTheme.shapes.small)
            .background(MaterialTheme.colorScheme.surfaceVariant),
    )
}

/// El mensaje de "no hay nada" de toda la app (listas, resultados de una
/// búsqueda, bandeja de avisos), a la manera de iOS: ícono en un círculo del
/// color de la sección, título y una línea que dice qué hacer.
///
/// Con `verticalScroll` aunque no haya nada que desplazar: sin un hijo
/// desplazable, tirar hacia abajo para sincronizar no responde.
@Composable
internal fun EstadoVacio(titulo: String, detalle: String, icono: ImageVector = Icons.Default.Groups) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(top = 48.dp, start = 24.dp, end = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Box(
            modifier = Modifier
                .size(64.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.primaryContainer),
            contentAlignment = Alignment.Center,
        ) {
            Icon(icono, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(30.dp))
        }
        Text(titulo, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        Text(
            detalle,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
}

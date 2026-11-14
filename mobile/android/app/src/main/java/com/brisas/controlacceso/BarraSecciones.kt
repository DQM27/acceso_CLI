package com.brisas.controlacceso

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Event
import androidx.compose.material.icons.filled.Route
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.getValue

// La barra de secciones de abajo y las secciones mismas, sacadas de
// `PantallaPrincipal.kt` (fase 4 de `docs/handoff-rediseno-movil-ux.md`: un
// archivo, una responsabilidad).

/// Las secciones en una cápsula flotante abajo (como la barra nueva de
/// WhatsApp, pedido del usuario 2026-10-21), en vez de la fila de píldoras
/// arriba: se cambia de sección con el pulgar y arriba queda una sola fila de
/// opciones. Más baja que la `NavigationBar` de Material 3 (80dp) y separada
/// de los bordes.
///
/// - El fondo: la superficie, opaca, con el borde suave (era un velo al 94 %,
///   hasta 2026-11-14). Sin desenfoque a propósito (Haze, 2026-10-22):
///   redibujar el contenido desenfocado detrás de la barra en cada frame
///   duplicó el render de Activos (p90 de 5,5 a 10,1 ms).
/// - La pestaña elegida: una píldora gris suave que acompaña al carrusel de
///   secciones ([posicion], en páginas: se mueve con el dedo al deslizar y
///   con la animación al tocar); su ícono y su nombre van en el acento de la
///   sección (ver `AcentoSeccion.kt`).
/// - Cada sección lleva un globito con cuántos hay adentro.
///
/// "KOF" a secas, no "Gafetes KOF" (pedido explícito del usuario
/// 2026-09-19). "Rutas" no aparece (pedido del usuario 2026-09-23: control de
/// rutas sin terminar); `PantallaRutas` sigue intacta y para volver a
/// mostrarla basta con sacarla de `enDesarrollo` en [SeccionPrincipal].
@Composable
internal fun BarraSecciones(
    seleccionada: SeccionPrincipal,
    // Se lee al dibujar: seguir al dedo no recompone la barra en cada frame.
    posicion: () -> Float,
    adentro: Map<SeccionPrincipal, Int>,
    onSeleccionar: (SeccionPrincipal) -> Unit,
    modifier: Modifier = Modifier,
) {
    val oscuro = temaActual().oscuro
    val superficie = MaterialTheme.colorScheme.surface
    val colorPildora = MaterialTheme.colorScheme.onSurface.copy(alpha = if (oscuro) 0.12f else 0.07f)
    // Menos redonda que una cápsula e igual de los dos lados (pedidos del
    // usuario 2026-10-29). La píldora de la sección elegida, concéntrica: el
    // radio de afuera menos el margen interior, así las curvas quedan
    // paralelas.
    val forma = FormaPiezaEntera
    val radioPildora = RADIO_PIEZA - MARGEN_INTERIOR_BARRA
    val formaPildora = RoundedCornerShape(radioPildora)
    val hapticos = rememberHapticos()
    Row(
        modifier = modifier
            .height(ALTO_BARRA_SECCIONES)
            .clip(forma)
            .background(superficie)
            .border(ANCHO_BORDE_PIEZA, paletaActual.bordeSuave, forma)
            .padding(MARGEN_INTERIOR_BARRA)
            .drawBehind {
                val cantidad = SECCIONES_VISIBLES.size
                val ancho = size.width / cantidad
                val actual = posicion()
                val radio = radioPildora.toPx()
                drawRoundRect(
                    color = colorPildora,
                    topLeft = Offset(actual * ancho, 0f),
                    size = Size(ancho, size.height),
                    cornerRadius = CornerRadius(radio),
                )
            },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        SECCIONES_VISIBLES.forEach { seccion ->
            val elegida = seccion == seleccionada
            val acento = colorDeSeccion(seccion, oscuro) ?: MaterialTheme.colorScheme.primary
            val color by animateColorAsState(
                targetValue = if (elegida) acento else MaterialTheme.colorScheme.onSurfaceVariant,
                animationSpec = tween(MS_FUNDIDO_ENTRADA),
                label = "colorSeccion",
            )
            val cantidad = adentro[seccion] ?: 0
            Column(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxHeight()
                    .clip(formaPildora)
                    .selectable(
                        selected = elegida,
                        role = Role.Tab,
                        onClick = {
                            if (!elegida) hapticos(Haptico.SELECCION)
                            onSeleccionar(seccion)
                        },
                    )
                    .semantics { if (cantidad > 0) stateDescription = "$cantidad adentro" },
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                // Globito gris, no rojo: es una cuenta, no algo que pida
                // atención (en iOS el rojo es para lo pendiente, como la
                // campanita de avisos).
                BadgedBox(
                    badge = {
                        if (cantidad > 0) {
                            Badge(
                                containerColor = MaterialTheme.colorScheme.onSurfaceVariant,
                                contentColor = MaterialTheme.colorScheme.surface,
                            ) { Text("$cantidad") }
                        }
                    },
                ) {
                    Icon(iconoSeccion(seccion), contentDescription = null, tint = color, modifier = Modifier.size(22.dp))
                }
                Text(
                    seccion.etiqueta,
                    color = color,
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = if (elegida) FontWeight.SemiBold else FontWeight.Medium,
                    maxLines = 1,
                    // Con la letra del sistema agrandada, mejor "Contratis…"
                    // que la palabra cortada en seco.
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

internal val ALTO_BARRA_SECCIONES = 58.dp
private val MARGEN_INTERIOR_BARRA = 5.dp
/// El mismo margen que las tarjetas de las listas: con 12dp la barra y la
/// búsqueda sobresalían 4dp de cada lado (pruebas del usuario 2026-10-29).
internal val MARGEN_LATERAL_BARRA = 16.dp
/// Despegada del borde (pedido del usuario 2026-10-21): pegada abajo quedaba
/// incómoda para el pulgar. 18dp se sentía bajo y 28dp alto; 24dp, y 20dp
/// desde que la barra dejó de ser una cápsula (2026-10-29).
internal val MARGEN_INFERIOR_BARRA = 20.dp
internal val MARGEN_SUPERIOR_BARRA = 6.dp

/// Entre la píldora de búsqueda y la barra de secciones: la misma ranura del
/// corte entre la barra y su botón, así se leen como un solo bloque (eran
/// 8dp).
internal val ESPACIO_PILDORA_BARRA = ESPACIO_CORTE

/// Lo que tapa la barra flotante con la píldora de búsqueda encima (altos +
/// margen de abajo + un respiro), para [LocalMargenBarraInferior]. Fijo
/// aunque la píldora se repliegue al desplazar: si cambiara, la lista
/// saltaría mientras se la desliza.
internal val MARGEN_BARRA_SECCIONES =
    ALTO_BARRA_SECCIONES + ALTO_PILDORA_BUSQUEDA + ESPACIO_PILDORA_BARRA + MARGEN_INFERIOR_BARRA + 6.dp

/// Los mismos íconos que el escritorio (ver [IconosLucide]).
internal fun iconoSeccion(seccion: SeccionPrincipal): ImageVector = when (seccion) {
    SeccionPrincipal.CONTRATISTAS -> IconosLucide.UsuarioConCheck
    SeccionPrincipal.KOF -> IconosLucide.InsigniaConCheck
    SeccionPrincipal.PROVEEDORES -> IconosLucide.Cajas
    SeccionPrincipal.CORREO -> IconosLucide.Sobre
    // Ocultas de la barra (en desarrollo).
    SeccionPrincipal.RUTAS -> Icons.Default.Route
    SeccionPrincipal.VISITAS -> Icons.Default.Event
}

/// Secciones de la pantalla principal, en el orden de la barra de abajo
/// (punto M5 de la auditoría móvil: antes eran textos sueltos).
/// `enDesarrollo` las oculta de la barra sin borrar su pantalla (ver
/// [BarraSecciones]).
enum class SeccionPrincipal(val etiqueta: String, val enDesarrollo: Boolean = false) {
    // "Contratistas" y no "Activos" (pedido del usuario 2026-10-26): la
    // pestaña es sólo de contratistas; KOF, Proveedores y Correo también
    // tienen gente adentro.
    // En el orden de lo que más se usa (pedido del usuario 2026-10-29).
    CONTRATISTAS("Contratistas"),
    RUTAS("Rutas", enDesarrollo = true),
    // Proveedores y Correo eran una sola sección, "Externos", con un
    // selector adentro; se separaron (pedido del usuario 2026-10-29): con el
    // deslizamiento entre secciones, cada una queda a un gesto y con su
    // propio globito.
    PROVEEDORES("Proveedores"),
    CORREO("Correo"),
    KOF("KOF"),
    // Visitas agendadas en la web (`PantallaVisitas`): oculta desde
    // 2026-10-17; para volver a mostrarla basta con sacarla de `enDesarrollo`.
    VISITAS("Visitas", enDesarrollo = true),
}

internal val SECCIONES_VISIBLES = SeccionPrincipal.entries.filterNot { it.enDesarrollo }

/// La sección de una tabla de la nube que llega por un aviso en vivo (las que
/// `control_acceso::nube::aplicar_cambio_en_vivo` guarda al instante); `null`
/// si no corresponde a ninguna.
internal fun seccionDeTabla(tabla: String?): SeccionPrincipal? = when (tabla) {
    "ingresos" -> SeccionPrincipal.CONTRATISTAS
    "ingresos_proveedor" -> SeccionPrincipal.PROVEEDORES
    "ingresos_correo" -> SeccionPrincipal.CORREO
    "prestamos_gafete_provisional" -> SeccionPrincipal.KOF
    else -> null
}

/// Un aviso en vivo ya guardado en la base local: qué sección refrescar.
/// `id` distingue dos avisos seguidos de la misma sección: cada uno refresca.
internal data class CambioEnVivo(val seccion: SeccionPrincipal?, val id: Long)

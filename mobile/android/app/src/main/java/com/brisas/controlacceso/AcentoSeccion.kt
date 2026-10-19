package com.brisas.controlacceso

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp

/// Color de cada sección principal (pedido del usuario 2026-10-19): con un
/// solo acento azul para todo, Contratistas, KOF y Externos se confundían de
/// un vistazo. Lleva el contexto del guardia: "estoy en la pestaña roja".
///
/// El acento pinta sólo el marco (pestaña activa, botones, bordes, íconos y
/// puntos: todo lo que ya usaba `colorScheme.primary`). Los colores de estado
/// (confirmado, error, vencido: `EscaneoCompartido.kt`) NO cambian entre
/// secciones, para que siempre signifiquen lo mismo.
///
/// Tonos elegidos para no parecerse a los de estado: verde azulado (no el
/// verde pasto de "confirmado"), rojo KOF franco (el de error es más apagado
/// y sólo sale como mensaje) y mango tostado (más oscuro que el ámbar de
/// "vencido"). Contraste medido contra Tokyo Day / Tokyo Night en
/// `AcentoSeccionTest`: el acento contra el fondo llega a 3:1 y el texto sobre
/// el acento a 4,5:1.
@Immutable
class AcentoSeccion(
    val color: Color,
    /// Texto e íconos sobre un relleno de [color].
    val sobreColor: Color,
)

private val TextoSobreAcentoClaro = Color(0xFFFFFFFF)
private val TextoSobreAcentoOscuro = Color(0xFF16161E)

/// `null` = sin acento propio: la sección conserva el de la paleta base
/// (Rutas, que está en desarrollo y no se ve en la fila de pestañas).
fun acentoDeSeccion(seccion: SeccionPrincipal, oscuro: Boolean): AcentoSeccion? {
    val color = when (seccion) {
        SeccionPrincipal.ACTIVOS -> if (oscuro) Color(0xFF00E0B0) else Color(0xFF00806A)
        SeccionPrincipal.KOF -> if (oscuro) Color(0xFFFF4B3E) else Color(0xFFC4161C)
        SeccionPrincipal.EXTERNOS -> if (oscuro) Color(0xFFFFA000) else Color(0xFFB85C00)
        SeccionPrincipal.RUTAS -> return null
    }
    return AcentoSeccion(color, if (oscuro) TextoSobreAcentoOscuro else TextoSobreAcentoClaro)
}

/// Ejecuta [contenido] con el acento de [seccion] en lugar del color
/// principal de la paleta. Todo lo que lee `MaterialTheme.colorScheme.primary`
/// lo toma solo; no hace falta tocar cada pantalla.
@Composable
fun TemaSeccion(seccion: SeccionPrincipal, contenido: @Composable () -> Unit) {
    val base = MaterialTheme.colorScheme
    val oscuro = temaActual().oscuro
    val esquema = remember(base, seccion, oscuro) {
        val acento = acentoDeSeccion(seccion, oscuro) ?: return@remember base
        base.copy(
            primary = acento.color,
            onPrimary = acento.sobreColor,
            // Fondo suave del acento (círculo detrás de un ícono, por ejemplo).
            primaryContainer = lerp(base.surface, acento.color, if (oscuro) 0.22f else 0.16f),
            onPrimaryContainer = acento.color,
            surfaceTint = acento.color,
            inversePrimary = acento.color,
        )
    }
    MaterialTheme(colorScheme = esquema, content = contenido)
}

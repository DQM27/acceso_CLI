// Identidad visual propia de Android (2026-09-15) -- hasta acá este archivo
// era `DisenoGenerado.kt`, generado desde `design/brisas.json` junto con
// desktop y web-visitas (ver `design/generar.mjs`). Se desacopló a pedido
// explícito: el usuario quería el look de la app "Kash" (acento
// verde-azulado, fondo lavanda muy claro, esquinas bien redondeadas) SOLO
// en el teléfono -- desktop sigue con su navy ya publicado (hasta v1.5.4) y
// web-visitas acaba de pasar por un rediseño propio con acento rojo
// (`docs/features-futuras/plan-rediseno-web-visitas.md`); pisarlo con este
// cambio habría sido una regresión de marca no pedida.
//
// A partir de ahora este archivo se edita a mano -- `design/generar.mjs` ya
// no lo toca (ver ese archivo). Si algún día se decide unificar de nuevo la
// marca entre las tres plataformas, hay que sumar `mobile` de vuelta al
// generador en vez de reintroducir un archivo "Generado" que nadie genera.
package com.brisas.controlacceso

import androidx.compose.runtime.Composable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Typography
import androidx.compose.material3.Shapes
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import androidx.compose.ui.unit.dp

internal val FormaControlBrisas = RoundedCornerShape(16.dp)
internal val FormasBrisas = Shapes(
    extraSmall = FormaControlBrisas,
    small = FormaControlBrisas,
    medium = RoundedCornerShape(20.dp),
    large = RoundedCornerShape(20.dp),
    extraLarge = RoundedCornerShape(20.dp),
)
// Los colores salen de `PaletaLattis.kt`, según el tema elegido en la app
// ([temaActual]), no sólo el del sistema.
internal val ColorRellenoBrisas: Color
    @Composable get() = paletaActual.relleno
internal val ColorSobreRellenoBrisas: Color
    @Composable get() = paletaActual.sobreRelleno
internal val EspacioControlBrisas = 12.dp
// 20% más chico que los 48.dp originales, a pedido explícito (2026-09-15) --
// no estaba "muy grande", pero se veía mejor un poco más compacto.
internal val AlturaControlBrisas = 38.dp
internal val ColorExitoBrisas: Color
    @Composable get() = paletaActual.exito
/// Ámbar de advertencia para íconos y puntos (el triángulo, el punto de
/// conexión).
internal val ColorAdvertenciaBrisas: Color
    @Composable get() = paletaActual.advertencia
/// El mismo ámbar para texto: en el tema claro, más oscuro para que se lea
/// sobre fondo blanco.
internal val ColorTextoAdvertenciaBrisas: Color
    @Composable get() = paletaActual.textoAdvertencia
/// Escala completa, con saltos claros entre niveles a la manera de iOS
/// (título grande, cuerpo, subtítulo, nota): antes `bodyLarge` y
/// `bodyMedium` medían lo mismo (14sp) y el nombre de una persona casi no se
/// distinguía de su detalle. El cuerpo (`bodyLarge`, el de los nombres y los
/// campos) sube a 16sp para leerse bien al sol en la portería.
internal val TipografiaBrisas = Typography(
    headlineMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 26.sp, lineHeight = 32.sp, fontWeight = FontWeight(700)),
    headlineSmall = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 24.sp, lineHeight = 30.sp, fontWeight = FontWeight(700)),
    titleLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 22.sp, lineHeight = 28.sp, fontWeight = FontWeight(700)),
    titleMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 17.sp, lineHeight = 22.sp, fontWeight = FontWeight(600)),
    titleSmall = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 15.sp, lineHeight = 20.sp, fontWeight = FontWeight(600)),
    bodyLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 16.sp, lineHeight = 22.sp),
    bodyMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp, lineHeight = 20.sp),
    bodySmall = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 13.sp, lineHeight = 18.sp),
    labelLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 15.sp, lineHeight = 20.sp, fontWeight = FontWeight(600)),
    labelMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 12.sp, lineHeight = 16.sp, fontWeight = FontWeight(500)),
    labelSmall = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 11.sp, lineHeight = 14.sp, fontWeight = FontWeight(500)),
)

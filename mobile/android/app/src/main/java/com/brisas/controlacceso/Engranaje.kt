package com.brisas.controlacceso

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import java.util.Locale
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

/// Trazo SVG de un engranaje centrado en (50, 50) de un lienzo de 100 x 100:
/// `dientes` dientes trapezoidales de radio `radioExterior`, unidos por arcos de
/// radio `radioBase`. Misma fórmula que `trazoEngranaje` del escritorio
/// (`desktop/src/componentes/Engranaje.logica.ts`), así el engranaje de la
/// cuenta es idéntico en las dos apps.
internal fun trazoEngranaje(dientes: Int, radioExterior: Int, radioBase: Int): String {
    val paso = 2 * PI / dientes
    val mitadBase = paso * 0.27
    val mitadPunta = paso * 0.17
    fun punto(radio: Int, angulo: Double) =
        "${dosDecimales(50 + radio * sin(angulo))} ${dosDecimales(50 - radio * cos(angulo))}"

    val trazo = StringBuilder()
    for (i in 0 until dientes) {
        val centro = i * paso
        trazo.append(if (trazo.isEmpty()) "M" else "A$radioBase $radioBase 0 0 1 ")
        trazo.append("${punto(radioBase, centro - mitadBase)} ")
        trazo.append("L${punto(radioExterior, centro - mitadPunta)} ")
        trazo.append("L${punto(radioExterior, centro + mitadPunta)} ")
        trazo.append("L${punto(radioBase, centro + mitadBase)} ")
    }
    return "${trazo}A$radioBase $radioBase 0 0 1 ${punto(radioBase, -mitadBase)} Z"
}

private fun dosDecimales(valor: Double): String = String.format(Locale.ROOT, "%.2f", valor)

/// El del escritorio: 8 dientes, trazo de 6 sobre 100 (ver `.engranaje svg` en
/// `desktop/src/index.css`).
private val VectorEngranaje: ImageVector by lazy {
    ImageVector.Builder(
        name = "Engranaje",
        defaultWidth = 24.dp,
        defaultHeight = 24.dp,
        viewportWidth = 100f,
        viewportHeight = 100f,
    ).addPath(
        pathData = addPathNodes(trazoEngranaje(8, 47, 37)),
        fill = null,
        stroke = SolidColor(Color.Black),
        strokeLineWidth = 6f,
        strokeLineJoin = StrokeJoin.Round,
    ).build()
}

/// La cuenta de quien tiene la sesión, como en el escritorio: su inicial dentro
/// de un engranaje ("ajustes de mi cuenta"). Dibujado con [color]; la inicial
/// escala con el tamaño. [giro] (grados) rota sólo la rueda, no la letra.
@Composable
internal fun EngranajeConInicial(
    inicial: String,
    color: Color,
    tamano: Dp,
    modifier: Modifier = Modifier,
    giro: () -> Float = { 0f },
) {
    Box(modifier = modifier.size(tamano), contentAlignment = Alignment.Center) {
        Icon(
            VectorEngranaje,
            contentDescription = null,
            tint = color,
            // Leído en la fase de dibujo: girar no recompone nada.
            modifier = Modifier.size(tamano).graphicsLayer { rotationZ = giro() },
        )
        Text(
            inicial,
            color = color,
            fontWeight = FontWeight.Bold,
            fontSize = (tamano.value * 0.36f).sp,
        )
    }
}

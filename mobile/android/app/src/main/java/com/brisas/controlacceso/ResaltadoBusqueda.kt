package com.brisas.controlacceso

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import java.text.Normalizer

/// Dónde aparece lo buscado dentro de [texto], para resaltarlo en los
/// resultados como hace iOS. No distingue mayúsculas ni tildes ("jose"
/// resalta "José") y cada palabra de la consulta se busca por separado
/// ("mora ana" resalta las dos). Los rangos salen ordenados y sin solaparse.
internal fun rangosCoincidencia(texto: String, consulta: String): List<IntRange> {
    val palabras = consulta.split(' ', ',').map(::plano).filter { it.isNotBlank() }
    if (palabras.isEmpty() || texto.isEmpty()) return emptyList()
    val base = plano(texto)
    val rangos = mutableListOf<IntRange>()
    for (palabra in palabras) {
        var desde = base.indexOf(palabra)
        while (desde >= 0) {
            rangos.add(desde until desde + palabra.length)
            desde = base.indexOf(palabra, desde + palabra.length)
        }
    }
    return unirRangos(rangos)
}

/// Minúsculas y sin tildes, letra por letra: el resultado mide lo mismo que
/// [texto], así los índices sirven para el original.
private fun plano(texto: String): String = buildString(texto.length) {
    for (letra in texto) {
        val descompuesta = Normalizer.normalize(letra.toString(), Normalizer.Form.NFD)
        append(descompuesta.firstOrNull()?.lowercaseChar() ?: letra)
    }
}

private fun unirRangos(rangos: List<IntRange>): List<IntRange> {
    val unidos = mutableListOf<IntRange>()
    for (rango in rangos.sortedBy { it.first }) {
        val ultimo = unidos.lastOrNull()
        if (ultimo != null && rango.first <= ultimo.last + 1) {
            unidos[unidos.lastIndex] = ultimo.first..maxOf(ultimo.last, rango.last)
        } else {
            unidos.add(rango)
        }
    }
    return unidos
}

/// Lo que se está buscando ahora: lo pone [BuscadorInferior] para sus
/// resultados y las filas lo resaltan solas con [textoResaltado]. Vacío fuera
/// de una búsqueda.
internal val LocalConsultaResaltada = compositionLocalOf { "" }

/// [texto] con lo buscado en el color de la sección y en negrita.
@Composable
internal fun textoResaltado(texto: String): AnnotatedString {
    val consulta = LocalConsultaResaltada.current
    if (consulta.isBlank()) return AnnotatedString(texto)
    val estilo = SpanStyle(color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold)
    return buildAnnotatedString { agregarResaltado(texto, consulta, estilo) }
}

/// Agrega [texto] con lo que coincide con [consulta] en [estilo].
internal fun AnnotatedString.Builder.agregarResaltado(texto: String, consulta: String, estilo: SpanStyle) {
    val inicio = length
    append(texto)
    for (rango in rangosCoincidencia(texto, consulta)) {
        addStyle(estilo, inicio + rango.first, inicio + rango.last + 1)
    }
}

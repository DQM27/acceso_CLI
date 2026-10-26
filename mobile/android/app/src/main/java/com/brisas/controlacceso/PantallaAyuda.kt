package com.brisas.controlacceso

import androidx.compose.foundation.layout.imePadding
import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withLink
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.BloqueManual
import uniffi.control_acceso_mobile.CeldaManual
import uniffi.control_acceso_mobile.TramoManual
import uniffi.control_acceso_mobile.buscarEnManual
import uniffi.control_acceso_mobile.indiceManual
import uniffi.control_acceso_mobile.temaManual

/// Un tema abierto y, si hace falta, la sección donde arrancar.
internal data class DestinoAyuda(val ruta: String, val ancla: String?)

/// Ayuda del menú de la cuenta: el manual de uso de la app
/// (`docs/manual/movil/`), sin internet y sin IA. El núcleo trae el manual
/// adentro, arma el índice, busca y entrega cada tema ya en bloques
/// (`mobile/rust-core/src/manual/`); acá sólo se dibuja.
///
/// Dos vistas: la lista (índice o resultados de la búsqueda) y el tema. Los
/// enlaces entre temas abren el otro tema; "Volver" y atrás del sistema
/// desandan el camino hasta la lista, y de ahí salen de la Ayuda.
@Composable
fun PantallaAyuda(onVolver: () -> Unit) {
    RegistrarPantalla("ayuda")
    var consulta by rememberSaveable { mutableStateOf("") }
    var actual by remember { mutableStateOf<DestinoAyuda?>(null) }
    val historial = remember { mutableStateListOf<DestinoAyuda>() }

    fun abrir(destino: DestinoAyuda) {
        actual?.let { historial.add(it) }
        actual = destino
    }

    fun volver() {
        when {
            historial.isNotEmpty() -> actual = historial.removeAt(historial.lastIndex)
            actual != null -> actual = null
            else -> onVolver()
        }
    }

    // Atrás del sistema, con su gesto, desanda igual que "‹ Volver".
    PantallaConGestoVolver(onVolver = ::volver) {
        AnimatedContent(
            targetState = actual,
            modifier = Modifier.fillMaxSize(),
            transitionSpec = { transicionDePantalla(avanza = targetState != null) },
            label = "ayuda",
        ) { destino ->
            if (destino == null) {
                ListaAyuda(consulta = consulta, onConsulta = { consulta = it }, onAbrir = ::abrir, onVolver = ::volver)
            } else {
                TemaAyuda(destino, onAbrir = ::abrir, onVolver = ::volver)
            }
        }
    }
}

/// Como el resto de las pantallas completas: "‹ Volver" chico arriba y el
/// título grande debajo ([EncabezadoPantalla]).
@Composable
private fun EncabezadoAyuda(titulo: String, onVolver: () -> Unit) {
    EncabezadoPantalla(titulo = titulo, onVolver = onVolver)
}

@Composable
private fun ListaAyuda(
    consulta: String,
    onConsulta: (String) -> Unit,
    onAbrir: (DestinoAyuda) -> Unit,
    onVolver: () -> Unit,
) {
    val indice = remember { indiceManual() }
    val buscando = consulta.trim().length >= 2
    val resultados = remember(consulta) { if (buscando) buscarEnManual(consulta) else emptyList() }

    // El buscador abajo, pegado al teclado, como en el resto de la app.
    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp).imePadding()) {
        EncabezadoAyuda("Ayuda", onVolver)
        LazyColumn(
            contentPadding = PaddingValues(top = 12.dp, bottom = 12.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp),
            modifier = Modifier.weight(1f),
        ) {
            if (buscando) {
                if (resultados.isEmpty()) {
                    item {
                        Text(
                            "No hay nada con esas palabras. Pruebe con otras más cortas (por ejemplo, «gafete» o «salida»).",
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(vertical = 16.dp),
                        )
                    }
                }
                items(resultados, key = { it.ruta }) { resultado ->
                    FilaAyuda(
                        titulo = resultado.titulo,
                        detalle = resultado.seccion,
                        fragmento = resultado.fragmento.takeIf { it.isNotEmpty() },
                        onClick = { onAbrir(DestinoAyuda(resultado.ruta, resultado.ancla)) },
                    )
                }
            } else {
                for (grupo in indice) {
                    if (grupo.titulo.isNotEmpty()) {
                        item(key = "grupo-${grupo.titulo}") {
                            Text(
                                grupo.titulo.uppercase(),
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(top = 16.dp, bottom = 4.dp),
                            )
                        }
                    }
                    items(grupo.temas, key = { it.ruta }) { tema ->
                        FilaAyuda(titulo = tema.titulo, onClick = { onAbrir(DestinoAyuda(tema.ruta, null)) })
                    }
                }
            }
        }
        CampoBusquedaBrisas(
            valor = consulta,
            onCambiar = onConsulta,
            ayuda = "Buscar, o pegar un mensaje",
            modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
        )
    }
}

@Composable
private fun FilaAyuda(titulo: String, onClick: () -> Unit, detalle: String? = null, fragmento: String? = null) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(vertical = 10.dp),
    ) {
        Text(titulo, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        if (detalle != null) {
            Text(detalle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
        }
        if (fragmento != null) {
            Text(fragmento, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun TemaAyuda(destino: DestinoAyuda, onAbrir: (DestinoAyuda) -> Unit, onVolver: () -> Unit) {
    val tema = remember(destino.ruta) { temaManual(destino.ruta) }
    val estadoLista = rememberLazyListState()
    // El título grande va aparte (arriba): se saltea el primero.
    val bloques = remember(tema) { tema?.bloques?.dropWhile { it is BloqueManual.Titulo && it.nivel == 1u }.orEmpty() }
    LaunchedEffect(destino) {
        val indice = indiceDeAncla(bloques, destino.ancla)
        if (indice >= 0) estadoLista.scrollToItem(indice)
    }
    val enlace: (TramoManual) -> Unit = { tramo -> tramo.enlace?.let { onAbrir(DestinoAyuda(it, tramo.anclaEnlace)) } }

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp)) {
        EncabezadoAyuda(tema?.titulo ?: "Ayuda", onVolver)
        LazyColumn(
            state = estadoLista,
            contentPadding = PaddingValues(top = 8.dp, bottom = 32.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
            modifier = Modifier.weight(1f),
        ) {
            items(bloques) { bloque -> BloqueAyuda(bloque, enlace) }
        }
    }
}

/// Posición del título con esa ancla entre los bloques, o -1.
internal fun indiceDeAncla(bloques: List<BloqueManual>, ancla: String?): Int =
    if (ancla == null) -1 else bloques.indexOfFirst { it is BloqueManual.Titulo && it.ancla == ancla }

@Composable
private fun BloqueAyuda(bloque: BloqueManual, onEnlace: (TramoManual) -> Unit) {
    val colorEnlace = MaterialTheme.colorScheme.primary
    val texto = { tramos: List<TramoManual> -> textoConEstilo(tramos, colorEnlace, onEnlace) }
    when (bloque) {
        is BloqueManual.Titulo -> Text(
            texto(bloque.tramos),
            style = if (bloque.nivel <= 2u) MaterialTheme.typography.titleMedium else MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 14.dp),
        )
        is BloqueManual.Parrafo -> Text(
            texto(bloque.tramos),
            style = MaterialTheme.typography.bodyLarge,
            modifier = Modifier.padding(start = sangria(bloque.sangria)),
        )
        is BloqueManual.Elemento -> Row(modifier = Modifier.padding(start = sangria(bloque.sangria))) {
            Text(
                bloque.marca,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.width(26.dp),
            )
            Text(texto(bloque.tramos), style = MaterialTheme.typography.bodyLarge)
        }
        // En el teléfono no entra una tabla de tres columnas: cada fila es
        // una tarjeta con la primera celda arriba (el mensaje) y las demás
        // debajo, con el nombre de su columna.
        is BloqueManual.Tabla -> Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            for (fila in bloque.filas) {
                Surface(
                    shape = RoundedCornerShape(16.dp),
                    color = MaterialTheme.colorScheme.surface,
                    border = BorderStroke(0.5.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.3f)),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(
                        modifier = Modifier.padding(14.dp),
                        verticalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        fila.celdas.forEachIndexed { i, celda ->
                            if (i == 0) {
                                Text(texto(celda.tramos), style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold)
                            } else {
                                CeldaConColumna(bloque.encabezado.getOrNull(i), celda, texto)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun CeldaConColumna(columna: CeldaManual?, celda: CeldaManual, texto: (List<TramoManual>) -> AnnotatedString) {
    Column {
        if (columna != null) {
            Text(
                texto(columna.tramos),
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Text(texto(celda.tramos), style = MaterialTheme.typography.bodyMedium)
    }
}

private fun sangria(nivel: UInt) = (nivel.toInt() * 20).dp

/// Los tramos de un bloque con su estilo: negrita, código y enlaces tocables
/// a otro tema.
private fun textoConEstilo(
    tramos: List<TramoManual>,
    colorEnlace: Color,
    onEnlace: (TramoManual) -> Unit,
): AnnotatedString = buildAnnotatedString {
    for (tramo in tramos) {
        val estilo = SpanStyle(
            fontWeight = if (tramo.negrita) FontWeight.SemiBold else null,
            fontFamily = if (tramo.codigo) FontFamily.Monospace else null,
        )
        if (tramo.enlace != null) {
            val enlace = LinkAnnotation.Clickable(
                tag = tramo.enlace ?: "",
                styles = TextLinkStyles(SpanStyle(color = colorEnlace, textDecoration = TextDecoration.Underline)),
            ) { onEnlace(tramo) }
            withLink(enlace) { withStyle(estilo) { append(tramo.texto) } }
        } else {
            withStyle(estilo) { append(tramo.texto) }
        }
    }
}

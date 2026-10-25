package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.AddCircle
import androidx.compose.material.icons.filled.DocumentScanner
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import kotlinx.coroutines.launch

// Piezas de los formularios al estilo de iOS (Ajustes, Contactos): grupos
// redondeados con un título chico encima y una nota al pie, renglones de
// campo con su etiqueta, renglones que abren una lista o un selector, e
// interruptores. Las usan "Nuevo contratista" y los ingresos de proveedor y
// por correo (rediseño 2026-10-25: antes eran campos con borde sueltos y
// tarjetas numeradas). Los campos en sí son los del login
// ([GrupoCampos]/[CampoAgrupado] en `HojaPorteria.kt`).

/// Un grupo de un formulario: título chico en mayúsculas encima, el bloque
/// redondeado con sus renglones ([GrupoCampos]) y, si hace falta, una nota al
/// pie ([pie]; en rojo con [pieEsError], como "PRAIND vencido").
@Composable
internal fun SeccionFormulario(
    titulo: String?,
    modifier: Modifier = Modifier,
    pie: String? = null,
    pieEsError: Boolean = false,
    contenido: @Composable ColumnScope.() -> Unit,
) {
    Column(modifier = modifier.fillMaxWidth().padding(top = 20.dp)) {
        if (titulo != null) {
            Text(
                titulo.uppercase(),
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 16.dp, bottom = 6.dp),
            )
        }
        GrupoCampos(contenido = contenido)
        if (pie != null) {
            Text(
                pie,
                style = MaterialTheme.typography.bodySmall,
                color = if (pieEsError) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                fontWeight = if (pieEsError) FontWeight.SemiBold else FontWeight.Normal,
                modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 6.dp),
            )
        }
    }
}

/// El escáner como primera pieza del formulario: casi siempre los datos los
/// llena la cámara (OCR), así que va arriba, grande y tocable entera, en el
/// color suave de la sección. No es un botón relleno: la acción principal del
/// formulario sigue siendo una sola, la de abajo (guardar o registrar).
@Composable
internal fun TarjetaEscaner(titulo: String, detalle: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(top = 12.dp)
            .clip(RoundedCornerShape(14.dp))
            .background(MaterialTheme.colorScheme.primaryContainer)
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier
                .size(44.dp)
                .clip(RoundedCornerShape(12.dp))
                .background(MaterialTheme.colorScheme.primary),
            contentAlignment = Alignment.Center,
        ) {
            Icon(Icons.Default.DocumentScanner, contentDescription = null, tint = MaterialTheme.colorScheme.onPrimary)
        }
        Spacer(Modifier.width(14.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(titulo, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
            Text(detalle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Icon(
            Icons.AutoMirrored.Filled.KeyboardArrowRight,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
        )
    }
}

/// Un renglón que abre otra cosa (una lista con buscador, un selector de
/// fecha): la etiqueta a la izquierda, el valor elegido a la derecha (o
/// [ayuda] en gris si no hay) y la flechita ›. Con [error], el valor en rojo.
@Composable
internal fun FilaSelector(
    etiqueta: String,
    valor: String?,
    ayuda: String,
    onClick: () -> Unit,
    error: Boolean = false,
    habilitado: Boolean = true,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 52.dp)
            .clickable(enabled = habilitado, role = Role.Button, onClick = onClick)
            .padding(start = 16.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(etiqueta, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.width(ANCHO_ETIQUETA_CAMPO))
        Text(
            valor ?: ayuda,
            style = MaterialTheme.typography.bodyLarge,
            color = when {
                error -> MaterialTheme.colorScheme.error
                valor == null -> MaterialTheme.colorScheme.onSurfaceVariant
                else -> MaterialTheme.colorScheme.onSurface
            },
            textAlign = TextAlign.End,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        Icon(
            Icons.AutoMirrored.Filled.KeyboardArrowRight,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/// Un renglón con interruptor, como en Ajustes de iOS (en vez de una casilla
/// de Material). Toda la fila lo cambia.
@Composable
internal fun FilaInterruptor(etiqueta: String, marcado: Boolean, onCambiar: (Boolean) -> Unit) {
    val hapticos = rememberHapticos()
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 52.dp)
            .clickable(role = Role.Switch) {
                hapticos(if (marcado) Haptico.DESACTIVAR else Haptico.ACTIVAR)
                onCambiar(!marcado)
            }
            .padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(etiqueta, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
        Switch(
            checked = marcado,
            onCheckedChange = null,
            colors = SwitchDefaults.colors(checkedTrackColor = MaterialTheme.colorScheme.primary),
        )
    }
}

/// El ícono al final de un renglón de campo (la cámara de la placa).
@Composable
internal fun BotonIconoFila(icono: ImageVector, descripcion: String, onClick: () -> Unit) {
    IconButton(onClick = onClick) {
        Icon(icono, contentDescription = descripcion, tint = MaterialTheme.colorScheme.primary)
    }
}

/// Un resultado tocable dentro del buscador de abajo de un formulario (una
/// empresa, por ejemplo), en una tarjeta como las del resto de las listas.
@Composable
internal fun FilaResultado(texto: String, onClick: () -> Unit) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
    ) {
        Text(textoResaltado(texto), style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
    }
}

/// "Crear «TEXTO»" como primer resultado cuando lo buscado no existe, como
/// cuando Contactos ofrece agregar lo que se escribió.
@Composable
internal fun FilaCrear(texto: String, onClick: () -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(Icons.Default.AddCircle, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
        Spacer(Modifier.width(12.dp))
        Text(
            "Crear «$texto»",
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.primary,
            fontWeight = FontWeight.Medium,
        )
    }
}

/// Elegir una fecha en una hoja que sube desde abajo, en vez de escribirla.
/// [fecha] y lo que devuelve [onElegir] van en el mismo formato de siempre,
/// DD-MM-AAAA, así lo que llena el escáner (casi siempre) y lo que se elige
/// acá son lo mismo para el ViewModel.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun HojaFecha(titulo: String, fecha: String, onElegir: (String) -> Unit, onCerrar: () -> Unit) {
    val estadoHoja = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val estadoFecha = rememberDatePickerState(initialSelectedDateMillis = fechaDDMMAAAAaMillis(fecha))
    val alcance = rememberCoroutineScope()
    ModalBottomSheet(
        onDismissRequest = onCerrar,
        sheetState = estadoHoja,
        containerColor = MaterialTheme.colorScheme.surface,
    ) {
        Column(modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp)) {
            Text(
                titulo,
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(horizontal = 24.dp),
            )
            DatePicker(
                state = estadoFecha,
                showModeToggle = false,
                title = null,
                headline = null,
                colors = DatePickerDefaults.colors(containerColor = MaterialTheme.colorScheme.surface),
            )
            BotonBrisas(
                onClick = {
                    estadoFecha.selectedDateMillis?.let { onElegir(millisAFechaDDMMAAAA(it)) }
                    alcance.launch { estadoHoja.hide() }.invokeOnCompletion { onCerrar() }
                },
                enabled = estadoFecha.selectedDateMillis != null,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp),
            ) {
                Text("Listo")
            }
        }
    }
}

/// "DD-MM-AAAA" a los milisegundos (medianoche UTC) que usa el selector, o
/// `null` si todavía no es una fecha completa y válida (por ejemplo, una
/// lectura del escáner a medias).
internal fun fechaDDMMAAAAaMillis(texto: String): Long? {
    val partes = texto.split("-")
    if (partes.size != 3 || partes[2].length != 4) return null
    val (dia, mes, anio) = partes.map { it.toIntOrNull() ?: return null }
    return runCatching { LocalDate.of(anio, mes, dia) }.getOrNull()
        ?.atStartOfDay(ZoneOffset.UTC)?.toInstant()?.toEpochMilli()
}

/// Lo que devuelve el selector, en "DD-MM-AAAA".
internal fun millisAFechaDDMMAAAA(millis: Long): String {
    val fecha = Instant.ofEpochMilli(millis).atZone(ZoneOffset.UTC).toLocalDate()
    return "%02d-%02d-%04d".format(fecha.dayOfMonth, fecha.monthValue, fecha.year)
}

/// "12-11-2026" como se lee en iOS: "12 nov 2026". Si no es una fecha
/// completa, el texto tal cual (para que se vea lo que leyó el escáner);
/// vacío es `null` (no hay fecha).
internal fun textoFechaLegible(texto: String): String? {
    if (texto.isBlank()) return null
    val millis = fechaDDMMAAAAaMillis(texto) ?: return texto
    val fecha = Instant.ofEpochMilli(millis).atZone(ZoneOffset.UTC).toLocalDate()
    return "${fecha.dayOfMonth} ${MESES_CORTOS[fecha.monthValue - 1]} ${fecha.year}"
}

private val MESES_CORTOS = listOf("ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic")

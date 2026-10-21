package com.brisas.controlacceso

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ExpandLess
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material.icons.filled.SyncProblem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.ConflictoGafeteActivo
import uniffi.control_acceso_mobile.ConflictoIngresoActivo
import uniffi.control_acceso_mobile.ConflictoIngresoProveedorActivo

/// Los avisos de conflicto que trae cada sincronización (ingresos activos acá
/// Y en otro sitio, y gafetes que la nube no aceptó), en el orden en que se
/// muestran. Mismos textos que escritorio (`App.tsx`, `conflictoGafete.ts`)
/// para no dar un aviso distinto según qué interfaz lo muestre.
///
/// Los de ingreso son simétricos (los dos lados "tienen razón" hasta que
/// alguien cierra uno); en los de gafete Postgres ya decidió y el movimiento
/// de ESTE teléfono es el que no quedó (ver [mensajeConflictoGafete]). Siguen
/// saliendo en cada sincronización mientras no se resuelvan: no son errores
/// pasajeros que convenga ocultar solos.
internal fun mensajesConflictos(
    ingreso: List<ConflictoIngresoActivo>,
    proveedor: List<ConflictoIngresoProveedorActivo>,
    correo: List<ConflictoIngresoProveedorActivo>,
    gafete: List<ConflictoGafeteActivo>,
    hora: (String) -> String = ::textoFechaHora,
): List<String> =
    ingreso.map { "${it.contratistaNombre} tiene un ingreso activo acá Y en ${it.sitioConflicto} — hay que resolverlo." } +
        proveedor.map { "${it.nombre} tiene un ingreso de proveedor activo acá Y en ${it.sitioConflicto} — hay que resolverlo." } +
        correo.map { "${it.nombre} tiene un ingreso por correo activo acá Y en ${it.sitioConflicto} — hay que resolverlo." } +
        gafete.map { mensajeConflictoGafete(it, hora(it.fechaHora)) }

/// Título de la franja cuando hay más de un conflicto.
internal fun tituloConflictos(cantidad: Int): String =
    if (cantidad == 1) "1 conflicto por resolver" else "$cantidad conflictos por resolver"

/// Una sola franja para todos los conflictos, en vez de un renglón rojo por
/// cada uno (con tres o cuatro se comían media pantalla y no se leían como un
/// aviso). Con uno solo muestra el mensaje directo; con varios, el total, y
/// al tocarla se despliegan todos (y se vuelven a plegar). Mismo estilo que
/// [FranjaEmergencia], con otro ícono: no es un aviso de la bandeja.
@Composable
fun FranjaConflictos(mensajes: List<String>, modifier: Modifier = Modifier) {
    if (mensajes.isEmpty()) return
    val varios = mensajes.size > 1
    var desplegada by rememberSaveable { mutableStateOf(false) }
    Surface(
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.errorContainer,
        contentColor = MaterialTheme.colorScheme.onErrorContainer,
        modifier = modifier
            .fillMaxWidth()
            .animateContentSize()
            .then(if (varios) Modifier.clickable { desplegada = !desplegada } else Modifier),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Default.SyncProblem, contentDescription = null, modifier = Modifier.size(20.dp))
                Spacer(Modifier.width(12.dp))
                Text(
                    if (varios) tituloConflictos(mensajes.size) else mensajes.first(),
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = if (varios) FontWeight.SemiBold else FontWeight.Normal,
                    modifier = Modifier.weight(1f),
                )
                if (varios) {
                    Icon(
                        if (desplegada) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                        contentDescription = if (desplegada) "Ocultar detalle" else "Ver detalle",
                    )
                }
            }
            if (varios && desplegada) {
                // Con tope de alto: muchos conflictos a la vez no deben tapar
                // la pantalla entera; el resto se ve deslizando.
                Column(
                    modifier = Modifier
                        .padding(start = 32.dp, top = 8.dp)
                        .heightIn(max = 220.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    mensajes.forEach { mensaje ->
                        Text(mensaje, style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
        }
    }
}

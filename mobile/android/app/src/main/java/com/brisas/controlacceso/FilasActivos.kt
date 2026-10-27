package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.IngresoRemoto
import uniffi.control_acceso_mobile.TipoIngreso

// Filas de la lista de Activos (sacado de `PantallaActivos.kt`, punto M6
// de la auditoría móvil: sólo se movió código).

@Composable
internal fun FilaActivo(fila: FilaActiva, onClick: () -> Unit) {
    when (fila) {
        is FilaActiva.Local -> FilaActivoLocal(fila.activo, onClick)
        is FilaActiva.Remota -> FilaActivoRemota(fila.remoto, onClick)
    }
}

@Composable
private fun FilaActivoLocal(activo: IngresoActivoResumen, onClick: () -> Unit) {
    TarjetaActivo(
        nombre = activo.contratistaNombre,
        detalle = "${activo.cedula} · ${activo.empresaNombre}",
        gafeteNumero = activo.gafeteNumero,
        fechaHoraIngreso = activo.fechaHoraIngreso,
        otroEquipo = false,
        onClick = onClick,
    )
}

/// Ver el doc-comment de [FilaActiva] -- un ingreso abierto por el otro
/// dispositivo del sitio (la PC del puesto de control), cacheado en
/// `ingresos_remotos`. Cédula, empresa y gafete sí viajan en la caché desde
/// la migración 24, así que no hay motivo para mostrar menos acá.
@Composable
private fun FilaActivoRemota(remoto: IngresoRemoto, onClick: () -> Unit) {
    TarjetaActivo(
        nombre = remoto.contratistaNombre,
        detalle = "${remoto.contratistaCedula ?: "—"} · ${remoto.empresaNombre ?: "—"}",
        gafeteNumero = remoto.gafeteNumero,
        fechaHoraIngreso = remoto.horaEntrada,
        otroEquipo = true,
        onClick = onClick,
    )
}

/// Tarjeta de quien está adentro (tocar = salida). La comparten
/// contratistas, proveedores, por correo y visitas para que se vean y se usen
/// igual (pedido del usuario 2026-10-06).
///
/// Sólo lo esencial (rediseño 2026-10-27, "menos es más"): el nombre; la
/// cédula y la empresa en gris; la hora (con el día sólo si fue otro día); el
/// gafete en una pastilla a la derecha, que es lo que se compara en la
/// salida; y la nube si entró por otro equipo. Quién dio el ingreso quedó
/// para el historial del escritorio y el panel.
@Composable
internal fun TarjetaActivo(
    nombre: String,
    detalle: String,
    gafeteNumero: Long?,
    fechaHoraIngreso: String,
    otroEquipo: Boolean,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                textoResaltado(nombre),
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Medium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            // Dentro de una búsqueda, lo buscado (una cédula, por ejemplo) se
            // subraya en el color de la sección.
            //
            // Los textos se recuerdan por valor: cada refresco de la lista
            // trae objetos nuevos del núcleo (sus campos son `var`, así que
            // Compose no los da por iguales) y sin esto cada fila visible
            // volvía a armar la línea y a leer y formatear la fecha.
            val consulta = LocalConsultaResaltada.current
            val acento = MaterialTheme.colorScheme.primary
            val lineaDetalle = remember(detalle, consulta, acento) {
                val estiloCoincidencia = SpanStyle(color = acento, textDecoration = TextDecoration.Underline)
                buildAnnotatedString { agregarResaltado(detalle, consulta, estiloCoincidencia) }
            }
            Text(
                lineaDetalle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            val hora = remember(fechaHoraIngreso) { horaIngreso(fechaHoraIngreso) }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    hora.texto,
                    style = MaterialTheme.typography.bodySmall,
                    color = if (hora.deOtroDia) ColorTextoAdvertenciaBrisas else MaterialTheme.colorScheme.onSurfaceVariant,
                    fontWeight = if (hora.deOtroDia) FontWeight.SemiBold else FontWeight.Normal,
                )
                if (otroEquipo) {
                    Spacer(Modifier.width(6.dp))
                    IconoOtroEquipo(Modifier.size(14.dp))
                }
            }
        }
        Spacer(Modifier.width(12.dp))
        PastillaGafete(gafeteNumero)
    }
}

/// El número de gafete, en el color de la sección; "S/G" en gris si entró
/// sin gafete.
@Composable
private fun PastillaGafete(gafeteNumero: Long?) {
    val conGafete = gafeteNumero != null
    Text(
        gafeteNumero?.toString() ?: "S/G",
        style = MaterialTheme.typography.titleSmall,
        fontWeight = FontWeight.Bold,
        color = if (conGafete) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
        textAlign = TextAlign.Center,
        modifier = Modifier
            .widthIn(min = 44.dp)
            .clip(RoundedCornerShape(percent = 50))
            .background(if (conGafete) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant)
            .padding(horizontal = 10.dp, vertical = 4.dp)
            .semantics { contentDescription = if (conGafete) "Gafete $gafeteNumero" else "Sin gafete" },
    )
}

/// La marca de "entró por otro equipo de la unidad": una nube, la misma en
/// todas las listas (contratistas, visitas, por correo, proveedores y
/// gafetes provisionales), nunca texto. Antes era una computadora aunque el
/// ingreso viniera de otro teléfono; lo que importa es que llegó de otro lado
/// (y que cerrarlo pide conexión).
@Composable
internal fun IconoOtroEquipo(modifier: Modifier = Modifier.size(18.dp)) {
    Icon(
        IconosLucide.Nube,
        contentDescription = "Registrado en otro equipo",
        tint = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = modifier,
    )
}

@Composable
internal fun FilaContratista(contratista: ContratistaResumen, onClick: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(
            textoResaltado(contratista.nombre),
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = FontWeight.Medium,
        )
        Text(
            textoResaltado("${contratista.cedula} · ${contratista.empresaNombre} · ${etiquetaTipoIngreso(contratista.tipoIngreso)}"),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        // Texto y regla del núcleo (`ContratistaResumen.avisoAcceso`), con
        // su reloj: sólo se muestra, en negrita para que se lea con fuerza.
        contratista.avisoAcceso?.let { aviso ->
            Text(
                aviso,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                fontWeight = FontWeight.Bold,
            )
        }
        if (contratista.tieneIngresoActivo) {
            Text(
                "Adentro · toque para registrar su salida",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.primary,
            )
        }
    }
}

private fun etiquetaTipoIngreso(tipo: TipoIngreso): String =
    when (tipo) {
        TipoIngreso.PRAIND -> "PRAIND"
        TipoIngreso.IN_HOUSE -> "IN HOUSE"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }

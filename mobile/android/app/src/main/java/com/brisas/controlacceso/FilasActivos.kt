package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Computer
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
        dioIngreso = activo.usuarioIngresoNombre,
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
        dioIngreso = remoto.usuarioEntradaNombre ?: "—",
        otroEquipo = true,
        onClick = onClick,
    )
}

/// Tarjeta de quien está adentro (tocar = salida). La comparten
/// contratistas y visitas para que se vean y se usen igual (pedido del
/// usuario 2026-10-06). Lo que entró por el otro equipo de la unidad lleva
/// el ícono de la PC a la derecha, en vez de texto, para ahorrar espacio.
@Composable
internal fun TarjetaActivo(
    nombre: String,
    detalle: String,
    gafeteNumero: Long?,
    fechaHoraIngreso: String,
    dioIngreso: String,
    otroEquipo: Boolean,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(textoResaltado(nombre), style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
            // Mayúscula + negrita en toda la línea, gafete en azul -- pedido
            // explícito del usuario 2026-09-20: es lo que el guardia de
            // salida confirma contra lo que la persona trae puesto, tiene
            // que resaltar más que cédula/empresa. Dentro de una búsqueda, lo
            // buscado (una cédula, por ejemplo) se subraya en el color de la
            // sección.
            //
            // Los textos se recuerdan por valor: cada refresco de la lista
            // trae objetos nuevos del núcleo (sus campos son `var`, así que
            // Compose no los da por iguales) y sin esto cada fila visible
            // volvía a armar la línea y a leer y formatear la fecha.
            val consulta = LocalConsultaResaltada.current
            val acento = MaterialTheme.colorScheme.primary
            val lineaDetalle = remember(detalle, gafeteNumero, consulta, acento) {
                val estiloCoincidencia = SpanStyle(color = acento, textDecoration = TextDecoration.Underline)
                buildAnnotatedString {
                    agregarResaltado("$detalle · ".uppercase(), consulta, estiloCoincidencia)
                    withStyle(SpanStyle(color = acento)) {
                        append((if (gafeteNumero != null) "Gafete $gafeteNumero" else "Sin gafete").uppercase())
                    }
                }
            }
            Text(
                lineaDetalle,
                style = MaterialTheme.typography.bodySmall,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            val lineaIngreso = remember(fechaHoraIngreso, dioIngreso) {
                "Ingresó ${textoFechaHora(fechaHoraIngreso)} · dio ingreso $dioIngreso"
            }
            Text(
                lineaIngreso,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (otroEquipo) IconoOtroEquipo()
    }
}

/// La marca de "entró por el otro equipo de la unidad": el mismo ícono en
/// todas las listas (contratistas, visitas, por correo, proveedores y
/// gafetes provisionales), nunca texto.
@Composable
internal fun IconoOtroEquipo() {
    Icon(
        Icons.Default.Computer,
        contentDescription = "Registrado en otro dispositivo",
        tint = MaterialTheme.colorScheme.primary,
        modifier = Modifier.size(20.dp),
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

package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import java.time.LocalDate
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
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(activo.contratistaNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        // Mayúscula + negrita en toda la línea, gafete en azul -- pedido
        // explícito del usuario 2026-09-20: es un dato importante (lo que
        // el guardia de salida necesita confirmar contra lo que la persona
        // trae puesto), tiene que resaltar más que cédula/empresa. El
        // estado de acceso (antes "Al día"/"PRAIND próximo a vencer"/motivo
        // de denegación) se sacó de acá -- ya se mostró y se aceptó al
        // momento de registrar el ingreso, repetirlo en cada tarjeta activa
        // era ruido, no información nueva.
        Text(
            buildAnnotatedString {
                append("${activo.cedula} · ${activo.empresaNombre} · ".uppercase())
                withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary)) {
                    append(
                        (if (activo.gafeteNumero != null) "Gafete ${activo.gafeteNumero}" else "Sin gafete")
                            .uppercase(),
                    )
                }
            },
            style = MaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Ingresó ${textoFechaHora(activo.fechaHoraIngreso)} · dio ingreso ${activo.usuarioIngresoNombre}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/// Ver el doc-comment de [FilaActiva] -- un ingreso abierto por el otro
/// dispositivo del sitio, sin cédula/empresa/gafete propios (esos datos
/// nunca viajan en la caché `ingresos_remotos`, sólo lo mínimo para
/// mostrarlo y poder cerrarlo).
@Composable
private fun FilaActivoRemota(remoto: IngresoRemoto, onClick: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(remoto.contratistaNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            "Entrada ${textoFechaHora(remoto.horaEntrada)}" +
                (remoto.usuarioEntradaNombre?.let { " ($it)" } ?: ""),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Otro dispositivo",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.primary,
        )
    }
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
            contratista.nombre,
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = FontWeight.Medium,
        )
        Text(
            "${contratista.cedula} · ${contratista.empresaNombre} · ${etiquetaTipoIngreso(contratista.tipoIngreso)}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (!contratista.tieneAcceso) {
            // Mismo criterio que el motivo SIN_ACCESO del texto que Rust
            // ya resuelve para el bloqueo de ingreso (`PreparacionIngreso.mensajeBloqueo`,
            // `mensajes::mensaje_bloqueo_ingreso` en el crate raíz) -- acá
            // esta fila sólo conoce el toggle crudo (`tieneAcceso`), no el
            // resto de `verificar_acceso` (PRAIND, empresa, etc.), así que
            // el único motivo posible en este catálogo es justo ese.
            // Mayúscula y negrita, pedido explícito del usuario
            // 2026-09-20 para que se lea con fuerza.
            Text(
                "ACCESO DENEGADO",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                fontWeight = FontWeight.Bold,
            )
        } else if (contratista.fechaVencimientoPraind?.let { LocalDate.parse(it) < LocalDate.now() } == true) {
            // Independiente del toggle de arriba -- acá sí hay fecha
            // (`ContratistaResumen.fechaVencimientoPraind`), a diferencia
            // del "Acceso denegado" de arriba que no la necesita. Sin fecha
            // en el texto a propósito (pedido explícito del usuario
            // 2026-09-20): sólo "PRAIND VENCIDO", el detalle con la fecha ya
            // aparece en la pantalla de confirmar ingreso.
            Text(
                "PRAIND VENCIDO",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                fontWeight = FontWeight.Bold,
            )
        }
        if (contratista.tieneIngresoActivo) {
            Text(
                "Ingreso activo (sin salida registrada)",
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

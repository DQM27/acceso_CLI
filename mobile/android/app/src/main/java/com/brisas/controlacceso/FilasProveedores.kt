package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import uniffi.control_acceso_mobile.IngresoProveedorRemoto
import uniffi.control_acceso_mobile.RegistroIngresoProveedorActivoResumen

// Filas de proveedores activos y diálogo de salida (sacado de
// `PantallaProveedores.kt`, punto M6 de la auditoría móvil: sólo se movió código).

@Composable
internal fun FilaProveedorActivo(fila: FilaProveedorActiva, onConfirmarSalida: () -> Unit) {
    when (fila) {
        is FilaProveedorActiva.Local -> FilaProveedorActivoLocal(fila.registro, onConfirmarSalida)
        is FilaProveedorActiva.Remota -> FilaProveedorActivoRemota(fila.remoto, onConfirmarSalida)
    }
}

/// Mismo orden de campos que `FilaActivoLocal`/`FilaActivoRemota`
/// (PantallaActivos.kt, contratista) -- nombre primero (lo más importante:
/// quién es), luego identidad + afiliación + gafete, luego cuándo entró.
/// Antes esta tarjeta abría con "Gafete N" en vez del nombre -- pedido
/// explícito del usuario en pruebas reales, 2026-09-17: unificar el orden
/// de importancia entre las dos pantallas.
///
/// Mayúscula + negrita + gafete en azul, y "dio ingreso" en la última
/// línea -- mismo tratamiento que `FilaActivoLocal` (contratista), pedido
/// explícito del usuario 2026-09-20 para unificar las dos tarjetas.
@Composable
private fun FilaProveedorActivoLocal(
    registro: RegistroIngresoProveedorActivoResumen,
    onConfirmarSalida: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onConfirmarSalida)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(registro.nombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            buildAnnotatedString {
                append(
                    (
                        "${registro.cedula} · ${registro.empresaNombre}" +
                            (registro.placa?.let { " · $it" } ?: "") + " · "
                    ).uppercase(),
                )
                withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary)) {
                    append("Gafete ${registro.gafeteNumero}".uppercase())
                }
            },
            style = MaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Ingresó ${textoFechaHora(registro.fechaHoraIngreso)} · dio ingreso ${registro.usuarioIngresoNombre}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/// Ver el doc-comment de [FilaProveedorActiva] -- un ingreso abierto por
/// OTRO dispositivo del sitio, sin `id` local (sólo `uuid` de la nube).
/// Mismo orden que [FilaProveedorActivoLocal] -- ver ese doc-comment.
@Composable
private fun FilaProveedorActivoRemota(remoto: IngresoProveedorRemoto, onConfirmarSalida: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onConfirmarSalida)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(remoto.nombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            buildAnnotatedString {
                append(
                    (
                        "${remoto.cedula} · ${remoto.empresaNombre}" +
                            (remoto.placa?.let { " · $it" } ?: "") + " · "
                    ).uppercase(),
                )
                withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary)) {
                    append("Gafete ${remoto.gafeteNumero}".uppercase())
                }
            },
            style = MaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Ingresó ${textoFechaHora(remoto.horaEntrada)} · dio ingreso ${remoto.usuarioEntradaNombre}",
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

/// Mismo layout e información que `DialogoConfirmarSalida` de
/// PantallaActivos.kt (contratista) -- ícono circular, título "Registrar
/// salida", y nombre · cédula · empresa (o "registrado en otro dispositivo"
/// para una fila remota) en vez de sólo "Gafete N · nombre". Pedido
/// explícito del usuario en pruebas reales, 2026-09-17: el modal de
/// Proveedores quedaba "muy laxo" comparado con el de Contratista.
@Composable
internal fun DialogoConfirmarSalidaProveedor(
    fila: FilaProveedorActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaProveedorActiva) -> Unit,
) {
    if (fila == null) return

    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface, MaterialTheme.shapes.medium)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(
                modifier = Modifier
                    .size(56.dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.primaryContainer),
                contentAlignment = Alignment.Center,
            ) {
                Icon(Icons.AutoMirrored.Filled.Logout, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
            }
            Text(
                "Registrar salida",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(top = 16.dp),
            )
            Text(
                when (fila) {
                    is FilaProveedorActiva.Local ->
                        "${fila.registro.nombre} · ${fila.registro.cedula} · ${fila.registro.empresaNombre} · Gafete ${fila.registro.gafeteNumero}"
                    is FilaProveedorActiva.Remota ->
                        "${fila.remoto.nombre} · registrado en otro dispositivo de la unidad operativa"
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
            BotonBrisas(
                onClick = { onConfirmar(fila) },
                modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
            ) {
                Text("Confirmar")
            }
            BotonDiscretoBrisas(onClick = onDismiss, modifier = Modifier.padding(top = 4.dp)) {
                Text("Cancelar")
            }
        }
    }
}

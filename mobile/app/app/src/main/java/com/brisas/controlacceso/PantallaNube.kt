package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.IngresoRemoto
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.ResumenSincronizacion
import uniffi.control_acceso_mobile.RolUsuario
import uniffi.control_acceso_mobile.UsuarioSesion

/// Sincronización con la nube (ver docs/plan-persistencia-nube.md) — todo
/// el estado y las llamadas a [Nucleo] viven en [NubeViewModel] (ver
/// mobile/app/ARQUITECTURA.md), este Composable sólo dibuja.
///
/// La sección de secreto del dispositivo sólo se dibuja para Root
/// (`Operacion::GestionarNube` es exclusivo de Root del lado de Rust, ver
/// `src/application/nube.rs`) — por eso `actualizarEstadoSecreto` sólo se
/// dispara en `LaunchedEffect` cuando `esRoot`, igual que el propio
/// [NubeViewModel] evita llamarlo desde `init` para no generar un error a
/// un Operador que ni ve ese botón. `sincronizar`/`cerrarIngresoRemoto` sí
/// están disponibles para cualquier rol, sin gateo acá.
@Composable
fun PantallaNube(nucleo: Nucleo, sesion: UsuarioSesion, directorio: String, refrescarNube: Int = 0) {
    val viewModel: NubeViewModel = viewModel(factory = NubeViewModel.factory(nucleo, directorio))
    LaunchedEffect(refrescarNube) {
        if (refrescarNube > 0) {
            viewModel.refrescarCacheLocal()
        }
    }
    val esRoot = sesion.rol == RolUsuario.ROOT

    LaunchedEffect(Unit) {
        if (esRoot) viewModel.actualizarEstadoSecreto()
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        if (esRoot) {
            SeccionSecretoDispositivo(
                secretoGuardado = viewModel.secretoGuardado,
                onGuardar = { viewModel.guardarSecreto(it) },
            )
            HorizontalDivider(modifier = Modifier.padding(vertical = 16.dp))
        }

        Button(
            onClick = { viewModel.sincronizar() },
            enabled = !viewModel.sincronizando,
            colors = ButtonDefaults.buttonColors(
                containerColor = MaterialTheme.colorScheme.primary,
                contentColor = MaterialTheme.colorScheme.onPrimary,
            ),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(if (viewModel.sincronizando) "Sincronizando…" else "Sincronizar")
        }

        val mensajeError = viewModel.error
        if (mensajeError != null) {
            Text(
                mensajeError,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(top = 12.dp),
            )
        }

        val resumen = viewModel.ultimoResumen
        if (resumen != null) {
            TarjetaResumenSincronizacion(resumen, modifier = Modifier.padding(top = 16.dp))
        }

        LazyColumn(modifier = Modifier.padding(top = 12.dp)) {
            items(viewModel.ingresosRemotos, key = { it.uuid }) { ingreso ->
                FilaIngresoRemoto(ingreso, onCerrar = { viewModel.cerrarIngresoRemoto(ingreso.uuid) })
                HorizontalDivider(color = MaterialTheme.colorScheme.outline)
            }
        }
    }
}

/// Sólo Root — mostrar/pegar el secreto es `Operacion::GestionarNube`. El
/// campo de texto se queda en `rememberSaveable` (mismo criterio que
/// `PantallaLogin`): es un valor a medio escribir, no estado que deba
/// sobrevivir en el ViewModel una vez guardado.
@Composable
private fun SeccionSecretoDispositivo(secretoGuardado: Boolean, onGuardar: (String) -> Unit) {
    if (secretoGuardado) {
        Text(
            "Dispositivo configurado",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.primary,
        )
        return
    }

    var secreto by rememberSaveable { mutableStateOf("") }
    Text(
        "Este dispositivo todavía no tiene secreto guardado",
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
    OutlinedTextField(
        value = secreto,
        onValueChange = { secreto = it },
        label = { Text("Secreto del dispositivo") },
        singleLine = true,
        colors = OutlinedTextFieldDefaults.colors(
            focusedBorderColor = MaterialTheme.colorScheme.primary,
            focusedLabelColor = MaterialTheme.colorScheme.primary,
        ),
        modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
    )
    Button(
        onClick = { onGuardar(secreto) },
        colors = ButtonDefaults.buttonColors(
            containerColor = MaterialTheme.colorScheme.primary,
            contentColor = MaterialTheme.colorScheme.onPrimary,
        ),
        modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
    ) {
        Text("Guardar")
    }
}

@Composable
private fun TarjetaResumenSincronizacion(resumen: ResumenSincronizacion, modifier: Modifier = Modifier) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
        modifier = modifier.fillMaxWidth(),
    ) {
        Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(
                "Sitio ${resumen.sitioId} · dispositivo ${resumen.dispositivoId} (${resumen.tipo})",
                style = MaterialTheme.typography.bodySmall,
                fontWeight = FontWeight.Medium,
            )
            Text(
                "${resumen.enviados} enviados, ${resumen.fallidos} fallidos, " +
                    "${resumen.remotosAbiertos} abiertos del otro dispositivo",
                style = MaterialTheme.typography.bodySmall,
            )
            Text(
                "${resumen.empresasRecibidas} empresas y ${resumen.contratistasRecibidos} " +
                    "contratistas recibidos del catálogo del sitio",
                style = MaterialTheme.typography.bodySmall,
            )
        }
    }
}

@Composable
private fun FilaIngresoRemoto(ingreso: IngresoRemoto, onCerrar: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 10.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(ingreso.contratistaNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
            Text(
                "Entrada ${textoFechaHora(ingreso.horaEntrada)}" +
                    (ingreso.usuarioEntradaNombre?.let { " ($it)" } ?: ""),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        OutlinedButton(onClick = onCerrar) {
            Text("Cerrar")
        }
    }
}

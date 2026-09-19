package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.EncargadoRuta
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.PrestamoGafeteProvisionalActivoResumen
import uniffi.control_acceso_mobile.PrestamoGafeteProvisionalRemoto

/// Entrega/devolución de gafetes provisionales KOF -- ver
/// `docs/features-futuras/plan-gafetes-provisionales-kof.md`. Resuelve el
/// caso de un colaborador interno de KOF que olvida su gafete permanente:
/// se busca por nombre o código de empleado en el mismo catálogo ya
/// importado para rutas (`encargados_ruta`), se coteja el código contra lo
/// que la persona dice de palabra (el guardia mira su cédula física para
/// el nombre -- el sistema no captura ese dato), y se le presta un número
/// de gafete de la categoría física aparte. Mucho más simple que
/// [PantallaRutas]: un solo campo bloqueante (encargado), sin OCR, sin
/// wizard de pasos -- un formulario y una lista.
@Composable
fun PantallaGafetesProvisionales(
    nucleo: Nucleo,
    secretoStore: SecretoDispositivoStore,
    refrescarNube: Int = 0,
) {
    val viewModel: GafetesProvisionalesViewModel =
        viewModel(factory = GafetesProvisionalesViewModel.factory(nucleo, secretoStore))
    var gafeteTexto by remember { mutableStateOf("") }
    var filaParaDevolver by remember { mutableStateOf<FilaGafeteProvisionalActiva?>(null) }
    val focoGafete = remember { FocusRequester() }
    LaunchedEffect(refrescarNube) {
        if (refrescarNube > 0) {
            viewModel.refrescarActivos()
        }
    }
    // Salta directo al número de gafete al elegir encargado -- guardia no
    // tiene que tocar la pantalla para seguir con el flujo. Pedido explícito
    // del usuario en pruebas reales, 2026-09-17.
    LaunchedEffect(viewModel.encargadoSeleccionado) {
        if (viewModel.encargadoSeleccionado != null) {
            focoGafete.requestFocus()
        }
    }

    val puedeEntregar =
        viewModel.encargadoSeleccionado != null && gafeteTexto.isNotBlank() && !viewModel.registrando

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        Text(
            "Gafete provisional KOF",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(bottom = 10.dp),
        )

        // Mismo look que el buscador de `PantallaActivos`/`PantallaProveedores`
        // (ícono de lupa, mismo campo/alto) pero sin el botón de cámara -- acá
        // no hay documento que escanear, sólo nombre/código escrito. Los
        // resultados salen como tarjetas tocables abajo (igual que
        // `FilaContratista` en modo Ingreso), no en un menú desplegable --
        // pedido explícito del usuario en pruebas reales, 2026-09-17:
        // unificar el look de esta pantalla con el de Contratista.
        OutlinedTextField(
            value = viewModel.textoEncargado,
            onValueChange = viewModel::cambiarTextoEncargado,
            placeholder = { Text("Nombre o código de empleado") },
            leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
        )

        if (viewModel.encargadoSeleccionado == null && viewModel.resultadosEncargado.isNotEmpty()) {
            // Tope de 6 -- mismo criterio que el buscador de Contratista: esta
            // lista vive dentro de un `Column` sin scroll propio (a
            // diferencia de `ContenidoModoEntrada`, que reemplaza toda la
            // pantalla), así que sin un tope el `LazyColumn` crecía sin
            // límite y empujaba el campo de número de gafete fuera de la
            // pantalla -- bug reportado en pruebas reales, 2026-09-17.
            ListaConDesvanecido {
                LazyColumn(
                    contentPadding = PaddingValues(top = 5.dp),
                    verticalArrangement = Arrangement.spacedBy(5.dp),
                ) {
                    items(viewModel.resultadosEncargado.take(6), key = { it.id }) { encargado ->
                        FilaEncargadoProvisional(encargado, onClick = { viewModel.elegirEncargado(encargado) })
                    }
                }
            }
        }

        viewModel.encargadoSeleccionado?.let { encargado ->
            Text(
                "Código de empleado: ${encargado.codigoEmpleado} -- corroborar contra lo que dice la persona",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp),
            )

            // Sin `.height(AlturaBusquedaBrisas)` a propósito -- ese alto
            // (50.dp, ver ControlesBrisas.kt) queda por debajo del mínimo
            // cómodo de `TextField` de Material3 cuando el campo no tiene
            // `leadingIcon` (a diferencia del buscador de arriba, que sí
            // tiene uno): el placeholder terminaba recortado contra el borde
            // superior del campo en vez de centrado -- bug reportado en
            // pruebas reales, 2026-09-17 ("el input... sale cortado").
            OutlinedTextField(
                value = gafeteTexto,
                onValueChange = { gafeteTexto = it.filter(Char::isDigit) },
                placeholder = { Text("Número de gafete") },
                singleLine = true,
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().padding(top = 8.dp).focusRequester(focoGafete),
            )

            viewModel.error?.let { mensaje ->
                Text(
                    mensaje,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(top = 8.dp),
                )
            }

            BotonBrisas(
                onClick = {
                    val numero = gafeteTexto.toLongOrNull() ?: return@BotonBrisas
                    viewModel.entregar(numero) { gafeteTexto = "" }
                },
                enabled = puedeEntregar,
                modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
            ) {
                Text("Entregar")
            }
        }

        // Sin encabezado "Gafetes prestados" -- pedido explícito del usuario
        // en pruebas reales, 2026-09-17: dejarlo limpio, igual que la lista
        // de activos de Contratista (que tampoco tiene título propio).
        ListaConDesvanecido {
            LazyColumn(
                contentPadding = PaddingValues(top = 16.dp),
                verticalArrangement = Arrangement.spacedBy(5.dp),
            ) {
                items(viewModel.activos, key = { it.clave() }) { fila ->
                    FilaPrestamoGafeteProvisional(
                        fila = fila,
                        onConfirmarDevolucion = { filaParaDevolver = fila },
                    )
                }
            }
        }
    }

    DialogoConfirmarDevolucionGafeteProvisional(
        fila = filaParaDevolver,
        onDismiss = { filaParaDevolver = null },
        onConfirmar = {
            viewModel.registrarDevolucion(it)
            filaParaDevolver = null
        },
    )
}

private fun FilaGafeteProvisionalActiva.clave(): String = when (this) {
    is FilaGafeteProvisionalActiva.Local -> "local-${prestamo.id}"
    is FilaGafeteProvisionalActiva.Remota -> "remota-${remoto.uuid}"
}

/// Mismo estilo que `FilaContratista` (PantallaActivos.kt, modo Ingreso) --
/// nombre primero, código de empleado abajo, tarjeta completa tocable.
@Composable
private fun FilaEncargadoProvisional(encargado: EncargadoRuta, onClick: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(encargado.nombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            "Código de empleado: ${encargado.codigoEmpleado}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun FilaPrestamoGafeteProvisional(
    fila: FilaGafeteProvisionalActiva,
    onConfirmarDevolucion: () -> Unit,
) {
    when (fila) {
        is FilaGafeteProvisionalActiva.Local ->
            FilaPrestamoGafeteProvisionalLocal(fila.prestamo, onConfirmarDevolucion)
        is FilaGafeteProvisionalActiva.Remota ->
            FilaPrestamoGafeteProvisionalRemota(fila.remoto, onConfirmarDevolucion)
    }
}

/// Mismo orden e interacción que las tarjetas de Contratista/Proveedores:
/// nombre primero, tarjeta completa tocable (sin botón "Devolver" aparte)
/// -- pedido explícito del usuario en pruebas reales, 2026-09-17.
@Composable
private fun FilaPrestamoGafeteProvisionalLocal(
    prestamo: PrestamoGafeteProvisionalActivoResumen,
    onConfirmarDevolucion: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onConfirmarDevolucion)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(prestamo.encargadoNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            "${prestamo.encargadoCodigoEmpleado} · Gafete ${prestamo.gafeteNumero}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Entregado ${textoFechaHora(prestamo.fechaHoraEntrega)}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/// Ver el doc-comment de [FilaGafeteProvisionalActiva] -- un préstamo
/// entregado por OTRO dispositivo del sitio, sin `id` local (sólo `uuid`
/// de la nube). Mismo orden que [FilaPrestamoGafeteProvisionalLocal].
@Composable
private fun FilaPrestamoGafeteProvisionalRemota(
    remoto: PrestamoGafeteProvisionalRemoto,
    onConfirmarDevolucion: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onConfirmarDevolucion)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(remoto.encargadoNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            "${remoto.encargadoCodigoEmpleado} · Gafete ${remoto.gafeteNumero}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Entregado ${textoFechaHora(remoto.horaEntrega)}",
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

/// Mismo layout que `DialogoConfirmarSalida`/`DialogoConfirmarSalidaProveedor`
/// -- ícono circular, título "Registrar devolución", info completa en vez de
/// sólo "Gafete N · nombre".
@Composable
private fun DialogoConfirmarDevolucionGafeteProvisional(
    fila: FilaGafeteProvisionalActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaGafeteProvisionalActiva) -> Unit,
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
                "Registrar devolución",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(top = 16.dp),
            )
            Text(
                when (fila) {
                    is FilaGafeteProvisionalActiva.Local ->
                        "${fila.prestamo.encargadoNombre} · ${fila.prestamo.encargadoCodigoEmpleado} · Gafete ${fila.prestamo.gafeteNumero}"
                    is FilaGafeteProvisionalActiva.Remota ->
                        "${fila.remoto.encargadoNombre} · registrado en otro dispositivo de la unidad operativa"
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

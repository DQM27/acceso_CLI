package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.EncargadoRuta
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.PrestamoGafeteProvisionalActivoResumen

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
fun PantallaGafetesProvisionales(nucleo: Nucleo) {
    val viewModel: GafetesProvisionalesViewModel =
        viewModel(factory = GafetesProvisionalesViewModel.factory(nucleo))
    var gafeteTexto by remember { mutableStateOf("") }
    var prestamoParaDevolver by remember { mutableStateOf<PrestamoGafeteProvisionalActivoResumen?>(null) }

    val puedeEntregar =
        viewModel.encargadoSeleccionado != null && gafeteTexto.isNotBlank() && !viewModel.registrando

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        Text(
            "Gafete provisional KOF",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(bottom = 10.dp),
        )

        BuscadorEncargadoProvisional(
            texto = viewModel.textoEncargado,
            onCambiarTexto = viewModel::cambiarTextoEncargado,
            resultados = viewModel.resultadosEncargado,
            onElegir = viewModel::elegirEncargado,
        )

        viewModel.encargadoSeleccionado?.let { encargado ->
            Text(
                "Código de empleado: ${encargado.codigoEmpleado} -- corroborar contra lo que dice la persona",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp),
            )
        }

        TextField(
            value = gafeteTexto,
            onValueChange = { gafeteTexto = it.filter(Char::isDigit) },
            placeholder = { Text("Número de gafete provisional") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas).padding(top = 8.dp),
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

        Text(
            "Gafetes prestados",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 16.dp, bottom = 6.dp),
        )

        ListaConDesvanecido {
            LazyColumn(
                contentPadding = PaddingValues(top = 5.dp),
                verticalArrangement = Arrangement.spacedBy(5.dp),
            ) {
                items(viewModel.activos, key = { it.id }) { prestamo ->
                    FilaPrestamoGafeteProvisional(
                        prestamo = prestamo,
                        onConfirmarDevolucion = { prestamoParaDevolver = prestamo },
                    )
                }
            }
        }
    }

    DialogoConfirmarDevolucionGafeteProvisional(
        prestamo = prestamoParaDevolver,
        onDismiss = { prestamoParaDevolver = null },
        onConfirmar = {
            viewModel.registrarDevolucion(it)
            prestamoParaDevolver = null
        },
    )
}

/// Mismo componente visual que `PasoEncargado` de `PantallaRutas.kt`, sin
/// el número de paso ni el botón de cámara -- ese `@Composable` es privado
/// a ese archivo (mismo caso ya documentado en la exploración de este
/// módulo), así que se duplica el mínimo necesario en vez de exportarlo.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun BuscadorEncargadoProvisional(
    texto: String,
    onCambiarTexto: (String) -> Unit,
    resultados: List<EncargadoRuta>,
    onElegir: (EncargadoRuta) -> Unit,
) {
    var menuAbierto by remember { mutableStateOf(false) }

    ExposedDropdownMenuBox(
        expanded = menuAbierto && resultados.isNotEmpty(),
        onExpandedChange = { menuAbierto = it },
    ) {
        TextField(
            value = texto,
            onValueChange = {
                onCambiarTexto(it)
                menuAbierto = true
            },
            placeholder = { Text("Nombre o código de empleado") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier
                .fillMaxWidth()
                .height(AlturaBusquedaBrisas)
                .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryEditable),
        )
        DropdownMenu(
            expanded = menuAbierto && resultados.isNotEmpty(),
            onDismissRequest = { menuAbierto = false },
        ) {
            resultados.forEach { encargado ->
                DropdownMenuItem(
                    text = { Text("${encargado.nombre} · ${encargado.codigoEmpleado}") },
                    onClick = {
                        onElegir(encargado)
                        menuAbierto = false
                    },
                )
            }
        }
    }
}

@Composable
private fun FilaPrestamoGafeteProvisional(
    prestamo: PrestamoGafeteProvisionalActivoResumen,
    onConfirmarDevolucion: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(
            "Gafete ${prestamo.gafeteNumero}",
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = FontWeight.Medium,
        )
        Text(
            "${prestamo.encargadoNombre} · ${prestamo.encargadoCodigoEmpleado}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Entregado ${textoFechaHora(prestamo.fechaHoraEntrega)}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Row(modifier = Modifier.padding(top = 8.dp)) {
            BotonBrisas(onClick = onConfirmarDevolucion) {
                Text("Devolver")
            }
        }
    }
}

@Composable
private fun DialogoConfirmarDevolucionGafeteProvisional(
    prestamo: PrestamoGafeteProvisionalActivoResumen?,
    onDismiss: () -> Unit,
    onConfirmar: (PrestamoGafeteProvisionalActivoResumen) -> Unit,
) {
    if (prestamo == null) return
    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface, MaterialTheme.shapes.medium)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                "Confirmar devolución",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )
            Text(
                "Gafete ${prestamo.gafeteNumero} · ${prestamo.encargadoNombre}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
            BotonBrisas(
                onClick = { onConfirmar(prestamo) },
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

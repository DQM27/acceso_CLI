package com.brisas.controlacceso

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.ResultadoAcceso
import uniffi.control_acceso_mobile.TipoIngreso

/// Una sola vista para el ciclo completo — entrada, permanencia y salida —,
/// igual que `Activos.tsx` en desktop (ahí "+Nuevo"/"Salida" abren modales
/// sobre la misma grilla de activos; ver docs/plan-app-movil.md, addendum
/// 2026-09-01). Acá no hay una pestaña "Buscar" aparte: el mismo campo de
/// texto cambia de sentido según el modo elegido en el selector de arriba
/// — mismo espíritu que el checkbox "Por gafete" de `SalidaModal.tsx` (un
/// solo campo, la interpretación cambia), llevado a un selector de tres
/// porque acá hace falta distinguir tres búsquedas, no dos:
///
/// - **Entrada** (por defecto): vacío lista quién está adentro (tocar un
///   nombre confirma su salida); con texto busca en el catálogo completo de
///   contratistas (antes pestaña "Buscar" aparte) para arrancar el flujo de
///   confirmar entrada.
/// - **Salida: nombre**: filtra la lista de activos por cédula/nombre —
///   tocar un resultado abre el mismo diálogo de confirmar salida de
///   siempre. Vacío no trae nada (es un buscador, no una lista para
///   recorrer — para eso ya está la pestaña Entrada).
/// - **Salida: gafete**: acepta varios números de gafete separados por
///   coma ("2, 25, 85") — igual que el modo gafete de `SalidaModal.tsx` —
///   y muestra a quién le corresponde cada uno antes de confirmar. Un solo
///   botón registra la salida de todos los que sí tienen ingreso activo de
///   una vez, sin diálogo por persona: es la misma decisión de desktop
///   (pensada para cargar varios gafetes de un tirón), la vista previa con
///   nombres hace las veces de confirmación.
///
/// Todo el estado y las llamadas a [Nucleo] viven en [ActivosViewModel]
/// (ver mobile/app/ARQUITECTURA.md) — este `@Composable` sólo dibuja lo que
/// el ViewModel expone y le reporta eventos, no toma ninguna decisión de
/// negocio por su cuenta.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PantallaActivos(nucleo: Nucleo) {
    val viewModel: ActivosViewModel = viewModel(factory = ActivosViewModel.factory(nucleo))

    when (val actual = viewModel.seleccionIngreso) {
        is SeleccionIngreso.Formulario -> {
            PantallaConfirmarIngreso(
                nucleo = nucleo,
                preparacion = actual.preparacion,
                onRegistrado = { viewModel.onIngresoRegistrado() },
                onCambiar = { viewModel.cancelarSeleccionIngreso() },
            )
            return
        }
        is SeleccionIngreso.Bloqueada -> {
            PantallaIngresoBloqueado(
                preparacion = actual.preparacion,
                mensaje = actual.mensaje,
                onCambiar = { viewModel.cancelarSeleccionIngreso() },
            )
            return
        }
        else -> Unit
    }

    // Color propio para "estoy buscando a quién SACAR" — evita confundir el
    // modo entrada (color normal de la app) con el de salida, que es la
    // acción de mayor consecuencia.
    val colorModo = if (viewModel.modo == ModoBusqueda.ENTRADA) {
        MaterialTheme.colorScheme.primary
    } else {
        MaterialTheme.colorScheme.secondary
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
            SegmentedButton(
                selected = viewModel.modo == ModoBusqueda.ENTRADA,
                onClick = { viewModel.cambiarModo(ModoBusqueda.ENTRADA) },
                shape = SegmentedButtonDefaults.itemShape(index = 0, count = 3),
            ) {
                Text("Entrada")
            }
            SegmentedButton(
                selected = viewModel.modo == ModoBusqueda.SALIDA_NOMBRE,
                onClick = { viewModel.cambiarModo(ModoBusqueda.SALIDA_NOMBRE) },
                shape = SegmentedButtonDefaults.itemShape(index = 1, count = 3),
            ) {
                Text("Salida: nombre")
            }
            SegmentedButton(
                selected = viewModel.modo == ModoBusqueda.SALIDA_GAFETE,
                onClick = { viewModel.cambiarModo(ModoBusqueda.SALIDA_GAFETE) },
                shape = SegmentedButtonDefaults.itemShape(index = 2, count = 3),
            ) {
                Text("Salida: gafete")
            }
        }

        OutlinedTextField(
            value = viewModel.texto,
            onValueChange = { viewModel.cambiarTexto(it) },
            label = {
                Text(
                    if (viewModel.modo == ModoBusqueda.SALIDA_GAFETE) {
                        "Números de gafete, separados por coma"
                    } else {
                        "Cédula o nombre"
                    },
                )
            },
            leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
            singleLine = true,
            keyboardOptions = if (viewModel.modo == ModoBusqueda.SALIDA_GAFETE) {
                KeyboardOptions(keyboardType = KeyboardType.Number)
            } else {
                KeyboardOptions.Default
            },
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = colorModo,
                focusedLabelColor = colorModo,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        )

        if (viewModel.modo != ModoBusqueda.SALIDA_GAFETE) {
            val leyenda = when {
                viewModel.texto.isBlank() && viewModel.modo == ModoBusqueda.ENTRADA ->
                    if (viewModel.activos.isEmpty()) {
                        "Nadie adentro"
                    } else {
                        "${viewModel.activos.size} adentro · toque un nombre para registrar salida"
                    }
                viewModel.texto.isBlank() -> "Escriba para buscar entre los activos"
                viewModel.modo == ModoBusqueda.ENTRADA -> "Buscando contratistas · toque un resultado para registrar entrada"
                viewModel.activos.isEmpty() -> "Sin coincidencias entre los activos"
                else -> "Buscando entre los activos · toque un nombre para registrar salida"
            }
            Text(
                leyenda,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
        }

        val mensajeError = viewModel.error
        if (mensajeError != null) {
            Text(mensajeError, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(top = 12.dp))
        }
        val mensajeActual = viewModel.mensaje
        if (mensajeActual != null) {
            Text(
                mensajeActual,
                color = if (viewModel.mensajeEsError) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(top = 12.dp),
            )
        }
        if (viewModel.seleccionIngreso is SeleccionIngreso.Cargando) {
            Text(
                "Verificando…",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 12.dp),
            )
        }

        if (viewModel.modo == ModoBusqueda.SALIDA_GAFETE) {
            if (viewModel.texto.isBlank()) {
                Text(
                    "Escriba uno o más números de gafete, separados por coma",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 8.dp),
                )
            } else {
                Column(modifier = Modifier.padding(top = 8.dp)) {
                    viewModel.coincidenciasGafete.forEach { coincidencia ->
                        val activoCoincidente = coincidencia.activo
                        Text(
                            "Gafete ${coincidencia.numero} · " +
                                (
                                    activoCoincidente?.let { "${it.contratistaNombre} · ${it.empresaNombre}" }
                                        ?: "Sin ingreso activo"
                                ),
                            style = MaterialTheme.typography.bodyMedium,
                            color = if (activoCoincidente != null) {
                                MaterialTheme.colorScheme.onSurface
                            } else {
                                MaterialTheme.colorScheme.error
                            },
                            modifier = Modifier.padding(vertical = 4.dp),
                        )
                    }

                    val encontrados = viewModel.coincidenciasGafete.filter { it.activo != null }
                    if (encontrados.isNotEmpty()) {
                        Button(
                            onClick = { viewModel.registrarSalidaPorGafetes() },
                            enabled = !viewModel.enviandoGafetes,
                            colors = ButtonDefaults.buttonColors(
                                containerColor = MaterialTheme.colorScheme.secondary,
                                contentColor = MaterialTheme.colorScheme.onSecondary,
                            ),
                            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
                        ) {
                            Text(if (viewModel.enviandoGafetes) "Registrando…" else "Registrar salida (${encontrados.size})")
                        }
                    }
                }
            }
        } else if (viewModel.modo != ModoBusqueda.ENTRADA || viewModel.texto.isBlank()) {
            LazyColumn(modifier = Modifier.padding(top = 8.dp)) {
                items(viewModel.activos, key = { it.registroId }) { activo ->
                    FilaActivo(activo, onClick = { viewModel.elegirSeleccionSalida(activo) })
                    HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                }
            }
        } else {
            if (viewModel.resultadosBusqueda.isEmpty() && viewModel.seleccionIngreso !is SeleccionIngreso.Cargando) {
                Text(
                    "Sin resultados",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 12.dp),
                )
            }
            LazyColumn(modifier = Modifier.padding(top = 8.dp)) {
                items(viewModel.resultadosBusqueda, key = { it.id }) { contratista ->
                    FilaContratista(contratista, onClick = { viewModel.elegir(contratista) })
                    HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                }
            }
        }
    }

    val activo = viewModel.seleccionSalida
    if (activo != null) {
        AlertDialog(
            onDismissRequest = { viewModel.elegirSeleccionSalida(null) },
            title = { Text("Registrar salida") },
            text = {
                Text("${activo.contratistaNombre} · ${activo.cedula} · ${activo.empresaNombre}")
            },
            confirmButton = {
                TextButton(onClick = { viewModel.confirmarSalida(activo) }) {
                    Text("Confirmar")
                }
            },
            dismissButton = {
                TextButton(onClick = { viewModel.elegirSeleccionSalida(null) }) {
                    Text("Cancelar")
                }
            },
        )
    }
}

@Composable
private fun FilaActivo(activo: IngresoActivoResumen, onClick: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick).padding(vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(activo.contratistaNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            "${activo.cedula} · ${activo.empresaNombre}" +
                if (activo.gafeteNumero != null) " · Gafete ${activo.gafeteNumero}" else " · Sin gafete",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Ingresó ${textoFechaHora(activo.fechaHoraIngreso)} · dio ingreso ${activo.usuarioIngresoNombre}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            textoEstadoAcceso(activo.resultadoAcceso),
            style = MaterialTheme.typography.bodySmall,
            color = colorEstadoAcceso(activo.resultadoAcceso),
        )
    }
}

@Composable
private fun FilaContratista(contratista: ContratistaResumen, onClick: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick).padding(vertical = 10.dp),
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
            Text(
                "Sin acceso autorizado",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
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
        TipoIngreso.IN_HOUSE -> "In-house"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }

private fun textoEstadoAcceso(resultado: ResultadoAcceso): String =
    when (resultado) {
        is ResultadoAcceso.Permitido -> "Al día"
        is ResultadoAcceso.PermitidoConAdvertencia -> "PRAIND próximo a vencer"
        is ResultadoAcceso.Denegado -> mensajeMotivoDenegacion(resultado.motivo)
    }

@Composable
private fun colorEstadoAcceso(resultado: ResultadoAcceso) =
    when (resultado) {
        is ResultadoAcceso.Permitido -> MaterialTheme.colorScheme.primary
        is ResultadoAcceso.PermitidoConAdvertencia -> MaterialTheme.colorScheme.error
        is ResultadoAcceso.Denegado -> MaterialTheme.colorScheme.error
    }

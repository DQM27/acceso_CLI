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
/// (ver mobile/app/ARQUITECTURA.md) — este archivo sólo dibuja lo que el
/// ViewModel expone y le reporta eventos. Esta función orquesta: delega el
/// selector, el campo, los mensajes y el contenido (uno por modo, ver
/// [ContenidoModoEntrada]/[ContenidoModoSalidaNombre]/[ContenidoModoSalidaGafete]
/// más abajo) a funciones chicas de una sola responsabilidad cada una, en
/// vez de tener los tres modos mezclados en un único bloque `if`/`else`.
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

    val verificando = viewModel.seleccionIngreso is SeleccionIngreso.Cargando

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        SelectorModoBusqueda(modo = viewModel.modo, onCambiar = { viewModel.cambiarModo(it) })

        CampoBusquedaActivos(
            modo = viewModel.modo,
            texto = viewModel.texto,
            onCambiarTexto = { viewModel.cambiarTexto(it) },
        )

        // Sólo fuera del modo gafete — ese modo tiene su propio texto de
        // ayuda dentro de ContenidoModoSalidaGafete en vez de esta leyenda.
        if (viewModel.modo != ModoBusqueda.SALIDA_GAFETE) {
            LeyendaBusqueda(modo = viewModel.modo, texto = viewModel.texto, activos = viewModel.activos)
        }

        MensajesEstado(
            error = viewModel.error,
            mensaje = viewModel.mensaje,
            mensajeEsError = viewModel.mensajeEsError,
            verificando = verificando,
        )

        when (viewModel.modo) {
            ModoBusqueda.ENTRADA -> ContenidoModoEntrada(
                texto = viewModel.texto,
                activos = viewModel.activos,
                resultadosBusqueda = viewModel.resultadosBusqueda,
                verificando = verificando,
                onElegirActivo = { viewModel.elegirSeleccionSalida(it) },
                onElegirContratista = { viewModel.elegir(it) },
            )
            ModoBusqueda.SALIDA_NOMBRE -> ContenidoModoSalidaNombre(
                activos = viewModel.activos,
                onElegirActivo = { viewModel.elegirSeleccionSalida(it) },
            )
            ModoBusqueda.SALIDA_GAFETE -> ContenidoModoSalidaGafete(
                texto = viewModel.texto,
                coincidencias = viewModel.coincidenciasGafete,
                enviando = viewModel.enviandoGafetes,
                onRegistrarSalidaGafetes = { viewModel.registrarSalidaPorGafetes() },
            )
        }
    }

    DialogoConfirmarSalida(
        activo = viewModel.seleccionSalida,
        onDismiss = { viewModel.elegirSeleccionSalida(null) },
        onConfirmar = { viewModel.confirmarSalida(it) },
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SelectorModoBusqueda(modo: ModoBusqueda, onCambiar: (ModoBusqueda) -> Unit) {
    SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
        SegmentedButton(
            selected = modo == ModoBusqueda.ENTRADA,
            onClick = { onCambiar(ModoBusqueda.ENTRADA) },
            shape = SegmentedButtonDefaults.itemShape(index = 0, count = 3),
        ) {
            Text("Entrada")
        }
        SegmentedButton(
            selected = modo == ModoBusqueda.SALIDA_NOMBRE,
            onClick = { onCambiar(ModoBusqueda.SALIDA_NOMBRE) },
            shape = SegmentedButtonDefaults.itemShape(index = 1, count = 3),
        ) {
            Text("Salida: nombre")
        }
        SegmentedButton(
            selected = modo == ModoBusqueda.SALIDA_GAFETE,
            onClick = { onCambiar(ModoBusqueda.SALIDA_GAFETE) },
            shape = SegmentedButtonDefaults.itemShape(index = 2, count = 3),
        ) {
            Text("Salida: gafete")
        }
    }
}

@Composable
private fun CampoBusquedaActivos(modo: ModoBusqueda, texto: String, onCambiarTexto: (String) -> Unit) {
    // Color propio para "estoy buscando a quién SACAR" — evita confundir el
    // modo entrada (color normal de la app) con el de salida, que es la
    // acción de mayor consecuencia.
    val colorModo = if (modo == ModoBusqueda.ENTRADA) {
        MaterialTheme.colorScheme.primary
    } else {
        MaterialTheme.colorScheme.secondary
    }

    OutlinedTextField(
        value = texto,
        onValueChange = onCambiarTexto,
        label = {
            Text(
                if (modo == ModoBusqueda.SALIDA_GAFETE) {
                    "Números de gafete, separados por coma"
                } else {
                    "Cédula o nombre"
                },
            )
        },
        leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
        singleLine = true,
        keyboardOptions = if (modo == ModoBusqueda.SALIDA_GAFETE) {
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
}

@Composable
private fun LeyendaBusqueda(modo: ModoBusqueda, texto: String, activos: List<IngresoActivoResumen>) {
    val leyenda = when {
        texto.isBlank() && modo == ModoBusqueda.ENTRADA ->
            if (activos.isEmpty()) {
                "Nadie adentro"
            } else {
                "${activos.size} adentro · toque un nombre para registrar salida"
            }
        texto.isBlank() -> "Escriba para buscar entre los activos"
        modo == ModoBusqueda.ENTRADA -> "Buscando contratistas · toque un resultado para registrar entrada"
        activos.isEmpty() -> "Sin coincidencias entre los activos"
        else -> "Buscando entre los activos · toque un nombre para registrar salida"
    }
    Text(
        leyenda,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(top = 8.dp),
    )
}

@Composable
private fun MensajesEstado(error: String?, mensaje: String?, mensajeEsError: Boolean, verificando: Boolean) {
    if (error != null) {
        Text(error, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(top = 12.dp))
    }
    if (mensaje != null) {
        Text(
            mensaje,
            color = if (mensajeEsError) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary,
            modifier = Modifier.padding(top = 12.dp),
        )
    }
    if (verificando) {
        Text(
            "Verificando…",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 12.dp),
        )
    }
}

/// Vacío: lista de quién está adentro (tocar = salida). Con texto: busca en
/// el catálogo completo (tocar = arrancar el flujo de entrada). Ver el
/// doc-comment de [PantallaActivos].
@Composable
private fun ContenidoModoEntrada(
    texto: String,
    activos: List<IngresoActivoResumen>,
    resultadosBusqueda: List<ContratistaResumen>,
    verificando: Boolean,
    onElegirActivo: (IngresoActivoResumen) -> Unit,
    onElegirContratista: (ContratistaResumen) -> Unit,
) {
    if (texto.isBlank()) {
        ListaActivos(activos, onClick = onElegirActivo)
        return
    }
    if (resultadosBusqueda.isEmpty() && !verificando) {
        Text(
            "Sin resultados",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 12.dp),
        )
    }
    LazyColumn(modifier = Modifier.padding(top = 8.dp)) {
        items(resultadosBusqueda, key = { it.id }) { contratista ->
            FilaContratista(contratista, onClick = { onElegirContratista(contratista) })
            HorizontalDivider(color = MaterialTheme.colorScheme.outline)
        }
    }
}

/// Filtra la lista de activos por cédula/nombre — vacío no trae nada, es un
/// buscador para acotar, no una lista para recorrer (esa es el modo
/// Entrada). Ver el doc-comment de [PantallaActivos].
@Composable
private fun ContenidoModoSalidaNombre(activos: List<IngresoActivoResumen>, onElegirActivo: (IngresoActivoResumen) -> Unit) {
    ListaActivos(activos, onClick = onElegirActivo)
}

/// Uno o más números de gafete separados por coma, con vista previa de a
/// quién le corresponde cada uno antes de un único botón que confirma
/// todos de una vez. Ver el doc-comment de [PantallaActivos].
@Composable
private fun ContenidoModoSalidaGafete(
    texto: String,
    coincidencias: List<CoincidenciaGafete>,
    enviando: Boolean,
    onRegistrarSalidaGafetes: () -> Unit,
) {
    if (texto.isBlank()) {
        Text(
            "Escriba uno o más números de gafete, separados por coma",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 8.dp),
        )
        return
    }

    Column(modifier = Modifier.padding(top = 8.dp)) {
        coincidencias.forEach { coincidencia ->
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

        val encontrados = coincidencias.filter { it.activo != null }
        if (encontrados.isNotEmpty()) {
            Button(
                onClick = onRegistrarSalidaGafetes,
                enabled = !enviando,
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.secondary,
                    contentColor = MaterialTheme.colorScheme.onSecondary,
                ),
                modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
            ) {
                Text(if (enviando) "Registrando…" else "Registrar salida (${encontrados.size})")
            }
        }
    }
}

/// Lista de activos compartida entre Entrada (campo vacío) y Salida:
/// nombre — mismas filas, mismo comportamiento, sólo cambia qué hace
/// `onClick` según quién la use.
@Composable
private fun ListaActivos(activos: List<IngresoActivoResumen>, onClick: (IngresoActivoResumen) -> Unit) {
    LazyColumn(modifier = Modifier.padding(top = 8.dp)) {
        items(activos, key = { it.registroId }) { activo ->
            FilaActivo(activo, onClick = { onClick(activo) })
            HorizontalDivider(color = MaterialTheme.colorScheme.outline)
        }
    }
}

@Composable
private fun DialogoConfirmarSalida(
    activo: IngresoActivoResumen?,
    onDismiss: () -> Unit,
    onConfirmar: (IngresoActivoResumen) -> Unit,
) {
    if (activo == null) return

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Registrar salida") },
        text = {
            Text("${activo.contratistaNombre} · ${activo.cedula} · ${activo.empresaNombre}")
        },
        confirmButton = {
            TextButton(onClick = { onConfirmar(activo) }) {
                Text("Confirmar")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancelar")
            }
        },
    )
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

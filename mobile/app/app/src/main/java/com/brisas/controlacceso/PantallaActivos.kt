package com.brisas.controlacceso

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.PreparacionIngreso
import uniffi.control_acceso_mobile.ResultadoAcceso
import uniffi.control_acceso_mobile.TipoIngreso

/// Mismo árbol de estados que `Seleccion` en NuevoIngresoModal.tsx: sin
/// selección (buscador visible), verificando (prepararIngreso en vuelo),
/// bloqueada (Rust ya decidió que no puede continuar) o lista para
/// confirmar. Kotlin sólo despacha sobre lo que Rust ya calculó.
private sealed class SeleccionIngreso {
    data object Ninguna : SeleccionIngreso()

    data class Cargando(val contratista: ContratistaResumen) : SeleccionIngreso()

    data class Bloqueada(val preparacion: PreparacionIngreso, val mensaje: String) : SeleccionIngreso()

    data class Formulario(val preparacion: PreparacionIngreso) : SeleccionIngreso()
}

/// Una sola vista para el ciclo completo — entrada, permanencia y salida —,
/// igual que `Activos.tsx` en desktop (ahí "+Nuevo"/"Salida" abren modales
/// sobre la misma grilla de activos; ver docs/plan-app-movil.md, addendum
/// 2026-09-01). Acá no hay una pestaña "Buscar" aparte: el mismo campo de
/// texto cambia de sentido según esté vacío o no — mismo truco que
/// `SalidaModal.tsx` (ahí un checkbox "Por gafete" hace lo mismo con un solo
/// campo en vez de duplicar pantallas casi idénticas):
///
/// - Vacío: lista quién está adentro (antes vivía en esta misma pantalla) —
///   tocar un nombre confirma su salida.
/// - Con texto: busca en el catálogo completo de contratistas (antes
///   pestaña "Buscar" aparte) — tocar un resultado arranca el flujo de
///   confirmar entrada.
///
/// A diferencia de desktop (SalidaModal.tsx), que agrega un modo "por
/// gafete" con texto separado por comas para aprovechar el teclado del
/// guardia en la PC, aquí no hay atajos de teclado que aprovechar — todo es
/// táctil.
@Composable
fun PantallaActivos(nucleo: Nucleo) {
    var texto by remember { mutableStateOf("") }
    var activos by remember { mutableStateOf<List<IngresoActivoResumen>>(emptyList()) }
    var resultadosBusqueda by remember { mutableStateOf<List<ContratistaResumen>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }
    var seleccionSalida by remember { mutableStateOf<IngresoActivoResumen?>(null) }
    var seleccionIngreso by remember { mutableStateOf<SeleccionIngreso>(SeleccionIngreso.Ninguna) }
    var recargas by remember { mutableIntStateOf(0) }
    val alcance = rememberCoroutineScope()

    LaunchedEffect(texto, recargas) {
        try {
            if (texto.isBlank()) {
                activos = withContext(Dispatchers.Default) { nucleo.listarIngresosActivos("") }
            } else {
                resultadosBusqueda = withContext(Dispatchers.Default) { nucleo.buscarContratistas(texto) }
            }
            error = null
        } catch (excepcion: Exception) {
            error = excepcion.message
        }
    }

    suspend fun elegir(contratista: ContratistaResumen) {
        seleccionIngreso = SeleccionIngreso.Cargando(contratista)
        try {
            val preparacion = withContext(Dispatchers.Default) { nucleo.prepararIngreso(contratista.id) }
            seleccionIngreso = if (puedeContinuar(preparacion)) {
                SeleccionIngreso.Formulario(preparacion)
            } else {
                SeleccionIngreso.Bloqueada(preparacion, mensajeBloqueo(preparacion))
            }
        } catch (excepcion: Exception) {
            error = excepcion.message
            seleccionIngreso = SeleccionIngreso.Ninguna
        }
    }

    when (val actual = seleccionIngreso) {
        is SeleccionIngreso.Formulario -> {
            PantallaConfirmarIngreso(
                nucleo = nucleo,
                preparacion = actual.preparacion,
                onRegistrado = {
                    seleccionIngreso = SeleccionIngreso.Ninguna
                    texto = ""
                    recargas++
                },
                onCambiar = { seleccionIngreso = SeleccionIngreso.Ninguna },
            )
            return
        }
        is SeleccionIngreso.Bloqueada -> {
            PantallaIngresoBloqueado(
                preparacion = actual.preparacion,
                mensaje = actual.mensaje,
                onCambiar = { seleccionIngreso = SeleccionIngreso.Ninguna },
            )
            return
        }
        else -> Unit
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        OutlinedTextField(
            value = texto,
            onValueChange = { texto = it },
            label = { Text("Cédula o nombre") },
            leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
            singleLine = true,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth(),
        )

        Text(
            if (texto.isBlank()) {
                if (activos.isEmpty()) "Nadie adentro" else "${activos.size} adentro · toque un nombre para registrar salida"
            } else {
                "Buscando contratistas · toque un resultado para registrar entrada"
            },
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 8.dp),
        )

        val mensajeError = error
        if (mensajeError != null) {
            Text(mensajeError, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(top = 12.dp))
        }
        if (seleccionIngreso is SeleccionIngreso.Cargando) {
            Text(
                "Verificando…",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 12.dp),
            )
        }

        if (texto.isBlank()) {
            LazyColumn(modifier = Modifier.padding(top = 8.dp)) {
                items(activos, key = { it.registroId }) { activo ->
                    FilaActivo(activo, onClick = { seleccionSalida = activo })
                    HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                }
            }
        } else {
            if (resultadosBusqueda.isEmpty() && seleccionIngreso !is SeleccionIngreso.Cargando) {
                Text(
                    "Sin resultados",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 12.dp),
                )
            }
            LazyColumn(modifier = Modifier.padding(top = 8.dp)) {
                items(resultadosBusqueda, key = { it.id }) { contratista ->
                    FilaContratista(contratista, onClick = { alcance.launch { elegir(contratista) } })
                    HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                }
            }
        }
    }

    val activo = seleccionSalida
    if (activo != null) {
        AlertDialog(
            onDismissRequest = { seleccionSalida = null },
            title = { Text("Registrar salida") },
            text = {
                Text("${activo.contratistaNombre} · ${activo.cedula} · ${activo.empresaNombre}")
            },
            confirmButton = {
                TextButton(onClick = {
                    seleccionSalida = null
                    alcance.launch {
                        try {
                            withContext(Dispatchers.Default) { nucleo.registrarSalida(activo.registroId) }
                            recargas++
                        } catch (excepcion: Exception) {
                            error = excepcion.message
                        }
                    }
                }) {
                    Text("Confirmar")
                }
            },
            dismissButton = {
                TextButton(onClick = { seleccionSalida = null }) {
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

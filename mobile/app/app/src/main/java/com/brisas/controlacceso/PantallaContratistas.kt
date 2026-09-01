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
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
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
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.PreparacionIngreso
import uniffi.control_acceso_mobile.TipoIngreso

/// Mismo árbol de estados que `Seleccion` en NuevoIngresoModal.tsx: sin
/// selección (buscador visible), verificando (prepararIngreso en vuelo),
/// bloqueada (Rust ya decidió que no puede continuar) o lista para
/// confirmar. Kotlin sólo despacha sobre lo que Rust ya calculó.
private sealed class Seleccion {
    data object Ninguna : Seleccion()

    data class Cargando(val contratista: ContratistaResumen) : Seleccion()

    data class Bloqueada(val preparacion: PreparacionIngreso, val mensaje: String) : Seleccion()

    data class Formulario(val preparacion: PreparacionIngreso) : Seleccion()
}

// Vía primaria del guardia (ver docs/plan-app-movil.md): busca directo sobre
// la copia local en SQLite, sin red de por medio, así que cada tecla llama
// de nuevo a Nucleo.buscarContratistas — la decisión de qué cuenta como
// coincidencia sigue siendo de Rust (BusquedaTexto), esta pantalla solo pinta.
@Composable
fun PantallaContratistas(nucleo: Nucleo) {
    var texto by remember { mutableStateOf("") }
    var resultados by remember { mutableStateOf<List<ContratistaResumen>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }
    var seleccion by remember { mutableStateOf<Seleccion>(Seleccion.Ninguna) }

    LaunchedEffect(texto) {
        try {
            resultados = withContext(Dispatchers.Default) { nucleo.buscarContratistas(texto) }
            error = null
        } catch (excepcion: Exception) {
            error = excepcion.message
        }
    }

    suspend fun elegir(contratista: ContratistaResumen) {
        seleccion = Seleccion.Cargando(contratista)
        try {
            val preparacion = withContext(Dispatchers.Default) { nucleo.prepararIngreso(contratista.id) }
            seleccion = if (puedeContinuar(preparacion)) {
                Seleccion.Formulario(preparacion)
            } else {
                Seleccion.Bloqueada(preparacion, mensajeBloqueo(preparacion))
            }
        } catch (excepcion: Exception) {
            error = excepcion.message
            seleccion = Seleccion.Ninguna
        }
    }

    when (val actual = seleccion) {
        is Seleccion.Formulario -> {
            PantallaConfirmarIngreso(
                nucleo = nucleo,
                preparacion = actual.preparacion,
                onRegistrado = {
                    seleccion = Seleccion.Ninguna
                    texto = ""
                },
                onCambiar = { seleccion = Seleccion.Ninguna },
            )
            return
        }
        is Seleccion.Bloqueada -> {
            PantallaIngresoBloqueado(
                preparacion = actual.preparacion,
                mensaje = actual.mensaje,
                onCambiar = { seleccion = Seleccion.Ninguna },
            )
            return
        }
        else -> Unit
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        OutlinedTextField(
            value = texto,
            onValueChange = { texto = it },
            label = { Text("Buscar contratista") },
            leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
            singleLine = true,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth(),
        )

        val mensajeError = error
        if (mensajeError != null) {
            Text(
                mensajeError,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(top = 12.dp),
            )
        }

        if (seleccion is Seleccion.Cargando) {
            Text(
                "Verificando…",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 12.dp),
            )
        }

        val alcance = rememberCoroutineScope()
        LazyColumn(modifier = Modifier.padding(top = 12.dp)) {
            items(resultados, key = { it.id }) { contratista ->
                FilaContratista(contratista, onClick = { alcance.launch { elegir(contratista) } })
                HorizontalDivider(color = MaterialTheme.colorScheme.outline)
            }
        }
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

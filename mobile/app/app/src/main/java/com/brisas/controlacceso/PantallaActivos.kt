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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.ModoBusquedaActivos
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

/// Con pocos activos alcanza con recorrer la lista a ojo, pero con muchos
/// (imaginemos 100) hace falta poder acotarla — el mismo campo de texto no
/// puede a la vez buscar en el catálogo completo (para entrada) Y filtrar
/// los activos (para salida), así que un selector de tres decide cuál de
/// las dos cosas está haciendo el campo. `SALIDA_NOMBRE`/`SALIDA_GAFETE` van
/// separados (y no uno solo "salida") porque la búsqueda de texto libre de
/// Rust ya mezcla nombre/cédula con gafete en el mismo OR — buscar "7" como
/// gafete también traería cualquier cédula que lo contenga, ruidoso con
/// muchos activos. `Nucleo.listarIngresosActivos` recibe `ModoBusquedaActivos`
/// para pedirle a Rust el filtro exacto en vez de resolverlo del lado del
/// teléfono.
private enum class ModoBusqueda { ENTRADA, SALIDA_NOMBRE, SALIDA_GAFETE }

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
/// - **Salida: nombre/Salida: gafete**: filtran la MISMA lista de activos
///   por cédula/nombre o por número de gafete exacto — para encontrar a
///   alguien puntual cuando hay muchos adentro a la vez, sin tener que
///   recorrer la lista a ojo.
///
/// A diferencia de desktop (SalidaModal.tsx), que en modo gafete acepta
/// varios números separados por coma y confirma todos de una sin diálogo
/// (pensado para el teclado físico de la PC), acá cada coincidencia se
/// confirma tocándola — mismo diálogo de siempre — porque todo es táctil y
/// confirmar una salida de más por un tropiezo en el teclado numérico sale
/// más caro que el ahorro de tiempo.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PantallaActivos(nucleo: Nucleo) {
    var texto by remember { mutableStateOf("") }
    var modo by remember { mutableStateOf(ModoBusqueda.ENTRADA) }
    var activos by remember { mutableStateOf<List<IngresoActivoResumen>>(emptyList()) }
    var resultadosBusqueda by remember { mutableStateOf<List<ContratistaResumen>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }
    var seleccionSalida by remember { mutableStateOf<IngresoActivoResumen?>(null) }
    var seleccionIngreso by remember { mutableStateOf<SeleccionIngreso>(SeleccionIngreso.Ninguna) }
    var recargas by remember { mutableIntStateOf(0) }
    val alcance = rememberCoroutineScope()

    fun cambiarModo(nuevo: ModoBusqueda) {
        modo = nuevo
        // Al cambiar de modo el texto que había queda escrito con otro
        // sentido (un nombre no significa nada en modo Gafete) — se limpia
        // para no arrastrar una búsqueda que ya no aplica.
        texto = ""
    }

    LaunchedEffect(texto, recargas, modo) {
        try {
            when (modo) {
                ModoBusqueda.ENTRADA -> {
                    if (texto.isBlank()) {
                        activos = withContext(Dispatchers.Default) {
                            nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA)
                        }
                    } else {
                        resultadosBusqueda = withContext(Dispatchers.Default) { nucleo.buscarContratistas(texto) }
                    }
                }
                ModoBusqueda.SALIDA_NOMBRE -> {
                    activos = withContext(Dispatchers.Default) {
                        nucleo.listarIngresosActivos(texto, ModoBusquedaActivos.NOMBRE_CEDULA)
                    }
                }
                ModoBusqueda.SALIDA_GAFETE -> {
                    activos = withContext(Dispatchers.Default) {
                        nucleo.listarIngresosActivos(texto, ModoBusquedaActivos.GAFETE)
                    }
                }
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

    // Color propio para "estoy buscando a quién SACAR" — evita confundir el
    // modo entrada (color normal de la app) con el de salida, que es la
    // acción de mayor consecuencia.
    val colorModo = if (modo == ModoBusqueda.ENTRADA) {
        MaterialTheme.colorScheme.primary
    } else {
        MaterialTheme.colorScheme.secondary
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
            SegmentedButton(
                selected = modo == ModoBusqueda.ENTRADA,
                onClick = { cambiarModo(ModoBusqueda.ENTRADA) },
                shape = SegmentedButtonDefaults.itemShape(index = 0, count = 3),
            ) {
                Text("Entrada")
            }
            SegmentedButton(
                selected = modo == ModoBusqueda.SALIDA_NOMBRE,
                onClick = { cambiarModo(ModoBusqueda.SALIDA_NOMBRE) },
                shape = SegmentedButtonDefaults.itemShape(index = 1, count = 3),
            ) {
                Text("Salida: nombre")
            }
            SegmentedButton(
                selected = modo == ModoBusqueda.SALIDA_GAFETE,
                onClick = { cambiarModo(ModoBusqueda.SALIDA_GAFETE) },
                shape = SegmentedButtonDefaults.itemShape(index = 2, count = 3),
            ) {
                Text("Salida: gafete")
            }
        }

        OutlinedTextField(
            value = texto,
            onValueChange = { nuevo ->
                texto = if (modo == ModoBusqueda.SALIDA_GAFETE) nuevo.filter(Char::isDigit) else nuevo
            },
            label = { Text(if (modo == ModoBusqueda.SALIDA_GAFETE) "Número de gafete" else "Cédula o nombre") },
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

        val leyenda = when {
            texto.isBlank() ->
                if (activos.isEmpty()) "Nadie adentro" else "${activos.size} adentro · toque un nombre para registrar salida"
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

        if (modo != ModoBusqueda.ENTRADA || texto.isBlank()) {
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

package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.imePadding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.EncargadoRuta
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
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
/// de gafete de la categoría física aparte.
///
/// Rediseño 2026-10-21 (como la app Teléfono de iOS 26, pedido del usuario):
/// la pantalla es sólo la lista de gafetes prestados. La lupa del botón
/// redondo de abajo abre el buscador de encargado pegado al teclado
/// ([BuscadorInferior]); elegir a alguien cierra la búsqueda y sube el panel
/// "Entregar gafete" con el número. Antes el buscador y el formulario vivían
/// arriba y había que subir a buscarlos.
@Composable
fun PantallaGafetesProvisionales(
    nucleo: Nucleo,
    refrescarNube: Int = 0,
    // Tirar de la lista hacia abajo sincroniza con la nube, igual que el botón
    // del encabezado de la pantalla principal (que es quien sabe hacerlo).
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    RegistrarPantalla("gafetes_provisionales")
    val viewModel: GafetesProvisionalesViewModel =
        viewModel(factory = GafetesProvisionalesViewModel.factory(nucleo))
    var filaParaDevolver by remember { mutableStateOf<FilaGafeteProvisionalActiva?>(null) }
    var buscando by remember { mutableStateOf(false) }
    LaunchedEffect(refrescarNube) {
        if (refrescarNube > 0) {
            viewModel.refrescarActivos()
        }
    }

    AccionPrincipal(Icons.Default.Search, "Buscar encargado para entregar un gafete", onClick = { buscando = true })

    Box(modifier = Modifier.fillMaxSize()) {
        Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
            Text(
                "Gafete provisional KOF",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
            )

            // Fallas de la lista (la entrega muestra las suyas en su panel).
            if (viewModel.encargadoSeleccionado == null) {
                viewModel.error?.let { mensaje ->
                    Text(
                        mensaje,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.padding(top = 8.dp),
                    )
                }
            }

            // Sin encabezado "Gafetes prestados" -- pedido explícito del usuario
            // en pruebas reales, 2026-09-17: dejarlo limpio, igual que la lista
            // de activos de Contratista (que tampoco tiene título propio).
            ListaConEstados(
                filas = viewModel.activos,
                clave = { it.clave() },
                cargando = viewModel.cargando,
                tituloVacio = "Ningún gafete prestado",
                detalleVacio = "Para entregar uno, toque la lupa de abajo.",
                sincronizando = sincronizando,
                onSincronizar = onSincronizar,
                contentPadding = PaddingValues(top = 10.dp, bottom = EspacioPieLista),
            ) { fila ->
                FilaPrestamoGafeteProvisional(
                    fila = fila,
                    onConfirmarDevolucion = { filaParaDevolver = fila },
                )
            }
        }
        PieFlotante(viewModel.confirmacion, modifier = Modifier.align(Alignment.BottomCenter))

        if (buscando) {
            BuscadorInferior(
                texto = viewModel.textoEncargado,
                onCambiar = viewModel::cambiarTextoEncargado,
                ayuda = "Nombre o código de empleado",
                onCerrar = {
                    buscando = false
                    viewModel.cambiarTextoEncargado("")
                },
                modifier = Modifier.background(MaterialTheme.colorScheme.background),
            ) {
                ResultadosEncargado(viewModel.textoEncargado, viewModel.resultadosEncargado) { encargado ->
                    viewModel.elegirEncargado(encargado)
                    buscando = false
                }
            }
        }
    }

    viewModel.encargadoSeleccionado?.let { encargado ->
        HojaEntregaGafete(
            encargado = encargado,
            error = viewModel.error,
            registrando = viewModel.registrando,
            // Al entregar bien, el ViewModel suelta al encargado y el panel se va.
            onEntregar = { numero -> viewModel.entregar(numero) {} },
            onCancelar = viewModel::cancelarEntrega,
        )
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

/// Los encargados que coinciden con lo escrito, como tarjetas tocables (igual
/// que `FilaContratista` en modo Ingreso). Sin texto, una indicación.
@Composable
private fun ResultadosEncargado(texto: String, resultados: List<EncargadoRuta>, onElegir: (EncargadoRuta) -> Unit) {
    if (resultados.isEmpty()) {
        Text(
            if (texto.isBlank()) "Escriba el nombre o el código de empleado." else "Sin resultados",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(24.dp),
        )
        return
    }
    LazyColumn(
        contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 8.dp),
        verticalArrangement = Arrangement.spacedBy(5.dp),
    ) {
        items(resultados, key = { it.id }) { encargado ->
            FilaEncargadoRuta(encargado, onClick = { onElegir(encargado) })
        }
    }
}

/// "Entregar gafete" en un panel que sube desde abajo, con el encargado ya
/// elegido: el código para cotejar con lo que dice la persona, el número de
/// gafete (con el teclado numérico ya abierto) y "Entregar". Al entregar,
/// el panel se va y queda la confirmación verde; cerrarlo cancela la entrega.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun HojaEntregaGafete(
    encargado: EncargadoRuta,
    error: String?,
    registrando: Boolean,
    onEntregar: (numero: Long) -> Unit,
    onCancelar: () -> Unit,
) {
    val estado = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val alcance = rememberCoroutineScope()
    var gafeteTexto by remember { mutableStateOf("") }
    val foco = remember { FocusRequester() }
    LaunchedEffect(Unit) { foco.requestFocus() }
    fun cerrarY(accion: () -> Unit) {
        alcance.launch { estado.hide() }.invokeOnCompletion { accion() }
    }
    val numero = gafeteTexto.toLongOrNull()
    ModalBottomSheet(
        onDismissRequest = onCancelar,
        sheetState = estado,
        containerColor = MaterialTheme.colorScheme.surface,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 24.dp, end = 24.dp, bottom = 24.dp)
                .imePadding(),
        ) {
            Text("Entregar gafete", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
            Text(
                encargado.nombre,
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.padding(top = 12.dp),
            )
            Text(
                buildAnnotatedString {
                    append("Código de empleado: ")
                    withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold)) {
                        append(encargado.codigoEmpleado)
                    }
                    append(" · corroborar contra lo que dice la persona")
                },
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 2.dp),
            )
            // Sin `.height(AlturaBusquedaBrisas)` a propósito -- a ese alto el
            // placeholder de un `OutlinedTextField` queda recortado (bug
            // reportado en pruebas reales, 2026-09-17).
            OutlinedTextField(
                value = gafeteTexto,
                onValueChange = { gafeteTexto = it.filter(Char::isDigit) },
                placeholder = { Text("Número de gafete") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().padding(top = 16.dp).focusRequester(foco),
            )
            if (error != null) {
                Text(
                    error,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(top = 8.dp),
                )
            }
            BotonBrisas(
                onClick = {
                    // Si el núcleo rechaza la entrega, el panel queda abierto
                    // con el error.
                    onEntregar(numero ?: return@BotonBrisas)
                },
                enabled = numero != null && !registrando,
                modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
            ) {
                Text(if (registrando) "Entregando…" else "Entregar")
            }
            BotonDiscretoBrisas(
                onClick = { cerrarY(onCancelar) },
                modifier = Modifier.align(Alignment.CenterHorizontally).padding(top = 4.dp),
            ) {
                Text("Cancelar")
            }
        }
    }
}

private fun FilaGafeteProvisionalActiva.clave(): String = when (this) {
    is FilaGafeteProvisionalActiva.Local -> "local-${prestamo.id}"
    is FilaGafeteProvisionalActiva.Remota -> "remota-${remoto.uuid}"
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
        // Mismo criterio que la tarjeta de Contratista activo
        // (`PantallaActivos.kt::FilaActivoLocal`) -- mayúscula + negrita en
        // toda la línea, gafete resaltado -- pero acá en rojo en vez de azul
        // (`colorScheme.error` en vez de `colorScheme.primary`), pedido
        // explícito del usuario 2026-09-21 para distinguir de un vistazo un
        // gafete PROVISIONAL KOF de uno de contratista. "Código de empleado:"
        // explícito (no sólo el valor pelado) para consistencia con el resto
        // de tarjetas del mismo módulo (ver ControlesBrisas.kt/el diálogo de
        // asignación más arriba en este archivo, que ya usan esa etiqueta).
        Text(
            buildAnnotatedString {
                append("Código de empleado: ${prestamo.encargadoCodigoEmpleado} · ".uppercase())
                withStyle(SpanStyle(color = MaterialTheme.colorScheme.error)) {
                    append("Gafete ${prestamo.gafeteNumero}".uppercase())
                }
            },
            style = MaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        // "· entregó X" -- mismo criterio que "dio ingreso X" en la tarjeta
        // de Contratista activo, pedido explícito del usuario 2026-09-21.
        Text(
            "Entregado ${textoFechaHora(prestamo.fechaHoraEntrega)} · entregó ${prestamo.usuarioEntregaNombre}",
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
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onConfirmarDevolucion)
            .padding(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(remoto.encargadoNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
            // Mismo criterio que FilaPrestamoGafeteProvisionalLocal -- ver ese
            // comentario.
            Text(
                buildAnnotatedString {
                    append("Código de empleado: ${remoto.encargadoCodigoEmpleado} · ".uppercase())
                    withStyle(SpanStyle(color = MaterialTheme.colorScheme.error)) {
                        append("Gafete ${remoto.gafeteNumero}".uppercase())
                    }
                },
                style = MaterialTheme.typography.bodySmall,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                "Entregado ${textoFechaHora(remoto.horaEntrega)} · entregó ${remoto.usuarioEntregaNombre}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        // El mismo ícono que el resto de las listas, en vez de texto.
        IconoOtroEquipo()
    }
}

/// El mismo panel que la salida de contratistas ([DialogoRegistrarSalida]),
/// con título "Registrar devolución" e info completa en vez de sólo
/// "Gafete N · nombre".
@Composable
private fun DialogoConfirmarDevolucionGafeteProvisional(
    fila: FilaGafeteProvisionalActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaGafeteProvisionalActiva) -> Unit,
) {
    if (fila == null) return
    DialogoRegistrarSalida(
        titulo = "Registrar devolución",
        detalle = when (fila) {
            is FilaGafeteProvisionalActiva.Local ->
                "${fila.prestamo.encargadoNombre} · ${fila.prestamo.encargadoCodigoEmpleado} · Gafete ${fila.prestamo.gafeteNumero}"
            is FilaGafeteProvisionalActiva.Remota ->
                "${fila.remoto.encargadoNombre} · $TEXTO_OTRO_DISPOSITIVO"
        },
        onDismiss = onDismiss,
        onConfirmar = { onConfirmar(fila) },
    )
}

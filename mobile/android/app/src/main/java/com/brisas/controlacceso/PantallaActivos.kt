package com.brisas.controlacceso

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
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
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.IngresoRemoto
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
/// - **Ingreso** (por defecto, `ModoBusqueda.ENTRADA`): vacío lista quién
///   está adentro (tocar un nombre confirma su salida); con texto busca en
///   el catálogo completo de contratistas (antes pestaña "Buscar" aparte)
///   para arrancar el flujo de confirmar entrada.
/// - **Salida** (`ModoBusqueda.SALIDA_NOMBRE`): filtra la lista de activos
///   por cédula/nombre — tocar un resultado abre el mismo diálogo de
///   confirmar salida de siempre. Vacío no trae nada (es un buscador, no
///   una lista para recorrer — para eso ya está la pestaña Ingreso).
/// - **Gafete** (`ModoBusqueda.SALIDA_GAFETE`): acepta varios números de
///   gafete separados por coma ("2, 25, 85") — igual que el modo gafete de
///   `SalidaModal.tsx` —
///   y muestra a quién le corresponde cada uno antes de confirmar. Un solo
///   botón registra la salida de todos los que sí tienen ingreso activo de
///   una vez, sin diálogo por persona: es la misma decisión de desktop
///   (pensada para cargar varios gafetes de un tirón), la vista previa con
///   nombres hace las veces de confirmación.
///
/// Todo el estado y las llamadas a [Nucleo] viven en [ActivosViewModel]
/// (ver mobile/android/ARQUITECTURA.md) — este archivo sólo dibuja lo que el
/// ViewModel expone y le reporta eventos. Esta función orquesta: delega el
/// selector, el campo, los mensajes y el contenido (uno por modo, ver
/// [ContenidoModoEntrada]/[ContenidoModoSalidaNombre]/[ContenidoModoSalidaGafete]
/// más abajo) a funciones chicas de una sola responsabilidad cada una, en
/// vez de tener los tres modos mezclados en un único bloque `if`/`else`.
@Composable
fun PantallaActivos(
    nucleo: Nucleo,
    secretoStore: SecretoDispositivoStore,
    refrescarNube: Int = 0,
) {
    val viewModel: ActivosViewModel =
        viewModel(factory = ActivosViewModel.factory(nucleo, secretoStore))
    var escanerAbierto by remember { mutableStateOf(false) }
    var escanerGafeteSalidaAbierto by remember { mutableStateOf(false) }
    LaunchedEffect(refrescarNube) {
        if (refrescarNube > 0) {
            viewModel.refrescar()
        }
    }

    when (val actual = viewModel.seleccionIngreso) {
        is SeleccionIngreso.Formulario -> {
            PantallaConfirmarIngreso(
                nucleo = nucleo,
                secretoStore = secretoStore,
                preparacion = actual.preparacion,
                ingresoAutomatico = viewModel.automatico,
                onCambiarIngresoAutomatico = { viewModel.cambiarAutomatico(it) },
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

    if (escanerAbierto) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
            onDocumentoDetectado = { documento ->
                escanerAbierto = false
                if (viewModel.modo != ModoBusqueda.ENTRADA) {
                    viewModel.cambiarModo(ModoBusqueda.ENTRADA)
                }
                viewModel.usarDocumentoEscaneadoIngreso(documento)
            },
            onCerrar = { escanerAbierto = false },
        )
        return
    }

    if (escanerGafeteSalidaAbierto) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.GAFETE_CONTRATISTA,
            continuo = viewModel.automatico,
            onDocumentoDetectado = { documento ->
                if (!viewModel.automatico) {
                    escanerGafeteSalidaAbierto = false
                }
                if (viewModel.modo != ModoBusqueda.SALIDA_GAFETE) {
                    viewModel.cambiarModo(ModoBusqueda.SALIDA_GAFETE)
                }
                if (viewModel.automatico) {
                    // Es suspend: el escáner no se rearma hasta que la
                    // mutación terminó. Así nunca entran dos gafetes en
                    // paralelo ni se cancela una salida ya iniciada.
                    viewModel.registrarSalidaPorGafeteEscaneado(documento)
                } else {
                    viewModel.cambiarTexto(documento.textoBusqueda ?: documento.numeroDocumento)
                }
            },
            onCerrar = { escanerGafeteSalidaAbierto = false },
        )
        return
    }

    val verificando = viewModel.seleccionIngreso is SeleccionIngreso.Cargando

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        SelectorModoBusqueda(modo = viewModel.modo, onCambiar = { viewModel.cambiarModo(it) })

        CampoBusquedaActivos(
            modo = viewModel.modo,
            texto = viewModel.texto,
            onCambiarTexto = { viewModel.cambiarTexto(it) },
            onEscanearCedula = { escanerAbierto = true },
            onEscanearGafete = { escanerGafeteSalidaAbierto = true },
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
                automatico = viewModel.automatico,
                onCambiarAutomatico = { viewModel.cambiarAutomatico(it) },
                onRegistrarSalidaGafetes = { viewModel.registrarSalidaPorGafetes() },
            )
        }
    }

    DialogoConfirmarSalida(
        fila = viewModel.seleccionSalida,
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
            EtiquetaSegmento("Ingreso")
        }
        SegmentedButton(
            selected = modo == ModoBusqueda.SALIDA_NOMBRE,
            onClick = { onCambiar(ModoBusqueda.SALIDA_NOMBRE) },
            shape = SegmentedButtonDefaults.itemShape(index = 1, count = 3),
        ) {
            EtiquetaSegmento("Salida")
        }
        SegmentedButton(
            selected = modo == ModoBusqueda.SALIDA_GAFETE,
            onClick = { onCambiar(ModoBusqueda.SALIDA_GAFETE) },
            shape = SegmentedButtonDefaults.itemShape(index = 2, count = 3),
        ) {
            EtiquetaSegmento("Gafete")
        }
    }
}

/// Antes las etiquetas eran "Entrada"/"Salida: nombre"/"Salida: gafete" --
/// las dos últimas, casi el doble de largo, se partían en dos líneas
/// dentro de su tercio del selector mientras la primera quedaba en una, y
/// el selector completo terminaba con altura despareja (reportado con
/// foto real: dos botones "enormes" al lado de uno chico). Ya no pasa con
/// las etiquetas cortas actuales ("Ingreso"/"Salida"/"Gafete"), pero
/// forzar una sola línea sigue siendo la defensa correcta si el texto
/// vuelve a crecer (más idioma, tipografía más grande por accesibilidad).
@Composable
private fun EtiquetaSegmento(texto: String) {
    Text(texto, maxLines = 1, overflow = TextOverflow.Ellipsis)
}

@Composable
private fun CampoBusquedaActivos(
    modo: ModoBusqueda,
    texto: String,
    onCambiarTexto: (String) -> Unit,
    onEscanearCedula: () -> Unit,
    onEscanearGafete: () -> Unit,
) {
    // Color propio para "estoy buscando a quién SACAR" — evita confundir el
    // modo entrada (color normal de la app) con el de salida, que es la
    // acción de mayor consecuencia.
    val colorModo = if (modo == ModoBusqueda.ENTRADA) {
        MaterialTheme.colorScheme.primary
    } else {
        MaterialTheme.colorScheme.secondary
    }

    Row(
        modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        OutlinedTextField(
            value = texto,
            onValueChange = onCambiarTexto,
            label = null,
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
            modifier = Modifier.weight(1f),
        )
        if (modo == ModoBusqueda.ENTRADA || modo == ModoBusqueda.SALIDA_GAFETE) {
            BotonDiscretoBrisas(
                onClick = if (modo == ModoBusqueda.ENTRADA) onEscanearCedula else onEscanearGafete,
            ) {
                Icon(
                    Icons.Default.PhotoCamera,
                    contentDescription = if (modo == ModoBusqueda.ENTRADA) "Escanear documento" else "Escanear gafete",
                    modifier = Modifier.size(32.dp),
                )
            }
        }
    }
}

@Composable
private fun LeyendaBusqueda(modo: ModoBusqueda, texto: String, activos: List<FilaActiva>) {
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
            color = if (mensajeEsError) MaterialTheme.colorScheme.error else ColorExitoBrisas,
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
    activos: List<FilaActiva>,
    resultadosBusqueda: List<ContratistaResumen>,
    verificando: Boolean,
    onElegirActivo: (FilaActiva) -> Unit,
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
private fun ContenidoModoSalidaNombre(activos: List<FilaActiva>, onElegirActivo: (FilaActiva) -> Unit) {
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
    automatico: Boolean,
    onCambiarAutomatico: (Boolean) -> Unit,
    onRegistrarSalidaGafetes: () -> Unit,
) {
    Column(modifier = Modifier.padding(top = 8.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Checkbox(
                checked = automatico,
                onCheckedChange = onCambiarAutomatico,
                enabled = !enviando,
            )
            Text(
                "Automático",
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier.padding(start = 4.dp),
            )
        }

        if (texto.isBlank()) {
            Text(
                "Escriba uno o más números de gafete, separados por coma",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            return
        }

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
            BotonBrisas(
                onClick = onRegistrarSalidaGafetes,
                enabled = !enviando,
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
private fun ListaActivos(activos: List<FilaActiva>, onClick: (FilaActiva) -> Unit) {
    LazyColumn(modifier = Modifier.padding(top = 8.dp)) {
        items(
            activos,
            key = { fila ->
                when (fila) {
                    is FilaActiva.Local -> "local-${fila.activo.registroId}"
                    is FilaActiva.Remota -> "remota-${fila.remoto.uuid}"
                }
            },
        ) { fila ->
            FilaActivo(fila, onClick = { onClick(fila) })
            HorizontalDivider(color = MaterialTheme.colorScheme.outline)
        }
    }
}

@Composable
private fun DialogoConfirmarSalida(
    fila: FilaActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaActiva) -> Unit,
) {
    if (fila == null) return

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Registrar salida") },
        text = {
            Text(
                when (fila) {
                    is FilaActiva.Local -> "${fila.activo.contratistaNombre} · ${fila.activo.cedula} · ${fila.activo.empresaNombre}"
                    is FilaActiva.Remota -> "${fila.remoto.contratistaNombre} · registrado en otro dispositivo de la unidad operativa"
                },
            )
        },
        confirmButton = {
            BotonDiscretoBrisas(onClick = { onConfirmar(fila) }) {
                Text("Confirmar")
            }
        },
        dismissButton = {
            BotonDiscretoBrisas(onClick = onDismiss) {
                Text("Cancelar")
            }
        },
    )
}

@Composable
private fun FilaActivo(fila: FilaActiva, onClick: () -> Unit) {
    when (fila) {
        is FilaActiva.Local -> FilaActivoLocal(fila.activo, onClick)
        is FilaActiva.Remota -> FilaActivoRemota(fila.remoto, onClick)
    }
}

@Composable
private fun FilaActivoLocal(activo: IngresoActivoResumen, onClick: () -> Unit) {
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

/// Ver el doc-comment de [FilaActiva] -- un ingreso abierto por el otro
/// dispositivo del sitio, sin cédula/empresa/gafete propios (esos datos
/// nunca viajan en la caché `ingresos_remotos`, sólo lo mínimo para
/// mostrarlo y poder cerrarlo).
@Composable
private fun FilaActivoRemota(remoto: IngresoRemoto, onClick: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick).padding(vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(remoto.contratistaNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            "Entrada ${textoFechaHora(remoto.horaEntrada)}" +
                (remoto.usuarioEntradaNombre?.let { " ($it)" } ?: ""),
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

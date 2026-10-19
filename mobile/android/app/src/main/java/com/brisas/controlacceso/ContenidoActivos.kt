package com.brisas.controlacceso

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material.icons.filled.Groups
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import uniffi.control_acceso_mobile.ContratistaResumen

// Bloques de la pantalla de Activos: selector de modo, buscador, mensajes,
// contenido por modo y diálogo de salida (sacado de `PantallaActivos.kt`,
// punto M6 de la auditoría móvil: sólo se movió código).

/// Píldoras "Ingreso"/"Salida"/"Gafete" (`FilaPildoras`, ver ControlesBrisas.kt)
/// en vez del `SegmentedButton` gris de Material3 -- mismos tres modos de
/// siempre, mismo doc-comment de [PantallaActivos] para el porqué de cada uno.
@Composable
internal fun SelectorModoBusqueda(modo: ModoBusqueda, onCambiar: (ModoBusqueda) -> Unit) {
    val opciones = listOf(ModoBusqueda.ENTRADA, ModoBusqueda.SALIDA_NOMBRE, ModoBusqueda.SALIDA_GAFETE)
    FilaPildoras(
        opciones = listOf("Ingreso", "Salida", "Gafete"),
        seleccionado = opciones.indexOf(modo),
        onSeleccionar = { onCambiar(opciones[it]) },
    )
}

@Composable
internal fun CampoBusquedaActivos(
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
        modifier = Modifier.fillMaxWidth().padding(top = 6.dp),
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
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.weight(1f).height(AlturaBusquedaBrisas),
        )
        if (modo == ModoBusqueda.ENTRADA || modo == ModoBusqueda.SALIDA_GAFETE) {
            Box(
                modifier = Modifier
                    .size(AlturaBusquedaBrisas)
                    .border(1.dp, colorModo, FormaCampoBrisas)
                    .clip(FormaCampoBrisas)
                    .clickable(onClick = if (modo == ModoBusqueda.ENTRADA) onEscanearCedula else onEscanearGafete),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    Icons.Default.PhotoCamera,
                    contentDescription = if (modo == ModoBusqueda.ENTRADA) "Escanear documento" else "Escanear gafete",
                    tint = colorModo,
                )
            }
        }
    }
}

@Composable
internal fun MensajesEstado(error: String?, mensaje: String?, mensajeEsError: Boolean, verificando: Boolean) {
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
internal fun ContenidoModoEntrada(
    texto: String,
    activos: List<FilaActiva>,
    resultadosBusqueda: List<ContratistaResumen>,
    verificando: Boolean,
    cargando: Boolean,
    onElegirActivo: (FilaActiva) -> Unit,
    onElegirContratista: (ContratistaResumen) -> Unit,
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    if (texto.isBlank()) {
        ListaActivos(
            activos = activos,
            cargando = cargando,
            tituloVacio = "Nadie adentro por ahora",
            detalleVacio = "Cuando alguien ingrese, aparece acá.",
            onClick = onElegirActivo,
            sincronizando = sincronizando,
            onSincronizar = onSincronizar,
        )
        return
    }
    if (resultadosBusqueda.isEmpty() && !verificando && !cargando) {
        Text(
            "Sin resultados",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 12.dp),
        )
    }
    ListaConDesvanecido {
        LazyColumn(
            contentPadding = PaddingValues(top = 5.dp),
            verticalArrangement = Arrangement.spacedBy(5.dp),
        ) {
            items(resultadosBusqueda, key = { it.id }) { contratista ->
                FilaContratista(contratista, onClick = { onElegirContratista(contratista) })
            }
        }
    }
}

/// Filtra la lista de activos por cédula/nombre — vacío no trae nada, es un
/// buscador para acotar, no una lista para recorrer (esa es el modo
/// Entrada). Ver el doc-comment de [PantallaActivos].
@Composable
internal fun ContenidoModoSalidaNombre(
    texto: String,
    activos: List<FilaActiva>,
    cargando: Boolean,
    onElegirActivo: (FilaActiva) -> Unit,
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    ListaActivos(
        activos = activos,
        cargando = cargando,
        tituloVacio = if (texto.isBlank()) "Busque a quien sale" else "Sin resultados",
        detalleVacio = if (texto.isBlank()) "Escriba un nombre o una cédula." else "Pruebe con otro nombre o cédula.",
        onClick = onElegirActivo,
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
    )
}

/// Sólo para el tipeo manual -- uno o más números de gafete separados por
/// coma, con vista previa de a quién le corresponde cada uno antes de un
/// único botón que confirma todos de una vez. El escaneo por cámara
/// (`escanerGafeteSalidaAbierto` en [PantallaActivos]) no pasa por acá:
/// escanear ya es la acción deliberada de sacar un gafete, así que registra
/// la salida al toque, sin botón de confirmar de por medio (pedido
/// explícito del usuario 2026-09-20). Ver el doc-comment de
/// [PantallaActivos].
@Composable
internal fun ContenidoModoSalidaGafete(
    texto: String,
    coincidencias: List<CoincidenciaGafete>,
    enviando: Boolean,
    onRegistrarSalidaGafetes: () -> Unit,
) {
    Column(modifier = Modifier.padding(top = 8.dp)) {
        if (texto.isBlank()) {
            Text(
                "Escriba uno o más números de gafete, separados por coma",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            return
        }

        coincidencias.forEach { coincidencia ->
            val filaCoincidente = coincidencia.fila
            Text(
                "Gafete ${coincidencia.numero} · " +
                    (
                        filaCoincidente?.let { fila ->
                            listOfNotNull(fila.contratistaNombre, fila.empresaNombre).joinToString(" · ")
                        } ?: "Sin ingreso activo"
                    ),
                style = MaterialTheme.typography.bodyMedium,
                color = if (filaCoincidente != null) {
                    MaterialTheme.colorScheme.onSurface
                } else {
                    MaterialTheme.colorScheme.error
                },
                modifier = Modifier.padding(vertical = 4.dp),
            )
        }

        val encontrados = coincidencias.filter { it.fila != null }
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

/// Lo que muestra la lista de activos: la propia lista, marcadores de
/// posición mientras carga por primera vez, o un mensaje si no hay nadie.
private enum class EstadoLista { MARCADORES, VACIA, LISTA }

/// Lista de activos compartida entre Entrada (campo vacío) y Salida:
/// nombre — mismas filas, mismo comportamiento, sólo cambia qué hace
/// `onClick` según quién la use.
///
/// - Las filas se acomodan con una animación cuando alguien entra o sale
///   (`animateItem`), en vez de saltar.
/// - Tocar una fila abre la confirmación de salida: registrar una salida no
///   se hace con un gesto sin confirmar (se probó deslizar con "Deshacer" y
///   se descartó, 2026-10-19).
/// - Tirar hacia abajo sincroniza con la nube, igual que el botón de arriba.
/// - Sin filas: marcadores mientras carga y un mensaje (no un hueco en
///   blanco) cuando de verdad no hay nadie. Los marcadores sólo salen si no
///   hay nada que mostrar: recargar con la lista llena no la tapa.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ListaActivos(
    activos: List<FilaActiva>,
    cargando: Boolean,
    tituloVacio: String,
    detalleVacio: String,
    onClick: (FilaActiva) -> Unit,
    sincronizando: Boolean,
    onSincronizar: () -> Unit,
) {
    val estado = when {
        activos.isNotEmpty() -> EstadoLista.LISTA
        cargando -> EstadoLista.MARCADORES
        else -> EstadoLista.VACIA
    }
    PullToRefreshBox(
        isRefreshing = sincronizando,
        onRefresh = onSincronizar,
        modifier = Modifier.fillMaxSize(),
    ) {
        AnimatedContent(
            targetState = estado,
            modifier = Modifier.fillMaxSize(),
            transitionSpec = {
                (fadeIn(tween(MS_FUNDIDO_ENTRADA)) togetherWith fadeOut(tween(MS_SALIDA))) using null
            },
            label = "estadoListaActivos",
        ) { mostrado ->
            when (mostrado) {
                EstadoLista.LISTA -> ListaConDesvanecido {
                    LazyColumn(
                        contentPadding = PaddingValues(top = 5.dp),
                        verticalArrangement = Arrangement.spacedBy(5.dp),
                    ) {
                        items(activos, key = { fila -> fila.clave }) { fila ->
                            Box(modifier = Modifier.animateItem()) {
                                FilaActivo(fila, onClick = { onClick(fila) })
                            }
                        }
                    }
                }
                EstadoLista.MARCADORES -> FilasMarcadoras()
                EstadoLista.VACIA -> EstadoVacioLista(tituloVacio, detalleVacio)
            }
        }
    }
}

/// Cuatro tarjetas grises con la forma de una fila, quietas (sin brillo que
/// recorra la pantalla: eso sería una animación infinita sólo para esperar).
@Composable
private fun FilasMarcadoras() {
    val descripcion = "Cargando"
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 5.dp)
            .semantics { contentDescription = descripcion },
        verticalArrangement = Arrangement.spacedBy(5.dp),
    ) {
        repeat(4) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(MaterialTheme.shapes.medium)
                    .background(MaterialTheme.colorScheme.surface)
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                BarraMarcadora(ancho = 0.5f, alto = 14.dp)
                BarraMarcadora(ancho = 0.8f, alto = 10.dp)
                BarraMarcadora(ancho = 0.6f, alto = 10.dp)
            }
        }
    }
}

@Composable
private fun BarraMarcadora(ancho: Float, alto: androidx.compose.ui.unit.Dp) {
    Box(
        modifier = Modifier
            .fillMaxWidth(ancho)
            .height(alto)
            .clip(MaterialTheme.shapes.small)
            .background(MaterialTheme.colorScheme.surfaceVariant),
    )
}

/// Con `verticalScroll` aunque no haya nada que desplazar: sin un hijo
/// desplazable, tirar hacia abajo para sincronizar no responde.
@Composable
private fun EstadoVacioLista(titulo: String, detalle: String) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(top = 48.dp, start = 24.dp, end = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Box(
            modifier = Modifier
                .size(64.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.primaryContainer),
            contentAlignment = Alignment.Center,
        ) {
            Icon(Icons.Default.Groups, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
        }
        Text(titulo, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        Text(
            detalle,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
}

@Composable
internal fun DialogoConfirmarSalida(
    fila: FilaActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaActiva) -> Unit,
) {
    if (fila == null) return
    DialogoRegistrarSalida(
        detalle = when (fila) {
            is FilaActiva.Local -> "${fila.activo.contratistaNombre} · ${fila.activo.cedula} · ${fila.activo.empresaNombre}"
            is FilaActiva.Remota -> "${fila.remoto.contratistaNombre} · $TEXTO_OTRO_DISPOSITIVO"
        },
        onDismiss = onDismiss,
        onConfirmar = { onConfirmar(fila) },
    )
}

internal const val TEXTO_OTRO_DISPOSITIVO = "registrado en otro dispositivo de la unidad operativa"

/// Modal "Registrar salida" a mano en vez de `AlertDialog` -- el mockup pide
/// un layout que `AlertDialog` no ofrece (icono circular arriba, botón
/// principal de ancho completo, "Cancelar" como link chico debajo, todo
/// centrado) en vez de los dos botones lado a lado de siempre. Lo comparten
/// contratistas y visitas.
@Composable
internal fun DialogoRegistrarSalida(
    detalle: String,
    onDismiss: () -> Unit,
    onConfirmar: () -> Unit,
) {
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
                "Registrar salida",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(top = 16.dp),
            )
            Text(
                detalle,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
            BotonBrisas(
                onClick = onConfirmar,
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

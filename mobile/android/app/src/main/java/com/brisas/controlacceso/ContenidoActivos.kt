package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material.icons.filled.PersonSearch
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.ContratistaResumen

// Bloques de la pantalla de Activos: selector de modo, buscador, mensajes,
// contenido por modo y la ficha de salida (sacado de `PantallaActivos.kt`,
// punto M6 de la auditoría móvil).

/// Píldoras "Ingreso"/"Salida"/"Gafete" (`FilaPildoras`, ver ControlesBrisas.kt).
/// Son los mismos tres modos de siempre (ver [PantallaActivos]) y, dentro del
/// buscador de abajo, hacen de barra de alcances como en iOS: se puede
/// cambiar de modo sin cerrar la búsqueda.
@Composable
internal fun SelectorModoBusqueda(modo: ModoBusqueda, onCambiar: (ModoBusqueda) -> Unit) {
    val opciones = listOf(ModoBusqueda.ENTRADA, ModoBusqueda.SALIDA_NOMBRE, ModoBusqueda.SALIDA_GAFETE)
    FilaPildoras(
        opciones = listOf("Ingreso", "Salida", "Gafete"),
        seleccionado = opciones.indexOf(modo),
        onSeleccionar = { onCambiar(opciones[it]) },
    )
}

/// Lo que dice la píldora y el campo de búsqueda de abajo en cada modo: el
/// mismo buscador cambia de sentido.
internal fun ayudaBusquedaActivos(modo: ModoBusqueda): String = when (modo) {
    ModoBusqueda.ENTRADA -> "Nombre o cédula para ingresar"
    ModoBusqueda.SALIDA_NOMBRE -> "Nombre o cédula de quien sale"
    ModoBusqueda.SALIDA_GAFETE -> "Gafetes: 2, 25, 85"
}

/// Teclado numérico para los gafetes; el de siempre para nombres y cédulas.
internal fun tecladoBusquedaActivos(modo: ModoBusqueda): KeyboardOptions =
    if (modo == ModoBusqueda.SALIDA_GAFETE) KeyboardOptions(keyboardType = KeyboardType.Number) else KeyboardOptions.Default

/// Lo que hace el botón redondo de abajo en Activos (ver `AccionInferior.kt`):
/// en Ingreso escanea el documento para entrar (lo que más se hace en la
/// portería); en Salida y en Gafete, el gafete de quien sale. Buscar ya no
/// pasa por acá: es la píldora de búsqueda, igual en los tres modos.
@Composable
internal fun AccionActivos(
    modo: ModoBusqueda,
    onEscanearCedula: () -> Unit,
    onEscanearGafete: () -> Unit,
) {
    when (modo) {
        ModoBusqueda.ENTRADA -> AccionPrincipal(Icons.Default.PhotoCamera, "Escanear documento", onEscanearCedula)
        ModoBusqueda.SALIDA_NOMBRE, ModoBusqueda.SALIDA_GAFETE ->
            AccionPrincipal(Icons.Default.PhotoCamera, "Escanear gafete", onEscanearGafete)
    }
}

/// Fallas de consulta, el resultado de una salida por gafetes y el
/// "Verificando…" de un ingreso, como franjas con ícono ([FranjaEstado]).
@Composable
internal fun MensajesEstado(error: String?, mensaje: String?, mensajeEsError: Boolean, verificando: Boolean) {
    if (error != null) {
        FranjaEstado(error, TipoEstado.ERROR, modifier = Modifier.padding(top = 8.dp))
    }
    if (mensaje != null) {
        FranjaEstado(
            mensaje,
            if (mensajeEsError) TipoEstado.ERROR else TipoEstado.EXITO,
            modifier = Modifier.padding(top = 8.dp),
            vibrar = true,
        )
    }
    if (verificando) {
        FranjaEstado("Verificando…", TipoEstado.CARGANDO, modifier = Modifier.padding(top = 8.dp))
    }
}

/// La lista de quién está adentro (tocar = salida), la de Ingreso y la de
/// Salida fuera del buscador.
@Composable
internal fun ContenidoListaActivos(
    activos: List<FilaActiva>,
    cargando: Boolean,
    onElegirActivo: (FilaActiva) -> Unit,
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    ListaActivos(
        activos = activos,
        cargando = cargando,
        tituloVacio = "Nadie adentro por ahora",
        detalleVacio = "Cuando alguien ingrese, aparece aquí.",
        onClick = onElegirActivo,
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
    )
}

/// Resultados del buscador en modo Ingreso: el catálogo completo de
/// contratistas (tocar = arrancar el flujo de entrada). Sin texto, qué
/// escribir.
@Composable
internal fun ResultadosIngreso(
    texto: String,
    resultadosBusqueda: List<ContratistaResumen>,
    cargando: Boolean,
    onElegirContratista: (ContratistaResumen) -> Unit,
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    if (texto.isBlank()) {
        EstadoVacio("Busque a quien ingresa", "Escriba un nombre o una cédula.", Icons.Default.PersonSearch)
        return
    }
    ListaConEstados(
        filas = resultadosBusqueda,
        clave = { it.id },
        cargando = cargando,
        tituloVacio = "Sin resultados",
        detalleVacio = "Pruebe con otro nombre o cédula.",
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
        contentPadding = PADDING_RESULTADOS,
        iconoVacio = Icons.Default.PersonSearch,
    ) { contratista ->
        FilaContratista(contratista, onClick = { onElegirContratista(contratista) })
    }
}

/// Resultados del buscador en modo Salida: quién está adentro, acotado por
/// cédula o nombre (sin texto, todos).
@Composable
internal fun ResultadosSalida(
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
        tituloVacio = if (texto.isBlank()) "Nadie adentro por ahora" else "Sin resultados",
        detalleVacio = if (texto.isBlank()) "Cuando alguien ingrese, aparece aquí." else "Pruebe con otro nombre o cédula.",
        onClick = onElegirActivo,
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
        contentPadding = PADDING_RESULTADOS,
    )
}

/// Modo Gafete fuera del buscador: qué hacer (escribir los números abajo o
/// escanear el gafete con el botón redondo).
@Composable
internal fun ContenidoModoSalidaGafete() {
    EstadoVacio(
        "Salida por gafete",
        "Escriba los números en la búsqueda de abajo, o escanee el gafete con la cámara.",
        IconosLucide.InsigniaConCheck,
    )
}

/// Resultados del buscador en modo Gafete -- uno o más números separados por
/// coma, cada uno con a quién le corresponde, y un único botón que confirma
/// todos de una vez (la vista previa con nombres hace de confirmación, como
/// en escritorio). El escaneo por cámara no pasa por acá: escanear ya es la
/// acción deliberada de sacar un gafete, así que registra la salida al toque
/// (pedido explícito del usuario 2026-09-20).
@Composable
internal fun ResultadosGafete(
    texto: String,
    coincidencias: List<CoincidenciaGafete>,
    enviando: Boolean,
    onRegistrarSalidaGafetes: () -> Unit,
) {
    if (texto.isBlank()) {
        EstadoVacio("Salida por gafete", "Escriba uno o más números, separados por coma.", IconosLucide.InsigniaConCheck)
        return
    }
    val encontrados = coincidencias.filter { it.fila != null }
    Column(modifier = Modifier.fillMaxSize()) {
        LazyColumn(
            modifier = Modifier.weight(1f),
            contentPadding = PADDING_RESULTADOS,
            verticalArrangement = Arrangement.spacedBy(5.dp),
        ) {
            // Sin `key`: el mismo número escrito dos veces ("2, 2") repetiría la clave.
            items(coincidencias) { coincidencia ->
                TarjetaCoincidenciaGafete(coincidencia)
            }
        }
        if (encontrados.isNotEmpty()) {
            BotonBrisas(
                onClick = onRegistrarSalidaGafetes,
                enabled = !enviando,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
            ) {
                Text(if (enviando) "Registrando…" else "Registrar salida (${encontrados.size})")
            }
        }
    }
}

/// Un número de gafete escrito: el número en una insignia del color de la
/// sección y, al lado, a quién le corresponde (o, en rojo, que nadie lo tiene).
@Composable
private fun TarjetaCoincidenciaGafete(coincidencia: CoincidenciaGafete, modifier: Modifier = Modifier) {
    val fila = coincidencia.fila
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp)
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier
                .size(44.dp)
                .clip(RoundedCornerShape(12.dp))
                .background(if (fila != null) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.errorContainer),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                coincidencia.numero.toString(),
                style = MaterialTheme.typography.titleMedium,
                color = if (fila != null) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
            )
        }
        Spacer(Modifier.width(12.dp))
        Column(modifier = Modifier.weight(1f)) {
            if (fila != null) {
                Text(fila.contratistaNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
                fila.empresaNombre?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            } else {
                Text("Sin ingreso activo", style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.error)
                Text(
                    "Nadie adentro tiene este gafete.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/// Resultados del buscador: arriba poco aire, abajo el justo (el campo está
/// pegado debajo y no hay barra que esquivar).
internal val PADDING_RESULTADOS = PaddingValues(top = 8.dp, bottom = 8.dp)

/// Lista de activos compartida: mismas filas, mismo comportamiento, sólo
/// cambia qué hace `onClick` según quién la use. El comportamiento de la
/// lista (animación, estados vacíos, tirar para sincronizar) vive en
/// [ListaConEstados].
@Composable
private fun ListaActivos(
    activos: List<FilaActiva>,
    cargando: Boolean,
    tituloVacio: String,
    detalleVacio: String,
    onClick: (FilaActiva) -> Unit,
    sincronizando: Boolean,
    onSincronizar: () -> Unit,
    // `null` = el de siempre de [ListaConEstados].
    contentPadding: PaddingValues? = null,
) {
    ListaConEstados(
        filas = activos,
        clave = { fila -> fila.clave },
        cargando = cargando,
        tituloVacio = tituloVacio,
        detalleVacio = detalleVacio,
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
        contentPadding = contentPadding ?: PaddingValues(top = 5.dp, bottom = EspacioPieLista),
    ) { fila ->
        FilaActivo(fila, onClick = { onClick(fila) })
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
        nombre = fila.contratistaNombre,
        datos = when (fila) {
            is FilaActiva.Local -> datosIngreso(
                cedula = fila.activo.cedula,
                afiliacion = "Empresa" to fila.activo.empresaNombre,
                gafete = fila.activo.gafeteNumero,
                fechaHora = fila.activo.fechaHoraIngreso,
                dioIngreso = fila.activo.usuarioIngresoNombre,
                otroEquipo = false,
            )
            is FilaActiva.Remota -> datosIngreso(
                cedula = fila.remoto.contratistaCedula,
                afiliacion = "Empresa" to fila.remoto.empresaNombre,
                gafete = fila.remoto.gafeteNumero,
                fechaHora = fila.remoto.horaEntrada,
                dioIngreso = fila.remoto.usuarioEntradaNombre,
                otroEquipo = true,
            )
        },
        onDismiss = onDismiss,
        onConfirmar = { onConfirmar(fila) },
    )
}

internal const val TEXTO_OTRO_DISPOSITIVO = "registrado en otro dispositivo de la unidad operativa"

/// Un renglón de la ficha de salida: la etiqueta a la izquierda y el valor a
/// la derecha; [destacado] lo pinta en el color de la sección (el gafete, que
/// es lo que el guardia confirma contra lo que la persona trae puesto).
internal data class DatoFicha(val etiqueta: String, val valor: String, val destacado: Boolean = false)

/// Los renglones de la ficha de quien está adentro, iguales para
/// contratistas, proveedores y correo. Lo que falta (un ingreso remoto viejo
/// sin cédula, por ejemplo) no se muestra.
internal fun datosIngreso(
    cedula: String?,
    afiliacion: Pair<String, String?>,
    gafete: Long?,
    fechaHora: String,
    dioIngreso: String?,
    otroEquipo: Boolean,
    placa: String? = null,
    hora: (String) -> String = ::textoFechaHora,
): List<DatoFicha> = listOfNotNull(
    cedula?.takeIf { it.isNotBlank() }?.let { DatoFicha("Cédula", it) },
    afiliacion.second?.takeIf { it.isNotBlank() }?.let { DatoFicha(afiliacion.first, it) },
    placa?.takeIf { it.isNotBlank() }?.let { DatoFicha("Placa", it) },
    DatoFicha("Gafete", gafete?.toString() ?: "Sin gafete", destacado = true),
    DatoFicha("Ingresó", hora(fechaHora)),
    dioIngreso?.takeIf { it.isNotBlank() }?.let { DatoFicha("Dio ingreso", it) },
    if (otroEquipo) DatoFicha("Equipo", "Otro dispositivo de la unidad") else null,
)

/// "Registrar salida" (o "Registrar devolución" en KOF) como ficha que sube
/// desde abajo: el nombre grande y debajo los datos en un bloque agrupado
/// (como una ficha de Contactos en iOS), y el botón principal al alcance del
/// pulgar. Antes era una sola línea "nombre · cédula · empresa" difícil de
/// leer de un vistazo. Lo comparten contratistas, proveedores, correo,
/// visitas y KOF.
///
/// Al confirmar, el panel baja primero y recién entonces avisa: si el que
/// llama lo quitara de la composición en el acto, desaparecería de golpe.
/// `confirmado` evita que un segundo toque registre dos veces mientras baja.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun DialogoRegistrarSalida(
    nombre: String,
    datos: List<DatoFicha>,
    onDismiss: () -> Unit,
    onConfirmar: () -> Unit,
    accion: String = "Registrar salida",
) {
    val estado = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val alcance = rememberCoroutineScope()
    var confirmado by remember { mutableStateOf(false) }
    fun cerrarY(accionAlCerrar: () -> Unit) {
        alcance.launch { estado.hide() }.invokeOnCompletion { accionAlCerrar() }
    }
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = estado,
        containerColor = MaterialTheme.colorScheme.background,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 20.dp, end = 20.dp, bottom = 24.dp),
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
                nombre,
                style = MaterialTheme.typography.titleLarge,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(top = 12.dp),
            )
            if (datos.isNotEmpty()) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 16.dp)
                        .clip(MaterialTheme.shapes.medium)
                        .background(MaterialTheme.colorScheme.surface),
                ) {
                    datos.forEachIndexed { indice, dato ->
                        if (indice > 0) {
                            HorizontalDivider(
                                modifier = Modifier.padding(start = 16.dp),
                                color = MaterialTheme.colorScheme.outline.copy(alpha = 0.2f),
                            )
                        }
                        Row(
                            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 11.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                dato.etiqueta,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Spacer(Modifier.width(16.dp))
                            Text(
                                dato.valor,
                                style = MaterialTheme.typography.bodyMedium,
                                color = if (dato.destacado) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                                fontWeight = if (dato.destacado) FontWeight.Bold else FontWeight.Medium,
                                textAlign = TextAlign.End,
                                modifier = Modifier.weight(1f),
                            )
                        }
                    }
                }
            }
            BotonBrisas(
                onClick = {
                    if (!confirmado) {
                        confirmado = true
                        cerrarY(onConfirmar)
                    }
                },
                modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
            ) {
                Text(accion)
            }
            BotonDiscretoBrisas(onClick = { cerrarY(onDismiss) }, modifier = Modifier.padding(top = 4.dp)) {
                Text("Cancelar")
            }
        }
    }
}

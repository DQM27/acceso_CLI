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
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.ExperimentalMaterial3Api
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

/// Píldoras "Persona"/"Gafete" (`FilaPildoras`, ver ControlesBrisas.kt): los
/// dos modos de [PantallaActivos]. Dentro del buscador de abajo hacen de
/// barra de alcances como en iOS: se puede cambiar de modo sin cerrar la
/// búsqueda. Antes eran tres (Ingreso, Salida y Gafete); "Salida" se fue
/// porque elegir a alguien que ya está adentro abre su salida.
@Composable
internal fun SelectorModoBusqueda(modo: ModoBusqueda, onCambiar: (ModoBusqueda) -> Unit) {
    val opciones = listOf(ModoBusqueda.ENTRADA, ModoBusqueda.SALIDA_GAFETE)
    FilaPildoras(
        opciones = listOf("Persona", "Gafete"),
        seleccionado = opciones.indexOf(modo),
        onSeleccionar = { onCambiar(opciones[it]) },
    )
}

/// Lo que dice la píldora y el campo de búsqueda de abajo en cada modo: el
/// mismo buscador cambia de sentido.
internal fun ayudaBusquedaActivos(modo: ModoBusqueda): String = when (modo) {
    ModoBusqueda.ENTRADA -> "Nombre o cédula"
    ModoBusqueda.SALIDA_GAFETE -> "Gafetes: 2, 25, 85"
}

/// Teclado numérico para los gafetes; el de siempre para nombres y cédulas.
internal fun tecladoBusquedaActivos(modo: ModoBusqueda): KeyboardOptions =
    if (modo == ModoBusqueda.SALIDA_GAFETE) KeyboardOptions(keyboardType = KeyboardType.Number) else KeyboardOptions.Default

/// Lo que hace el botón redondo de abajo en Activos (ver `AccionInferior.kt`):
/// en Persona escanea el documento (para entrar o, si ya está adentro, para
/// salir: lo decide [ActivosViewModel.elegir]); en Gafete, el gafete de
/// quien sale. Buscar ya no pasa por acá: es la píldora de búsqueda.
@Composable
internal fun AccionActivos(
    modo: ModoBusqueda,
    onEscanearCedula: () -> Unit,
    onEscanearGafete: () -> Unit,
) {
    when (modo) {
        ModoBusqueda.ENTRADA -> AccionPrincipal(Icons.Default.PhotoCamera, "Escanear documento", onEscanearCedula)
        ModoBusqueda.SALIDA_GAFETE -> AccionPrincipal(Icons.Default.PhotoCamera, "Escanear gafete", onEscanearGafete)
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

/// Resultados del buscador en modo Persona: el catálogo completo de
/// contratistas (tocar = entrar, o salir si ya está adentro). Sin texto, qué
/// escribir. Sin resultados, registrarlo ahí mismo: antes había que volver,
/// buscar el "+" de arriba y, después de guardarlo, volver a buscarlo para
/// darle el ingreso.
@Composable
internal fun ResultadosIngreso(
    texto: String,
    resultadosBusqueda: List<ContratistaResumen>,
    cargando: Boolean,
    onElegirContratista: (ContratistaResumen) -> Unit,
    onNuevoContratista: (String) -> Unit,
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    if (texto.isBlank()) {
        EstadoVacio("Busque a quien entra o sale", "Escriba un nombre o una cédula.", Icons.Default.PersonSearch)
        return
    }
    if (resultadosBusqueda.isEmpty() && !cargando) {
        Column(modifier = Modifier.fillMaxSize().padding(top = 8.dp)) {
            FilaCrear(texto, onClick = { onNuevoContratista(texto) }, etiqueta = "Registrar nuevo contratista")
            EstadoVacio(
                "No está registrado",
                "Regístrelo con su carnet PRAIND y enseguida se le da el ingreso.",
                Icons.Default.PersonSearch,
            )
        }
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
    // Se escaneó (o se eligió en la búsqueda) a alguien que ya estaba
    // adentro: la hoja lo dice con el triángulo ámbar.
    yaAdentro: Boolean = false,
) {
    if (fila == null) return
    DialogoRegistrarSalida(
        nombre = fila.contratistaNombre,
        detalle = textoGafeteSalida(
            when (fila) {
                is FilaActiva.Local -> fila.activo.gafeteNumero
                is FilaActiva.Remota -> fila.remoto.gafeteNumero
            },
        ),
        onDismiss = onDismiss,
        onConfirmar = { onConfirmar(fila) },
        aviso = if (yaAdentro) AVISO_SALIDA_YA_ADENTRO else null,
    )
}

/// Lo que dice la hoja de salida cuando se escaneó a alguien que ya estaba
/// adentro.
internal const val AVISO_SALIDA_YA_ADENTRO = "Persona con ingreso activo"

internal const val TEXTO_OTRO_DISPOSITIVO = "registrado en otro dispositivo de la unidad operativa"

/// Lo único que acompaña al nombre en la confirmación de salida: el gafete,
/// que es lo que el guardia coteja contra lo que la persona trae puesto.
internal fun textoGafeteSalida(gafete: Long?): String = if (gafete != null) "Gafete $gafete" else "Sin gafete"

/// "Registrar salida" (o "Registrar devolución" en KOF) como panel que sube
/// desde abajo, a la manera de una hoja de acción de iOS: el nombre grande,
/// debajo sólo el gafete ([detalle]) y el botón al alcance del pulgar. El
/// resto de los datos ya está en la tarjeta que se tocó (pedido del usuario
/// 2026-10-25: la ficha con cédula, empresa, hora y quién dio ingreso era
/// demasiado para confirmar una salida). Lo comparten contratistas,
/// proveedores, correo, visitas y KOF.
///
/// Al confirmar, el panel baja primero y recién entonces avisa: si el que
/// llama lo quitara de la composición en el acto, desaparecería de golpe.
/// `confirmado` evita que un segundo toque registre dos veces mientras baja.
///
/// Con [aviso] (se escaneó a alguien que ya estaba adentro, pedido del
/// usuario 2026-10-26): triángulo ámbar en vez del ícono de salida, el aviso
/// en ámbar arriba del nombre y la pregunta antes del botón, para que no se
/// confunda con un ingreso.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun DialogoRegistrarSalida(
    nombre: String,
    detalle: String?,
    onDismiss: () -> Unit,
    onConfirmar: () -> Unit,
    accion: String = "Registrar salida",
    aviso: String? = null,
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
                    .background(
                        if (aviso != null) ColorAdvertenciaBrisas.copy(alpha = 0.16f) else MaterialTheme.colorScheme.primaryContainer,
                    ),
                contentAlignment = Alignment.Center,
            ) {
                if (aviso != null) {
                    Icon(Icons.Default.Warning, contentDescription = null, tint = ColorAdvertenciaBrisas)
                } else {
                    Icon(Icons.AutoMirrored.Filled.Logout, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                }
            }
            if (aviso != null) {
                Text(
                    aviso.uppercase(),
                    style = MaterialTheme.typography.labelLarge,
                    color = ColorTextoAdvertenciaBrisas,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(top = 10.dp),
                )
            }
            Text(
                nombre,
                style = MaterialTheme.typography.titleLarge,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(top = if (aviso != null) 6.dp else 12.dp),
            )
            if (detalle != null) {
                Text(
                    detalle.uppercase(),
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }
            if (aviso != null) {
                Text(
                    "¿Desea registrar su salida?",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(top = 16.dp),
                )
            }
            BotonBrisas(
                onClick = {
                    if (!confirmado) {
                        confirmado = true
                        cerrarY(onConfirmar)
                    }
                },
                modifier = Modifier.fillMaxWidth().padding(top = if (aviso != null) 12.dp else 20.dp),
            ) {
                Text(accion)
            }
            BotonDiscretoBrisas(onClick = { cerrarY(onDismiss) }, modifier = Modifier.padding(top = 4.dp)) {
                Text("Cancelar")
            }
        }
    }
}

package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PersonSearch
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.GafeteSinPortador
import uniffi.control_acceso_mobile.SituacionGafete

// Bloques de la pantalla de Activos: selector de modo, buscador, mensajes,
// contenido por modo y la ficha de salida (sacado de `PantallaActivos.kt`,
// punto M6 de la auditoría móvil).

/// Lo que dice la píldora y el campo de búsqueda de abajo. Un solo buscador
/// para personas y gafetes (2026-10-28: antes eran dos pestañas, "Persona" y
/// "Gafete"); qué se busca lo decide el núcleo
/// (`interpretarBusquedaActivos`) según lo escrito.
internal const val AYUDA_BUSQUEDA_ACTIVOS = "Nombre, cédula o gafete"

/// El botón redondo de abajo en Activos (ver `AccionInferior.kt`): escanea el
/// documento, para entrar o, si ya está adentro, para salir (lo decide
/// [ActivosViewModel.elegir]). El escáner de gafetes se desconectó con las
/// pestañas: en la práctica los gafetes se escriben.
@Composable
internal fun AccionActivos(onEscanearCedula: () -> Unit) {
    AccionPrincipal(Icons.Default.PhotoCamera, "Escanear documento", onEscanearCedula)
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

/// Resultados del buscador para una persona: el catálogo completo de
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
        EstadoVacio("Busque a quien entra o sale", "Escriba un nombre, una cédula o un gafete.", Icons.Default.PersonSearch)
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

/// Resultados del buscador cuando se escribieron gafetes ("25" o "2, 25,
/// 85"): la misma tarjeta de la lista de adentro, con el gafete en su cuadro,
/// pintada de azul de salida; tocarla abre la salida, igual que en la lista
/// (2026-10-28: antes tenía un diseño propio). Con dos o más encontrados,
/// además, un botón saca a todos de una vez (la vista previa con nombres hace
/// de confirmación, como en escritorio). Un gafete que nadie tiene queda en
/// una fila chica en rojo, para que no desaparezca en silencio.
@Composable
internal fun ResultadosGafete(
    coincidencias: List<CoincidenciaGafete>,
    enviando: Boolean,
    onElegir: (FilaActiva) -> Unit,
    onRegistrarSalidaGafetes: () -> Unit,
) {
    val encontrados = coincidencias.filter { it.fila != null }
    val tinte = paletaActual.tinteSalida
    // Sin resaltar: "25" no debe subrayarse dentro de una cédula o un nombre.
    // El gafete coincide exacto y ya se ve en su cuadro.
    CompositionLocalProvider(LocalConsultaResaltada provides "") {
    Column(modifier = Modifier.fillMaxSize()) {
        LazyColumn(
            modifier = Modifier.weight(1f),
            contentPadding = PADDING_RESULTADOS,
            verticalArrangement = Arrangement.spacedBy(5.dp),
        ) {
            // El núcleo ya quitó los números repetidos: cada gafete es su clave.
            items(coincidencias, key = { it.numero }) { coincidencia ->
                val fila = coincidencia.fila
                Box(modifier = Modifier.padding(horizontal = 16.dp)) {
                    if (fila != null) {
                        FilaActivo(fila, onClick = { onElegir(fila) }, tinte = tinte, pista = PISTA_SALIDA)
                    } else {
                        FilaGafeteSinIngreso(coincidencia.numero, coincidencia.sinPortador)
                    }
                }
            }
        }
        if (encontrados.size >= 2) {
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
}

/// Un gafete escrito que nadie adentro tiene puesto, con el porqué que da
/// el núcleo ([GafeteSinPortador]): libre va en gris, porque no es un
/// error; no existe, perdido o de baja, en rojo.
@Composable
private fun FilaGafeteSinIngreso(numero: Int, sinPortador: GafeteSinPortador?) {
    val libre = sinPortador?.situacion == SituacionGafete.LIBRE
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(if (libre) MaterialTheme.colorScheme.surfaceVariant else MaterialTheme.colorScheme.errorContainer)
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            sinPortador?.mensaje ?: "Gafete $numero · nadie adentro",
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.Bold,
            color = if (libre) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.error,
        )
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
/// Al abrir, la flecha entra hacia la izquierda ([IconoPaso]). Al confirmar,
/// el panel baja primero y recién entonces avisa: si el que llama lo quitara
/// de la composición en el acto, desaparecería de golpe. `confirmado` evita
/// que un segundo toque registre dos veces mientras baja.
///
/// Con [aviso] (se escaneó a alguien que ya estaba adentro, pedido del
/// usuario 2026-10-26): señal de tránsito ([SenalAdvertencia]) en vez del
/// ícono de salida, el aviso en ámbar arriba del nombre y la pregunta antes
/// del botón, para que no se confunda con un ingreso.
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
    // Con el teclado del buscador todavía bajando, la hoja espera.
    if (!rememberTecladoCerrado()) return
    val estado = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val alcance = rememberCoroutineScope()
    var confirmado by remember { mutableStateOf(false) }
    val arriba = rememberHojaArriba(estado)
    fun cerrarY(accionAlCerrar: () -> Unit) {
        alcance.launch { estado.hide() }.invokeOnCompletion { accionAlCerrar() }
    }
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = estado,
        containerColor = MaterialTheme.colorScheme.background,
        // Sin el lugar del teclado: no tiene campos.
        contentWindowInsets = { WindowInsets.navigationBars },
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 20.dp, end = 20.dp, bottom = 24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            if (aviso != null) {
                SenalAdvertencia(arriba = arriba)
            } else {
                IconoPaso(Paso.SALIDA, arriba = arriba)
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

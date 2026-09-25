package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.lifecycle.viewmodel.compose.viewModel
import java.time.LocalDate
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.IngresoRemoto
import uniffi.control_acceso_mobile.Nucleo
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
/// (ver mobile/android/arquitectura.md) — este archivo sólo dibuja lo que el
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
            // Siempre continuo -- pedido explícito del usuario 2026-09-20:
            // un toggle aparte para decidir "¿sigo escaneando o no?" es un
            // paso de más, cuando la propia cámara ya tiene un botón para
            // cerrarla (arriba a la derecha) el día que la persona termine.
            // Abrir la cámara ya es la acción deliberada de sacar un gafete
            // -- cada uno que detecta se registra al toque, sin botón de
            // confirmar, y la cámara se queda armada para el siguiente
            // hasta que alguien la cierra a mano.
            continuo = true,
            // Llena el hueco que dejaba el modo continuo: antes, mientras
            // la cámara seguía abierta, el mensaje sólo confirmaba que se
            // LEYÓ el gafete ("Gafete N procesado"), nunca si la salida en
            // verdad se registró -- un gafete sin ingreso activo quedaba en
            // silencio, tapado por la propia cámara. Acá se le pasa el
            // resultado real de la última mutación para que la pantalla de
            // escaneo lo pinte en el momento (verde/rojo), no sólo el
            // guardia que mira la lista de abajo después de cerrarla.
            resultadoUltimoEscaneo = { viewModel.mensaje?.let { it to viewModel.mensajeEsError } },
            onDocumentoDetectado = { documento ->
                if (viewModel.modo != ModoBusqueda.SALIDA_GAFETE) {
                    viewModel.cambiarModo(ModoBusqueda.SALIDA_GAFETE)
                }
                // Es suspend: el escáner no se rearma hasta que la mutación
                // terminó. Así nunca entran dos gafetes en paralelo ni se
                // cancela una salida ya iniciada.
                viewModel.registrarSalidaPorGafeteEscaneado(documento)
            },
            onCerrar = { escanerGafeteSalidaAbierto = false },
        )
        return
    }

    val verificando = viewModel.seleccionIngreso is SeleccionIngreso.Cargando

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        SelectorModoBusqueda(modo = viewModel.modo, onCambiar = { viewModel.cambiarModo(it) })

        CampoBusquedaActivos(
            modo = viewModel.modo,
            texto = viewModel.texto,
            onCambiarTexto = { viewModel.cambiarTexto(it) },
            onEscanearCedula = { escanerAbierto = true },
            onEscanearGafete = { escanerGafeteSalidaAbierto = true },
        )

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
                cargando = viewModel.cargando,
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
        fila = viewModel.seleccionSalida,
        onDismiss = { viewModel.elegirSeleccionSalida(null) },
        onConfirmar = { viewModel.confirmarSalida(it) },
    )
}

/// Píldoras "Ingreso"/"Salida"/"Gafete" (`FilaPildoras`, ver ControlesBrisas.kt)
/// en vez del `SegmentedButton` gris de Material3 -- mismos tres modos de
/// siempre, mismo doc-comment de [PantallaActivos] para el porqué de cada uno.
@Composable
private fun SelectorModoBusqueda(modo: ModoBusqueda, onCambiar: (ModoBusqueda) -> Unit) {
    val opciones = listOf(ModoBusqueda.ENTRADA, ModoBusqueda.SALIDA_NOMBRE, ModoBusqueda.SALIDA_GAFETE)
    FilaPildoras(
        opciones = listOf("Ingreso", "Salida", "Gafete"),
        seleccionado = opciones.indexOf(modo),
        onSeleccionar = { onCambiar(opciones[it]) },
    )
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
    cargando: Boolean,
    onElegirActivo: (FilaActiva) -> Unit,
    onElegirContratista: (ContratistaResumen) -> Unit,
) {
    if (texto.isBlank()) {
        ListaActivos(activos, onClick = onElegirActivo)
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
private fun ContenidoModoSalidaNombre(activos: List<FilaActiva>, onElegirActivo: (FilaActiva) -> Unit) {
    ListaActivos(activos, onClick = onElegirActivo)
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
private fun ContenidoModoSalidaGafete(
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
    ListaConDesvanecido {
        LazyColumn(
            contentPadding = PaddingValues(top = 5.dp),
            verticalArrangement = Arrangement.spacedBy(5.dp),
        ) {
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
            }
        }
    }
}


/// Modal "Registrar salida" a mano en vez de `AlertDialog` -- el mockup pide
/// un layout que `AlertDialog` no ofrece (icono circular arriba, botón
/// principal de ancho completo, "Cancelar" como link chico debajo, todo
/// centrado) en vez de los dos botones lado a lado de siempre.
@Composable
private fun DialogoConfirmarSalida(
    fila: FilaActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaActiva) -> Unit,
) {
    if (fila == null) return

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
                when (fila) {
                    is FilaActiva.Local -> "${fila.activo.contratistaNombre} · ${fila.activo.cedula} · ${fila.activo.empresaNombre}"
                    is FilaActiva.Remota -> "${fila.remoto.contratistaNombre} · registrado en otro dispositivo de la unidad operativa"
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
            BotonBrisas(
                onClick = { onConfirmar(fila) },
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
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(activo.contratistaNombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        // Mayúscula + negrita en toda la línea, gafete en azul -- pedido
        // explícito del usuario 2026-09-20: es un dato importante (lo que
        // el guardia de salida necesita confirmar contra lo que la persona
        // trae puesto), tiene que resaltar más que cédula/empresa. El
        // estado de acceso (antes "Al día"/"PRAIND próximo a vencer"/motivo
        // de denegación) se sacó de acá -- ya se mostró y se aceptó al
        // momento de registrar el ingreso, repetirlo en cada tarjeta activa
        // era ruido, no información nueva.
        Text(
            buildAnnotatedString {
                append("${activo.cedula} · ${activo.empresaNombre} · ".uppercase())
                withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary)) {
                    append(
                        (if (activo.gafeteNumero != null) "Gafete ${activo.gafeteNumero}" else "Sin gafete")
                            .uppercase(),
                    )
                }
            },
            style = MaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Ingresó ${textoFechaHora(activo.fechaHoraIngreso)} · dio ingreso ${activo.usuarioIngresoNombre}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
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
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
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
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
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
            // Mismo criterio que el motivo SIN_ACCESO del texto que Rust
            // ya resuelve para el bloqueo de ingreso (`PreparacionIngreso.mensajeBloqueo`,
            // `mensajes::mensaje_bloqueo_ingreso` en el crate raíz) -- acá
            // esta fila sólo conoce el toggle crudo (`tieneAcceso`), no el
            // resto de `verificar_acceso` (PRAIND, empresa, etc.), así que
            // el único motivo posible en este catálogo es justo ese.
            // Mayúscula y negrita, pedido explícito del usuario
            // 2026-09-20 para que se lea con fuerza.
            Text(
                "ACCESO DENEGADO",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                fontWeight = FontWeight.Bold,
            )
        } else if (contratista.fechaVencimientoPraind?.let { LocalDate.parse(it) < LocalDate.now() } == true) {
            // Independiente del toggle de arriba -- acá sí hay fecha
            // (`ContratistaResumen.fechaVencimientoPraind`), a diferencia
            // del "Acceso denegado" de arriba que no la necesita. Sin fecha
            // en el texto a propósito (pedido explícito del usuario
            // 2026-09-20): sólo "PRAIND VENCIDO", el detalle con la fecha ya
            // aparece en la pantalla de confirmar ingreso.
            Text(
                "PRAIND VENCIDO",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                fontWeight = FontWeight.Bold,
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
        TipoIngreso.IN_HOUSE -> "IN HOUSE"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }


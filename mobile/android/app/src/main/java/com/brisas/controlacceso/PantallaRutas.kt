package com.brisas.controlacceso

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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.DecisionRetornoViaje
import uniffi.control_acceso_mobile.EncargadoRuta
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.Ruta
import uniffi.control_acceso_mobile.SalidaRutaActivaResumen
import uniffi.control_acceso_mobile.TramoRutaResumen
import uniffi.control_acceso_mobile.VehiculoRuta
import uniffi.control_acceso_mobile.ViajeRuta

/// Checklist real del módulo de rutas -- ver
/// `docs/planes-implementados/plan-control-rutas.md`, sección "Rediseño
/// del núcleo de rutas -- documento/tramo/viaje". Redistribución de esta
/// vuelta (2026-09-20), todas a partir del rediseño del núcleo: (1) el
/// paso "Documento de ruta" pasa de un campo a una lista repetible ("+
/// Agregar documento" -- confirmado que todos se declaran juntos, al
/// momento de la salida); (2) gana un campo de fecha manual por documento
/// -- arregla el hallazgo de esta ronda (la fecha ya no defaultea a "hoy"
/// en silencio, ver el doc-comment de [BorradorDocumentoRuta]); (3) si el
/// vehículo elegido ya tiene un viaje abierto hoy, aparece la opción de
/// continuarlo en vez de abrir uno nuevo; (4) el diálogo de retorno pasa
/// de un solo "Confirmar" a las 3 ramas reales de "¿vuelve a salir?".
@Composable
fun PantallaRutas(nucleo: Nucleo) {
    val viewModel: RutasViewModel = viewModel(factory = RutasViewModel.factory(nucleo))

    var salidaParaDecisionRetorno by remember { mutableStateOf<SalidaRutaActivaResumen?>(null) }
    var escanerCarnetKofAbierto by remember { mutableStateOf(false) }
    var escanerVehiculoAbierto by remember { mutableStateOf(false) }
    // Documento que se está escaneando -- `null` cuando el escáner de
    // comprobante está cerrado; a la vez sólo puede haber uno abierto.
    var idDocumentoEscaneando by remember { mutableStateOf<Long?>(null) }

    val idEscaneo = idDocumentoEscaneando
    if (idEscaneo != null) {
        PantallaEscanearComprobanteRuta(
            onComprobanteDetectado = { comprobante ->
                idDocumentoEscaneando = null
                extraerDigitosRuta(comprobante.numeroRuta)?.let {
                    viewModel.usarNumeroRutaEscaneadoDocumento(idEscaneo, it)
                }
                viewModel.fijarSubNumeroDocumento(idEscaneo, comprobante.subNumero)
                viewModel.cambiarNumeroDocumento(idEscaneo, comprobante.numeroDocumento)
                comprobante.fecha?.let { viewModel.cambiarFechaDocumento(idEscaneo, it.aTextoDDMMYYYYRuta()) }
            },
            onCerrar = { idDocumentoEscaneando = null },
        )
        return
    }

    if (escanerCarnetKofAbierto) {
        PantallaEscanearCarnetKof(
            onCarnetDetectado = { carnet ->
                escanerCarnetKofAbierto = false
                // El código de empleado identifica sin ambigüedad -- se
                // prefiere sobre el nombre cuando el carnet trae los dos
                // (mismo criterio que el buscador de contratistas, que
                // resuelve por cédula antes que por nombre cuando ambos
                // vienen del OCR).
                val texto = carnet.codigoEmpleado ?: carnet.nombre
                if (texto != null) viewModel.usarEncargadoEscaneado(texto)
            },
            onCerrar = { escanerCarnetKofAbierto = false },
        )
        return
    }

    if (escanerVehiculoAbierto) {
        PantallaEscanearVehiculoRuta(
            onVehiculoDetectado = { detectado ->
                escanerVehiculoAbierto = false
                // Un solo buscador contra el catálogo ahora -- da igual si
                // el OCR leyó placa o número de unidad, los dos buscan
                // contra el mismo [VehiculoRuta] (ver
                // [RutasViewModel.usarVehiculoEscaneado]).
                viewModel.usarVehiculoEscaneado(detectado.valor)
            },
            onCerrar = { escanerVehiculoAbierto = false },
        )
        return
    }

    val encargadoSeleccionado = viewModel.encargadoSeleccionado
    val paso1Completo = encargadoSeleccionado != null
    val documentos = viewModel.documentos
    val paso2Completo = documentos.isNotEmpty() && documentos.all(::documentoCompleto)
    val vehiculoSeleccionado = viewModel.vehiculoSeleccionado
    val paso3Completo = vehiculoSeleccionado != null
    val hayDocumentoVencido = documentos.any(::documentoVencido)
    val bloqueadoPorFecha = hayDocumentoVencido && !viewModel.tieneCorreo
    val puedeConfirmar =
        paso1Completo && paso2Completo && paso3Completo && !bloqueadoPorFecha && !viewModel.registrando

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        Text(
            "Registrar salida",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(bottom = 10.dp),
        )

        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            PasoEncargado(
                completado = paso1Completo,
                texto = viewModel.textoEncargado,
                onCambiarTexto = viewModel::cambiarTextoEncargado,
                resultados = viewModel.resultadosEncargado,
                onElegir = viewModel::elegirEncargado,
                sinCoincidencias =
                    viewModel.textoEncargado.isNotBlank() &&
                        encargadoSeleccionado == null &&
                        viewModel.resultadosEncargado.isEmpty(),
                onEscanear = { escanerCarnetKofAbierto = true },
            )
            PasoDocumentosRuta(
                completado = paso2Completo,
                documentos = documentos,
                onCambiarTextoRuta = viewModel::cambiarTextoRutaDocumento,
                onElegirRuta = viewModel::elegirRutaDocumento,
                onTocarTipo = viewModel::alternarSubNumeroDocumento,
                onCambiarNumeroDocumento = viewModel::cambiarNumeroDocumento,
                onCambiarFecha = viewModel::cambiarFechaDocumento,
                onEscanear = { idDocumentoEscaneando = it },
                onQuitar = viewModel::quitarDocumento,
                onAgregar = viewModel::agregarDocumento,
                tieneCorreo = viewModel.tieneCorreo,
                onCambiarTieneCorreo = viewModel::alternarTieneCorreo,
            )
            PasoVehiculo(
                completado = paso3Completo,
                texto = viewModel.textoVehiculo,
                onCambiarTexto = viewModel::cambiarTextoVehiculo,
                resultados = viewModel.resultadosVehiculo,
                onElegir = viewModel::elegirVehiculo,
                sinCoincidencias =
                    viewModel.textoVehiculo.isNotBlank() &&
                        vehiculoSeleccionado == null &&
                        viewModel.resultadosVehiculo.isEmpty(),
                onEscanear = { escanerVehiculoAbierto = true },
            )
            viewModel.viajeAbiertoParaVehiculo?.let { viaje ->
                TarjetaContinuarViaje(
                    viaje = viaje,
                    continuar = viewModel.continuarViaje,
                    onCambiar = viewModel::alternarContinuarViaje,
                )
            }
        }

        viewModel.error?.let { mensaje ->
            Text(
                mensaje,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(top = 8.dp),
            )
        }

        BotonBrisas(
            onClick = { viewModel.registrarSalida(onExito = {}) },
            enabled = puedeConfirmar,
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        ) {
            Text("Confirmar salida")
        }

        Text(
            "Rutas activas",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 16.dp, bottom = 6.dp),
        )

        ListaConDesvanecido {
            LazyColumn(
                contentPadding = PaddingValues(top = 5.dp),
                verticalArrangement = Arrangement.spacedBy(5.dp),
            ) {
                items(viewModel.activas, key = { it.id }) { salida ->
                    TarjetaViajeActivo(
                        salida = salida,
                        tramos = viewModel.tramosPorViaje[salida.viajeId] ?: emptyList(),
                        onConfirmarRetorno = { salidaParaDecisionRetorno = salida },
                    )
                }
            }
        }
    }

    DialogoDecisionRetornoRuta(
        salida = salidaParaDecisionRetorno,
        onDismiss = { salidaParaDecisionRetorno = null },
        onElegir = { salida, decision ->
            viewModel.registrarRetorno(salida, decision)
            salidaParaDecisionRetorno = null
        },
    )
}

/// Sólo dígitos -- el comprobante trae el número de ruta con el prefijo
/// impreso "CRR" (ej. `CRR079`, ver [ComprobanteRutaDetectado.numeroRuta]),
/// pero el catálogo real (`Nucleo.buscarRutas`) es puramente numérico. Un
/// texto sin ningún dígito (OCR mal leído) no debería ni intentar la
/// búsqueda -- de ahí que devuelva `null` en vez de una cadena vacía.
private fun extraerDigitosRuta(texto: String): Int? = texto.filter(Char::isDigit).toIntOrNull()

/// Mismo cómputo que la vieja `DocumentoRuta.etiquetaTipo` -- el guardia ve
/// "Principal"/"H2"/"H3"/"H4" en el chip tocable, no un dígito suelto.
private fun etiquetaSubNumero(n: Int): String = if (n <= 1) "Principal" else "H$n"

private fun fechaHoyTextoRuta(): String {
    val hoy = fechaDeHoy()
    return hoy.aTextoDDMMYYYYRuta()
}

/// Un documento está completo cuando tiene ruta elegida (bloqueante,
/// pedido explícito del usuario, 2026-09-15), número de documento y
/// fecha -- ésta última ya no defaultea a "hoy" (ver el doc-comment de
/// [BorradorDocumentoRuta]), así que si el OCR no la trajo el guardia
/// tiene que escribirla a mano antes de poder confirmar.
private fun documentoCompleto(documento: BorradorDocumentoRuta): Boolean =
    documento.rutaSeleccionada != null && documento.numeroDocumento.isNotBlank() && documento.fechaTexto.isNotBlank()

private fun documentoVencido(documento: BorradorDocumentoRuta): Boolean =
    documento.fechaTexto.isNotBlank() && documento.fechaTexto != fechaHoyTextoRuta()

/// Mismo formato que `FechaDocumento.aTextoDDMMYYYY` en
/// `PantallaNuevoContratista.kt` (día-mes-año con guiones) -- cada pantalla
/// dueña de su propia conversión de fecha, sin un util compartido, mismo
/// criterio que ya existe ahí.
private fun FechaDocumento.aTextoDDMMYYYYRuta(): String = "%02d-%02d-%04d".format(dia, mes, anio)

/// Inverso de [aTextoDDMMYYYYRuta] -- si el texto no tiene la forma
/// esperada se devuelve tal cual: Rust igual la rechaza con un error
/// legible que cita el texto original. `internal`, no `private` -- lo usa
/// también [RutasViewModel.registrarSalida] para armar la solicitud.
internal fun textoDDMMYYYYaIsoRuta(texto: String): String {
    val partes = texto.split("-")
    if (partes.size != 3) return texto
    val (dia, mes, anio) = partes
    return "%s-%s-%s".format(anio.padStart(4, '0'), mes.padStart(2, '0'), dia.padStart(2, '0'))
}

@Composable
private fun PasoEncabezado(numero: Int, titulo: String, completado: Boolean) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Box(
            modifier = Modifier
                .size(24.dp)
                .clip(CircleShape)
                .background(if (completado) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceVariant),
            contentAlignment = Alignment.Center,
        ) {
            if (completado) {
                Icon(
                    Icons.Default.Check,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onPrimary,
                    modifier = Modifier.size(16.dp),
                )
            } else {
                Text(
                    "$numero",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        Text(titulo, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
    }
}

@Composable
private fun BotonCamaraCuadrado(onEscanear: () -> Unit) {
    Box(
        modifier = Modifier
            .size(AlturaBusquedaBrisas)
            .border(1.dp, MaterialTheme.colorScheme.primary, FormaCampoBrisas)
            .clip(FormaCampoBrisas)
            .clickable(onClick = onEscanear),
        contentAlignment = Alignment.Center,
    ) {
        Icon(Icons.Default.PhotoCamera, contentDescription = "Escanear", tint = MaterialTheme.colorScheme.primary)
    }
}

/// Paso "Encargado" -- buscador real por nombre o código de empleado
/// (pedido explícito del usuario, 2026-09-15: "que funcione de las dos
/// formas, como ahora funciona contratista"). BLOQUEANTE desde
/// 2026-09-19 (pedido explícito: "más de lo mismo, debe ser un buscador",
/// igual que [PasoVehiculo]) -- el paso no se da por completo sin elegir
/// un [EncargadoRuta] real de la lista.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PasoEncargado(
    completado: Boolean,
    texto: String,
    onCambiarTexto: (String) -> Unit,
    resultados: List<EncargadoRuta>,
    onElegir: (EncargadoRuta) -> Unit,
    sinCoincidencias: Boolean,
    onEscanear: () -> Unit,
) {
    var menuAbierto by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        PasoEncabezado(1, "Encargado de Ruta", completado)
        // Fila propia (sin el encabezado ni el texto de error) -- mismo
        // motivo que en [PasoVehiculo]: el alto variable del texto "no
        // existe" desfasa el botón si queda dentro de la Row centrada.
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            ExposedDropdownMenuBox(
                expanded = menuAbierto && resultados.isNotEmpty(),
                onExpandedChange = { menuAbierto = it },
                modifier = Modifier.weight(1f),
            ) {
                OutlinedTextField(
                    value = texto,
                    onValueChange = {
                        onCambiarTexto(it)
                        menuAbierto = true
                    },
                    placeholder = { Text("Nombre o código de empleado") },
                    singleLine = true,
                    shape = FormaCampoBrisas,
                    colors = ColoresCampoBrisas(),
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(AlturaBusquedaBrisas)
                        .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryEditable),
                )
                DropdownMenu(
                    expanded = menuAbierto && resultados.isNotEmpty(),
                    onDismissRequest = { menuAbierto = false },
                ) {
                    resultados.forEach { encargado ->
                        DropdownMenuItem(
                            text = { Text("${encargado.nombre} · ${encargado.codigoEmpleado}") },
                            onClick = {
                                onElegir(encargado)
                                menuAbierto = false
                            },
                        )
                    }
                }
            }
            BotonCamaraCuadrado(onEscanear)
        }
        if (sinCoincidencias) {
            Text(
                "Ese encargado no existe en el catálogo.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}

/// Sección "Documento(s) de ruta" -- lista repetible, 1+ por solicitud
/// (confirmado explícito del usuario: todos se declaran juntos, al
/// momento de la salida). Una sola tarjeta para toda la sección, con un
/// [BloqueDocumentoRuta] por documento adentro -- mismo criterio que el
/// resto del checklist (una tarjeta por paso), no una tarjeta por
/// documento, para no perder la numeración de pasos 1/2/3.
@Composable
private fun PasoDocumentosRuta(
    completado: Boolean,
    documentos: List<BorradorDocumentoRuta>,
    onCambiarTextoRuta: (Long, String) -> Unit,
    onElegirRuta: (Long, Ruta) -> Unit,
    onTocarTipo: (Long) -> Unit,
    onCambiarNumeroDocumento: (Long, String) -> Unit,
    onCambiarFecha: (Long, String) -> Unit,
    onEscanear: (Long) -> Unit,
    onQuitar: (Long) -> Unit,
    onAgregar: () -> Unit,
    tieneCorreo: Boolean,
    onCambiarTieneCorreo: (Boolean) -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        PasoEncabezado(
            2,
            if (documentos.size > 1) "Documentos de ruta" else "Documento de ruta",
            completado,
        )
        documentos.forEachIndexed { indice, documento ->
            if (indice > 0) HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
            BloqueDocumentoRuta(
                documento = documento,
                mostrarQuitar = documentos.size > 1,
                onCambiarTextoRuta = { onCambiarTextoRuta(documento.id, it) },
                onElegirRuta = { onElegirRuta(documento.id, it) },
                onTocarTipo = { onTocarTipo(documento.id) },
                onCambiarNumeroDocumento = { onCambiarNumeroDocumento(documento.id, it) },
                onCambiarFecha = { onCambiarFecha(documento.id, it) },
                onEscanear = { onEscanear(documento.id) },
                onQuitar = { onQuitar(documento.id) },
            )
            if (documentoVencido(documento)) {
                Text(
                    "Ese documento no es de hoy -- requiere correo de autorización para continuar.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        }
        // Un solo checkbox para toda la solicitud -- mismo criterio que el
        // núcleo (`tiene_correo_autorizacion` aplica a todos los
        // documentos de una salida, no uno por uno).
        if (documentos.any(::documentoVencido)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(checked = tieneCorreo, onCheckedChange = onCambiarTieneCorreo)
                Text("Tengo el correo de autorización", style = MaterialTheme.typography.bodySmall)
            }
        }
        BotonDiscretoBrisas(onClick = onAgregar, modifier = Modifier.padding(top = 4.dp)) {
            Text("+ Agregar documento")
        }
    }
}

/// Un documento dentro de [PasoDocumentosRuta] -- ruta (bloqueante),
/// tipo/sub-número (chip tocable), número de documento y fecha, más su
/// propia cámara (cada documento es un comprobante distinto, puede
/// escanearse por separado). `fechaTexto` es un campo editable a mano a
/// propósito -- el OCR lo llena cuando puede leerlo, pero nunca hay un
/// valor por defecto que lo reemplace en silencio (ver el doc-comment de
/// [BorradorDocumentoRuta]).
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun BloqueDocumentoRuta(
    documento: BorradorDocumentoRuta,
    mostrarQuitar: Boolean,
    onCambiarTextoRuta: (String) -> Unit,
    onElegirRuta: (Ruta) -> Unit,
    onTocarTipo: () -> Unit,
    onCambiarNumeroDocumento: (String) -> Unit,
    onCambiarFecha: (String) -> Unit,
    onEscanear: () -> Unit,
    onQuitar: () -> Unit,
) {
    var menuRutaAbierto by remember { mutableStateOf(false) }
    val rutaSinCoincidencias =
        documento.textoRuta.isNotBlank() && documento.rutaSeleccionada == null && documento.resultadosRuta.isEmpty()

    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                ExposedDropdownMenuBox(
                    expanded = menuRutaAbierto && documento.resultadosRuta.isNotEmpty(),
                    onExpandedChange = { menuRutaAbierto = it },
                ) {
                    OutlinedTextField(
                        value = documento.textoRuta,
                        onValueChange = {
                            onCambiarTextoRuta(it.filter(Char::isDigit))
                            menuRutaAbierto = true
                        },
                        placeholder = { Text("Ruta") },
                        singleLine = true,
                        shape = FormaCampoBrisas,
                        colors = ColoresCampoBrisas(),
                        modifier = Modifier
                            .width(110.dp)
                            .height(AlturaBusquedaBrisas)
                            .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryEditable),
                    )
                    DropdownMenu(
                        expanded = menuRutaAbierto && documento.resultadosRuta.isNotEmpty(),
                        onDismissRequest = { menuRutaAbierto = false },
                    ) {
                        documento.resultadosRuta.forEach { ruta ->
                            DropdownMenuItem(
                                text = { Text("${ruta.numero}") },
                                onClick = {
                                    onElegirRuta(ruta)
                                    menuRutaAbierto = false
                                },
                            )
                        }
                    }
                }
                Text(
                    etiquetaSubNumero(documento.subNumero),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.clickable(onClick = onTocarTipo),
                )
                if (mostrarQuitar) {
                    IconButton(onClick = onQuitar, modifier = Modifier.size(AlturaBusquedaBrisas)) {
                        Icon(
                            Icons.Default.Close,
                            contentDescription = "Quitar documento",
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            if (rutaSinCoincidencias) {
                Text(
                    "Esa ruta no existe en el catálogo.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }
            OutlinedTextField(
                value = documento.numeroDocumento,
                onValueChange = onCambiarNumeroDocumento,
                placeholder = { Text("No. de transporte / documento") },
                singleLine = true,
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
            )
            OutlinedTextField(
                value = documento.fechaTexto,
                onValueChange = onCambiarFecha,
                placeholder = { Text("Fecha del documento (DD-MM-AAAA)") },
                singleLine = true,
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
            )
        }
        BotonCamaraCuadrado(onEscanear)
    }
}

/// Paso "Vehículo" -- un solo buscador contra el catálogo `VehiculoRuta`
/// (placa o número de unidad), no dos campos de texto libre. Antes eran dos
/// `OutlinedTextField` separados donde se podía tipear cualquier cosa; pedido
/// explícito del usuario (2026-09-19): "es un buscador con dos criterios,
/// placa o número de unidad -- no se puede poner lo que uno quiera, sino
/// sólo lo que la tabla proporciona". Mismo criterio BLOQUEANTE que
/// [PasoDocumentosRuta] con el número de ruta -- el paso no se da por
/// completo sin elegir un [VehiculoRuta] real de la lista.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PasoVehiculo(
    completado: Boolean,
    texto: String,
    onCambiarTexto: (String) -> Unit,
    resultados: List<VehiculoRuta>,
    onElegir: (VehiculoRuta) -> Unit,
    sinCoincidencias: Boolean,
    onEscanear: () -> Unit,
) {
    var menuAbierto by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        PasoEncabezado(3, "Vehículo", completado)
        // Fila propia (sin el encabezado ni el texto de error) -- mismo
        // motivo que en [PasoEncargado]/[PasoEmpresaProveedora]: la cámara
        // se centra sólo contra el campo. El texto "no existe" vive fuera de
        // esta Row porque su alto variable, si quedara adentro, recalcula el
        // centrado vertical de la Row entera y desfasa el botón cada vez que
        // aparece/desaparece (bug reportado 2026-09-19).
        //
        // `weight(1f)` va directo en el [ExposedDropdownMenuBox], no en una
        // `Column` que lo envuelva -- una `Column` intermedia rompe el
        // cálculo de alto que hace Material3 para el menú desplegable y deja
        // un espacio en blanco reservado al enfocar el campo, aun con la
        // lista de resultados vacía (bug reportado 2026-09-19, mismo
        // criterio ya usado en [PasoEncargado]).
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            ExposedDropdownMenuBox(
                expanded = menuAbierto && resultados.isNotEmpty(),
                onExpandedChange = { menuAbierto = it },
                modifier = Modifier.weight(1f),
            ) {
                OutlinedTextField(
                    value = texto,
                    onValueChange = {
                        onCambiarTexto(it)
                        menuAbierto = true
                    },
                    placeholder = { Text("Placa o número de unidad") },
                    singleLine = true,
                    shape = FormaCampoBrisas,
                    colors = ColoresCampoBrisas(),
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(AlturaBusquedaBrisas)
                        .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryEditable),
                )
                DropdownMenu(
                    expanded = menuAbierto && resultados.isNotEmpty(),
                    onDismissRequest = { menuAbierto = false },
                ) {
                    resultados.forEach { vehiculo ->
                        DropdownMenuItem(
                            text = {
                                Text(
                                    vehiculo.numeroUnidad?.let { "${vehiculo.placa} · $it" } ?: vehiculo.placa,
                                )
                            },
                            onClick = {
                                onElegir(vehiculo)
                                menuAbierto = false
                            },
                        )
                    }
                }
            }
            BotonCamaraCuadrado(onEscanear)
        }
        if (sinCoincidencias) {
            Text(
                "Ese vehículo no existe en el catálogo.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}

/// Aparece cuando la unidad elegida en el paso 3 ya tiene un viaje
/// abierto hoy (ver `Nucleo.buscarViajeRutaAbiertoPorPlaca`) -- recarga:
/// volvió y el guardia dijo "sí, misma ruta" al confirmar el retorno
/// anterior. `continuar` nunca se asume solo con que exista el viaje
/// abierto -- el guardia lo activa a mano, y el núcleo igual valida que
/// el encargado/vehículo de esta solicitud coincidan exacto con los del
/// viaje (`RutaServiceError::ViajeNoCoincide` si no).
@Composable
private fun TarjetaContinuarViaje(
    viaje: ViajeRuta,
    continuar: Boolean,
    onCambiar: (Boolean) -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surfaceVariant)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    "Esta unidad ya tiene un viaje abierto hoy",
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                )
                Text(
                    "${viaje.encargadoNombre} · ${viaje.vehiculoPlaca}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Switch(checked = continuar, onCheckedChange = onCambiar)
        }
        if (continuar) {
            Text(
                "Se registrará como continuación del mismo viaje (recarga).",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/// Una tarjeta por unidad, tramos del viaje apilados adentro -- mockup
/// "Opción A" ya aprobado por el usuario. `tramos` trae el historial
/// completo del viaje (ver `Nucleo.listarTramosRutaDeViaje`), no sólo el
/// tramo activo -- casi siempre es 1 solo (un vehículo no puede tener dos
/// tramos abiertos a la vez), pero puede traer 2+ si hubo una recarga
/// ("sí, misma ruta") antes de éste. El botón "Confirmar retorno" actúa
/// siempre sobre `salida` (el tramo realmente abierto), nunca sobre uno
/// ya cerrado del historial.
@Composable
private fun TarjetaViajeActivo(
    salida: SalidaRutaActivaResumen,
    tramos: List<TramoRutaResumen>,
    onConfirmarRetorno: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                "${salida.vehiculoPlaca}" + (salida.vehiculoNumeroUnidad?.let { " ($it)" } ?: ""),
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Medium,
            )
            if (tramos.size > 1) {
                Text(
                    "${tramos.size} tramos",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
            }
        }
        // Ya no trae el documento/número de ruta -- un tramo puede llevar
        // varios ahora, ver el doc-comment de `SalidaRutaActivaResumen`.
        // Mostrarlos queda pendiente (consultar `SalidaRutaDocumentoRepository`).
        Text(
            salida.encargadoNombre,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (tramos.size > 1) {
            HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp))
        }
        // `tramos` puede llegar vacío por un instante mientras
        // `refrescarActivas` todavía está resolviendo el historial --
        // `salida` sola alcanza para no dejar la tarjeta en blanco.
        val filas = tramos.ifEmpty {
            listOf(TramoRutaResumen(id = salida.id, fechaHoraSalida = salida.fechaHoraSalida, fechaHoraRetorno = null))
        }
        filas.forEachIndexed { indice, tramo ->
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                if (filas.size > 1) {
                    Text(
                        "Tramo ${indice + 1}",
                        style = MaterialTheme.typography.bodySmall,
                        fontWeight = FontWeight.Medium,
                    )
                }
                Text(
                    "Salió ${textoFechaHora(tramo.fechaHoraSalida)}" +
                        (tramo.fechaHoraRetorno?.let { " → Volvió ${textoFechaHora(it)}" } ?: " → en ruta"),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        Row(modifier = Modifier.padding(top = 8.dp)) {
            BotonBrisas(onClick = onConfirmarRetorno) {
                Text("Confirmar retorno")
            }
        }
    }
}

/// "¿Vuelve a salir?" es obligatoria en el mismo acto de confirmar el
/// retorno (confirmado explícito del usuario) -- de ahí las 3 ramas acá
/// mismo, en vez de un solo "Confirmar" seguido de una pregunta aparte.
@Composable
private fun DialogoDecisionRetornoRuta(
    salida: SalidaRutaActivaResumen?,
    onDismiss: () -> Unit,
    onElegir: (SalidaRutaActivaResumen, DecisionRetornoViaje) -> Unit,
) {
    if (salida == null) return
    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface, MaterialTheme.shapes.medium)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                "Confirmar retorno",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )
            Text(
                "${salida.encargadoNombre} · ${salida.vehiculoPlaca}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
            Text(
                "¿Vuelve a salir?",
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.padding(top = 20.dp, bottom = 4.dp),
            )
            BotonBrisas(
                onClick = { onElegir(salida, DecisionRetornoViaje.MISMA_RUTA) },
                modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
            ) {
                Text("Sí, misma ruta (recarga)")
            }
            BotonBrisas(
                onClick = { onElegir(salida, DecisionRetornoViaje.OTRA_RUTA_O_TERCERO) },
                modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
            ) {
                Text("Sí, otra ruta / tercero")
            }
            BotonBrisas(
                onClick = { onElegir(salida, DecisionRetornoViaje.NO_VUELVE_A_SALIR) },
                modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
            ) {
                Text("No vuelve a salir")
            }
            BotonDiscretoBrisas(onClick = onDismiss, modifier = Modifier.padding(top = 4.dp)) {
                Text("Cancelar")
            }
        }
    }
}

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
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
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
import uniffi.control_acceso_mobile.EncargadoRuta
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.Ruta
import uniffi.control_acceso_mobile.SalidaRutaActivaResumen
import uniffi.control_acceso_mobile.SolicitudSalidaRuta
import uniffi.control_acceso_mobile.VehiculoRuta

/// Checklist real del módulo de rutas -- ver
/// `docs/planes-implementados/plan-control-rutas.md`. Conectado al núcleo
/// real desde 2026-09-15 (antes era un mock en memoria, ver
/// [RutasViewModel]). Orden de la tarjeta sin cambios respecto al primer
/// corte (encargado → documento de ruta → vehículo) -- fue el desktop el
/// que se reordenó para igualar este orden, no al revés. Tres cambios de
/// fondo en esta vuelta, los tres a pedido explícito del usuario
/// (2026-09-15): (1) "Encargado" ahora es un buscador real por nombre o
/// código (mismo criterio que el buscador de contratistas), ya no
/// descarta el código en silencio; (2) "N.º de ruta" ahora es un
/// buscador BLOQUEANTE contra el catálogo -- el paso no se da por
/// completo sin elegir una [Ruta] real; (3) se quitó el botón
/// "Documento (H)" (agregar sub-documentos H2-H4 a una salida ya
/// registrada) -- no tiene contraparte en el núcleo, así que se retira
/// del checklist por ahora en vez de dejarlo simulado.
@Composable
fun PantallaRutas(nucleo: Nucleo) {
    val viewModel: RutasViewModel = viewModel(factory = RutasViewModel.factory(nucleo))

    var subNumeroTexto by remember { mutableStateOf("1") }
    var numeroDocumento by remember { mutableStateOf("") }
    var fechaDocumentoTexto by remember { mutableStateOf(fechaHoyTextoRuta()) }
    var tieneCorreo by remember { mutableStateOf(false) }

    var salidaParaConfirmarRetorno by remember { mutableStateOf<SalidaRutaActivaResumen?>(null) }
    var escanerRutaAbierto by remember { mutableStateOf(false) }
    var escanerCarnetKofAbierto by remember { mutableStateOf(false) }
    var escanerVehiculoAbierto by remember { mutableStateOf(false) }

    if (escanerRutaAbierto) {
        PantallaEscanearComprobanteRuta(
            onComprobanteDetectado = { comprobante ->
                escanerRutaAbierto = false
                extraerDigitosRuta(comprobante.numeroRuta)?.let(viewModel::usarNumeroRutaEscaneado)
                subNumeroTexto = comprobante.subNumero.toString()
                numeroDocumento = comprobante.numeroDocumento
                comprobante.fecha?.let { fechaDocumentoTexto = it.aTextoDDMMYYYYRuta() }
            },
            onCerrar = { escanerRutaAbierto = false },
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

    val rutaSeleccionada = viewModel.rutaSeleccionada
    val encargadoSeleccionado = viewModel.encargadoSeleccionado
    val paso1Completo = encargadoSeleccionado != null
    val paso2Completo = rutaSeleccionada != null && numeroDocumento.isNotBlank() && fechaDocumentoTexto.isNotBlank()
    val vehiculoSeleccionado = viewModel.vehiculoSeleccionado
    val paso3Completo = vehiculoSeleccionado != null
    val fechaVencida = fechaDocumentoTexto.isNotBlank() && fechaDocumentoTexto != fechaHoyTextoRuta()
    val bloqueadoPorFecha = fechaVencida && !tieneCorreo
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
            PasoDocumentoRuta(
                completado = paso2Completo,
                textoRuta = viewModel.textoRuta,
                onCambiarTextoRuta = viewModel::cambiarTextoRuta,
                resultadosRuta = viewModel.resultadosRuta,
                onElegirRuta = viewModel::elegirRuta,
                rutaSinCoincidencias =
                    viewModel.textoRuta.isNotBlank() && rutaSeleccionada == null && viewModel.resultadosRuta.isEmpty(),
                etiquetaTipo = etiquetaSubNumero(subNumeroTexto),
                onTocarTipo = {
                    subNumeroTexto = ((subNumeroTexto.toIntOrNull() ?: 1) % 4 + 1).toString()
                },
                numeroDocumento = numeroDocumento,
                onCambiarNumeroDocumento = { numeroDocumento = it },
                onEscanear = { escanerRutaAbierto = true },
                fechaVencida = fechaVencida,
                tieneCorreo = tieneCorreo,
                onCambiarTieneCorreo = { tieneCorreo = it },
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
            onClick = {
                val ruta = rutaSeleccionada ?: return@BotonBrisas
                val vehiculo = vehiculoSeleccionado ?: return@BotonBrisas
                val encargado = encargadoSeleccionado ?: return@BotonBrisas
                viewModel.registrarSalida(
                    SolicitudSalidaRuta(
                        vehiculoPlaca = vehiculo.placa,
                        vehiculoNumeroUnidad = vehiculo.numeroUnidad,
                        encargadoNombre = encargado.nombre,
                        encargadoCodigoEmpleado = encargado.codigoEmpleado,
                        numeroRuta = ruta.numero,
                        subNumero = (subNumeroTexto.toLongOrNull() ?: 1L),
                        numeroDocumento = numeroDocumento,
                        fechaDocumento = textoDDMMYYYYaIsoRuta(fechaDocumentoTexto),
                        tieneCorreoAutorizacion = tieneCorreo,
                    ),
                    onExito = {
                        subNumeroTexto = "1"
                        numeroDocumento = ""
                        fechaDocumentoTexto = fechaHoyTextoRuta()
                        tieneCorreo = false
                    },
                )
            },
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
                    FilaSalidaRuta(
                        salida = salida,
                        onConfirmarRetorno = { salidaParaConfirmarRetorno = salida },
                    )
                }
            }
        }
    }

    DialogoConfirmarRetornoRuta(
        salida = salidaParaConfirmarRetorno,
        onDismiss = { salidaParaConfirmarRetorno = null },
        onConfirmar = {
            viewModel.registrarRetorno(it)
            salidaParaConfirmarRetorno = null
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
private fun etiquetaSubNumero(texto: String): String {
    val n = texto.toIntOrNull() ?: 1
    return if (n <= 1) "Principal" else "H$n"
}

private fun fechaHoyTextoRuta(): String {
    val hoy = fechaDeHoy()
    return hoy.aTextoDDMMYYYYRuta()
}

/// Mismo formato que `FechaDocumento.aTextoDDMMYYYY` en
/// `PantallaNuevoContratista.kt` (día-mes-año con guiones) -- cada pantalla
/// dueña de su propia conversión de fecha, sin un util compartido, mismo
/// criterio que ya existe ahí.
private fun FechaDocumento.aTextoDDMMYYYYRuta(): String = "%02d-%02d-%04d".format(dia, mes, anio)

/// Inverso de [aTextoDDMMYYYYRuta] -- si el texto no tiene la forma
/// esperada se devuelve tal cual: Rust igual la rechaza con un error
/// legible que cita el texto original.
private fun textoDDMMYYYYaIsoRuta(texto: String): String {
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

/// Paso "Documento de ruta" -- el número de ruta ahora es un buscador
/// BLOQUEANTE contra el catálogo (pedido explícito del usuario,
/// 2026-09-15: "sin restricción podrías poner la ruta 222 y no existe" —
/// mismo motivo que llevó a crear el catálogo de números de ruta en
/// desktop). El resto de la tarjeta no cambia respecto al primer corte:
/// una sola cámara para toda la tarjeta (ruta, tipo y documento vienen del
/// mismo comprobante impreso).
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PasoDocumentoRuta(
    completado: Boolean,
    textoRuta: String,
    onCambiarTextoRuta: (String) -> Unit,
    resultadosRuta: List<Ruta>,
    onElegirRuta: (Ruta) -> Unit,
    rutaSinCoincidencias: Boolean,
    etiquetaTipo: String,
    onTocarTipo: () -> Unit,
    numeroDocumento: String,
    onCambiarNumeroDocumento: (String) -> Unit,
    onEscanear: () -> Unit,
    fechaVencida: Boolean,
    tieneCorreo: Boolean,
    onCambiarTieneCorreo: (Boolean) -> Unit,
) {
    var menuRutaAbierto by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        PasoEncabezado(2, "Documento de ruta", completado)
        // Fila propia (sin el encabezado) -- mismo motivo que en
        // [PasoEncargado]: la cámara se centra contra los 2 campos (ruta +
        // documento), no contra la tarjeta entera.
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
                        expanded = menuRutaAbierto && resultadosRuta.isNotEmpty(),
                        onExpandedChange = { menuRutaAbierto = it },
                    ) {
                        OutlinedTextField(
                            value = textoRuta,
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
                            expanded = menuRutaAbierto && resultadosRuta.isNotEmpty(),
                            onDismissRequest = { menuRutaAbierto = false },
                        ) {
                            resultadosRuta.forEach { ruta ->
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
                        etiquetaTipo,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.clickable(onClick = onTocarTipo),
                    )
                }
                if (rutaSinCoincidencias) {
                    Text(
                        "Esa ruta no existe en el catálogo.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                OutlinedTextField(
                    value = numeroDocumento,
                    onValueChange = onCambiarNumeroDocumento,
                    placeholder = { Text("No. de transporte / documento") },
                    singleLine = true,
                    shape = FormaCampoBrisas,
                    colors = ColoresCampoBrisas(),
                    modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
                )
                if (fechaVencida) {
                    Text(
                        "El documento no es de hoy -- requiere correo de autorización para continuar.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = tieneCorreo, onCheckedChange = onCambiarTieneCorreo)
                        Text("Tengo el correo de autorización", style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
            BotonCamaraCuadrado(onEscanear)
        }
    }
}

/// Paso "Vehículo" -- un solo buscador contra el catálogo `VehiculoRuta`
/// (placa o número de unidad), no dos campos de texto libre. Antes eran dos
/// `OutlinedTextField` separados donde se podía tipear cualquier cosa; pedido
/// explícito del usuario (2026-09-19): "es un buscador con dos criterios,
/// placa o número de unidad -- no se puede poner lo que uno quiera, sino
/// sólo lo que la tabla proporciona". Mismo criterio BLOQUEANTE que
/// [PasoDocumentoRuta] con el número de ruta -- el paso no se da por
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
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                ExposedDropdownMenuBox(
                    expanded = menuAbierto && resultados.isNotEmpty(),
                    onExpandedChange = { menuAbierto = it },
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

@Composable
private fun FilaSalidaRuta(
    salida: SalidaRutaActivaResumen,
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
        Text(
            "${salida.numeroRuta} · ${etiquetaSubNumero(salida.subNumero.toString())}",
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = FontWeight.Medium,
        )
        Text(
            "${salida.encargadoNombre} · ${salida.vehiculoPlaca}" +
                (salida.vehiculoNumeroUnidad?.let { " ($it)" } ?: ""),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Salió ${textoFechaHora(salida.fechaHoraSalida)}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Row(modifier = Modifier.padding(top = 8.dp)) {
            BotonBrisas(onClick = onConfirmarRetorno) {
                Text("Confirmar retorno")
            }
        }
    }
}

@Composable
private fun DialogoConfirmarRetornoRuta(
    salida: SalidaRutaActivaResumen?,
    onDismiss: () -> Unit,
    onConfirmar: (SalidaRutaActivaResumen) -> Unit,
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
                "${salida.numeroRuta} · ${salida.encargadoNombre} · ${salida.vehiculoPlaca}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
            BotonBrisas(
                onClick = { onConfirmar(salida) },
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

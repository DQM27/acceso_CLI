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
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.Checkbox
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
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

/// MVP mobile del módulo de rutas -- ver
/// `docs/planes-implementados/plan-control-rutas.md`. Orden de trabajo
/// invertido a pedido explícito del usuario (mobile primero, núcleo al
/// final): todo el estado vive en [RutasViewModel], en memoria, sin
/// [uniffi.control_acceso_mobile.Nucleo] todavía -- por eso el "escaneo"
/// de cada paso es simulado (rellena un valor de ejemplo editable), no OCR
/// real. Flujo guiado en una sola tarjeta con 3 pasos (carnet KOF →
/// documento → placa/unidad), en vez de 3 pantallas separadas -- mismo
/// criterio que ya usan los flujos de check-in de patio/yard (menos
/// pantallas, menos toques) y de onboarding KYC (guía + dato editable
/// antes de aceptar el paso, nunca automático a ciegas).
@Composable
fun PantallaRutas() {
    val viewModel: RutasViewModel = viewModel()

    var numeroRuta by remember { mutableStateOf("") }
    var encargado by remember { mutableStateOf("") }
    var numeroDocumento by remember { mutableStateOf("") }
    var subNumeroTexto by remember { mutableStateOf("1") }
    var fechaDocumento by remember { mutableStateOf(fechaHoyTexto()) }
    var tieneCorreo by remember { mutableStateOf(false) }
    var vehiculo by remember { mutableStateOf("") }

    var salidaParaAgregarDocumento by remember { mutableStateOf<SalidaRutaActiva?>(null) }
    var salidaParaConfirmarRetorno by remember { mutableStateOf<SalidaRutaActiva?>(null) }

    val paso1Completo = encargado.isNotBlank()
    val paso2Completo = numeroRuta.isNotBlank() && numeroDocumento.isNotBlank() && fechaDocumento.isNotBlank()
    val paso3Completo = vehiculo.isNotBlank()
    val fechaVencida = fechaDocumento.isNotBlank() && fechaDocumento != fechaHoyTexto()
    val bloqueadoPorFecha = fechaVencida && !tieneCorreo
    val puedeConfirmar = paso1Completo && paso2Completo && paso3Completo && !bloqueadoPorFecha

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        Text(
            "Registrar salida",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(bottom = 10.dp),
        )

        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            PasoChecklistUnCampo(
                numero = 1,
                titulo = "Gafete KOF",
                completado = paso1Completo,
                valor = encargado,
                onCambiar = { encargado = it },
                placeholder = "Nombre del encargado",
                onEscanear = { encargado = "Carlos Balmaceda (demo)" },
            )
            PasoDocumentoRuta(
                completado = paso2Completo,
                numeroRuta = numeroRuta,
                onCambiarNumeroRuta = { numeroRuta = it },
                etiquetaTipo = etiquetaSubNumero(subNumeroTexto),
                onTocarTipo = {
                    subNumeroTexto = ((subNumeroTexto.toIntOrNull() ?: 1) % 4 + 1).toString()
                },
                numeroDocumento = numeroDocumento,
                onCambiarNumeroDocumento = { numeroDocumento = it },
                // Ruta, tipo y documento vienen del mismo comprobante
                // impreso ("CRR079/ 0001") -- un solo escaneo carga los
                // tres, por eso una sola cámara para toda la tarjeta. La
                // fecha tampoco tiene campo propio -- sólo se muestra si
                // difiere de hoy (ver más abajo). El mock simula que todo
                // coincide con hoy.
                onEscanear = {
                    numeroRuta = "CRR079 (demo)"
                    numeroDocumento = "700101452 (demo)"
                    fechaDocumento = fechaHoyTexto()
                },
                fechaVencida = fechaVencida,
                tieneCorreo = tieneCorreo,
                onCambiarTieneCorreo = { tieneCorreo = it },
            )
            PasoChecklistUnCampo(
                numero = 3,
                titulo = "Placa o número de unidad",
                completado = paso3Completo,
                valor = vehiculo,
                onCambiar = { vehiculo = it },
                placeholder = "Placa o número de unidad",
                onEscanear = { vehiculo = "SJB-123 (demo)" },
            )
        }

        BotonBrisas(
            onClick = {
                viewModel.registrarSalida(
                    numeroRuta = numeroRuta,
                    encargado = encargado,
                    vehiculo = vehiculo,
                    documento = DocumentoRuta(
                        subNumero = subNumeroTexto.toIntOrNull() ?: 1,
                        numeroDocumento = numeroDocumento,
                        fechaDocumento = fechaDocumento,
                    ),
                )
                encargado = ""
                numeroDocumento = ""
                subNumeroTexto = "1"
                fechaDocumento = fechaHoyTexto()
                tieneCorreo = false
                vehiculo = ""
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
                        onAgregarDocumento = { salidaParaAgregarDocumento = salida },
                        onConfirmarRetorno = { salidaParaConfirmarRetorno = salida },
                    )
                }
            }
        }
    }

    DialogoAgregarDocumento(
        salida = salidaParaAgregarDocumento,
        onDismiss = { salidaParaAgregarDocumento = null },
        onConfirmar = { documento ->
            salidaParaAgregarDocumento?.let { viewModel.agregarDocumento(it.id, documento) }
            salidaParaAgregarDocumento = null
        },
    )

    DialogoConfirmarRetornoRuta(
        salida = salidaParaConfirmarRetorno,
        onDismiss = { salidaParaConfirmarRetorno = null },
        onConfirmar = {
            viewModel.confirmarRetorno(it.id)
            salidaParaConfirmarRetorno = null
        },
    )
}

/// Mismo cómputo que [DocumentoRuta.etiquetaTipo] -- acá aplica al chip
/// tocable que reemplaza el campo crudo de sub-número (2026-09-15): el
/// guardia ve "Principal"/"H2"/"H3"/"H4", no un dígito suelto.
private fun etiquetaSubNumero(texto: String): String {
    val n = texto.toIntOrNull() ?: 1
    return if (n <= 1) "Principal" else "H$n"
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
private fun PasoChecklist(
    numero: Int,
    titulo: String,
    completado: Boolean,
    contenido: @Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        PasoEncabezado(numero, titulo, completado)
        contenido()
    }
}

/// Paso de un solo campo (carnet KOF, placa) -- a diferencia de
/// [PasoChecklist], acá la cámara no va en su propia fila debajo del
/// encabezado: se ubica al lado, centrada verticalmente contra el bloque
/// encabezado+campo completo -- menos alto de tarjeta para el mismo
/// contenido (pedido explícito 2026-09-15, las tarjetas de un solo campo
/// se veían con mucho aire vertical).
@Composable
private fun PasoChecklistUnCampo(
    numero: Int,
    titulo: String,
    completado: Boolean,
    valor: String,
    onCambiar: (String) -> Unit,
    placeholder: String,
    onEscanear: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            PasoEncabezado(numero, titulo, completado)
            TextField(
                value = valor,
                onValueChange = onCambiar,
                placeholder = { Text(placeholder) },
                singleLine = true,
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
            )
        }
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
}

/// Paso "Documento de ruta" -- mismo criterio que [PasoChecklistUnCampo]:
/// una sola cámara para toda la tarjeta (ruta, tipo y documento vienen
/// del mismo comprobante impreso), centrada contra encabezado+campos
/// juntos, no sólo contra el campo de documento (pedido explícito
/// 2026-09-15, se había quedado sin centrar en el primer corte). El campo
/// de ruta y la etiqueta del tipo (Principal/H2/...) van pegados -- son un
/// solo dato, no dos elementos separados.
@Composable
private fun PasoDocumentoRuta(
    completado: Boolean,
    numeroRuta: String,
    onCambiarNumeroRuta: (String) -> Unit,
    etiquetaTipo: String,
    onTocarTipo: () -> Unit,
    numeroDocumento: String,
    onCambiarNumeroDocumento: (String) -> Unit,
    onEscanear: () -> Unit,
    fechaVencida: Boolean,
    tieneCorreo: Boolean,
    onCambiarTieneCorreo: (Boolean) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            PasoEncabezado(2, "Documento de ruta", completado)
            Row(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                TextField(
                    value = numeroRuta,
                    onValueChange = onCambiarNumeroRuta,
                    placeholder = { Text("Ruta") },
                    singleLine = true,
                    shape = FormaCampoBrisas,
                    colors = ColoresCampoBrisas(),
                    modifier = Modifier.width(140.dp).height(AlturaBusquedaBrisas),
                )
                Text(
                    etiquetaTipo,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.clickable(onClick = onTocarTipo),
                )
            }
            TextField(
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
}

/// Mismo patrón que `CampoBusquedaActivos` en `PantallaActivos.kt` -- campo
/// sin borde + botón de cámara al lado, OCR siempre opcional, nunca
/// obligatorio.
@Composable
private fun CampoConEscaneo(
    valor: String,
    onCambiar: (String) -> Unit,
    placeholder: String,
    onEscanear: () -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextField(
            value = valor,
            onValueChange = onCambiar,
            placeholder = { Text(placeholder) },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.weight(1f).height(AlturaBusquedaBrisas),
        )
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
}

@Composable
private fun FilaSalidaRuta(
    salida: SalidaRutaActiva,
    onAgregarDocumento: () -> Unit,
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
            "${salida.numeroRuta} · ${salida.documentos.joinToString(", ") { it.etiquetaTipo }}",
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = FontWeight.Medium,
        )
        Text(
            "${salida.encargadoNombre} · ${salida.vehiculo}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Salió ${salida.horaSalidaTexto}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Row(modifier = Modifier.padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            BotonDiscretoBrisas(onClick = onAgregarDocumento) {
                Icon(Icons.Default.Add, contentDescription = null, modifier = Modifier.size(18.dp))
                Text("Documento (H)", modifier = Modifier.padding(start = 4.dp))
            }
            BotonBrisas(onClick = onConfirmarRetorno) {
                Text("Confirmar retorno")
            }
        }
    }
}

@Composable
private fun DialogoAgregarDocumento(
    salida: SalidaRutaActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (DocumentoRuta) -> Unit,
) {
    if (salida == null) return
    var subNumeroTexto by remember(salida.id) { mutableStateOf("${(salida.documentos.maxOf { it.subNumero }) + 1}") }
    var numeroDocumento by remember(salida.id) { mutableStateOf("") }
    var fechaDocumento by remember(salida.id) { mutableStateOf(fechaHoyTexto()) }

    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface, MaterialTheme.shapes.medium)
                .padding(24.dp),
        ) {
            Text(
                "Agregar documento adicional",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
            )
            Text(
                "${salida.numeroRuta} · ${salida.encargadoNombre}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(bottom = 12.dp, top = 4.dp),
            )
            TextField(
                value = subNumeroTexto,
                onValueChange = { subNumeroTexto = it.filter(Char::isDigit) },
                placeholder = { Text("Sub-número (H2, H3...)") },
                singleLine = true,
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas).padding(bottom = 8.dp),
            )
            CampoConEscaneo(
                valor = numeroDocumento,
                onCambiar = { numeroDocumento = it },
                placeholder = "No. de transporte / documento",
                onEscanear = { numeroDocumento = "700101453 (demo)" },
            )
            TextField(
                value = fechaDocumento,
                onValueChange = { fechaDocumento = it },
                placeholder = { Text("Fecha (dd.MM.yyyy)") },
                singleLine = true,
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas).padding(top = 8.dp),
            )
            BotonBrisas(
                onClick = {
                    onConfirmar(
                        DocumentoRuta(
                            subNumero = subNumeroTexto.toIntOrNull() ?: 2,
                            numeroDocumento = numeroDocumento,
                            fechaDocumento = fechaDocumento,
                        ),
                    )
                },
                enabled = numeroDocumento.isNotBlank() && fechaDocumento.isNotBlank(),
                modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
            ) {
                Text("Agregar")
            }
            BotonDiscretoBrisas(onClick = onDismiss, modifier = Modifier.padding(top = 4.dp)) {
                Text("Cancelar")
            }
        }
    }
}

@Composable
private fun DialogoConfirmarRetornoRuta(
    salida: SalidaRutaActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (SalidaRutaActiva) -> Unit,
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
                "${salida.numeroRuta} · ${salida.encargadoNombre} · ${salida.vehiculo}",
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

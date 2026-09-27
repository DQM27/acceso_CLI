package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.EncargadoRuta
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.Ruta
import uniffi.control_acceso_mobile.VehiculoRuta

// Pasos del checklist de salida de ruta (sacado de `PantallaRutas.kt`,
// punto M6 de la auditoría móvil: sólo se movió código).

/// Sólo dígitos -- el comprobante trae el número de ruta con el prefijo
/// impreso "CRR" (ej. `CRR079`, ver [ComprobanteRutaDetectado.numeroRuta]),
/// pero el catálogo real (`Nucleo.buscarRutas`) es puramente numérico. Un
/// texto sin ningún dígito (OCR mal leído) no debería ni intentar la
/// búsqueda -- de ahí que devuelva `null` en vez de una cadena vacía.
internal fun extraerDigitosRuta(texto: String): Int? = texto.filter(Char::isDigit).toIntOrNull()

/// Mismo cómputo que la vieja `DocumentoRuta.etiquetaTipo` -- el guardia ve
/// "Principal"/"H2"/"H3"/"H4" en el chip tocable, no un dígito suelto.
internal fun etiquetaSubNumero(texto: String): String {
    val n = texto.toIntOrNull() ?: 1
    return if (n <= 1) "Principal" else "H$n"
}

internal fun fechaHoyTextoRuta(): String {
    val hoy = fechaDeHoy()
    return hoy.aTextoDDMMYYYY()
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
///
/// Resultados como tarjetas tocables (2026-09-20, pedido explícito del
/// usuario), no en un `DropdownMenu` como antes -- mismo motivo que llevó a
/// cambiar el buscador de Gafetes Provisionales: `ExposedDropdownMenuBox`
/// sacaba el foco del campo y cerraba el teclado al borrar texto hasta
/// vaciar la lista de resultados (bug reportado en pruebas reales,
/// 2026-09-20) -- el popup de `DropdownMenu` compite por el foco con el
/// `TextField` en cada recomposición del anclaje.
///
/// La lista de resultados vive FUERA de la tarjeta numerada (blanca), no
/// adentro -- pedido explícito del usuario tras un primer intento que sí la
/// metía adentro: `colorScheme.surface` (tarjeta) y el fondo de cada
/// resultado son el MISMO blanco, así que sin el contraste del fondo de
/// página (`colorScheme.background`) los resultados se veían como un solo
/// bloque estirado en vez de tarjetas separadas -- exactamente el look que
/// ya tenía bien resuelto Gafetes Provisionales (`ListaConDesvanecido` +
/// `LazyColumn` flotando sobre el fondo de página, sin ninguna tarjeta
/// blanca por debajo). `FilaEncargadoRuta` (`ControlesBrisas.kt`) es la
/// misma tarjeta que usa esa pantalla -- una sola fuente de verdad.
@Composable
internal fun PasoEncargado(
    completado: Boolean,
    texto: String,
    onCambiarTexto: (String) -> Unit,
    resultados: List<EncargadoRuta>,
    onElegir: (EncargadoRuta) -> Unit,
    sinCoincidencias: Boolean,
    onEscanear: () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
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
                OutlinedTextField(
                    value = texto,
                    onValueChange = onCambiarTexto,
                    placeholder = { Text("Nombre o código de empleado") },
                    singleLine = true,
                    shape = FormaCampoBrisas,
                    colors = ColoresCampoBrisas(),
                    modifier = Modifier.weight(1f).height(AlturaBusquedaBrisas),
                )
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
        if (resultados.isNotEmpty()) {
            // `Column`, no `LazyColumn` -- a diferencia de Gafetes
            // Provisionales (pantalla propia, sin scroll), este paso vive
            // dentro del `Column.verticalScroll(...)` de todo el formulario
            // de "Registrar salida"; un `LazyColumn` anidado en un
            // `Column` que ya scrollea revienta en runtime (altura máxima
            // infinita). Tope de 6 -- mismo criterio que Contratista/Gafetes
            // Provisionales para no empujar el resto del formulario fuera
            // de pantalla.
            ListaConDesvanecido {
                Column(verticalArrangement = Arrangement.spacedBy(5.dp)) {
                    resultados.take(6).forEach { encargado ->
                        FilaEncargadoRuta(encargado, onClick = { onElegir(encargado) })
                    }
                }
            }
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
internal fun PasoDocumentoRuta(
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
        PasoEncabezado(2, "Documento de Ruta", completado)
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
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
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
internal fun PasoVehiculo(
    completado: Boolean,
    texto: String,
    onCambiarTexto: (String) -> Unit,
    resultados: List<VehiculoRuta>,
    onElegir: (VehiculoRuta) -> Unit,
    sinCoincidencias: Boolean,
    onEscanear: () -> Unit,
    onEnfocado: () -> Unit,
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
                            .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryEditable)
                            .onFocusChanged { if (it.isFocused) onEnfocado() },
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

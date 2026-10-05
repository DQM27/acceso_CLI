package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.DocumentScanner
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.VerificacionVisita
import uniffi.control_acceso_mobile.VisitaAdentro
import uniffi.control_acceso_mobile.VisitaParaEntrar

/// Visitas agendadas en la portería, pensada para la velocidad: un solo
/// campo de cédula (escrita o escaneada) y el núcleo decide qué sigue --
/// entrada si tiene cita hoy, salida si ya está adentro, o un aviso (azul si
/// la cita es para otro día). Sin listas ni historial: eso se consulta en
/// escritorio y en el panel web. Vive bajo la pestaña "Externos"
/// ([PantallaExternos]).
@Composable
fun PantallaVisitas(nucleo: Nucleo) {
    RegistrarPantalla("visitas")
    val viewModel: VisitasViewModel = viewModel(factory = VisitasViewModel.factory(nucleo))
    var escaneando by remember { mutableStateOf(false) }
    var escaneandoPlaca by remember { mutableStateOf(false) }

    if (escaneando) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
            onDocumentoDetectado = { documento ->
                escaneando = false
                // Tal como se leyó: el núcleo la pasa a su forma única
                // (cédula o pasaporte).
                viewModel.verificar(documento.numeroDocumento)
            },
            onCerrar = { escaneando = false },
        )
        return
    }

    if (escaneandoPlaca) {
        PantallaEscanearVehiculoRuta(
            onVehiculoDetectado = { detectado ->
                escaneandoPlaca = false
                viewModel.cambiarPlaca(placaComoSeImprime(detectado.valor))
            },
            onCerrar = { escaneandoPlaca = false },
            mensajeInicial = "Apunte a la placa del vehículo",
            mensajePermiso = "Se necesita permiso de cámara para escanear la placa.",
        )
        return
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .imePadding()
            .padding(horizontal = 16.dp, vertical = 6.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedTextField(
                value = viewModel.cedula,
                onValueChange = viewModel::cambiarCedula,
                placeholder = { Text("Cédula o pasaporte") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(
                    capitalization = KeyboardCapitalization.Characters,
                    keyboardType = KeyboardType.Ascii,
                    imeAction = ImeAction.Search,
                ),
                keyboardActions = KeyboardActions(onSearch = { viewModel.verificar() }),
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.weight(1f),
            )
            BotonCuadrado(Icons.Default.DocumentScanner, "Escanear documento") { escaneando = true }
        }

        if (viewModel.verificacion == null) {
            BotonBrisas(
                onClick = { viewModel.verificar() },
                enabled = viewModel.cedula.isNotBlank() && !viewModel.verificando,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(if (viewModel.verificando) "Verificando…" else "Verificar")
            }
        }

        viewModel.hecho?.let { Confirmacion(it) }

        when (val verificacion = viewModel.verificacion) {
            is VerificacionVisita.Entrada -> Entrada(
                visita = verificacion.visita,
                viewModel = viewModel,
                onEscanearPlaca = { escaneandoPlaca = true },
            )
            is VerificacionVisita.Salida -> Salida(verificacion.visita, viewModel)
            is VerificacionVisita.Aviso -> Aviso(
                mensaje = verificacion.mensaje,
                informativo = verificacion.informativo,
                onOtraCedula = viewModel::limpiar,
            )
            null -> {}
        }

        viewModel.error?.let { mensaje ->
            Text(mensaje, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error)
        }
    }
}

@Composable
private fun Entrada(visita: VisitaParaEntrar, viewModel: VisitasViewModel, onEscanearPlaca: () -> Unit) {
    Tarjeta {
        Text(visita.nombre, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
        visita.empresa?.let { Dato(it) }
        Dato("Visita a ${visita.anfitrion}" + (visita.motivo?.let { " · $it" } ?: ""))
        Dato(vigencia(visita.fechaHasta) + (visita.horaEstimada?.let { " · llega ~$it" } ?: ""))
    }

    FilaPildoras(
        opciones = listOf("Caminando", "Vehículo"),
        seleccionado = if (viewModel.enVehiculo) 1 else 0,
        onSeleccionar = { viewModel.cambiarEnVehiculo(it == 1) },
    )
    if (viewModel.enVehiculo) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            OutlinedTextField(
                value = viewModel.placa,
                onValueChange = viewModel::cambiarPlaca,
                placeholder = { Text("Placa") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters),
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.weight(1f),
            )
            BotonCuadrado(Icons.Default.PhotoCamera, "Escanear placa", onClick = onEscanearPlaca)
        }
    }
    OutlinedTextField(
        value = viewModel.gafete,
        onValueChange = viewModel::cambiarGafete,
        placeholder = { Text("Gafete de visita (vacío = sin gafete)") },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = { viewModel.registrarEntrada() }),
        shape = FormaCampoBrisas,
        colors = ColoresCampoBrisas(),
        modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas + 6.dp),
    )
    BotonBrisas(
        onClick = viewModel::registrarEntrada,
        enabled = !viewModel.registrando && !(viewModel.enVehiculo && viewModel.placa.isBlank()),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(if (viewModel.registrando) "Registrando…" else "Registrar entrada")
    }
}

@Composable
private fun Salida(visita: VisitaAdentro, viewModel: VisitasViewModel) {
    Tarjeta {
        Text(visita.nombre, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
        Dato("Adentro desde ${hora(visita.fechaHoraEntrada)} · visita a ${visita.anfitrion}")
        Dato(
            (visita.gafeteNumero?.let { "Gafete $it" } ?: "Sin gafete") +
                (visita.placa?.let { " · $it" } ?: " · caminando"),
            destacado = true,
        )
    }
    BotonBrisas(
        onClick = viewModel::registrarSalida,
        enabled = !viewModel.registrando,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(if (viewModel.registrando) "Registrando…" else "Registrar salida")
    }
}

/// Azul (`tertiary`) si la cita existe para otro día; rojo si no hay nada
/// que lo deje entrar.
@Composable
private fun Aviso(mensaje: String, informativo: Boolean, onOtraCedula: () -> Unit) {
    val fondo = if (informativo) MaterialTheme.colorScheme.tertiaryContainer else MaterialTheme.colorScheme.errorContainer
    val texto = if (informativo) MaterialTheme.colorScheme.onTertiaryContainer else MaterialTheme.colorScheme.onErrorContainer
    Row(
        modifier = Modifier.fillMaxWidth().background(fondo, MaterialTheme.shapes.medium).padding(16.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Icon(Icons.Default.Info, contentDescription = null, tint = texto)
        Text(mensaje, style = MaterialTheme.typography.bodyLarge, color = texto)
    }
    BotonDiscretoBrisas(onClick = onOtraCedula, modifier = Modifier.fillMaxWidth()) {
        Text("Otra cédula")
    }
}

@Composable
private fun Confirmacion(texto: String) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(Icons.Default.CheckCircle, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
        Text(texto, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.primary)
    }
}

@Composable
private fun Tarjeta(contenido: @Composable ColumnScope.() -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
        content = contenido,
    )
}

@Composable
private fun Dato(texto: String, destacado: Boolean = false) {
    Text(
        texto,
        style = MaterialTheme.typography.bodyMedium,
        fontWeight = if (destacado) FontWeight.SemiBold else FontWeight.Normal,
        color = if (destacado) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun BotonCuadrado(icono: ImageVector, descripcion: String, onClick: () -> Unit) {
    Box(
        modifier = Modifier
            .size(AlturaBusquedaBrisas + 6.dp)
            .border(1.dp, MaterialTheme.colorScheme.primary, FormaCampoBrisas)
            .clip(FormaCampoBrisas)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icono, contentDescription = descripcion, tint = MaterialTheme.colorScheme.primary)
    }
}

private val ZONA_COSTA_RICA: ZoneId = ZoneId.of("America/Costa_Rica")

/// "Sólo hoy" o "Hasta el 07/10" -- `fechaHasta` viene como `AAAA-MM-DD`.
internal fun vigencia(fechaHasta: String, hoy: LocalDate = LocalDate.now(ZONA_COSTA_RICA)): String {
    val hasta = runCatching { LocalDate.parse(fechaHasta) }.getOrNull() ?: return "Vigente hoy"
    return if (!hasta.isAfter(hoy)) "Sólo hoy" else "Hasta el ${hasta.format(DateTimeFormatter.ofPattern("dd/MM"))}"
}

private fun hora(iso: String): String {
    val instante = instanteFechaHora(iso) ?: return "—"
    return ZonedDateTime.ofInstant(instante, ZoneId.systemDefault()).format(DateTimeFormatter.ofPattern("HH:mm"))
}

package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.EmpresaProveedor
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.RegistroIngresoProveedorActivoResumen

/// Control de proveedores -- ver
/// `docs/features-futuras/plan-control-proveedores.md`. Mismo esqueleto
/// autocontenido que [PantallaGafetesProvisionales] (formulario + lista de
/// activos propia, sin integrarse a la lista compartida de "Activos" --
/// ver el doc-comment de [ProveedoresViewModel]), con dos diferencias: acá
/// SÍ hay OCR de cédula (reusando `PantallaEscanearCedula` con el modo
/// `DOCUMENTO_CONTRATISTA` -- el criterio de aceptación del documento es
/// idéntico, sólo cambia a quién se le atribuye después) y SÍ se puede dar
/// de alta una empresa nueva inline si la búsqueda no trae nada.
@Composable
fun PantallaProveedores(nucleo: Nucleo, secretoStore: SecretoDispositivoStore) {
    val viewModel: ProveedoresViewModel =
        viewModel(factory = ProveedoresViewModel.factory(nucleo, secretoStore))
    var gafeteTexto by remember { mutableStateOf("") }
    var escaneando by remember { mutableStateOf(false) }
    var registroParaSalida by remember {
        mutableStateOf<RegistroIngresoProveedorActivoResumen?>(null)
    }

    val puedeRegistrar =
        viewModel.cedula.isNotBlank() &&
            viewModel.nombre.isNotBlank() &&
            viewModel.empresaSeleccionada != null &&
            gafeteTexto.isNotBlank() &&
            !viewModel.registrando

    if (escaneando) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
            onDocumentoDetectado = { documento ->
                escaneando = false
                val numero = documento.numeroDocumento.filter(Char::isDigit)
                    .ifBlank { documento.numeroDocumento }
                viewModel.rellenarDesdeDocumento(numero, documento.nombre)
            },
            onCerrar = { escaneando = false },
        )
        return
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp),
    ) {
        Text(
            "Proveedores",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(bottom = 10.dp),
        )

        BotonBrisas(
            onClick = { escaneando = true },
            modifier = Modifier.fillMaxWidth(),
        ) {
            Icon(Icons.Default.PhotoCamera, contentDescription = null, modifier = Modifier.padding(end = 8.dp))
            Text("Escanear cédula")
        }

        TextField(
            value = viewModel.cedula,
            onValueChange = viewModel::cambiarCedula,
            placeholder = { Text("Cédula") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas).padding(top = 12.dp),
        )

        TextField(
            value = viewModel.nombre,
            onValueChange = viewModel::cambiarNombre,
            placeholder = { Text("Nombre") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas).padding(top = 8.dp),
        )

        BuscadorEmpresaProveedora(
            texto = viewModel.textoEmpresa,
            resultados = viewModel.resultadosEmpresa,
            creando = viewModel.creandoEmpresa,
            onCambiarTexto = viewModel::cambiarTextoEmpresa,
            onElegir = viewModel::elegirEmpresa,
            onCrear = viewModel::crearEmpresa,
        )

        TextField(
            value = viewModel.placa,
            onValueChange = viewModel::cambiarPlaca,
            placeholder = { Text("Placa (opcional)") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas).padding(top = 8.dp),
        )

        TextField(
            value = gafeteTexto,
            onValueChange = { gafeteTexto = it.filter(Char::isDigit) },
            placeholder = { Text("Número de gafete") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas).padding(top = 8.dp),
        )

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
                val numero = gafeteTexto.toLongOrNull() ?: return@BotonBrisas
                viewModel.registrarIngreso(numero) { gafeteTexto = "" }
            },
            enabled = puedeRegistrar,
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        ) {
            Text("Registrar ingreso")
        }

        Text(
            "Proveedores activos",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 16.dp, bottom = 6.dp),
        )

        ListaConDesvanecido {
            LazyColumn(
                contentPadding = PaddingValues(top = 5.dp),
                verticalArrangement = Arrangement.spacedBy(5.dp),
            ) {
                items(viewModel.activos, key = { it.id }) { registro ->
                    FilaProveedorActivo(
                        registro = registro,
                        onConfirmarSalida = { registroParaSalida = registro },
                    )
                }
            }
        }
    }

    DialogoConfirmarSalidaProveedor(
        registro = registroParaSalida,
        onDismiss = { registroParaSalida = null },
        onConfirmar = {
            viewModel.registrarSalida(it)
            registroParaSalida = null
        },
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun BuscadorEmpresaProveedora(
    texto: String,
    resultados: List<EmpresaProveedor>,
    creando: Boolean,
    onCambiarTexto: (String) -> Unit,
    onElegir: (EmpresaProveedor) -> Unit,
    onCrear: (String) -> Unit,
) {
    var menuAbierto by remember { mutableStateOf(false) }
    val sinCoincidencias = texto.isNotBlank() && resultados.isEmpty()

    Column(modifier = Modifier.padding(top = 8.dp)) {
        ExposedDropdownMenuBox(
            expanded = menuAbierto && resultados.isNotEmpty(),
            onExpandedChange = { menuAbierto = it },
        ) {
            TextField(
                value = texto,
                onValueChange = {
                    onCambiarTexto(it)
                    menuAbierto = true
                },
                placeholder = { Text("Empresa proveedora") },
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
                resultados.forEach { empresa ->
                    DropdownMenuItem(
                        text = { Text(empresa.nombre) },
                        onClick = {
                            onElegir(empresa)
                            menuAbierto = false
                        },
                    )
                }
            }
        }

        if (sinCoincidencias) {
            BotonDiscretoBrisas(
                onClick = { onCrear(texto) },
                enabled = !creando,
                modifier = Modifier.padding(top = 4.dp),
            ) {
                Text(if (creando) "Creando…" else "Crear empresa \"$texto\"")
            }
        }
    }
}

@Composable
private fun FilaProveedorActivo(
    registro: RegistroIngresoProveedorActivoResumen,
    onConfirmarSalida: () -> Unit,
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
            "Gafete ${registro.gafeteNumero}",
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = FontWeight.Medium,
        )
        Text(
            "${registro.nombre} · ${registro.cedula}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            registro.empresaNombre + (registro.placa?.let { " · $it" } ?: ""),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Ingresó ${textoFechaHora(registro.fechaHoraIngreso)}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Row(modifier = Modifier.padding(top = 8.dp)) {
            BotonBrisas(onClick = onConfirmarSalida) {
                Text("Salida")
            }
        }
    }
}

@Composable
private fun DialogoConfirmarSalidaProveedor(
    registro: RegistroIngresoProveedorActivoResumen?,
    onDismiss: () -> Unit,
    onConfirmar: (RegistroIngresoProveedorActivoResumen) -> Unit,
) {
    if (registro == null) return
    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface, MaterialTheme.shapes.medium)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                "Confirmar salida",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )
            Text(
                "Gafete ${registro.gafeteNumero} · ${registro.nombre}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
            BotonBrisas(
                onClick = { onConfirmar(registro) },
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

package com.brisas.controlacceso

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import java.util.UUID
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.TipoIngreso

/// Mismo formulario que desktop/src/pantallas/FormularioContratista.tsx —
/// sólo alta, no edición (ver docs/plan-app-movil.md). La validación real
/// vuelve a correr en Rust (ContratistaService::crear); lo de acá es sólo
/// feedback inmediato, igual que el esquema de zod del lado desktop.
///
/// Recuperada 2026-09-12 tras haberse sacado el 2026-09-06 (ver
/// ARQUITECTURA.md) -- ahora con una segunda vía de captura además del
/// tipeo manual: escanear el carnet PRAIND del contratista con la misma
/// cámara/OCR que ya usa `PantallaEscanearCedula` para confirmar ingresos.
/// El carnet PRAIND es el único tipo de documento que trae, además de
/// cédula y nombre, la empresa y el vencimiento de la inducción -- por eso
/// es el que de verdad ahorra tipeo acá (ver `LectorDocumentosIdentidad.extraerPraind`).
///
private fun etiquetaTipo(tipo: TipoIngreso): String =
    when (tipo) {
        TipoIngreso.PRAIND -> "PRAIND"
        TipoIngreso.IN_HOUSE -> "IN HOUSE"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }

/// Estado y llamadas al núcleo en [NuevoContratistaViewModel] (punto M1
/// de la auditoría móvil); acá sólo lo visual. Cada apertura del
/// formulario es un intento fresco: la clave del ViewModel se guarda con
/// `rememberSaveable` (sobrevive a una rotación) y cambia cada vez que la
/// pantalla vuelve a entrar en composición.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PantallaNuevoContratista(nucleo: Nucleo, onVolver: () -> Unit) {
    RegistrarPantalla("nuevo_contratista")
    // Mismo destino que el botón "← Volver" visible de abajo -- sin esto,
    // atrás del sistema se escapaba a la Activity en vez de volver a
    // Activos (hallazgo 2026-09-19).
    BackHandler(onBack = onVolver)
    val claveFormulario = rememberSaveable { "nuevo-contratista-${UUID.randomUUID()}" }
    val vm: NuevoContratistaViewModel = viewModel(
        key = claveFormulario,
        factory = NuevoContratistaViewModel.factory(nucleo),
    )
    var menuEmpresaAbierto by remember { mutableStateOf(false) }
    var menuTipoAbierto by remember { mutableStateOf(false) }
    var escaneando by remember { mutableStateOf(false) }

    if (escaneando) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
            // Acá sólo sirve el carnet PRAIND: sin buscar el PDF417 de la
            // cédula, todo el procesador queda para el texto.
            lectorPdf417 = false,
            onDocumentoDetectado = { documento ->
                escaneando = false
                vm.aplicarDocumentoEscaneado(documento)
            },
            onCerrar = { escaneando = false },
        )
        return
    }

    // Mismo criterio que [PantallaRutas]/[PantallaProveedores]:
    // `imePadding()` va en un Column SIN `fillMaxSize` -- combinarlo con
    // `fillMaxSize` deja un hueco enorme entre el teclado y el contenido
    // (bug reportado 2026-09-20). El scroll hasta el fondo al enfocar el
    // último input (vencimiento PRAIND) es la misma receta: sin él, el foco
    // sólo garantiza que el campo entre en pantalla, no el botón "Guardar"
    // de abajo.
    val scrollStateFormulario = rememberScrollState()
    val alcanceScroll = rememberCoroutineScope()
    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
    Column(
        modifier = Modifier.verticalScroll(scrollStateFormulario).imePadding(),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text("Nuevo contratista", style = MaterialTheme.typography.titleMedium)
            BotonDiscretoBrisas(onClick = onVolver) {
                Text("← Volver")
            }
        }

        BotonBrisas(
            onClick = {
                // Un solo botón llena TODOS los campos, así que se parte de
                // cero (a diferencia de Proveedores/Rutas, donde cada
                // escáner llena un solo paso).
                vm.limpiarParaEscanear()
                escaneando = true
            },
            modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
        ) {
            Icon(Icons.Default.PhotoCamera, contentDescription = null, modifier = Modifier.padding(end = 8.dp))
            Text("ESCANEAR PRAIND")
        }

        OutlinedTextField(
            value = vm.cedula,
            onValueChange = vm::cambiarCedula,
            label = { Text("Cédula") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
        )

        OutlinedTextField(
            value = vm.nombre,
            onValueChange = vm::cambiarNombre,
            label = { Text("Nombre") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        )

        ExposedDropdownMenuBox(
            expanded = menuEmpresaAbierto,
            onExpandedChange = { menuEmpresaAbierto = it },
            modifier = Modifier.padding(top = 12.dp),
        ) {
            OutlinedTextField(
                value = vm.empresaSeleccionada?.nombre ?: "",
                onValueChange = {},
                readOnly = true,
                label = { Text("Empresa") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = menuEmpresaAbierto) },
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
            )
            DropdownMenu(expanded = menuEmpresaAbierto, onDismissRequest = { menuEmpresaAbierto = false }) {
                vm.empresas.forEach { empresa ->
                    DropdownMenuItem(
                        text = { Text(empresa.nombre) },
                        onClick = {
                            vm.elegirEmpresa(empresa)
                            menuEmpresaAbierto = false
                        },
                    )
                }
            }
        }
        val sugerenciaEmpresa = vm.empresaSugeridaTexto
        if (sugerenciaEmpresa != null) {
            Text(
                "El carnet dice \"$sugerenciaEmpresa\" — no hay ninguna empresa igual en la lista, elija la correcta arriba.",
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
            )
        }

        ExposedDropdownMenuBox(
            expanded = menuTipoAbierto,
            onExpandedChange = { menuTipoAbierto = it },
            modifier = Modifier.padding(top = 12.dp),
        ) {
            OutlinedTextField(
                value = etiquetaTipo(vm.tipoIngreso),
                onValueChange = {},
                readOnly = true,
                label = { Text("Tipo de ingreso") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = menuTipoAbierto) },
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
            )
            DropdownMenu(expanded = menuTipoAbierto, onDismissRequest = { menuTipoAbierto = false }) {
                // Sin "Por correo": dejó de ser un tipo de contratista
                // (2026-10-03); esas visitas van en Externos → Por correo.
                TipoIngreso.entries.filter { it != TipoIngreso.POR_CORREO }.forEach { tipo ->
                    DropdownMenuItem(
                        text = { Text(etiquetaTipo(tipo)) },
                        onClick = {
                            vm.cambiarTipoIngreso(tipo)
                            menuTipoAbierto = false
                        },
                    )
                }
            }
        }

        // Sin checkbox "Con acceso" -- pedido explícito del usuario
        // 2026-09-20: registrar a alguien acá es siempre en persona, frente
        // al guardia, así que el acceso ya está implícito -- un toggle
        // aparte para negarlo era un paso de más para un caso que en la
        // práctica nunca ocurre en este flujo (a diferencia de desktop/web,
        // donde sí hace falta dar de alta gente sin tenerla en frente).
        //
        // "Personal de ruta" sólo tiene sentido para PRAIND/IN HOUSE --
        // pedido explícito del usuario 2026-09-20: "Por correo" y "SWAT" no
        // son tipos de personal fijo en sitio, mostrar el check ahí sólo
        // invitaba a marcarlo sin que signifique nada para esos dos casos.
        if (vm.muestraPersonalRuta) {
            Row(modifier = Modifier.padding(top = 8.dp)) {
                Checkbox(checked = vm.personalRuta, onCheckedChange = vm::cambiarPersonalRuta)
                Text("Personal de ruta", modifier = Modifier.padding(top = 12.dp))
            }
        }

        val praindVencido = vm.praindVencido

        if (vm.requierePraind) {
            OutlinedTextField(
                value = vm.fechaPraind,
                onValueChange = vm::cambiarFechaPraind,
                label = { Text("Vencimiento PRAIND (DD-MM-AAAA)") },
                singleLine = true,
                isError = praindVencido,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().padding(top = 4.dp)
                    .onFocusChanged {
                        if (it.isFocused) {
                            alcanceScroll.launch {
                                repeat(15) {
                                    scrollStateFormulario.animateScrollTo(scrollStateFormulario.maxValue)
                                    delay(30)
                                }
                            }
                        }
                    },
            )
            // Mismo criterio que el resto de la app (mayúscula + negrita
            // para que un motivo de bloqueo se lea con fuerza, ver
            // `PantallaConfirmarIngreso`/`FilaContratista`) -- pedido
            // explícito del usuario 2026-09-20: avisar acá mismo, sin
            // esperar a que Rust lo rechace al guardar, y bloquear el
            // guardado mientras la fecha siga vencida (ver `enabled` del
            // botón "Guardar" más abajo).
            if (praindVencido) {
                Text(
                    "PRAIND VENCIDO — INGRESE UNA FECHA VIGENTE",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }
        }

        val mensajeError = vm.error
        if (mensajeError != null) {
            Text(mensajeError, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(top = 16.dp))
        }
        val mensajeExito = vm.mensaje
        if (mensajeExito != null) {
            Text(mensajeExito, color = ColorExitoBrisas, modifier = Modifier.padding(top = 16.dp))
        }

        BotonBrisas(
            onClick = { vm.guardar(onGuardado = onVolver) },
            enabled = !vm.enviando && !praindVencido,
            // Sin padding inferior -- igual que el botón de Proveedores/
            // Rutas: ese `bottom = 32.dp` sumado al `imePadding()` del
            // teclado dejaba un hueco visible de más entre el botón y el
            // teclado (reportado 2026-09-20).
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text(if (vm.enviando) "Guardando…" else "Guardar")
        }
    }
    }
}

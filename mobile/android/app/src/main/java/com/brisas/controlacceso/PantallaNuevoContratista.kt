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
import androidx.compose.runtime.LaunchedEffect
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
import java.time.LocalDate
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.DatosContratista
import uniffi.control_acceso_mobile.Empresa
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
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
/// MV-10 (auditoría 2026-09-24): esto reimplementaba a mano en Kotlin la
/// regla real de `control_acceso::domain::contratista::requiere_praind_de`
/// -- ahora se llama a `Nucleo.requierePraindParaFormulario` (agregada para
/// esto), que delega en esa misma función. No hace SQLite ni red (pura,
/// sin bloquear ningún `Mutex`), así que llamarla en cada recomposición
/// -- por ejemplo, en cada tecla mientras se tipea la fecha -- es seguro.
private fun requierePraind(nucleo: Nucleo, tipo: TipoIngreso, personalRuta: Boolean): Boolean =
    nucleo.requierePraindParaFormulario(tipo, personalRuta)

private fun etiquetaTipo(tipo: TipoIngreso): String =
    when (tipo) {
        TipoIngreso.PRAIND -> "PRAIND"
        TipoIngreso.IN_HOUSE -> "IN HOUSE"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }

/// Máscara del campo de vencimiento PRAIND -- descarta todo lo que no sea
/// dígito (así el usuario no puede meter un "-" de más ni queda uno duplicado
/// al reconstruir) y reinserta los guiones en las posiciones DD-MM-AAAA a
/// medida que tipea, hasta los 8 dígitos que ocupa la fecha completa. Mismo
/// criterio que un campo de tarjeta con máscara -- pedido explícito del
/// usuario 2026-09-21: sin esto tocaba tipear los guiones a mano con el
/// teclado completo en vez del numérico.
private fun formatearFechaDDMMYYYY(texto: String): String {
    val digitos = texto.filter(Char::isDigit).take(8)
    return buildString {
        for (indice in digitos.indices) {
            append(digitos[indice])
            if (indice == 1 || indice == 3) append('-')
        }
    }
}

/// Compara texto libre del carnet contra los nombres reales de
/// `Nucleo.listarEmpresas()` -- ninguna de las dos fuentes garantiza
/// mayúsculas/espacios idénticos, así que primero se intenta una igualdad
/// exacta (sin distinguir mayúsculas) y sólo si eso falla se cae a
/// "una contiene a la otra" (tolera un "S.A." de más o de menos a un lado).
private fun buscarEmpresaPorNombre(empresas: List<Empresa>, textoCarnet: String): Empresa? {
    val normalizado = textoCarnet.trim()
    if (normalizado.isEmpty()) return null
    return empresas.firstOrNull { it.nombre.equals(normalizado, ignoreCase = true) }
        ?: empresas.firstOrNull {
            it.nombre.contains(normalizado, ignoreCase = true) ||
                normalizado.contains(it.nombre, ignoreCase = true)
        }
}

/// Sin ViewModel a propósito — mismo motivo que `PantallaConfirmarIngreso`
/// (ver su doc-comment y ARQUITECTURA.md): `PantallaPrincipal` desmonta
/// este formulario por completo al volver, así que cada entrada ya es un
/// intento fresco sin necesidad de un dueño de estado que sobreviva más
/// que eso.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PantallaNuevoContratista(nucleo: Nucleo, onVolver: () -> Unit) {
    // Mismo destino que el botón "← Volver" visible de abajo -- sin esto,
    // atrás del sistema se escapaba a la Activity en vez de volver a
    // Activos (hallazgo 2026-09-19).
    BackHandler(onBack = onVolver)
    var empresas by remember { mutableStateOf<List<Empresa>>(emptyList()) }
    var cedula by rememberSaveable { mutableStateOf("") }
    var nombre by rememberSaveable { mutableStateOf("") }
    // `Empresa` es un `data class` generado por uniffi sin `Serializable` —
    // `rememberSaveable` fallaría en tiempo de ejecución acá. Se pierde en
    // una rotación (a diferencia del resto del formulario); no vale la
    // complejidad de un `Saver` a mano sólo para este campo.
    var empresaSeleccionada by remember { mutableStateOf<Empresa?>(null) }
    // Nombre de empresa leído del carnet cuando no calzó con ninguna fila
    // de `empresas` -- se muestra como pista para elegir a mano, nunca se
    // manda a Rust (que sólo acepta un `empresa_id` real).
    var empresaSugeridaTexto by remember { mutableStateOf<String?>(null) }
    var tipoIngreso by rememberSaveable { mutableStateOf(TipoIngreso.PRAIND) }
    var personalRuta by rememberSaveable { mutableStateOf(false) }
    var fechaPraind by rememberSaveable { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var mensaje by remember { mutableStateOf<String?>(null) }
    var enviando by remember { mutableStateOf(false) }
    var menuEmpresaAbierto by remember { mutableStateOf(false) }
    var menuTipoAbierto by remember { mutableStateOf(false) }
    var escaneando by remember { mutableStateOf(false) }
    val alcance = rememberCoroutineScope()

    LaunchedEffect(Unit) {
        try {
            // MV-09 (auditoría 2026-09-24): `Dispatchers.Default` es el
            // pool de CPU (tamaño = núcleos del dispositivo) -- una llamada
            // FFI bloqueante a SQLite no es trabajo de CPU, es E/S, y
            // corriendo ahí le quita hilos al OCR y al resto de tareas de
            // CPU reales mientras esta consulta hace su round-trip. `IO`
            // es el pool pensado para esto (mismo criterio que ya usa
            // `PantallaConfirmarIngreso.registrarIngreso`).
            empresas = withContext(Dispatchers.IO) { nucleo.listarEmpresas() }
        } catch (excepcion: NucleoException) {
            error = excepcion.message
        }
    }

    if (escaneando) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
            onDocumentoDetectado = { documento ->
                escaneando = false
                mensaje = null
                // Sólo PRAIND -- pedido explícito del usuario 2026-09-20:
                // ese es el único carnet que trae empresa y vencimiento
                // además de cédula/nombre (ver el doc-comment de esta
                // pantalla), así que una cédula nacional o de residencia acá
                // no aporta nada que valga la pena rellenar a medias. Antes
                // sí tomaba cédula/nombre de cualquier documento -- ahora un
                // documento distinto se rechaza entero, sin tocar ningún
                // campo.
                if (documento.tipo == TipoDocumento.CARNET_INDUCCION_PRAIND) {
                    error = null
                    if (documento.numeroDocumento.isNotBlank()) {
                        cedula = documento.numeroDocumento.filter(Char::isDigit).ifBlank { documento.numeroDocumento }
                    }
                    // Mayúscula acá también -- pedido explícito del usuario
                    // 2026-09-20: el nombre debe quedar en mayúscula sea que
                    // se tipee a mano o venga del OCR del carnet PRAIND, no
                    // sólo en el campo de abajo.
                    documento.nombre?.let { nombre = it.uppercase() }
                    tipoIngreso = TipoIngreso.PRAIND
                    documento.vencimiento?.let { fechaPraind = it.aTextoDDMMYYYY() }
                    val textoEmpresa = documento.empresa?.trim()
                    when {
                        textoEmpresa.isNullOrBlank() -> Unit
                        else -> {
                            val coincidencia = buscarEmpresaPorNombre(empresas, textoEmpresa)
                            if (coincidencia != null) {
                                empresaSeleccionada = coincidencia
                                empresaSugeridaTexto = null
                            } else {
                                empresaSugeridaTexto = textoEmpresa
                            }
                        }
                    }
                    mensaje = "Carnet PRAIND leído — revise los datos antes de guardar"
                } else {
                    error = "Documento inválido"
                }
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
                // Limpia todo antes de abrir la cámara -- pedido explícito
                // del usuario 2026-09-20: sin esto, si se escanea un
                // segundo carnet (persona equivocada, mala lectura, etc.)
                // los datos del primero podían quedar mezclados con los del
                // segundo en vez de partir de cero. Este formulario es un
                // caso simple para esto: un solo botón llena TODOS los
                // campos de una vez, a diferencia de Proveedores/Rutas
                // donde cada escáner llena sólo un paso del formulario y
                // borrar todo ahí sí destruiría trabajo ya hecho en otros
                // pasos.
                cedula = ""
                nombre = ""
                empresaSeleccionada = null
                empresaSugeridaTexto = null
                tipoIngreso = TipoIngreso.PRAIND
                personalRuta = false
                fechaPraind = ""
                error = null
                mensaje = null
                escaneando = true
            },
            modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
        ) {
            Icon(Icons.Default.PhotoCamera, contentDescription = null, modifier = Modifier.padding(end = 8.dp))
            Text("ESCANEAR PRAIND")
        }

        OutlinedTextField(
            value = cedula,
            onValueChange = { cedula = it.filter(Char::isDigit) },
            label = { Text("Cédula") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
        )

        OutlinedTextField(
            value = nombre,
            // Mayúscula automática al tipear -- mismo criterio que el OCR
            // del carnet PRAIND de acá arriba (pedido explícito del usuario
            // 2026-09-20): el nombre del contratista siempre queda en
            // mayúscula, sin importar de dónde vino.
            onValueChange = { nombre = it.uppercase() },
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
                value = empresaSeleccionada?.nombre ?: "",
                onValueChange = {},
                readOnly = true,
                label = { Text("Empresa") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = menuEmpresaAbierto) },
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
            )
            DropdownMenu(expanded = menuEmpresaAbierto, onDismissRequest = { menuEmpresaAbierto = false }) {
                empresas.forEach { empresa ->
                    DropdownMenuItem(
                        text = { Text(empresa.nombre) },
                        onClick = {
                            empresaSeleccionada = empresa
                            empresaSugeridaTexto = null
                            menuEmpresaAbierto = false
                        },
                    )
                }
            }
        }
        val sugerenciaEmpresa = empresaSugeridaTexto
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
                value = etiquetaTipo(tipoIngreso),
                onValueChange = {},
                readOnly = true,
                label = { Text("Tipo de ingreso") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = menuTipoAbierto) },
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
            )
            DropdownMenu(expanded = menuTipoAbierto, onDismissRequest = { menuTipoAbierto = false }) {
                TipoIngreso.entries.forEach { tipo ->
                    DropdownMenuItem(
                        text = { Text(etiquetaTipo(tipo)) },
                        onClick = {
                            tipoIngreso = tipo
                            // El check "Personal de ruta" se oculta para
                            // estos dos tipos (más arriba) -- sin este
                            // reseteo, un valor `true` que quedó de un tipo
                            // anterior seguía contando para
                            // `requierePraind` aunque el check ya no
                            // estuviera visible para destildarlo.
                            if (tipo == TipoIngreso.POR_CORREO || tipo == TipoIngreso.SWAT) {
                                personalRuta = false
                            }
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
        if (tipoIngreso != TipoIngreso.POR_CORREO && tipoIngreso != TipoIngreso.SWAT) {
            Row(modifier = Modifier.padding(top = 8.dp)) {
                Checkbox(checked = personalRuta, onCheckedChange = { personalRuta = it })
                Text("Personal de ruta", modifier = Modifier.padding(top = 12.dp))
            }
        }

        // Fecha vencida en el instante en que se tipea, no recién al
        // guardar -- `runCatching` porque mientras se escribe el texto pasa
        // por formas incompletas ("10-09-202") que no parsean todavía como
        // fecha real. Declarado acá afuera (no sólo dentro del `if` de más
        // abajo) porque el botón "Guardar" también lo necesita para
        // deshabilitarse -- pedido explícito del usuario 2026-09-20: si el
        // PRAIND está vencido, no debe dejar registrar.
        val praindVencido = requierePraind(nucleo, tipoIngreso, personalRuta) && fechaPraind.isNotBlank() &&
            runCatching { LocalDate.parse(textoDDMMYYYYaIso(fechaPraind)) < LocalDate.now() }.getOrDefault(false)

        if (requierePraind(nucleo, tipoIngreso, personalRuta)) {
            OutlinedTextField(
                value = fechaPraind,
                onValueChange = { fechaPraind = formatearFechaDDMMYYYY(it) },
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

        val mensajeError = error
        if (mensajeError != null) {
            Text(mensajeError, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(top = 16.dp))
        }
        val mensajeExito = mensaje
        if (mensajeExito != null) {
            Text(mensajeExito, color = ColorExitoBrisas, modifier = Modifier.padding(top = 16.dp))
        }

        BotonBrisas(
            onClick = {
                error = null
                mensaje = null
                val empresa = empresaSeleccionada
                if (cedula.isBlank() || nombre.isBlank() || empresa == null) {
                    error = "Complete cédula, nombre y empresa"
                    return@BotonBrisas
                }
                if (praindVencido) {
                    error = "El PRAIND está vencido — ingrese una fecha vigente"
                    return@BotonBrisas
                }
                enviando = true
                alcance.launch {
                    try {
                        // MV-09: mismo motivo que en `listarEmpresas` --
                        // `crearContratista` hace SQLite + FFI bloqueantes,
                        // no cómputo de CPU.
                        withContext(Dispatchers.IO) {
                            nucleo.crearContratista(
                                DatosContratista(
                                    cedula = cedula,
                                    nombre = nombre,
                                    empresaId = empresa.id,
                                    tipoIngreso = tipoIngreso,
                                    fechaVencimientoPraind = fechaPraind.ifBlank { null }?.let(::textoDDMMYYYYaIso),
                                    esPersonalRuta = personalRuta,
                                    // Siempre true -- ver el comentario junto al
                                    // checkbox que se sacó más arriba.
                                    tieneAcceso = true,
                                ),
                            )
                        }
                        // MV-09: si `alcance` (atado a esta composición) se
                        // cancela justo al volver del `withContext` de
                        // arriba -- la escritura en Rust ya terminó, no es
                        // cancelable a mitad de camino -- `CambiosNube.
                        // solicitar()` se saltaría igual, dejando la
                        // sincronización esperando el próximo pulso
                        // automático en vez de dispararse al toque.
                        // `NonCancellable` fuerza que este aviso puntual
                        // corra siempre.
                        withContext(NonCancellable) { CambiosNube.solicitar() }
                        cedula = ""
                        nombre = ""
                        empresaSeleccionada = null
                        empresaSugeridaTexto = null
                        fechaPraind = ""
                        personalRuta = false
                        // Antes se quedaba en el formulario como si se fuera a
                        // dar de alta otro contratista -- pedido explícito del
                        // usuario en pruebas reales, 2026-09-17: cerrar y
                        // volver a la lista de activos al terminar.
                        onVolver()
                    } catch (excepcion: NucleoException) {
                        error = excepcion.message
                    } finally {
                        enviando = false
                    }
                }
            },
            enabled = !enviando && !praindVencido,
            // Sin padding inferior -- igual que el botón de Proveedores/
            // Rutas: ese `bottom = 32.dp` sumado al `imePadding()` del
            // teclado dejaba un hueco visible de más entre el botón y el
            // teclado (reportado 2026-09-20).
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text(if (enviando) "Guardando…" else "Guardar")
        }
    }
    }
}

package com.brisas.controlacceso

import androidx.activity.compose.BackHandler
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
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.PersonAdd
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material.icons.filled.Search
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.EmpresaProveedor
import uniffi.control_acceso_mobile.IngresoProveedorRemoto
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.RegistroIngresoProveedorActivoResumen

/// Clave estable para `key` de `LazyColumn`/filtro de texto -- `Local` e
/// `Remota` usan ids de mundos distintos (`Int` autoincremental vs `UUID`
/// de la nube), sin esto la lista fusionada no tiene una sola noción de
/// identidad. Mismo criterio que `claveFilaProveedorActiva` en
/// `desktop/src/api/proveedores.ts`.
private fun FilaProveedorActiva.clave(): String = when (this) {
    is FilaProveedorActiva.Local -> "local-${registro.id}"
    is FilaProveedorActiva.Remota -> "remota-${remoto.uuid}"
}

private fun FilaProveedorActiva.coincideCon(busqueda: String): Boolean = when (this) {
    is FilaProveedorActiva.Local -> registro.cedula.contains(busqueda, ignoreCase = true) ||
        registro.nombre.contains(busqueda, ignoreCase = true) ||
        registro.empresaNombre.contains(busqueda, ignoreCase = true) ||
        (registro.placa?.contains(busqueda, ignoreCase = true) == true)
    is FilaProveedorActiva.Remota -> remoto.cedula.contains(busqueda, ignoreCase = true) ||
        remoto.nombre.contains(busqueda, ignoreCase = true) ||
        remoto.empresaNombre.contains(busqueda, ignoreCase = true) ||
        (remoto.placa?.contains(busqueda, ignoreCase = true) == true)
}

/// Control de proveedores -- ver
/// `docs/features-futuras/plan-control-proveedores.md`. Mismo esqueleto
/// autocontenido que [PantallaGafetesProvisionales] (formulario + lista de
/// activos propia, sin integrarse a la lista compartida de "Activos" --
/// ver el doc-comment de [ProveedoresViewModel]), con dos diferencias: acá
/// SÍ hay OCR de cédula (reusando `PantallaEscanearCedula` con el modo
/// `DOCUMENTO_CONTRATISTA` -- el criterio de aceptación del documento es
/// idéntico, sólo cambia a quién se le atribuye después) y SÍ se puede dar
/// de alta una empresa nueva inline si la búsqueda no trae nada.
///
/// Rediseñada 2026-09-17 tras la primera prueba real en emulador: el
/// formulario completo (cédula, nombre, empresa, placa, gafete) vivía
/// siempre visible arriba de la lista de activos, apiñado -- ahora el
/// estado base es igual a [PantallaActivos] (buscador + lista) y el
/// formulario se abre aparte, en pantalla completa, con el mismo lenguaje
/// de tarjetas numeradas de [PantallaRutas] (que el usuario señaló como
/// referencia): un paso por tarjeta, encabezado con círculo numerado que
/// se pone check al completarse.
@Composable
fun PantallaProveedores(
    nucleo: Nucleo,
    secretoStore: SecretoDispositivoStore,
    refrescarNube: Int = 0,
) {
    val viewModel: ProveedoresViewModel =
        viewModel(factory = ProveedoresViewModel.factory(nucleo, secretoStore))
    // Sin esto, un cambio que llega de OTRO dispositivo (pulso periódico o
    // aviso Realtime) actualiza la caché local (`ingresos_proveedor_remotos`)
    // pero esta pantalla nunca se entera -- mismo criterio que
    // `PantallaActivos` (bug reportado en pruebas reales, 2026-09-17: un
    // ingreso o salida hecho en la PC no se reflejaba en el teléfono hasta
    // salir y volver a entrar a Proveedores).
    LaunchedEffect(refrescarNube) {
        if (refrescarNube > 0) {
            viewModel.refrescarActivos()
        }
    }
    var mostrandoFormulario by remember { mutableStateOf(false) }
    var gafeteTexto by remember { mutableStateOf("") }
    var busqueda by remember { mutableStateOf("") }
    var escaneando by remember { mutableStateOf(false) }
    // Mismo lector que el paso 3 de [PantallaRutas] (`extraerVehiculo`,
    // `LectorVehiculoRuta.kt`) -- genérico, no específico de rutas. Acá
    // sólo interesa la placa: si el OCR detecta un número de unidad
    // (calcomanía de flota) en vez de una placa, igual se usa el texto
    // leído -- un proveedor no trae ese distintivo, pero no vale la pena
    // rechazar una lectura válida sólo por el tipo detectado.
    var escaneandoPlaca by remember { mutableStateOf(false) }
    var filaParaSalida by remember {
        mutableStateOf<FilaProveedorActiva?>(null)
    }

    if (escaneando) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
            onDocumentoDetectado = { documento ->
                escaneando = false
                val numero = documento.numeroDocumento.filter(Char::isDigit)
                    .ifBlank { documento.numeroDocumento }
                viewModel.rellenarDesdeDocumento(numero, documento.nombre, documento.apellidos)
            },
            onCerrar = { escaneando = false },
        )
        return
    }

    if (escaneandoPlaca) {
        PantallaEscanearVehiculoRuta(
            onVehiculoDetectado = { detectado ->
                escaneandoPlaca = false
                viewModel.cambiarPlaca(detectado.valor)
            },
            onCerrar = { escaneandoPlaca = false },
            mensajeInicial = "Apunte a la placa del vehículo",
            mensajePermiso = "Se necesita permiso de cámara para escanear la placa.",
        )
        return
    }

    if (mostrandoFormulario) {
        FormularioNuevoIngresoProveedor(
            viewModel = viewModel,
            gafeteTexto = gafeteTexto,
            onCambiarGafeteTexto = { gafeteTexto = it },
            onEscanear = { escaneando = true },
            onEscanearPlaca = { escaneandoPlaca = true },
            onVolver = {
                viewModel.cancelarFormulario()
                gafeteTexto = ""
                mostrandoFormulario = false
            },
        )
        return
    }

    val activosFiltrados = if (busqueda.isBlank()) {
        viewModel.activos
    } else {
        viewModel.activos.filter { it.coincideCon(busqueda) }
    }

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        Text(
            "Proveedores",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
        )

        Row(
            modifier = Modifier.fillMaxWidth().padding(top = 10.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Sin `.height(AlturaBusquedaBrisas)` a propósito -- combinado
            // con `leadingIcon` (a diferencia del buscador de
            // `PantallaActivos`, que no tiene `placeholder`) ese alto
            // dejaba el texto del placeholder recortado, casi invisible.
            OutlinedTextField(
                value = busqueda,
                onValueChange = { busqueda = it },
                placeholder = { Text("Cédula, nombre, empresa…") },
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                singleLine = true,
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.weight(1f),
            )
            // Mismo look que `BotonCamaraCuadrado` de [PantallaRutas]
            // (cuadrado, borde y ícono en el color primario) -- pedido
            // explícito del usuario en vez del botón "+ Nuevo ingreso"
            // ancho de antes, para que quede al lado del buscador como el
            // botón de cámara junto al buscador de [PantallaActivos].
            Box(
                modifier = Modifier
                    .size(AlturaBusquedaBrisas)
                    .border(1.dp, MaterialTheme.colorScheme.primary, FormaCampoBrisas)
                    .clip(FormaCampoBrisas)
                    .clickable(onClick = { mostrandoFormulario = true }),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    Icons.Default.PersonAdd,
                    contentDescription = "Nuevo ingreso de proveedor",
                    tint = MaterialTheme.colorScheme.primary,
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

        ListaConDesvanecido {
            LazyColumn(
                contentPadding = PaddingValues(top = 10.dp),
                verticalArrangement = Arrangement.spacedBy(5.dp),
            ) {
                items(activosFiltrados, key = { it.clave() }) { fila ->
                    FilaProveedorActivo(
                        fila = fila,
                        onConfirmarSalida = { filaParaSalida = fila },
                    )
                }
            }
        }
    }

    DialogoConfirmarSalidaProveedor(
        fila = filaParaSalida,
        onDismiss = { filaParaSalida = null },
        onConfirmar = {
            viewModel.registrarSalida(it)
            filaParaSalida = null
        },
    )
}

/// Formulario en pantalla completa (mismo criterio que
/// [PantallaNuevoContratista]: `PantallaProveedores` lo desmonta al volver,
/// así que no hace falta que el estado de los pasos sobreviva más que eso).
/// Tres tarjetas numeradas, mismo estilo que los "pasos" de [PantallaRutas]
/// -- acá sí puede ir todo en un único `Column` con `verticalScroll` porque,
/// a diferencia del estado base, no hay ningún `LazyColumn` adentro.
@Composable
private fun FormularioNuevoIngresoProveedor(
    viewModel: ProveedoresViewModel,
    gafeteTexto: String,
    onCambiarGafeteTexto: (String) -> Unit,
    onEscanear: () -> Unit,
    onEscanearPlaca: () -> Unit,
    onVolver: () -> Unit,
) {
    val paso1Completo = viewModel.cedula.isNotBlank() && viewModel.nombre.isNotBlank()
    val paso2Completo = viewModel.empresaSeleccionada != null
    val paso3Completo = gafeteTexto.isNotBlank()
    val puedeRegistrar = paso1Completo && paso2Completo && paso3Completo &&
        !viewModel.registrando && !viewModel.cedulaConIngresoActivo

    // Mismo destino que el botón "← Volver" visible de abajo -- sin esto,
    // atrás del sistema se escapaba a la Activity en vez de volver a la
    // lista (hallazgo 2026-09-19, mismo patrón ya usado en las pantallas de
    // escaneo/PantallaConfirmarIngreso).
    BackHandler(onBack = onVolver)

    // Mismo criterio que [PantallaRutas]: `imePadding()` va en un `Column`
    // SIN `fillMaxSize` -- combinarlo con `fillMaxSize` deja un hueco enorme
    // entre el teclado y el contenido (bug reportado 2026-09-20), porque el
    // padding se suma sobre un alto que ya estaba fijado a pantalla completa
    // en vez de sobre el alto real del contenido.
    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
    val scrollState = rememberScrollState()
    val scope = rememberCoroutineScope()
    Column(
        modifier = Modifier.verticalScroll(scrollState).imePadding(),
    ) {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text("Nuevo ingreso", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            BotonDiscretoBrisas(onClick = onVolver) {
                Text("← Volver")
            }
        }

        Column(modifier = Modifier.padding(top = 10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            PasoDatosProveedor(
                completado = paso1Completo,
                cedula = viewModel.cedula,
                nombre = viewModel.nombre,
                onCambiarCedula = viewModel::cambiarCedula,
                onCambiarNombre = viewModel::cambiarNombre,
                onEscanear = onEscanear,
            )
            PasoEmpresaProveedora(
                completado = paso2Completo,
                texto = viewModel.textoEmpresa,
                resultados = viewModel.resultadosEmpresa,
                creando = viewModel.creandoEmpresa,
                onCambiarTexto = viewModel::cambiarTextoEmpresa,
                onElegir = viewModel::elegirEmpresa,
                onCrear = viewModel::crearEmpresa,
            )
            PasoVehiculoYGafete(
                completado = paso3Completo,
                placa = viewModel.placa,
                onCambiarPlaca = viewModel::cambiarPlaca,
                onEscanearPlaca = onEscanearPlaca,
                gafeteTexto = gafeteTexto,
                onCambiarGafeteTexto = { onCambiarGafeteTexto(it.filter(Char::isDigit)) },
                // Al enfocar el último input, lleva el scroll hasta el
                // fondo -- ahí vive el botón "Registrar ingreso", último
                // elemento de esta misma Column. El foco por sí solo sólo
                // garantiza que el campo entre en pantalla, no el botón de
                // abajo (pedido explícito del usuario 2026-09-20: quiere ver
                // campo y botón juntos, no sólo el campo). Un solo
                // `animateScrollTo` no alcanza: el teclado tarda ~250ms en
                // animarse y el `imePadding()` va agrandando la Column
                // cuadro a cuadro, así que `maxValue` todavía no refleja el
                // alto final en el instante del foco -- se repite mientras
                // dura esa animación para perseguir el nuevo fondo.
                onGafeteEnfocado = {
                    scope.launch {
                        repeat(15) {
                            scrollState.animateScrollTo(scrollState.maxValue)
                            delay(30)
                        }
                    }
                },
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
        viewModel.mensaje?.let { mensaje ->
            Text(
                mensaje,
                style = MaterialTheme.typography.bodySmall,
                color = ColorExitoBrisas,
                modifier = Modifier.padding(top = 8.dp),
            )
        }

        BotonBrisas(
            onClick = {
                val numero = gafeteTexto.toLongOrNull() ?: return@BotonBrisas
                // Antes sólo limpiaba el gafete y dejaba el formulario abierto,
                // como si se fuera a registrar otro ingreso -- pedido explícito
                // del usuario en pruebas reales, 2026-09-17: cerrar y volver a
                // la lista de activos, igual que se espera del alta de
                // contratista.
                viewModel.registrarIngreso(numero) {
                    onCambiarGafeteTexto("")
                    onVolver()
                }
            },
            enabled = puedeRegistrar,
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        ) {
            Text(if (viewModel.registrando) "Registrando…" else "Registrar ingreso")
        }
    }
    }
}

/// Círculo numerado + título -- mismo patrón visual que
/// `PasoEncabezado` de [PantallaRutas], duplicado a propósito acá (es
/// `private` en ese archivo, y cada pantalla ya es dueña de su propio
/// checklist -- mismo criterio que `ToggleVista` en desktop).
@Composable
private fun PasoEncabezadoProveedor(numero: Int, titulo: String, completado: Boolean) {
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

/// Paso 1 -- cédula y nombre, con el mismo botón de cámara cuadrado que las
/// tarjetas de [PantallaRutas] (acá alcanza uno solo para la tarjeta: el
/// documento trae los dos datos de una vez, igual que el carnet PRAIND en
/// [PantallaNuevoContratista]).
@Composable
private fun PasoDatosProveedor(
    completado: Boolean,
    cedula: String,
    nombre: String,
    onCambiarCedula: (String) -> Unit,
    onCambiarNombre: (String) -> Unit,
    onEscanear: () -> Unit,
) {
    TarjetaPasoProveedor(1, "Datos del proveedor", completado, onEscanear = onEscanear) {
        OutlinedTextField(
            value = cedula,
            onValueChange = onCambiarCedula,
            placeholder = { Text("Cédula") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
        )
        OutlinedTextField(
            value = nombre,
            onValueChange = onCambiarNombre,
            placeholder = { Text("Nombre") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
        )
    }
}

/// Paso 2 -- mismo buscador con alta inline que ya existía, ahora dentro de
/// su propia tarjeta numerada.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PasoEmpresaProveedora(
    completado: Boolean,
    texto: String,
    resultados: List<EmpresaProveedor>,
    creando: Boolean,
    onCambiarTexto: (String) -> Unit,
    onElegir: (EmpresaProveedor) -> Unit,
    onCrear: (String) -> Unit,
) {
    var menuAbierto by remember { mutableStateOf(false) }
    // `resultados` queda vacío tanto "sin coincidencias todavía" como justo
    // después de elegir una empresa (`elegirEmpresa` los limpia) -- sin
    // `!completado` acá, el botón "Crear empresa" seguía apareciendo con
    // una empresa YA seleccionada (bug reportado en pruebas reales,
    // 2026-09-17: dejaba crear un duplicado de una empresa que ya existía).
    val sinCoincidencias = !completado && texto.isNotBlank() && resultados.isEmpty()

    TarjetaPasoProveedor(
        2,
        "Empresa proveedora",
        completado,
        onEscanear = { onCrear(texto) },
        icono = Icons.Default.Add,
        descripcionIcono = "Añadir empresa",
        botonHabilitado = sinCoincidencias && !creando,
        contenidoExtra = {
            if (sinCoincidencias) {
                // Ya no es un botón clickeable -- la acción de crear ahora
                // vive en el botón "+" al lado del campo (mismo lugar que la
                // cámara en las otras 2 tarjetas). Esto queda como aviso de
                // que esa empresa no existe todavía, nada más -- y vive
                // fuera de la fila que centra el botón para no correrlo.
                Text(
                    if (creando) "Creando…" else "Empresa no existe",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
    ) {
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
                placeholder = { Text("Nombre de la empresa") },
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
    }
}

/// Paso 3 -- placa (opcional, con OCR: mismo lector de `PantallaRutas`,
/// `extraerVehiculo`/`LectorVehiculoRuta.kt`, genérico) y número de gafete
/// (siempre tipeado -- no sale de ningún documento).
@Composable
private fun PasoVehiculoYGafete(
    completado: Boolean,
    placa: String,
    onCambiarPlaca: (String) -> Unit,
    onEscanearPlaca: () -> Unit,
    gafeteTexto: String,
    onCambiarGafeteTexto: (String) -> Unit,
    onGafeteEnfocado: () -> Unit,
) {
    TarjetaPasoProveedor(3, "Vehículo y gafete", completado, onEscanear = onEscanearPlaca) {
        OutlinedTextField(
            value = placa,
            onValueChange = onCambiarPlaca,
            placeholder = { Text("Placa (opcional)") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
        )
        OutlinedTextField(
            value = gafeteTexto,
            onValueChange = onCambiarGafeteTexto,
            placeholder = { Text("Número de gafete") },
            singleLine = true,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas)
                .onFocusChanged { if (it.isFocused) onGafeteEnfocado() },
        )
    }
}

/// Tarjeta compartida por los tres pasos -- mismo fondo/forma/padding que
/// las tarjetas de [PantallaRutas], con un botón de cámara cuadrado
/// opcional a la derecha (pasos 1 y 3, el paso 2 no tiene nada
/// escaneable), mismo look que `BotonCamaraCuadrado` de ese archivo --
/// duplicado acá por el mismo motivo que [PasoEncabezadoProveedor]: es
/// `private` allá).
@Composable
private fun TarjetaPasoProveedor(
    numero: Int,
    titulo: String,
    completado: Boolean,
    onEscanear: (() -> Unit)? = null,
    // Mismo botón cuadrado para las 3 tarjetas -- ícono/descripción/estado
    // habilitado configurables para que sirva tanto de "escanear" (pasos 1 y
    // 3) como de "añadir empresa" (paso 2, pedido explícito del usuario
    // 2026-09-19: un botón al lado del campo, no un texto clickeable abajo
    // -- las 3 tarjetas quedan con la misma utilidad visual).
    icono: ImageVector = Icons.Default.PhotoCamera,
    descripcionIcono: String = "Escanear",
    botonHabilitado: Boolean = true,
    // Contenido debajo de la fila campo(s)+botón, FUERA de ella a propósito
    // -- si un mensaje variable (ej. "Empresa no existe") viviera adentro
    // de la columna que el botón centra, la altura de esa columna cambiaría
    // con el mensaje y el botón se corriría (bug reportado 2026-09-19: el
    // input quedaba desfasado del botón). Así el botón siempre se centra
    // sólo contra el/los campo(s), nunca contra contenido variable.
    contenidoExtra: (@Composable () -> Unit)? = null,
    contenido: @Composable () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        PasoEncabezadoProveedor(numero, titulo, completado)
        // Fila propia (sin el encabezado) -- mismo motivo que en
        // `PantallaRutas`: el botón se centra contra el/los campo(s), no
        // contra la tarjeta entera (pedido explícito del usuario,
        // 2026-09-19).
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                contenido()
            }
            if (onEscanear != null) {
                val colorBoton =
                    if (botonHabilitado) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline
                Box(
                    modifier = Modifier
                        .size(AlturaBusquedaBrisas)
                        .border(1.dp, colorBoton, FormaCampoBrisas)
                        .clip(FormaCampoBrisas)
                        .clickable(enabled = botonHabilitado, onClick = onEscanear),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(icono, contentDescription = descripcionIcono, tint = colorBoton)
                }
            }
        }
        contenidoExtra?.invoke()
    }
}

@Composable
private fun FilaProveedorActivo(fila: FilaProveedorActiva, onConfirmarSalida: () -> Unit) {
    when (fila) {
        is FilaProveedorActiva.Local -> FilaProveedorActivoLocal(fila.registro, onConfirmarSalida)
        is FilaProveedorActiva.Remota -> FilaProveedorActivoRemota(fila.remoto, onConfirmarSalida)
    }
}

/// Mismo orden de campos que `FilaActivoLocal`/`FilaActivoRemota`
/// (PantallaActivos.kt, contratista) -- nombre primero (lo más importante:
/// quién es), luego identidad + afiliación + gafete, luego cuándo entró.
/// Antes esta tarjeta abría con "Gafete N" en vez del nombre -- pedido
/// explícito del usuario en pruebas reales, 2026-09-17: unificar el orden
/// de importancia entre las dos pantallas.
///
/// Mayúscula + negrita + gafete en azul, y "dio ingreso" en la última
/// línea -- mismo tratamiento que `FilaActivoLocal` (contratista), pedido
/// explícito del usuario 2026-09-20 para unificar las dos tarjetas.
@Composable
private fun FilaProveedorActivoLocal(
    registro: RegistroIngresoProveedorActivoResumen,
    onConfirmarSalida: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onConfirmarSalida)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(registro.nombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            buildAnnotatedString {
                append(
                    (
                        "${registro.cedula} · ${registro.empresaNombre}" +
                            (registro.placa?.let { " · $it" } ?: "") + " · "
                    ).uppercase(),
                )
                withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary)) {
                    append("Gafete ${registro.gafeteNumero}".uppercase())
                }
            },
            style = MaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Ingresó ${textoFechaHora(registro.fechaHoraIngreso)} · dio ingreso ${registro.usuarioIngresoNombre}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/// Ver el doc-comment de [FilaProveedorActiva] -- un ingreso abierto por
/// OTRO dispositivo del sitio, sin `id` local (sólo `uuid` de la nube).
/// Mismo orden que [FilaProveedorActivoLocal] -- ver ese doc-comment.
@Composable
private fun FilaProveedorActivoRemota(remoto: IngresoProveedorRemoto, onConfirmarSalida: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onConfirmarSalida)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(remoto.nombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Text(
            buildAnnotatedString {
                append(
                    (
                        "${remoto.cedula} · ${remoto.empresaNombre}" +
                            (remoto.placa?.let { " · $it" } ?: "") + " · "
                    ).uppercase(),
                )
                withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary)) {
                    append("Gafete ${remoto.gafeteNumero}".uppercase())
                }
            },
            style = MaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Ingresó ${textoFechaHora(remoto.horaEntrada)} · dio ingreso ${remoto.usuarioEntradaNombre}",
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

/// Mismo layout e información que `DialogoConfirmarSalida` de
/// PantallaActivos.kt (contratista) -- ícono circular, título "Registrar
/// salida", y nombre · cédula · empresa (o "registrado en otro dispositivo"
/// para una fila remota) en vez de sólo "Gafete N · nombre". Pedido
/// explícito del usuario en pruebas reales, 2026-09-17: el modal de
/// Proveedores quedaba "muy laxo" comparado con el de Contratista.
@Composable
private fun DialogoConfirmarSalidaProveedor(
    fila: FilaProveedorActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaProveedorActiva) -> Unit,
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
                    is FilaProveedorActiva.Local ->
                        "${fila.registro.nombre} · ${fila.registro.cedula} · ${fila.registro.empresaNombre} · Gafete ${fila.registro.gafeteNumero}"
                    is FilaProveedorActiva.Remota ->
                        "${fila.remoto.nombre} · registrado en otro dispositivo de la unidad operativa"
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

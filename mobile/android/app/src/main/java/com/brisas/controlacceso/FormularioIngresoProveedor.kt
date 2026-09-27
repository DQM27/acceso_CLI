package com.brisas.controlacceso

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupProperties
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.EmpresaProveedor

// Formulario de nuevo ingreso de proveedor, por pasos (sacado de
// `PantallaProveedores.kt`, punto M6 de la auditoría móvil: sólo se movió código).

/// Formulario en pantalla completa (mismo criterio que
/// [PantallaNuevoContratista]: `PantallaProveedores` lo desmonta al volver,
/// así que no hace falta que el estado de los pasos sobreviva más que eso).
/// Tres tarjetas numeradas, mismo estilo que los "pasos" de [PantallaRutas]
/// -- acá sí puede ir todo en un único `Column` con `verticalScroll` porque,
/// a diferencia del estado base, no hay ningún `LazyColumn` adentro.
@Composable
internal fun FormularioNuevoIngresoProveedor(
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
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
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
    // `resultados` queda vacío tanto "sin coincidencias todavía" como justo
    // después de elegir una empresa (`elegirEmpresa` los limpia) -- sin
    // `!completado` acá, el botón "Crear empresa" seguía apareciendo con
    // una empresa YA seleccionada (bug reportado en pruebas reales,
    // 2026-09-17: dejaba crear un duplicado de una empresa que ya existía).
    val sinCoincidencias = !completado && texto.isNotBlank() && resultados.isEmpty()

    // Resultados en un `Popup` propio (no un `ExposedDropdownMenuBox`/
    // `DropdownMenu` como antes) -- `DropdownMenu` usa por dentro un Popup
    // FOCUSABLE, que compite por el foco con el `TextField` en cada
    // recomposición del anclaje y cerraba el teclado con cada letra tipeada
    // (bug reportado en pruebas reales, 2026-09-21). `PopupProperties(
    // focusable = false)` es la diferencia clave: nunca le saca el foco al
    // campo, así que el teclado se queda abierto sea cual sea el resultado.
    // Flota por ENCIMA del campo (arriba, tapando la tarjeta "Datos del
    // proveedor" si hace falta), nunca abajo -- con el teclado abierto no
    // queda espacio visible debajo del campo, y ahí tapaba justo lo que se
    // estaba tipeando (pedido explícito del usuario, 2026-09-21, tras probar
    // la versión con `alignment = BottomStart`). `altoListaPx` mide el alto
    // real de la lista ya renderizada; el offset Y = -(alto + gap) empuja el
    // Popup hacia arriba esa misma distancia, dejando su borde inferior justo
    // encima del campo. En el primer frame (`altoListaPx == 0`, todavía sin
    // medir) el Popup nace superpuesto al campo y salta a su lugar apenas se
    // mide -- parpadeo de un frame, aceptable frente a tapar el texto.
    var anchoCampoPx by remember { mutableStateOf(0) }
    var altoListaPx by remember { mutableStateOf(0) }
    val densidad = LocalDensity.current

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
        Box(modifier = Modifier.onSizeChanged { anchoCampoPx = it.width }) {
            OutlinedTextField(
                value = texto,
                onValueChange = onCambiarTexto,
                placeholder = { Text("Nombre de la empresa") },
                singleLine = true,
                // El teclado abre ya en mayúscula; el ViewModel igual lo
                // convierte (`cambiarTextoEmpresa`) por si se pega texto.
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters),
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().height(AlturaBusquedaBrisas),
            )
            if (resultados.isNotEmpty()) {
                Popup(
                    alignment = Alignment.TopStart,
                    offset = IntOffset(0, -(altoListaPx + with(densidad) { 4.dp.roundToPx() })),
                    properties = PopupProperties(focusable = false),
                ) {
                    Surface(
                        modifier = Modifier
                            .width(with(densidad) { anchoCampoPx.toDp() })
                            .onSizeChanged { altoListaPx = it.height },
                        shape = MaterialTheme.shapes.medium,
                        color = MaterialTheme.colorScheme.surface,
                        shadowElevation = 6.dp,
                    ) {
                        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            resultados.take(6).forEach { empresa ->
                                FilaEmpresaProveedor(empresa, onClick = { onElegir(empresa) })
                            }
                        }
                    }
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
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
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

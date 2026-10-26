package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Business
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

// Formulario de nuevo ingreso de proveedor (sacado de `PantallaProveedores.kt`,
// punto M6 de la auditoría móvil).

/// Formulario en pantalla completa (mismo criterio que
/// [PantallaNuevoContratista]: `PantallaProveedores` lo desmonta al volver,
/// así que no hace falta que su estado sobreviva más que eso).
///
/// Rediseño 2026-10-25, al estilo de los formularios de iOS: arriba el
/// escáner (casi siempre los datos los llena la cámara) y debajo grupos con
/// título — Persona, Empresa, Vehículo y gafete — con un renglón por dato y
/// su etiqueta, en vez de tarjetas numeradas con campos con borde. La
/// empresa se elige en el buscador de abajo, que ofrece crearla si no existe
/// (antes, una lista flotante encima del campo y un "+" al costado).
@Composable
internal fun FormularioNuevoIngresoProveedor(
    viewModel: ProveedoresViewModel,
    gafeteTexto: String,
    onCambiarGafeteTexto: (String) -> Unit,
    onEscanear: () -> Unit,
    onEscanearPlaca: () -> Unit,
    onVolver: () -> Unit,
) {
    val puedeRegistrar = viewModel.cedula.isNotBlank() && viewModel.nombre.isNotBlank() &&
        viewModel.empresaSeleccionada != null && gafeteTexto.isNotBlank() &&
        !viewModel.registrando && !viewModel.cedulaConIngresoActivo
    var buscandoEmpresa by remember { mutableStateOf(false) }
    val registrar = {
        gafeteTexto.toLongOrNull()?.let { numero ->
            // Al registrar, cierra y vuelve a la lista de activos (pedido
            // explícito del usuario en pruebas reales, 2026-09-17).
            viewModel.registrarIngreso(numero) {
                onCambiarGafeteTexto("")
                onVolver()
            }
        }
    }

    // Mientras se llenan los datos, las conexiones con la nube se calientan
    // (ver `ProveedoresViewModel.precalentarNube`).
    LaunchedEffect(Unit) { viewModel.precalentarNube() }

    // Mismo criterio que [PantallaRutas]: `imePadding()` va en un `Column`
    // SIN `fillMaxSize` -- combinarlo con `fillMaxSize` deja un hueco enorme
    // entre el teclado y el contenido (bug reportado 2026-09-20).
    //
    // Atrás del sistema (con su gesto) va al mismo lugar que "‹ Volver": sin
    // esto se escapaba a la Activity (hallazgo 2026-09-19).
    PantallaConGestoVolver(onVolver = onVolver) {
    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
    val scrollState = rememberScrollState()
    val scope = rememberCoroutineScope()
    Column(
        modifier = Modifier.verticalScroll(scrollState).imePadding(),
    ) {
        EncabezadoPantalla(titulo = "Ingreso de proveedor", onVolver = onVolver)

        TarjetaEscaner("Escanear cédula", "Llena la cédula y el nombre.", onClick = onEscanear)

        SeccionPersona(
            cedula = viewModel.cedula,
            nombre = viewModel.nombre,
            onCambiarCedula = viewModel::cambiarCedula,
            onCambiarNombre = viewModel::cambiarNombre,
        )

        SeccionFormulario("Empresa", pie = if (viewModel.creandoEmpresa) "Creando la empresa…" else null) {
            FilaSelector(
                etiqueta = "Empresa",
                valor = viewModel.empresaSeleccionada?.nombre,
                ayuda = "Elegir",
                onClick = { buscandoEmpresa = true },
                habilitado = !viewModel.creandoEmpresa,
            )
        }

        SeccionVehiculoYGafete(
            placa = viewModel.placa,
            onCambiarPlaca = viewModel::cambiarPlaca,
            onEscanearPlaca = onEscanearPlaca,
            gafeteTexto = gafeteTexto,
            onCambiarGafeteTexto = { onCambiarGafeteTexto(it.filter(Char::isDigit)) },
            // Al enfocar el gafete, lleva el scroll hasta el fondo: ahí vive
            // "Registrar ingreso" (pedido del usuario 2026-09-20: ver campo y
            // botón juntos). Se repite mientras dura la animación del teclado:
            // `maxValue` todavía no refleja el alto final en el instante del foco.
            onGafeteEnfocado = {
                scope.launch {
                    repeat(15) {
                        scrollState.animateScrollTo(scrollState.maxValue)
                        delay(30)
                    }
                }
            },
            onListo = { if (puedeRegistrar) registrar() },
        )

        viewModel.error?.let { mensaje ->
            FranjaEstado(mensaje, TipoEstado.ERROR, modifier = Modifier.padding(top = 16.dp), vibrar = true)
        }
        viewModel.mensaje?.let { mensaje ->
            FranjaEstado(mensaje, TipoEstado.EXITO, modifier = Modifier.padding(top = 16.dp))
        }

        BotonBrisas(
            onClick = { registrar() },
            enabled = puedeRegistrar,
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text(if (viewModel.registrando) "Registrando…" else "Registrar ingreso")
        }
    }
    }

    // La empresa, en el buscador de abajo como el resto de las búsquedas. Si
    // no existe, el primer resultado ofrece crearla.
    BuscadorInferior(
        visible = buscandoEmpresa,
        texto = viewModel.textoEmpresa,
        onCambiar = viewModel::cambiarTextoEmpresa,
        ayuda = "Nombre de la empresa",
        onCerrar = { buscandoEmpresa = false },
        // El teclado abre ya en mayúscula; el ViewModel igual lo convierte
        // (`cambiarTextoEmpresa`) por si se pega texto.
        teclado = KeyboardOptions(capitalization = KeyboardCapitalization.Characters),
    ) {
        Box(modifier = Modifier.padding(horizontal = 16.dp)) {
            val texto = viewModel.textoEmpresa.trim()
            // Con una empresa ya elegida la lista queda vacía: no se ofrece
            // crearla de nuevo (bug de 2026-09-17, creaba duplicados).
            val ofrecerCrear = texto.isNotEmpty() && viewModel.resultadosEmpresa.isEmpty() &&
                viewModel.empresaSeleccionada?.nombre != texto.uppercase()
            if (texto.isEmpty()) {
                EstadoVacio("Busque la empresa", "Escriba el nombre; si no existe, la puede crear aquí.", Icons.Default.Business)
            } else {
                LazyColumn(contentPadding = PADDING_RESULTADOS, verticalArrangement = Arrangement.spacedBy(5.dp)) {
                    if (ofrecerCrear) {
                        item(key = "crear") {
                            FilaCrear(texto.uppercase()) {
                                viewModel.crearEmpresa(texto)
                                buscandoEmpresa = false
                            }
                        }
                    }
                    items(viewModel.resultadosEmpresa, key = { it.id }) { empresa ->
                        FilaResultado(empresa.nombre) {
                            viewModel.elegirEmpresa(empresa)
                            buscandoEmpresa = false
                        }
                    }
                }
            }
        }
    }
    }
}

/// Grupo "Persona": cédula y nombre, cada uno con su etiqueta. Lo comparten
/// proveedores y correo; casi siempre lo llena el escáner de arriba.
@Composable
internal fun SeccionPersona(
    cedula: String,
    nombre: String,
    onCambiarCedula: (String) -> Unit,
    onCambiarNombre: (String) -> Unit,
    titulo: String = "Persona",
) {
    SeccionFormulario(titulo) {
        CampoAgrupado(
            valor = cedula,
            onCambiar = onCambiarCedula,
            etiqueta = "Cédula",
            ayuda = "Número",
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        )
        DivisorGrupo()
        CampoAgrupado(
            valor = nombre,
            onCambiar = onCambiarNombre,
            etiqueta = "Nombre",
            ayuda = "Nombre completo",
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words),
        )
    }
}

/// Grupo "Vehículo y gafete": la placa (opcional, con la cámara al final del
/// renglón: mismo lector que Rutas, `LectorVehiculoRuta.kt`) y el número de
/// gafete (siempre tipeado: no sale de ningún documento).
@Composable
internal fun SeccionVehiculoYGafete(
    placa: String,
    onCambiarPlaca: (String) -> Unit,
    onEscanearPlaca: () -> Unit,
    gafeteTexto: String,
    onCambiarGafeteTexto: (String) -> Unit,
    onGafeteEnfocado: () -> Unit,
    ayudaGafete: String = "Número",
    // La tecla ✓ del teclado en el gafete (lo último que se escribe):
    // registra si ya no falta nada.
    onListo: () -> Unit = {},
) {
    SeccionFormulario("Vehículo y gafete") {
        CampoAgrupado(
            valor = placa,
            onCambiar = onCambiarPlaca,
            etiqueta = "Placa",
            ayuda = "Opcional",
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters),
            alFinal = { BotonIconoFila(Icons.Default.PhotoCamera, "Escanear placa", onEscanearPlaca) },
        )
        DivisorGrupo()
        CampoAgrupado(
            valor = gafeteTexto,
            onCambiar = onCambiarGafeteTexto,
            etiqueta = "Gafete",
            ayuda = ayudaGafete,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { onListo() }),
            modifier = Modifier.onFocusChanged { if (it.isFocused) onGafeteEnfocado() },
        )
    }
}

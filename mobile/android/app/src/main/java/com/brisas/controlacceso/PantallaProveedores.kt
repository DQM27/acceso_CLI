package com.brisas.controlacceso

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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PersonAdd
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.Nucleo
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
    refrescarNube: Int = 0,
) {
    RegistrarPantalla("proveedores")
    val viewModel: ProveedoresViewModel =
        viewModel(factory = ProveedoresViewModel.factory(nucleo))
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
                // Como va impresa en la placa (`CL 371931`, `BPH-485`).
                viewModel.cambiarPlaca(placaComoSeImprime(detectado.valor))
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

    // Sin título propio: el selector de [PantallaExternos] ya dice dónde se
    // está.
    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
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

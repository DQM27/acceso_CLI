package com.brisas.controlacceso

import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PersonAdd
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
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

/// Sin distinguir mayúsculas ni tildes, igual que lo que se resalta
/// ([rangosCoincidencia]).
private fun FilaProveedorActiva.coincideCon(busqueda: String): Boolean {
    if (busqueda.isBlank()) return true
    val campos = when (this) {
        is FilaProveedorActiva.Local -> listOf(registro.cedula, registro.nombre, registro.empresaNombre, registro.placa)
        is FilaProveedorActiva.Remota -> listOf(remoto.cedula, remoto.nombre, remoto.empresaNombre, remoto.placa)
    }
    return campos.filterNotNull().any { rangosCoincidencia(it, busqueda).isNotEmpty() }
}

private const val AYUDA_BUSQUEDA_PROVEEDORES = "Cédula, nombre, empresa o placa"

/// La lista de proveedores adentro, la de la pantalla y la del buscador.
@Composable
private fun ListaProveedoresActivos(
    filas: List<FilaProveedorActiva>,
    cargando: Boolean,
    busqueda: String,
    sincronizando: Boolean,
    onSincronizar: () -> Unit,
    contentPadding: PaddingValues,
    onElegir: (FilaProveedorActiva) -> Unit,
) {
    ListaConEstados(
        filas = filas,
        clave = { it.clave() },
        cargando = cargando,
        tituloVacio = if (busqueda.isBlank()) "Ningún proveedor adentro" else "Sin resultados",
        detalleVacio = if (busqueda.isBlank()) "Cuando entre uno, aparece aquí." else "Pruebe con otra cédula, nombre o empresa.",
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
        contentPadding = contentPadding,
        iconoVacio = IconosLucide.Cajas,
    ) { fila ->
        FilaProveedorActivo(fila = fila, onConfirmarSalida = { onElegir(fila) })
    }
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
    // Tirar de la lista hacia abajo sincroniza con la nube, igual que el botón
    // del encabezado de la pantalla principal (que es quien sabe hacerlo).
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
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
    var buscando by remember { mutableStateOf(false) }
    var escaneando by remember { mutableStateOf(false) }
    // "+" abre la cámara de una vez (el 99% de las veces se escanea): ahí la
    // X cancela el ingreso y "Llenar a mano" lleva al formulario.
    var escaneoDirecto by remember { mutableStateOf(false) }
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
    // Se escaneó a alguien que ya está adentro (como proveedor o por otra
    // vía): en vez del formulario, el motivo, como en Contratistas.
    var bloqueoEscaneo by remember { mutableStateOf<String?>(null) }
    val contexto = LocalContext.current

    // Qué panel se ve ahora: la lista, el formulario o uno de los escáneres.
    // La lista y el formulario se abren y cierran con una transición corta; los
    // escáneres cambian al instante (ver `TransicionesPantalla.kt`).
    val panel = panelConFormulario(
        mostrandoFormulario = mostrandoFormulario,
        escaneandoCedula = escaneando,
        escaneandoPlaca = escaneandoPlaca,
    )
    AnimatedContent(
        targetState = panel,
        modifier = Modifier.fillMaxSize(),
        transitionSpec = { transicionDePanel(initialState, targetState) },
        label = "panelProveedores",
    ) { mostrado ->
        when (mostrado) {
            PanelConFormulario.ESCANER_CEDULA -> {
                PantallaEscanearCedula(
                    modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
                    onDocumentoDetectado = { documento ->
                        val numero = documento.numeroDocumento.filter(Char::isDigit)
                            .ifBlank { documento.numeroDocumento }
                        // La cámara se queda hasta saber si ya está adentro:
                        // si lo está, frena ahí con la alarma de denegado, sin
                        // abrir el formulario para llenarlo en balde.
                        val aviso = viewModel.rellenarDesdeDocumento(numero, documento.nombre, documento.apellidos)
                        if (aviso != null) {
                            sonarAlertaEscaneo(contexto, AlertaEscaneo.DENEGADO)
                            bloqueoEscaneo = aviso
                        }
                        escaneando = false
                        escaneoDirecto = false
                    },
                    onCerrar = {
                        escaneando = false
                        if (escaneoDirecto) {
                            escaneoDirecto = false
                            viewModel.cancelarFormulario()
                            mostrandoFormulario = false
                        }
                    },
                    onLlenarAMano = if (escaneoDirecto) {
                        {
                            escaneando = false
                            escaneoDirecto = false
                        }
                    } else {
                        null
                    },
                )
            }
            PanelConFormulario.ESCANER_PLACA -> {
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
            }
            PanelConFormulario.FORMULARIO -> {
                // Pantalla completa: sin barra de secciones encima.
                OcultarBarraSecciones()
                val motivoBloqueo = bloqueoEscaneo
                if (motivoBloqueo != null) {
                    PantallaBloqueoIngreso(
                        titulo = viewModel.nombre.ifBlank { "Proveedor" },
                        subtitulo = viewModel.cedula,
                        mensaje = motivoBloqueo,
                        onVolver = {
                            bloqueoEscaneo = null
                            viewModel.cancelarFormulario()
                            gafeteTexto = ""
                            mostrandoFormulario = false
                        },
                        // Cerrada la otra salida, queda el formulario con lo leído.
                        salida = viewModel.salidaPorOtraVia,
                        estadoSalida = viewModel.estadoSalidaOtraVia,
                        onRegistrarSalida = { viewModel.registrarSalidaPorOtraVia { bloqueoEscaneo = null } },
                    )
                } else FormularioNuevoIngresoProveedor(
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
            }
            PanelConFormulario.LISTA -> Box(modifier = Modifier.fillMaxSize()) {
                    // Sin título propio: el de la barra de arriba ya dice dónde se
                    // está. Se busca con la píldora de abajo, como en todas las pestañas.
                    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
                        viewModel.error?.let { mensaje ->
                            FranjaEstado(mensaje, TipoEstado.ERROR, modifier = Modifier.padding(bottom = 4.dp))
                        }

                        ListaProveedoresActivos(
                            filas = viewModel.activos,
                            cargando = viewModel.cargando,
                            busqueda = "",
                            sincronizando = sincronizando,
                            onSincronizar = onSincronizar,
                            contentPadding = PaddingValues(top = 4.dp, bottom = EspacioPieLista),
                            onElegir = { filaParaSalida = it },
                        )
                    }

                    // "Nuevo ingreso" va en el botón redondo de abajo y la
                    // búsqueda en la píldora (ver `AccionInferior.kt`).
                    AccionPrincipal(Icons.Default.PersonAdd, "Nuevo ingreso de proveedor", onClick = {
                        mostrandoFormulario = true
                        escaneando = true
                        escaneoDirecto = true
                    })
                    // Sólo con alguien adentro: con la lista vacía no hay qué buscar.
                    BusquedaPrincipal(
                        ayuda = AYUDA_BUSQUEDA_PROVEEDORES,
                        onAbrir = { buscando = true },
                        visible = viewModel.activos.isNotEmpty(),
                    )

                    BuscadorInferior(
                        visible = buscando,
                        texto = busqueda,
                        onCambiar = { busqueda = it },
                        ayuda = AYUDA_BUSQUEDA_PROVEEDORES,
                        onCerrar = {
                            buscando = false
                            busqueda = ""
                        },
                    ) {
                        Box(modifier = Modifier.padding(horizontal = 16.dp)) {
                            ListaProveedoresActivos(
                                filas = remember(viewModel.activos, busqueda) { viewModel.activos.filter { it.coincideCon(busqueda) } },
                                cargando = viewModel.cargando,
                                busqueda = busqueda,
                                sincronizando = sincronizando,
                                onSincronizar = onSincronizar,
                                contentPadding = PADDING_RESULTADOS,
                                onElegir = { fila ->
                                    filaParaSalida = fila
                                    buscando = false
                                    busqueda = ""
                                },
                            )
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
        }
    }
}

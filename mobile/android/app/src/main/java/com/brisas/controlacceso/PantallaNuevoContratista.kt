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
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Business
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import java.util.UUID
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.TipoIngreso

/// Mismo formulario que desktop/src/pantallas/FormularioContratista.tsx —
/// sólo alta, no edición (ver docs/plan-app-movil.md). La validación real
/// vuelve a correr en Rust (ContratistaService::crear); lo de acá es sólo
/// feedback inmediato, igual que el esquema de zod del lado desktop.
///
/// Recuperada 2026-09-12 tras haberse sacado el 2026-09-06 (ver
/// ARQUITECTURA.md) -- con una segunda vía de captura además del tipeo
/// manual: escanear el carnet PRAIND del contratista con la misma cámara/OCR
/// que ya usa `PantallaEscanearCedula` para confirmar ingresos. El carnet
/// PRAIND es el único tipo de documento que trae, además de cédula y nombre,
/// la empresa y el vencimiento de la inducción -- por eso es el que de verdad
/// ahorra tipeo acá (ver `LectorDocumentosIdentidad.extraerPraind`).
private fun etiquetaTipo(tipo: TipoIngreso): String =
    when (tipo) {
        TipoIngreso.PRAIND -> "PRAIND"
        TipoIngreso.IN_HOUSE -> "IN HOUSE"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }

/// Sin "Por correo": dejó de ser un tipo de contratista (2026-10-03); esas
/// visitas van en Externos → Por correo.
private val TIPOS_CONTRATISTA = TipoIngreso.entries.filter { it != TipoIngreso.POR_CORREO }

/// Estado y llamadas al núcleo en [NuevoContratistaViewModel] (punto M1
/// de la auditoría móvil); acá sólo lo visual. Cada apertura del
/// formulario es un intento fresco: la clave del ViewModel se guarda con
/// `rememberSaveable` (sobrevive a una rotación) y cambia cada vez que la
/// pantalla vuelve a entrar en composición.
///
/// Rediseño 2026-10-25, al estilo de los formularios de iOS: arriba el
/// escáner del carnet (el 99% de las veces llena todo), y debajo grupos con
/// título — Persona, Acceso, PRAIND — con un renglón por dato. La empresa se
/// elige en el buscador de abajo (antes, un desplegable con todas), el tipo
/// de ingreso con el control segmentado, "Personal de ruta" con un
/// interruptor y el vencimiento con un selector de fecha. Lo que llena el
/// escáner entra igual en todos: el ViewModel no cambió.
///
/// Menos pasos (2026-10-26): se abre directo en la cámara del carnet ("Llenar
/// a mano" lleva al formulario, la X cancela), arranca con lo que se buscó
/// o se leyó en Activos ([textoInicial]: la cédula o el nombre) y, al
/// guardar, [onGuardado] recibe el contratista nuevo para darle el ingreso
/// enseguida, sin volver a buscarlo.
@Composable
fun PantallaNuevoContratista(
    nucleo: Nucleo,
    onVolver: () -> Unit,
    onGuardado: (contratistaId: Long) -> Unit,
    textoInicial: String = "",
) {
    RegistrarPantalla("nuevo_contratista")
    val claveFormulario = rememberSaveable { "nuevo-contratista-${UUID.randomUUID()}" }
    val vm: NuevoContratistaViewModel = viewModel(
        key = claveFormulario,
        factory = NuevoContratistaViewModel.factory(nucleo),
    )
    LaunchedEffect(vm) { vm.prellenar(textoInicial) }
    var escaneando by rememberSaveable { mutableStateOf(true) }
    // La cámara que se abre sola al entrar: su X cancela todo.
    var escaneoDirecto by rememberSaveable { mutableStateOf(true) }
    var buscandoEmpresa by remember { mutableStateOf(false) }
    var textoEmpresa by remember { mutableStateOf("") }
    var eligiendoFecha by remember { mutableStateOf(false) }

    if (escaneando) {
        PantallaEscanearCedula(
            modo = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
            onDocumentoDetectado = { documento ->
                escaneando = false
                escaneoDirecto = false
                vm.aplicarDocumentoEscaneado(documento)
            },
            onCerrar = {
                escaneando = false
                if (escaneoDirecto) onVolver()
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
        return
    }

    // Mismo criterio que [PantallaRutas]/[PantallaProveedores]:
    // `imePadding()` va en un Column SIN `fillMaxSize` -- combinarlo con
    // `fillMaxSize` deja un hueco enorme entre el teclado y el contenido
    // (bug reportado 2026-09-20).
    //
    // Atrás del sistema (con su gesto) va al mismo lugar que "‹ Volver": sin
    // esto se escapaba a la Activity (hallazgo 2026-09-19). Con la cámara o el
    // buscador abiertos manda el atrás de ellos (se registran después).
    PantallaConGestoVolver(onVolver = onVolver) {
    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 6.dp)) {
    Column(
        modifier = Modifier.verticalScroll(rememberScrollState()).imePadding(),
    ) {
        EncabezadoPantalla(titulo = "Nuevo contratista", onVolver = onVolver)

        TarjetaEscaner(
            "Escanear carnet PRAIND",
            "Llena cédula, nombre, empresa y vencimiento.",
            onClick = {
                // Un solo escaneo llena TODOS los campos, así que se parte de
                // cero (a diferencia de Proveedores/Rutas, donde cada escáner
                // llena un solo dato).
                vm.limpiarParaEscanear()
                escaneando = true
            },
        )

        SeccionFormulario("Persona") {
            CampoAgrupado(
                valor = vm.cedula,
                onCambiar = vm::cambiarCedula,
                etiqueta = "Cédula",
                ayuda = "Número",
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            )
            DivisorGrupo()
            CampoAgrupado(
                valor = vm.nombre,
                onCambiar = vm::cambiarNombre,
                etiqueta = "Nombre",
                ayuda = "Nombre completo",
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words),
            )
        }

        // Sin "Con acceso" -- pedido explícito del usuario 2026-09-20:
        // registrar a alguien acá es siempre en persona, frente al guardia, así
        // que el acceso ya está implícito.
        //
        // "Personal de ruta" sólo para PRAIND/IN HOUSE (pedido del usuario
        // 2026-09-20; lo decide el núcleo con `muestraPersonalRuta`).
        val sugerenciaEmpresa = vm.empresaSugeridaTexto
        SeccionFormulario(
            "Acceso",
            pie = sugerenciaEmpresa?.let { "El carnet dice «$it»: no hay una empresa igual, elija la correcta." },
            pieEsError = true,
        ) {
            FilaSelector(
                etiqueta = "Empresa",
                valor = vm.empresaSeleccionada?.nombre,
                ayuda = "Elegir",
                onClick = {
                    // Arranca con lo que leyó el escáner, si no encontró la empresa.
                    textoEmpresa = if (vm.empresaSeleccionada == null) sugerenciaEmpresa.orEmpty() else ""
                    buscandoEmpresa = true
                },
                error = sugerenciaEmpresa != null && vm.empresaSeleccionada == null,
            )
            DivisorGrupo()
            Box(modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp)) {
                FilaPildoras(
                    opciones = TIPOS_CONTRATISTA.map(::etiquetaTipo),
                    seleccionado = TIPOS_CONTRATISTA.indexOf(vm.tipoIngreso),
                    onSeleccionar = { vm.cambiarTipoIngreso(TIPOS_CONTRATISTA[it]) },
                )
            }
            if (vm.muestraPersonalRuta) {
                DivisorGrupo()
                FilaInterruptor("Personal de ruta", vm.personalRuta, vm::cambiarPersonalRuta)
            }
        }

        // Vencido: se avisa acá mismo, sin esperar a que Rust lo rechace al
        // guardar, y el guardado queda bloqueado (pedido del usuario 2026-09-20).
        val praindVencido = vm.praindVencido
        if (vm.requierePraind) {
            SeccionFormulario(
                "PRAIND",
                pie = if (praindVencido) "PRAIND vencido: elija una fecha vigente." else null,
                pieEsError = true,
            ) {
                FilaSelector(
                    etiqueta = "Vence",
                    valor = textoFechaLegible(vm.fechaPraind),
                    ayuda = "Elegir fecha",
                    onClick = { eligiendoFecha = true },
                    error = praindVencido,
                )
            }
        }

        val mensajeError = vm.error
        if (mensajeError != null) {
            FranjaEstado(mensajeError, TipoEstado.ERROR, modifier = Modifier.padding(top = 16.dp), vibrar = true)
        }
        val mensajeExito = vm.mensaje
        if (mensajeExito != null) {
            FranjaEstado(mensajeExito, TipoEstado.EXITO, modifier = Modifier.padding(top = 16.dp))
        }

        BotonBrisas(
            onClick = { vm.guardar(onGuardado = onGuardado) },
            enabled = !vm.enviando && !praindVencido,
            // Sin padding inferior: sumado al `imePadding()` del teclado dejaba
            // un hueco de más entre el botón y el teclado (reportado 2026-09-20).
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text(if (vm.enviando) "Guardando…" else "Guardar")
        }
    }
    }

    // La empresa, en el buscador de abajo como el resto de las búsquedas. Las
    // empresas de contratistas se dan de alta en el panel web, no acá.
    BuscadorInferior(
        visible = buscandoEmpresa,
        texto = textoEmpresa,
        onCambiar = { textoEmpresa = it },
        ayuda = "Nombre de la empresa",
        onCerrar = { buscandoEmpresa = false },
    ) {
        Box(modifier = Modifier.padding(horizontal = 16.dp)) {
            val coincidentes = remember(vm.empresas, textoEmpresa) {
                vm.empresas.filter { textoEmpresa.isBlank() || rangosCoincidencia(it.nombre, textoEmpresa).isNotEmpty() }
            }
            if (coincidentes.isEmpty()) {
                EstadoVacio("Sin resultados", "Pruebe con otro nombre. Las empresas nuevas se dan de alta en el panel.", Icons.Default.Business)
            } else {
                LazyColumn(contentPadding = PADDING_RESULTADOS, verticalArrangement = Arrangement.spacedBy(5.dp)) {
                    items(coincidentes, key = { it.id }) { empresa ->
                        FilaResultado(empresa.nombre) {
                            vm.elegirEmpresa(empresa)
                            buscandoEmpresa = false
                        }
                    }
                }
            }
        }
    }
    }

    // El vencimiento en un selector de fecha. El escáner lo sigue llenando
    // solo (mismo DD-MM-AAAA en el ViewModel); esto es para corregirlo.
    if (eligiendoFecha) {
        HojaFecha(
            titulo = "Vencimiento PRAIND",
            fecha = vm.fechaPraind,
            onElegir = vm::cambiarFechaPraind,
            onCerrar = { eligiendoFecha = false },
        )
    }
}

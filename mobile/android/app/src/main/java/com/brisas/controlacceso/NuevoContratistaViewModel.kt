package com.brisas.controlacceso

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.DatosContratista
import uniffi.control_acceso_mobile.Empresa
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.TipoIngreso

/// Dueño del estado y de las llamadas a [Nucleo] del formulario "Nuevo
/// contratista" (punto M1 de
/// `docs/auditorias/auditoria-movil-arquitectura-2026-09-27.md`) -- mismo
/// criterio que [ActivosViewModel]/[RutasViewModel]: el `@Composable` sólo
/// lee este estado y reporta eventos. Antes la pantalla llamaba al núcleo
/// directo, manejaba sus corrutinas y decidía si el PRAIND estaba vencido.
///
/// Sólo alta, no edición (ver docs/plan-app-movil.md). Este ViewModel no
/// decide ninguna regla de negocio: `requierePraind`, `praindVencido` y
/// `muestraPersonalRuta` le preguntan al núcleo (`domain::contratista`), y
/// al guardar `ContratistaService::crear_en_persona` vuelve a aplicarlas
/// todas (además deja el acceso habilitado). Lo de acá es sólo el aviso
/// inmediato mientras se completa el formulario.
class NuevoContratistaViewModel(
    private val nucleo: Nucleo,
    // Inyectable para tests con un dispatcher de tiempo controlado, mismo
    // motivo que en `ActivosViewModel`.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var empresas by mutableStateOf<List<Empresa>>(emptyList())
        private set
    var cedula by mutableStateOf("")
        private set
    var nombre by mutableStateOf("")
        private set
    var empresaSeleccionada by mutableStateOf<Empresa?>(null)
        private set

    /// Nombre de empresa leído del carnet cuando no calzó con ninguna de
    /// [empresas] -- se muestra como pista para elegir a mano, nunca se
    /// manda a Rust (que sólo acepta un `empresa_id` real).
    var empresaSugeridaTexto by mutableStateOf<String?>(null)
        private set
    var tipoIngreso by mutableStateOf(TipoIngreso.PRAIND)
        private set
    var personalRuta by mutableStateOf(false)
        private set
    var fechaPraind by mutableStateOf("")
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var mensaje by mutableStateOf<String?>(null)
        private set
    var enviando by mutableStateOf(false)
        private set

    val requierePraind: Boolean
        get() = medirNucleo("requierePraindParaFormulario") { nucleo.requierePraindParaFormulario(tipoIngreso, personalRuta) }

    /// Vencido en el instante en que se tipea, no recién al guardar. Una
    /// fecha a medio escribir ("10-09-202") el núcleo la toma como no
    /// vencida. Bloquea el botón Guardar (pedido del usuario 2026-09-20).
    val praindVencido: Boolean
        get() = requierePraind && fechaPraind.isNotBlank() &&
            medirNucleo("praindVencidoParaFormulario") { nucleo.praindVencidoParaFormulario(textoDDMMYYYYaIso(fechaPraind)) }

    val muestraPersonalRuta: Boolean
        get() = medirNucleo("admitePersonalRutaParaFormulario") { nucleo.admitePersonalRutaParaFormulario(tipoIngreso) }

    init {
        viewModelScope.launch {
            try {
                empresas = withContext(dispatcherIO) { medirNucleo("listarEmpresas") { nucleo.listarEmpresas() } }
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            }
        }
    }

    fun cambiarCedula(texto: String) {
        cedula = texto.filter(Char::isDigit)
    }

    /// Siempre en mayúscula, venga del teclado o del OCR (pedido del
    /// usuario 2026-09-20).
    fun cambiarNombre(texto: String) {
        nombre = texto.uppercase()
    }

    fun elegirEmpresa(empresa: Empresa) {
        empresaSeleccionada = empresa
        empresaSugeridaTexto = null
    }

    fun cambiarTipoIngreso(tipo: TipoIngreso) {
        tipoIngreso = tipo
        // El check se oculta para estos dos tipos: sin este reseteo un
        // `true` que quedó de un tipo anterior seguía contando para
        // `requierePraind` sin poder destildarse.
        if (!muestraPersonalRuta) personalRuta = false
    }

    fun cambiarPersonalRuta(valor: Boolean) {
        personalRuta = valor
    }

    fun cambiarFechaPraind(texto: String) {
        fechaPraind = formatearFechaDDMMYYYY(texto)
    }

    /// Todo en blanco antes de abrir la cámara: si se escanea un segundo
    /// carnet (persona equivocada, mala lectura) no se mezclan los datos del
    /// primero (pedido del usuario 2026-09-20).
    fun limpiarParaEscanear() {
        cedula = ""
        nombre = ""
        empresaSeleccionada = null
        empresaSugeridaTexto = null
        tipoIngreso = TipoIngreso.PRAIND
        personalRuta = false
        fechaPraind = ""
        error = null
        mensaje = null
    }

    /// Sólo el carnet PRAIND rellena el formulario: es el único documento
    /// que trae, además de cédula y nombre, la empresa y el vencimiento de
    /// la inducción. Cualquier otro se rechaza entero, sin tocar campos
    /// (pedido del usuario 2026-09-20).
    fun aplicarDocumentoEscaneado(documento: DocumentoDetectado) {
        mensaje = null
        if (documento.tipo != TipoDocumento.CARNET_INDUCCION_PRAIND) {
            error = "Documento inválido"
            return
        }
        error = null
        if (documento.numeroDocumento.isNotBlank()) {
            cedula = documento.numeroDocumento.filter(Char::isDigit).ifBlank { documento.numeroDocumento }
        }
        documento.nombre?.let { nombre = it.uppercase() }
        tipoIngreso = TipoIngreso.PRAIND
        documento.vencimiento?.let { fechaPraind = it.aTextoDDMMYYYY() }
        val textoEmpresa = documento.empresa?.trim()
        if (!textoEmpresa.isNullOrBlank()) {
            val coincidencia = buscarEmpresaPorNombre(empresas, textoEmpresa)
            if (coincidencia != null) elegirEmpresa(coincidencia) else empresaSugeridaTexto = textoEmpresa
        }
        mensaje = "Carnet PRAIND leído — revise los datos antes de guardar"
    }

    /// Guarda y, si sale bien, llama a [onGuardado] (la pantalla vuelve a
    /// la lista de activos, pedido del usuario 2026-09-17). Corre en
    /// `viewModelScope`: salir de la pantalla a mitad de camino ya no
    /// cancela el aviso a la nube (antes hacía falta `NonCancellable`).
    fun guardar(onGuardado: () -> Unit) {
        error = null
        mensaje = null
        // Sin empresa no hay `empresa_id` que mandar; el resto (cédula,
        // nombre, PRAIND) lo valida el núcleo y su mensaje se muestra tal cual.
        val empresa = empresaSeleccionada
        if (empresa == null) {
            error = "Elija la empresa"
            return
        }
        if (enviando) return
        enviando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    medirNucleo("crearContratista") { nucleo.crearContratista(
                        DatosContratista(
                            cedula = cedula,
                            nombre = nombre,
                            empresaId = empresa.id,
                            tipoIngreso = tipoIngreso,
                            fechaVencimientoPraind = fechaPraind.ifBlank { null }?.let(::textoDDMMYYYYaIso),
                            esPersonalRuta = personalRuta,
                        ),
                    ) }
                }
                CambiosNube.cambioLocal()
                onGuardado()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                enviando = false
            }
        }
    }

    companion object {
        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { NuevoContratistaViewModel(nucleo) }
        }
    }
}

/// Máscara DD-MM-AAAA del vencimiento PRAIND: descarta lo que no sea dígito
/// y reinserta los guiones mientras se tipea (pedido del usuario
/// 2026-09-21: sin esto había que tipear los guiones con el teclado
/// completo en vez del numérico).
internal fun formatearFechaDDMMYYYY(texto: String): String {
    val digitos = texto.filter(Char::isDigit).take(8)
    return buildString {
        for (indice in digitos.indices) {
            append(digitos[indice])
            if (indice == 1 || indice == 3) append('-')
        }
    }
}

/// Texto libre del carnet contra los nombres reales de las empresas:
/// primero igualdad exacta sin distinguir mayúsculas, después "una
/// contiene a la otra" (tolera un "S.A." de más o de menos).
internal fun buscarEmpresaPorNombre(empresas: List<Empresa>, textoCarnet: String): Empresa? {
    val normalizado = textoCarnet.trim()
    if (normalizado.isEmpty()) return null
    return empresas.firstOrNull { it.nombre.equals(normalizado, ignoreCase = true) }
        ?: empresas.firstOrNull {
            it.nombre.contains(normalizado, ignoreCase = true) ||
                normalizado.contains(it.nombre, ignoreCase = true)
        }
}

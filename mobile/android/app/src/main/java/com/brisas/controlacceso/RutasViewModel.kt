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
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.DecisionRetornoViaje
import uniffi.control_acceso_mobile.EncargadoRuta
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.Ruta
import uniffi.control_acceso_mobile.SalidaRutaActivaResumen
import uniffi.control_acceso_mobile.SolicitudDocumentoRuta
import uniffi.control_acceso_mobile.SolicitudSalidaRuta
import uniffi.control_acceso_mobile.TramoRutaResumen
import uniffi.control_acceso_mobile.VehiculoRuta
import uniffi.control_acceso_mobile.ViajeRuta

/// Un documento en construcción dentro del formulario de salida -- 1+ por
/// solicitud (confirmado explícito del usuario: todos se declaran juntos,
/// al momento de la salida, ver
/// `docs/planes-implementados/plan-control-rutas.md`). `id` es sólo una
/// clave estable para la UI (`items(documentos, key = { it.id })`), nunca
/// viaja al núcleo.
///
/// `fechaTexto` arranca vacío a propósito -- nunca defaultear a "hoy": es
/// el arreglo del hallazgo de esta ronda (el bloqueo por fecha vencida no
/// se activaba para un documento cuya fecha no se pudo leer del OCR, sin
/// que nadie se enterara). El campo se queda vacío hasta que el OCR lo
/// llene o el guardia lo escriba a mano.
data class BorradorDocumentoRuta(
    val id: Long,
    val textoRuta: String = "",
    val resultadosRuta: List<Ruta> = emptyList(),
    val rutaSeleccionada: Ruta? = null,
    val subNumero: Int = 1,
    val numeroDocumento: String = "",
    val fechaTexto: String = "",
)

/// Dueño del estado real de [PantallaRutas] y de las llamadas a [Nucleo] --
/// mismo criterio que [ActivosViewModel] (ver su doc-comment): el
/// `@Composable` sólo lee este estado y reporta eventos, ninguna decisión
/// de negocio ni llamada a `Nucleo` vive del lado de la UI. Ver
/// `docs/planes-implementados/plan-control-rutas.md`.
class RutasViewModel(
    private val nucleo: Nucleo,
    // Inyectable para tests con un dispatcher de tiempo controlado, mismo
    // motivo que en `ActivosViewModel`.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var activas by mutableStateOf<List<SalidaRutaActivaResumen>>(emptyList())
        private set
    /// Historial completo de cada viaje visible en [activas] (tramos ya
    /// retornados hoy + el que sigue abierto), llave = `viajeId` -- para
    /// la tarjeta agrupada por unidad (mockup "Opción A" ya aprobado).
    /// Casi siempre 1 solo tramo por viaje (un vehículo no puede tener dos
    /// tramos abiertos a la vez), pero puede traer 2+ si hubo una recarga
    /// ("sí, misma ruta") antes de este tramo.
    var tramosPorViaje by mutableStateOf<Map<Long, List<TramoRutaResumen>>>(emptyMap())
        private set
    var cargando by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var mensaje by mutableStateOf<String?>(null)
        private set
    var registrando by mutableStateOf(false)
        private set

    // Buscador de encargado (paso 1) -- por nombre o código de empleado,
    // pedido explícito del usuario, 2026-09-15: "que funcione de las dos
    // formas, como ahora funciona contratista, que busca por nombre o por
    // número de cédula". BLOQUEANTE desde 2026-09-19 (mismo pedido que ya
    // se le hizo al buscador de vehículo: "más de lo mismo, debe ser un
    // buscador") -- el paso sólo se da por completo cuando hay un
    // [EncargadoRuta] real elegido de `resultadosEncargado`, nunca por el
    // sólo hecho de que el campo de texto no esté vacío.
    var textoEncargado by mutableStateOf("")
        private set
    var resultadosEncargado by mutableStateOf<List<EncargadoRuta>>(emptyList())
        private set
    var encargadoSeleccionado by mutableStateOf<EncargadoRuta?>(null)
        private set
    private var trabajoBusquedaEncargado: Job? = null

    // Documento(s) de ruta (paso 2) -- lista repetible, 1+ por solicitud,
    // ver el doc-comment de [BorradorDocumentoRuta]. El número de ruta
    // sigue BLOQUEANTE por documento (pedido explícito del usuario,
    // 2026-09-15).
    var documentos by mutableStateOf(listOf(BorradorDocumentoRuta(id = 0L)))
        private set
    private var siguienteIdDocumento = 1L
    private val trabajosBusquedaRutaDocumento = mutableMapOf<Long, Job>()
    var tieneCorreo by mutableStateOf(false)
        private set

    // Buscador de vehículo (paso 3) -- por placa o número de unidad,
    // BLOQUEANTE (pedido explícito del usuario, 2026-09-19: "es un buscador,
    // no se puede poner lo que uno quiera, sólo lo que la tabla
    // proporciona") -- mismo criterio que el buscador de número de ruta, no
    // el de encargado.
    var textoVehiculo by mutableStateOf("")
        private set
    var resultadosVehiculo by mutableStateOf<List<VehiculoRuta>>(emptyList())
        private set
    var vehiculoSeleccionado by mutableStateOf<VehiculoRuta?>(null)
        private set
    private var trabajoBusquedaVehiculo: Job? = null

    // "+ Nuevo tramo" -- si la unidad elegida ya tiene un viaje abierto
    // (recarga: volvió y el guardia dijo "sí, misma ruta"), se ofrece
    // continuarlo en vez de abrir uno nuevo. `continuarViaje` es la
    // elección explícita del guardia -- nunca se asume sola, aunque haya
    // un viaje abierto detectado (confirmado: la decisión real vive en
    // que la placa/encargado coincidan exacto, y eso lo valida el núcleo).
    var viajeAbiertoParaVehiculo by mutableStateOf<ViajeRuta?>(null)
        private set
    var continuarViaje by mutableStateOf(false)
        private set
    private var trabajoBusquedaViajeAbierto: Job? = null

    init {
        refrescarActivas()
    }

    fun refrescarActivas() {
        viewModelScope.launch {
            cargando = true
            try {
                val nuevasActivas = withContext(dispatcherIO) { nucleo.listarRutasActivas() }
                activas = nuevasActivas
                tramosPorViaje = withContext(dispatcherIO) {
                    nuevasActivas
                        .map { it.viajeId }
                        .distinct()
                        .associateWith { viajeId -> nucleo.listarTramosRutaDeViaje(viajeId) }
                }
                error = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                cargando = false
            }
        }
    }

    /// Escribir de nuevo abandona cualquier selección previa -- mismo
    /// criterio que `cambiarFiltro` en `NuevoIngresoModal.tsx`: no dejar un
    /// match viejo pegado detrás de un texto que ya cambió de sentido.
    fun cambiarTextoEncargado(nuevo: String) {
        textoEncargado = nuevo
        encargadoSeleccionado = null
        trabajoBusquedaEncargado?.cancel()
        if (nuevo.isBlank()) {
            resultadosEncargado = emptyList()
            return
        }
        trabajoBusquedaEncargado = viewModelScope.launch {
            delay(DEBOUNCE_MS)
            try {
                resultadosEncargado = withContext(dispatcherIO) { nucleo.buscarEncargadosRuta(nuevo) }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    fun elegirEncargado(encargado: EncargadoRuta) {
        trabajoBusquedaEncargado?.cancel()
        textoEncargado = encargado.nombre
        encargadoSeleccionado = encargado
        resultadosEncargado = emptyList()
    }

    /// Autocompleta el buscador de encargado con lo que trajo el OCR del
    /// gafete KOF (código de empleado o nombre, ver
    /// `PantallaEscanearCarnetKof.kt`) y dispara la búsqueda -- mismo
    /// criterio que [usarVehiculoEscaneado]: si hay una única coincidencia
    /// exacta, la elige sola; si no, deja los resultados para que el
    /// guardia elija a mano.
    fun usarEncargadoEscaneado(texto: String) {
        cambiarTextoEncargado(texto)
        trabajoBusquedaEncargado?.cancel()
        trabajoBusquedaEncargado = viewModelScope.launch {
            try {
                val resultados = withContext(dispatcherIO) { nucleo.buscarEncargadosRuta(texto) }
                resultadosEncargado = resultados
                resultados
                    .singleOrNull { it.codigoEmpleado == texto || it.nombre == texto }
                    ?.let { elegirEncargado(it) }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    // ---- Documento(s) de ruta ----

    fun agregarDocumento() {
        documentos = documentos + BorradorDocumentoRuta(id = siguienteIdDocumento++)
    }

    /// Nunca deja la lista vacía -- una salida siempre necesita al menos
    /// un documento (mismo resguardo que ya exige el núcleo,
    /// `RutaServiceError::SinDocumentos`).
    fun quitarDocumento(id: Long) {
        if (documentos.size <= 1) return
        documentos = documentos.filterNot { it.id == id }
        trabajosBusquedaRutaDocumento.remove(id)?.cancel()
    }

    private fun actualizarDocumento(id: Long, transformar: (BorradorDocumentoRuta) -> BorradorDocumentoRuta) {
        documentos = documentos.map { if (it.id == id) transformar(it) else it }
    }

    fun cambiarTextoRutaDocumento(id: Long, nuevo: String) {
        actualizarDocumento(id) { it.copy(textoRuta = nuevo, rutaSeleccionada = null) }
        trabajosBusquedaRutaDocumento[id]?.cancel()
        if (nuevo.isBlank()) {
            actualizarDocumento(id) { it.copy(resultadosRuta = emptyList()) }
            return
        }
        trabajosBusquedaRutaDocumento[id] = viewModelScope.launch {
            delay(DEBOUNCE_MS)
            try {
                val resultados = withContext(dispatcherIO) { nucleo.buscarRutas(nuevo) }
                actualizarDocumento(id) { it.copy(resultadosRuta = resultados) }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    fun elegirRutaDocumento(id: Long, ruta: Ruta) {
        trabajosBusquedaRutaDocumento[id]?.cancel()
        actualizarDocumento(id) {
            it.copy(textoRuta = ruta.numero.toString(), rutaSeleccionada = ruta, resultadosRuta = emptyList())
        }
    }

    /// Chip tocable "Principal"/"H2"/"H3"/"H4" -- cicla 1→2→3→4→1, mismo
    /// cómputo que `etiquetaSubNumero`.
    fun alternarSubNumeroDocumento(id: Long) {
        actualizarDocumento(id) { it.copy(subNumero = it.subNumero % 4 + 1) }
    }

    /// A diferencia de [alternarSubNumeroDocumento] (chip tocable, cicla),
    /// esto fija un valor concreto -- lo usa el OCR del comprobante, que
    /// ya trae el sub-número real impreso, no hay nada que ciclar.
    fun fijarSubNumeroDocumento(id: Long, valor: Int) {
        actualizarDocumento(id) { it.copy(subNumero = valor.coerceIn(1, 4)) }
    }

    fun cambiarNumeroDocumento(id: Long, texto: String) {
        actualizarDocumento(id) { it.copy(numeroDocumento = texto) }
    }

    fun cambiarFechaDocumento(id: Long, texto: String) {
        actualizarDocumento(id) { it.copy(fechaTexto = texto) }
    }

    fun alternarTieneCorreo(valor: Boolean) {
        tieneCorreo = valor
    }

    /// Autocompleta el número de ruta de un documento con lo que trajo el
    /// OCR del comprobante y dispara la búsqueda -- mismo criterio que
    /// [usarVehiculoEscaneado]: si hay una única coincidencia exacta, la
    /// elige sola.
    fun usarNumeroRutaEscaneadoDocumento(id: Long, numero: Int) {
        cambiarTextoRutaDocumento(id, numero.toString())
        trabajosBusquedaRutaDocumento[id]?.cancel()
        trabajosBusquedaRutaDocumento[id] = viewModelScope.launch {
            try {
                val resultados = withContext(dispatcherIO) { nucleo.buscarRutas(numero.toString()) }
                actualizarDocumento(id) { it.copy(resultadosRuta = resultados) }
                resultados.singleOrNull { it.numero == numero.toLong() }?.let { elegirRutaDocumento(id, it) }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    // ---- Vehículo ----

    /// Mismo criterio que [cambiarTextoRutaDocumento]: escribir de nuevo
    /// abandona cualquier selección previa (vehículo Y el viaje abierto
    /// que esa selección haya detectado).
    fun cambiarTextoVehiculo(nuevo: String) {
        textoVehiculo = nuevo
        vehiculoSeleccionado = null
        limpiarViajeAbierto()
        trabajoBusquedaVehiculo?.cancel()
        if (nuevo.isBlank()) {
            resultadosVehiculo = emptyList()
            return
        }
        trabajoBusquedaVehiculo = viewModelScope.launch {
            delay(DEBOUNCE_MS)
            try {
                resultadosVehiculo = withContext(dispatcherIO) { nucleo.buscarVehiculosRuta(nuevo) }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    fun elegirVehiculo(vehiculo: VehiculoRuta) {
        trabajoBusquedaVehiculo?.cancel()
        textoVehiculo = vehiculo.placa
        vehiculoSeleccionado = vehiculo
        resultadosVehiculo = emptyList()
        buscarViajeAbiertoPara(vehiculo.placa)
    }

    /// Autocompleta el buscador de vehículo con lo que trajo el OCR (placa
    /// O número de unidad, mismo catálogo para ambos) y dispara la
    /// búsqueda -- mismo criterio que [usarNumeroRutaEscaneadoDocumento]:
    /// si hay una única coincidencia exacta, la elige sola; si no, deja
    /// los resultados para que el guardia elija a mano.
    fun usarVehiculoEscaneado(texto: String) {
        cambiarTextoVehiculo(texto)
        trabajoBusquedaVehiculo?.cancel()
        trabajoBusquedaVehiculo = viewModelScope.launch {
            try {
                val resultados = withContext(dispatcherIO) { nucleo.buscarVehiculosRuta(texto) }
                resultadosVehiculo = resultados
                resultados
                    .singleOrNull { it.placa == texto || it.numeroUnidad == texto }
                    ?.let { elegirVehiculo(it) }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    private fun buscarViajeAbiertoPara(placa: String) {
        trabajoBusquedaViajeAbierto?.cancel()
        trabajoBusquedaViajeAbierto = viewModelScope.launch {
            try {
                viajeAbiertoParaVehiculo = withContext(dispatcherIO) {
                    nucleo.buscarViajeRutaAbiertoPorPlaca(placa)
                }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    private fun limpiarViajeAbierto() {
        trabajoBusquedaViajeAbierto?.cancel()
        viajeAbiertoParaVehiculo = null
        continuarViaje = false
    }

    fun alternarContinuarViaje(valor: Boolean) {
        continuarViaje = valor
    }

    // ---- Registro ----

    /// Arma la solicitud desde el estado actual del formulario y la manda
    /// al núcleo -- espejo de `AppCore::registrar_salida_ruta`/
    /// `RutaService::registrar_salida`. `continuar_viaje_id` sólo viaja si
    /// el guardia activó explícitamente "continuar el mismo viaje" (ver
    /// [alternarContinuarViaje]).
    fun registrarSalida(onExito: () -> Unit) {
        if (registrando) return
        val encargado = encargadoSeleccionado ?: return
        val vehiculo = vehiculoSeleccionado ?: return
        registrando = true
        viewModelScope.launch {
            try {
                val solicitud = SolicitudSalidaRuta(
                    vehiculoPlaca = vehiculo.placa,
                    vehiculoNumeroUnidad = vehiculo.numeroUnidad,
                    encargadoNombre = encargado.nombre,
                    encargadoCodigoEmpleado = encargado.codigoEmpleado,
                    documentos = documentos.map { borrador ->
                        SolicitudDocumentoRuta(
                            numeroDocumento = borrador.numeroDocumento,
                            numeroRuta = borrador.rutaSeleccionada?.numero,
                            subNumero = borrador.subNumero.toLong(),
                            fechaDocumento = textoDDMMYYYYaIsoRuta(borrador.fechaTexto),
                        )
                    },
                    continuarViajeId = if (continuarViaje) viajeAbiertoParaVehiculo?.id else null,
                    tieneCorreoAutorizacion = tieneCorreo,
                )
                withContext(dispatcherIO) { nucleo.registrarSalidaRuta(solicitud) }
                CambiosNube.solicitar()
                mensaje = "Salida registrada"
                error = null
                limpiarFormulario()
                refrescarActivas()
                onExito()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                registrando = false
            }
        }
    }

    private fun limpiarFormulario() {
        textoEncargado = ""
        encargadoSeleccionado = null
        documentos = listOf(BorradorDocumentoRuta(id = siguienteIdDocumento++))
        tieneCorreo = false
        textoVehiculo = ""
        vehiculoSeleccionado = null
        limpiarViajeAbierto()
    }

    /// `decision` es la respuesta obligatoria del guardia a "¿vuelve a
    /// salir?", pedida en el mismo acto de confirmar este retorno -- ver
    /// `docs/planes-implementados/plan-control-rutas.md`.
    fun registrarRetorno(salida: SalidaRutaActivaResumen, decision: DecisionRetornoViaje) {
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) { nucleo.registrarRetornoRuta(salida.id, decision) }
                CambiosNube.solicitar()
                refrescarActivas()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    companion object {
        // Mismo valor que `ActivosViewModel` -- ver su comentario.
        private const val DEBOUNCE_MS = 300L

        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { RutasViewModel(nucleo) }
        }
    }
}

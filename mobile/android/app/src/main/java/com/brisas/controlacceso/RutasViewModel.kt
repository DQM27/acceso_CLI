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
import uniffi.control_acceso_mobile.EncargadoRuta
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.Ruta
import uniffi.control_acceso_mobile.SalidaRutaActivaResumen
import uniffi.control_acceso_mobile.SolicitudSalidaRuta
import uniffi.control_acceso_mobile.VehiculoRuta

/// Dueño del estado real de [PantallaRutas] y de las llamadas a [Nucleo] --
/// mismo criterio que [ActivosViewModel] (ver su doc-comment): el
/// `@Composable` sólo lee este estado y reporta eventos, ninguna decisión
/// de negocio ni llamada a `Nucleo` vive del lado de la UI. Reemplaza el
/// mock en memoria que existía hasta el 2026-09-15 (`activas` vivía sólo
/// acá, sin `Nucleo`) -- ver `docs/planes-implementados/plan-control-rutas.md`.
class RutasViewModel(
    private val nucleo: Nucleo,
    // Inyectable para tests con un dispatcher de tiempo controlado, mismo
    // motivo que en `ActivosViewModel`.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var activas by mutableStateOf<List<SalidaRutaActivaResumen>>(emptyList())
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

    // Buscador de número de ruta (paso 2) -- BLOQUEANTE (pedido explícito
    // del usuario, 2026-09-15: "sin restricción podrías poner la ruta 222
    // y no existe"). El paso sólo se da por completo cuando hay una
    // [Ruta] real elegida de `resultadosRuta`, nunca por el sólo hecho de
    // que el campo de texto no esté vacío.
    var textoRuta by mutableStateOf("")
        private set
    var resultadosRuta by mutableStateOf<List<Ruta>>(emptyList())
        private set
    var rutaSeleccionada by mutableStateOf<Ruta?>(null)
        private set
    private var trabajoBusquedaRuta: Job? = null

    // Buscador de vehículo (paso 3) -- por placa o número de unidad,
    // BLOQUEANTE (pedido explícito del usuario, 2026-09-19: "es un buscador,
    // no se puede poner lo que uno quiera, sólo lo que la tabla
    // proporciona") -- mismo criterio que el buscador de número de ruta, no
    // el de encargado. Reemplaza los dos campos de texto libre
    // (`vehiculoPlaca`/`vehiculoNumeroUnidad`) que antes vivían como estado
    // local de [PantallaRutas].
    var textoVehiculo by mutableStateOf("")
        private set
    var resultadosVehiculo by mutableStateOf<List<VehiculoRuta>>(emptyList())
        private set
    var vehiculoSeleccionado by mutableStateOf<VehiculoRuta?>(null)
        private set
    private var trabajoBusquedaVehiculo: Job? = null

    init {
        refrescarActivas()
    }

    fun refrescarActivas() {
        viewModelScope.launch {
            cargando = true
            try {
                activas = withContext(dispatcherIO) { nucleo.listarRutasActivas() }
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

    fun cambiarTextoRuta(nuevo: String) {
        textoRuta = nuevo
        rutaSeleccionada = null
        trabajoBusquedaRuta?.cancel()
        if (nuevo.isBlank()) {
            resultadosRuta = emptyList()
            return
        }
        trabajoBusquedaRuta = viewModelScope.launch {
            delay(DEBOUNCE_MS)
            try {
                resultadosRuta = withContext(dispatcherIO) { nucleo.buscarRutas(nuevo) }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    fun elegirRuta(ruta: Ruta) {
        trabajoBusquedaRuta?.cancel()
        textoRuta = ruta.numero.toString()
        rutaSeleccionada = ruta
        resultadosRuta = emptyList()
    }

    /// Autocompleta el buscador de rutas con lo que trajo el OCR del
    /// comprobante y dispara la búsqueda de una -- si hay una única
    /// coincidencia exacta por número, la elige sola (mismo criterio que
    /// `contratistaEscaneadoClaro` en `ActivosViewModel`); si no, deja los
    /// resultados para que el guardia elija a mano.
    fun usarNumeroRutaEscaneado(numero: Int) {
        cambiarTextoRuta(numero.toString())
        trabajoBusquedaRuta?.cancel()
        trabajoBusquedaRuta = viewModelScope.launch {
            try {
                val resultados = withContext(dispatcherIO) { nucleo.buscarRutas(numero.toString()) }
                resultadosRuta = resultados
                resultados.singleOrNull { it.numero == numero.toLong() }?.let { elegirRuta(it) }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    /// Mismo criterio que [cambiarTextoRuta]: escribir de nuevo abandona
    /// cualquier selección previa.
    fun cambiarTextoVehiculo(nuevo: String) {
        textoVehiculo = nuevo
        vehiculoSeleccionado = null
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
    }

    /// Autocompleta el buscador de vehículo con lo que trajo el OCR (placa
    /// O número de unidad, mismo catálogo para ambos) y dispara la
    /// búsqueda -- mismo criterio que [usarNumeroRutaEscaneado]: si hay una
    /// única coincidencia exacta, la elige sola; si no, deja los
    /// resultados para que el guardia elija a mano.
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

    fun registrarSalida(solicitud: SolicitudSalidaRuta, onExito: () -> Unit) {
        if (registrando) return
        registrando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) { nucleo.registrarSalidaRuta(solicitud) }
                CambiosNube.solicitar()
                mensaje = "Salida registrada"
                error = null
                textoEncargado = ""
                encargadoSeleccionado = null
                textoRuta = ""
                rutaSeleccionada = null
                textoVehiculo = ""
                vehiculoSeleccionado = null
                refrescarActivas()
                onExito()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                registrando = false
            }
        }
    }

    fun registrarRetorno(salida: SalidaRutaActivaResumen) {
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) { nucleo.registrarRetornoRuta(salida.id) }
                CambiosNube.solicitar()
                refrescarActivas()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    companion object {
        // Bajado a 150ms (pedido explícito del usuario, 2026-09-21), mismo
        // valor en los 5 buscadores con debounce -- ver el comentario de
        // `ActivosViewModel`.
        private const val DEBOUNCE_MS = 150L

        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { RutasViewModel(nucleo) }
        }
    }
}

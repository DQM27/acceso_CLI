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
import uniffi.control_acceso_mobile.PrestamoGafeteProvisionalActivoResumen

/// Dueño del estado real de [PantallaGafetesProvisionales] y de las
/// llamadas a [Nucleo] -- mismo criterio que [RutasViewModel]: el
/// `@Composable` sólo lee este estado y reporta eventos. Mucho más simple
/// que [RutasViewModel]: un solo buscador (encargado), sin catálogo
/// bloqueante ni OCR -- ver
/// `docs/features-futuras/plan-gafetes-provisionales-kof.md`.
class GafetesProvisionalesViewModel(
    private val nucleo: Nucleo,
    private val secretoStore: SecretoDispositivoStore,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var activos by mutableStateOf<List<PrestamoGafeteProvisionalActivoResumen>>(emptyList())
        private set
    var cargando by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var mensaje by mutableStateOf<String?>(null)
        private set
    var registrando by mutableStateOf(false)
        private set

    // Buscador de encargado -- mismo criterio que `PasoEncargado` de
    // rutas: por nombre o código de empleado, contra el mismo catálogo
    // (`encargados_ruta`) ya importado para ese módulo. A diferencia de
    // rutas, acá SÍ es bloqueante -- no tiene sentido entregar un gafete a
    // un texto libre sin encargado real detrás.
    var textoEncargado by mutableStateOf("")
        private set
    var resultadosEncargado by mutableStateOf<List<EncargadoRuta>>(emptyList())
        private set
    var encargadoSeleccionado by mutableStateOf<EncargadoRuta?>(null)
        private set
    private var trabajoBusquedaEncargado: Job? = null

    init {
        refrescarActivos()
    }

    fun refrescarActivos() {
        viewModelScope.launch {
            cargando = true
            try {
                activos = withContext(dispatcherIO) { nucleo.listarGafetesProvisionalesActivos() }
                error = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                cargando = false
            }
        }
    }

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
        textoEncargado = "${encargado.nombre} · ${encargado.codigoEmpleado}"
        encargadoSeleccionado = encargado
        resultadosEncargado = emptyList()
    }

    fun entregar(gafeteNumero: Long, onExito: () -> Unit) {
        val encargado = encargadoSeleccionado ?: return
        if (registrando) return
        registrando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    // Chequeo en vivo: dos dispositivos del mismo sitio sólo
                    // validan el gafete contra su propia base local, así que
                    // sin esto ambos podían aceptar el mismo número como
                    // activo a la vez -- mismo criterio que
                    // `PantallaConfirmarIngreso.registrarIngreso` para
                    // gafetes de contratista (`Nucleo.gafeteOcupadoEnSitio`).
                    val secreto = secretoStore.cargar()
                        ?: throw SecretoDispositivoNoEncontradoException()
                    if (nucleo.gafeteProvisionalOcupadoEnSitioConSecreto(secreto, gafeteNumero)) {
                        throw GafeteOcupadoEnSitioException(gafeteNumero)
                    }
                    nucleo.entregarGafeteProvisional(encargado.id, gafeteNumero)
                }
                CambiosNube.solicitar()
                mensaje = "Gafete entregado"
                error = null
                textoEncargado = ""
                encargadoSeleccionado = null
                refrescarActivos()
                onExito()
            } catch (excepcion: GafeteOcupadoEnSitioException) {
                error = excepcion.message
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoStoreException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoNoEncontradoException) {
                error = excepcion.message
            } finally {
                registrando = false
            }
        }
    }

    fun registrarDevolucion(prestamo: PrestamoGafeteProvisionalActivoResumen) {
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) { nucleo.registrarDevolucionGafeteProvisional(prestamo.id) }
                CambiosNube.solicitar()
                refrescarActivos()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    companion object {
        // Mismo valor que `RutasViewModel`/`ActivosViewModel`.
        private const val DEBOUNCE_MS = 300L

        fun factory(
            nucleo: Nucleo,
            secretoStore: SecretoDispositivoStore,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { GafetesProvisionalesViewModel(nucleo, secretoStore) }
        }
    }
}

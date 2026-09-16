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
import uniffi.control_acceso_mobile.EmpresaProveedor
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.RegistroIngresoProveedorActivoResumen

/// Dueño del estado real de [PantallaProveedores] y de las llamadas a
/// [Nucleo] -- mismo criterio que [GafetesProvisionalesViewModel]:
/// pantalla autocontenida (buscador + formulario + lista de activos), sin
/// wizard de pasos ni el ida-y-vuelta que exigiría integrarse a la lista
/// compartida de "Activos" (`ActivosViewModel`, que hoy sólo distingue
/// origen local/remoto de ingresos de contratista, ninguna otra entidad --
/// ver `docs/features-futuras/plan-control-proveedores.md`).
class ProveedoresViewModel(
    private val nucleo: Nucleo,
    private val secretoStore: SecretoDispositivoStore,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var activos by mutableStateOf<List<RegistroIngresoProveedorActivoResumen>>(emptyList())
        private set
    var cargando by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var mensaje by mutableStateOf<String?>(null)
        private set
    var registrando by mutableStateOf(false)
        private set

    var cedula by mutableStateOf("")
        private set
    var nombre by mutableStateOf("")
        private set
    var placa by mutableStateOf("")
        private set

    // Buscador de empresa proveedora -- mismo criterio que el buscador de
    // encargado en `GafetesProvisionalesViewModel`, con la diferencia de
    // que acá SÍ se puede crear una empresa nueva inline cuando la
    // búsqueda no trae nada (no hay catálogo cerrado como `encargados_ruta`).
    var textoEmpresa by mutableStateOf("")
        private set
    var resultadosEmpresa by mutableStateOf<List<EmpresaProveedor>>(emptyList())
        private set
    var empresaSeleccionada by mutableStateOf<EmpresaProveedor?>(null)
        private set
    var creandoEmpresa by mutableStateOf(false)
        private set
    private var trabajoBusquedaEmpresa: Job? = null

    init {
        refrescarActivos()
    }

    fun refrescarActivos() {
        viewModelScope.launch {
            cargando = true
            try {
                activos = withContext(dispatcherIO) { nucleo.listarProveedoresActivos() }
                error = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                cargando = false
            }
        }
    }

    fun cambiarCedula(nuevo: String) {
        cedula = nuevo.filter(Char::isDigit)
    }

    fun cambiarNombre(nuevo: String) {
        nombre = nuevo
    }

    fun cambiarPlaca(nuevo: String) {
        placa = nuevo.uppercase()
    }

    fun cambiarTextoEmpresa(nuevo: String) {
        textoEmpresa = nuevo
        empresaSeleccionada = null
        trabajoBusquedaEmpresa?.cancel()
        if (nuevo.isBlank()) {
            resultadosEmpresa = emptyList()
            return
        }
        trabajoBusquedaEmpresa = viewModelScope.launch {
            delay(DEBOUNCE_MS)
            try {
                resultadosEmpresa = withContext(dispatcherIO) { nucleo.buscarEmpresasProveedor(nuevo) }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    fun elegirEmpresa(empresa: EmpresaProveedor) {
        trabajoBusquedaEmpresa?.cancel()
        textoEmpresa = empresa.nombre
        empresaSeleccionada = empresa
        resultadosEmpresa = emptyList()
    }

    /// Alta inline desde el mismo selector -- pedido explícito del plan
    /// (Paso 2: "Crear empresa" cuando no aparece en la búsqueda).
    fun crearEmpresa(nombreNuevo: String) {
        if (creandoEmpresa || nombreNuevo.isBlank()) return
        creandoEmpresa = true
        viewModelScope.launch {
            try {
                val id = withContext(dispatcherIO) { nucleo.crearEmpresaProveedor(nombreNuevo.trim()) }
                empresaSeleccionada = EmpresaProveedor(id = id, nombre = nombreNuevo.trim(), activo = true)
                textoEmpresa = nombreNuevo.trim()
                resultadosEmpresa = emptyList()
                error = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                creandoEmpresa = false
            }
        }
    }

    fun rellenarDesdeDocumento(cedulaLeida: String?, nombreLeido: String?) {
        cedulaLeida?.let { cambiarCedula(it) }
        nombreLeido?.let { nombre = it }
    }

    fun registrarIngreso(gafeteNumero: Long, onExito: () -> Unit) {
        val empresa = empresaSeleccionada ?: return
        if (registrando || cedula.isBlank() || nombre.isBlank()) return
        registrando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    // Chequeo en vivo: mismo criterio que
                    // `GafetesProvisionalesViewModel.entregar` -- dos
                    // dispositivos del mismo sitio sólo validan el gafete
                    // contra su propia base local.
                    val secreto = secretoStore.cargar()
                        ?: throw SecretoDispositivoNoEncontradoException()
                    if (nucleo.gafeteDeProveedorOcupadoEnSitioConSecreto(secreto, gafeteNumero)) {
                        throw GafeteOcupadoEnSitioException(gafeteNumero)
                    }
                    nucleo.registrarIngresoProveedor(
                        cedula,
                        nombre,
                        empresa.id,
                        placa.trim().ifBlank { null },
                        gafeteNumero,
                    )
                }
                mensaje = "Ingreso registrado"
                error = null
                cedula = ""
                nombre = ""
                placa = ""
                textoEmpresa = ""
                empresaSeleccionada = null
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

    fun registrarSalida(registro: RegistroIngresoProveedorActivoResumen) {
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) { nucleo.registrarSalidaProveedor(registro.id) }
                refrescarActivos()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    companion object {
        // Mismo valor que `RutasViewModel`/`ActivosViewModel`/
        // `GafetesProvisionalesViewModel`.
        private const val DEBOUNCE_MS = 300L

        fun factory(
            nucleo: Nucleo,
            secretoStore: SecretoDispositivoStore,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { ProveedoresViewModel(nucleo, secretoStore) }
        }
    }
}

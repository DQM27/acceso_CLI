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
import uniffi.control_acceso_mobile.IngresoProveedorRemoto
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.RegistroIngresoProveedorActivoResumen

/// Fila fusionada local+remota de la lista de activos -- mismo criterio
/// que `FilaActiva` en [ActivosViewModel]: un ingreso de proveedor abierto
/// por OTRO dispositivo del mismo sitio nunca vive en la tabla local
/// (`registro_ingresos_proveedor`), sólo en la caché
/// `ingresos_proveedor_remotos` -- sin esta fusión, la lista de "activos"
/// de esta pantalla sólo mostraba lo que este mismo dispositivo había
/// registrado, nunca lo que otro dispositivo del sitio tenía abierto en
/// ese momento (bug reportado en pruebas reales, 2026-09-17).
sealed class FilaProveedorActiva {
    data class Local(val registro: RegistroIngresoProveedorActivoResumen) : FilaProveedorActiva()
    data class Remota(val remoto: IngresoProveedorRemoto) : FilaProveedorActiva()
}

private fun FilaProveedorActiva.cedula(): String = when (this) {
    is FilaProveedorActiva.Local -> registro.cedula
    is FilaProveedorActiva.Remota -> remoto.cedula
}

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
    var activos by mutableStateOf<List<FilaProveedorActiva>>(emptyList())
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

    // Pedido explícito del usuario 2026-09-20: antes el aviso de "esta
    // cédula ya tiene un ingreso activo" sólo salía al presionar "Registrar
    // ingreso" (el chequeo real vive en Rust, `IngresoProveedorServiceError::IngresoActivo`)
    // -- quien opera llenaba empresa/placa/gafete completos para recién ahí
    // enterarse. Se adelanta la misma validación acá contra `activos` (ya
    // cargado al entrar a la pantalla y refrescado tras cada alta/baja) en
    // cuanto la cédula cambia -- por escaneo o tecleada a mano, ambas pasan
    // por `cambiarCedula`. No reemplaza el chequeo de Rust (que sigue
    // siendo la fuente de verdad si otro dispositivo registró un ingreso
    // justo en el medio), sólo evita hacer avanzar a alguien con un dato ya
    // conocido como inválido.
    var cedulaConIngresoActivo by mutableStateOf(false)
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
                val (locales, remotos) = withContext(dispatcherIO) {
                    nucleo.listarProveedoresActivos() to nucleo.listarIngresosProveedorRemotos()
                }
                activos = locales.map { FilaProveedorActiva.Local(it) } +
                    remotos.map { FilaProveedorActiva.Remota(it) }
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
        cedulaConIngresoActivo = cedula.isNotBlank() && activos.any { it.cedula() == cedula }
        error = if (cedulaConIngresoActivo) {
            MENSAJE_CEDULA_CON_INGRESO_ACTIVO
        } else if (error == MENSAJE_CEDULA_CON_INGRESO_ACTIVO) {
            null
        } else {
            error
        }
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
                CambiosNube.solicitar()
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

    /// `nombreLeido`/`apellidosLeido` llegan separados de un documento MRZ
    /// (`DocumentoDetectado.nombre`/`.apellidos` -- ver
    /// `LectorDocumentosIdentidad.kt`, `ResultadoMrz.aDocumentoDetectado`);
    /// usar sólo `nombre` (como hacía antes) dejaba el campo vacío o
    /// incompleto cada vez que el nombre de pila viajaba en un campo MRZ
    /// distinto al apellido -- bug reportado en pruebas reales en
    /// dispositivo (2026-09-17).
    fun rellenarDesdeDocumento(cedulaLeida: String?, nombreLeido: String?, apellidosLeido: String? = null) {
        cedulaLeida?.let { cambiarCedula(it) }
        val nombreCompleto = listOfNotNull(nombreLeido, apellidosLeido)
            .filter { it.isNotBlank() }
            .joinToString(" ")
        if (nombreCompleto.isNotBlank()) nombre = nombreCompleto
    }

    fun registrarIngreso(gafeteNumero: Long, onExito: () -> Unit) {
        val empresa = empresaSeleccionada ?: return
        if (registrando || cedula.isBlank() || nombre.isBlank() || cedulaConIngresoActivo) return
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
                CambiosNube.solicitar()
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

    /// Se llama al cerrar el formulario sin registrar (botón "← Volver" o
    /// atrás del sistema) -- limpia los mismos campos que un registro
    /// exitoso. Sin esto, `error` (ej. "Esta cédula ya tiene un ingreso de
    /// proveedor activo", puesto por `cambiarCedula` al escanear) se
    /// quedaba pegado después de salir del formulario y aparecía en la
    /// pantalla de la lista de activos, que reusa el mismo campo `error`
    /// para sus propias fallas (`refrescarActivos`/`registrarSalida`) --
    /// bug reportado en pruebas reales, 2026-09-20.
    fun cancelarFormulario() {
        cedula = ""
        nombre = ""
        placa = ""
        textoEmpresa = ""
        empresaSeleccionada = null
        cedulaConIngresoActivo = false
        error = null
    }

    /// Local: cierra en `registro_ingresos_proveedor` (este dispositivo).
    /// Remota: cierra directo contra la nube (mismo criterio que
    /// `ActivosViewModel.confirmarSalida` para contratistas) -- nunca toca
    /// el historial local, esa fila no es -- ni fue -- de este dispositivo.
    fun registrarSalida(fila: FilaProveedorActiva) {
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    when (fila) {
                        is FilaProveedorActiva.Local -> nucleo.registrarSalidaProveedor(fila.registro.id)
                        is FilaProveedorActiva.Remota -> {
                            val secreto = secretoStore.cargar()
                                ?: throw SecretoDispositivoNoEncontradoException()
                            nucleo.cerrarIngresoProveedorRemotoConSecreto(secreto, fila.remoto.uuid)
                        }
                    }
                }
                CambiosNube.solicitar()
                refrescarActivos()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoStoreException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoNoEncontradoException) {
                error = excepcion.message
            }
        }
    }

    companion object {
        // Mismo valor que `RutasViewModel`/`ActivosViewModel`/
        // `GafetesProvisionalesViewModel`.
        private const val DEBOUNCE_MS = 300L

        // Mismo texto que `IngresoProveedorServiceError::IngresoActivo` en
        // Rust (`src/services/error.rs`) -- el chequeo local en
        // `cambiarCedula` es un adelanto de UX, no un reemplazo; que diga
        // lo mismo evita que alguien vea dos redacciones distintas para el
        // mismo motivo según en qué momento se entera.
        const val MENSAJE_CEDULA_CON_INGRESO_ACTIVO = "Esta cédula ya tiene un ingreso de proveedor activo"

        fun factory(
            nucleo: Nucleo,
            secretoStore: SecretoDispositivoStore,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { ProveedoresViewModel(nucleo, secretoStore) }
        }
    }
}

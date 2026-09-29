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

    // Aviso adelantado mientras se tipea o escanea la cédula: si ya tiene
    // un ingreso de proveedor abierto en este sitio (este equipo o el otro
    // dispositivo), quien opera se entera antes de llenar el resto. La
    // regla y el texto son del núcleo
    // (`Nucleo.avisoProveedorConIngresoActivo`); al registrar,
    // `registrarIngresoProveedorConSecreto` la vuelve a aplicar.
    var cedulaConIngresoActivo by mutableStateOf(false)
        private set
    private var avisoCedulaActiva: String? = null

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
    private val buscadorEmpresa = BuscadorConDebounce(viewModelScope)

    init {
        refrescarActivos()
    }

    fun refrescarActivos() {
        viewModelScope.launch {
            cargando = true
            try {
                val (locales, remotos) = withContext(dispatcherIO) {
                    medirNucleo("listarProveedoresActivos") { nucleo.listarProveedoresActivos() } to medirNucleo("listarIngresosProveedorRemotos") { nucleo.listarIngresosProveedorRemotos() }
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
        val consultada = cedula
        viewModelScope.launch {
            val aviso = try {
                withContext(dispatcherIO) { medirNucleo("avisoProveedorConIngresoActivo") { nucleo.avisoProveedorConIngresoActivo(consultada) } }
            } catch (excepcion: Exception) {
                // Sin aviso si la consulta falla: al registrar, el núcleo
                // vuelve a aplicar la regla. Lo inesperado se relanza.
                excepcion.mensajeDeErrorEsperado()
                null
            }
            if (consultada != cedula) return@launch // ya se tipeó otra
            val anterior = avisoCedulaActiva
            avisoCedulaActiva = aviso
            cedulaConIngresoActivo = aviso != null
            if (aviso != null) {
                error = aviso
            } else if (anterior != null && error == anterior) {
                error = null
            }
        }
    }

    fun cambiarNombre(nuevo: String) {
        nombre = nuevo
    }

    fun cambiarPlaca(nuevo: String) {
        placa = nuevo.uppercase()
    }

    /// Siempre en mayúscula, igual que `cambiarPlaca` -- pedido del usuario
    /// (2026-09-23): los nombres de empresa proveedora se guardan en
    /// mayúscula, y este mismo campo es el que da de alta una empresa nueva
    /// (`crearEmpresa`). La búsqueda del núcleo no distingue mayúsculas.
    fun cambiarTextoEmpresa(valor: String) {
        val texto = valor.uppercase()
        textoEmpresa = texto
        empresaSeleccionada = null
        buscadorEmpresa.cancelar()
        if (texto.isBlank()) {
            resultadosEmpresa = emptyList()
            return
        }
        buscadorEmpresa.buscar {
            try {
                resultadosEmpresa = withContext(dispatcherIO) { medirNucleo("buscarEmpresasProveedor") { nucleo.buscarEmpresasProveedor(texto) } }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    fun elegirEmpresa(empresa: EmpresaProveedor) {
        buscadorEmpresa.cancelar()
        textoEmpresa = empresa.nombre
        empresaSeleccionada = empresa
        resultadosEmpresa = emptyList()
    }

    /// Alta inline desde el mismo selector -- pedido explícito del plan
    /// (Paso 2: "Crear empresa" cuando no aparece en la búsqueda).
    fun crearEmpresa(nombreNuevo: String) {
        if (creandoEmpresa || nombreNuevo.isBlank()) return
        // Mayúscula también acá, no sólo en `cambiarTextoEmpresa` -- quien
        // llame con otro texto (no el del campo) no debe poder saltársela.
        val nombre = nombreNuevo.trim().uppercase()
        creandoEmpresa = true
        viewModelScope.launch {
            try {
                val id = withContext(dispatcherIO) { medirNucleo("crearEmpresaProveedor") { nucleo.crearEmpresaProveedor(nombre) } }
                CambiosNube.cambioLocal()
                empresaSeleccionada = EmpresaProveedor(id = id, nombre = nombre, activo = true)
                textoEmpresa = nombre
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
    /// `documento_desde_mrz` en `mobile/rust-core/src/lectura_documentos/identidad.rs`);
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
                    // Todas las reglas (cédula activa aquí o en otro sitio,
                    // gafete en uso en el otro dispositivo) las aplica el
                    // núcleo en esta misma llamada.
                    val secreto = secretoStore.cargar()
                        ?: throw SecretoDispositivoNoEncontradoException()
                    medirNucleo("registrarIngresoProveedorConSecreto") { nucleo.registrarIngresoProveedorConSecreto(
                        cedula,
                        nombre,
                        empresa.id,
                        placa.trim().ifBlank { null },
                        gafeteNumero,
                        secreto,
                    ) }
                }
                CambiosNube.cambioLocal()
                mensaje = "Ingreso registrado"
                error = null
                cedula = ""
                nombre = ""
                placa = ""
                textoEmpresa = ""
                empresaSeleccionada = null
                refrescarActivos()
                onExito()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
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
        avisoCedulaActiva = null
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
                        is FilaProveedorActiva.Local -> medirNucleo("registrarSalidaProveedor") { nucleo.registrarSalidaProveedor(fila.registro.id) }
                        is FilaProveedorActiva.Remota -> {
                            val secreto = secretoStore.cargar()
                                ?: throw SecretoDispositivoNoEncontradoException()
                            medirNucleo("cerrarIngresoProveedorRemotoConSecreto") { nucleo.cerrarIngresoProveedorRemotoConSecreto(secreto, fila.remoto.uuid) }
                        }
                    }
                }
                CambiosNube.cambioLocal()
                refrescarActivos()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            }
        }
    }

    companion object {
        fun factory(
            nucleo: Nucleo,
            secretoStore: SecretoDispositivoStore,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { ProveedoresViewModel(nucleo, secretoStore) }
        }
    }
}

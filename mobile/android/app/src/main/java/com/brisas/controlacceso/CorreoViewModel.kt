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
import uniffi.control_acceso_mobile.IngresoCorreoRemoto
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.RegistroIngresoCorreoActivoResumen
import uniffi.control_acceso_mobile.nombreMientrasSeEscribe

/// Fila fusionada local+remota de los ingresos por correo -- mismo criterio
/// que [FilaProveedorActiva].
sealed class FilaCorreoActiva {
    data class Local(val registro: RegistroIngresoCorreoActivoResumen) : FilaCorreoActiva()
    data class Remota(val remoto: IngresoCorreoRemoto) : FilaCorreoActiva()
}

/// Estado y llamadas de [PantallaPorCorreo]: ingresos de visitas autorizadas
/// por correo (generalmente entrevistas de RH), comodín mientras se termina
/// el módulo de Visitas. Mismo molde que [ProveedoresViewModel], con el
/// motivo en vez de la empresa y gafete de visita.
class CorreoViewModel(
    private val nucleo: Nucleo,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var activos by mutableStateOf<List<FilaCorreoActiva>>(emptyList())
        private set

    /// "Ingreso registrado · …" / "Salida registrada · …" (ver
    /// [FranjaConfirmacion]).
    val confirmacion = ConfirmacionReciente()
    var error by mutableStateOf<String?>(null)
        private set
    var mensaje by mutableStateOf<String?>(null)
        private set
    var registrando by mutableStateOf(false)
        private set

    /// Cargando la lista de quién está adentro: la pantalla muestra marcadores
    /// en vez de "nadie adentro" mientras no llega la primera respuesta.
    var cargando by mutableStateOf(false)
        private set

    var cedula by mutableStateOf("")
        private set
    var nombre by mutableStateOf("")
        private set
    var motivo by mutableStateOf("")
        private set
    var placa by mutableStateOf("")
        private set

    /// Mismo aviso adelantado que en proveedores: la regla y el texto son
    /// del núcleo (`Nucleo.avisoCorreoConIngresoActivo`).
    var cedulaConIngresoActivo by mutableStateOf(false)
        private set
    private var avisoCedulaActiva: String? = null

    init {
        refrescarActivos()
    }

    fun refrescarActivos() {
        viewModelScope.launch {
            cargando = true
            try {
                val (locales, remotos) = withContext(dispatcherIO) {
                    medirNucleo("listarCorreosActivos") { nucleo.listarCorreosActivos() } to
                        medirNucleo("listarIngresosCorreoRemotos") { nucleo.listarIngresosCorreoRemotos() }
                }
                activos = locales.map { FilaCorreoActiva.Local(it) } +
                    remotos.map { FilaCorreoActiva.Remota(it) }
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
        viewModelScope.launch { revisarCedula(consultada) }
    }

    /// Si ya está adentro (por correo o por otra vía): regla y texto del
    /// núcleo.
    private suspend fun revisarCedula(consultada: String) {
        val aviso = try {
            withContext(dispatcherIO) {
                medirNucleo("avisoCorreoConIngresoActivo") { nucleo.avisoCorreoConIngresoActivo(consultada) }
            }
        } catch (excepcion: Exception) {
            // Sin aviso si la consulta falla: al registrar, el núcleo
            // vuelve a aplicar la regla. Lo inesperado se relanza.
            excepcion.mensajeDeErrorEsperado()
            null
        }
        if (consultada != cedula) return // ya se tipeó otra
        val anterior = avisoCedulaActiva
        avisoCedulaActiva = aviso
        cedulaConIngresoActivo = aviso != null
        if (aviso != null) {
            error = aviso
        } else if (anterior != null && error == anterior) {
            error = null
        }
    }

    /// En mayúscula al escribir (y al leerlo del documento): la regla de todo
    /// nombre la dicta el núcleo (`nombreMientrasSeEscribe`).
    fun cambiarNombre(nuevo: String) {
        nombre = nombreMientrasSeEscribe(nuevo)
    }

    fun cambiarMotivo(nuevo: String) {
        motivo = nuevo
    }

    fun cambiarPlaca(nuevo: String) {
        placa = nuevo.uppercase()
    }

    /// Mismo criterio que [ProveedoresViewModel.rellenarDesdeDocumento]:
    /// devuelve el aviso si la persona ya está adentro (por cualquier vía).
    suspend fun rellenarDesdeDocumento(cedulaLeida: String?, nombreLeido: String?, apellidosLeido: String? = null): String? {
        val nombreCompleto = listOfNotNull(nombreLeido, apellidosLeido)
            .filter { it.isNotBlank() }
            .joinToString(" ")
        if (nombreCompleto.isNotBlank()) cambiarNombre(nombreCompleto)
        val leida = cedulaLeida?.filter(Char::isDigit) ?: return null
        cedula = leida
        revisarCedula(leida)
        return avisoCedulaActiva
    }

    /// Lo pide el formulario de ingreso al aparecer: mientras se escribe, las
    /// conexiones con la nube se calientan y la verificación al registrar ya
    /// no paga conexión nueva. Vuelve enseguida y nunca falla (sin nube o sin
    /// conexión no hace nada).
    fun precalentarNube() {
        viewModelScope.launch(dispatcherIO) { nucleo.precalentarNube() }
    }

    fun registrarIngreso(gafeteNumero: Long, onExito: () -> Unit) {
        if (registrando || cedula.isBlank() || nombre.isBlank() || motivo.isBlank() || cedulaConIngresoActivo) return
        registrando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    // Todas las reglas (cédula adentro aquí o en otra
                    // unidad, gafete de visita en uso en el otro
                    // dispositivo) las aplica el núcleo en esta llamada.
                    medirNucleo("registrarIngresoCorreoVerificado") {
                        nucleo.registrarIngresoCorreoVerificado(
                            cedula,
                            nombre,
                            motivo,
                            placa.trim().ifBlank { null },
                            gafeteNumero,
                        )
                    }
                }
                CambiosNube.cambioLocal()
                mensaje = "Ingreso registrado"
                confirmacion.mostrar("Ingreso registrado · $nombre")
                limpiarFormulario()
                refrescarActivos()
                onExito()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                registrando = false
            }
        }
    }

    /// Al cerrar el formulario sin registrar: mismo motivo que
    /// [ProveedoresViewModel.cancelarFormulario] (que el aviso de la cédula
    /// no quede pegado en la lista).
    fun cancelarFormulario() {
        limpiarFormulario()
    }

    private fun limpiarFormulario() {
        cedula = ""
        nombre = ""
        motivo = ""
        placa = ""
        cedulaConIngresoActivo = false
        avisoCedulaActiva = null
        error = null
    }

    /// Local: cierra en este equipo. Remota: cierra directo contra la nube.
    fun registrarSalida(fila: FilaCorreoActiva) {
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    when (fila) {
                        is FilaCorreoActiva.Local ->
                            medirNucleo("registrarSalidaCorreo") { nucleo.registrarSalidaCorreo(fila.registro.id) }
                        is FilaCorreoActiva.Remota ->
                            medirNucleo("cerrarIngresoCorreoRemoto") { nucleo.cerrarIngresoCorreoRemoto(fila.remoto.uuid) }
                    }
                }
                CambiosNube.cambioLocal()
                val nombreSalida = when (fila) {
                    is FilaCorreoActiva.Local -> fila.registro.nombre
                    is FilaCorreoActiva.Remota -> fila.remoto.nombre
                }
                confirmacion.mostrar("Salida registrada · $nombreSalida")
                refrescarActivos()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            }
        }
    }

    companion object {
        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { CorreoViewModel(nucleo) }
        }
    }
}

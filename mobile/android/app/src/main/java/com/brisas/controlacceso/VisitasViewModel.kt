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
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.VerificacionVisita

/// Estado y llamadas de [PantallaVisitas]: una cédula, y el núcleo decide qué
/// sigue (`Nucleo.verificarVisita`): entrada si tiene cita hoy, salida si ya
/// está adentro, o un aviso. Sin listas ni historial: el historial se
/// consulta en escritorio y en el panel web. Sin reglas propias: lee lo
/// que responde el núcleo y lo muestra.
class VisitasViewModel(
    private val nucleo: Nucleo,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var cedula by mutableStateOf("")
        private set

    /// `null` mientras no se verificó la cédula escrita.
    var verificacion by mutableStateOf<VerificacionVisita?>(null)
        private set
    var verificando by mutableStateOf(false)
        private set

    var gafete by mutableStateOf("")
        private set
    var enVehiculo by mutableStateOf(false)
        private set
    var placa by mutableStateOf("")
        private set
    var registrando by mutableStateOf(false)
        private set

    var error by mutableStateOf<String?>(null)
        private set

    /// Confirmación de lo último que se registró ("Entrada registrada ·
    /// Carlos Rojas"), mientras la pantalla ya espera la próxima cédula.
    var hecho by mutableStateOf<String?>(null)
        private set

    fun cambiarCedula(nueva: String) {
        // Tal cual: qué es un documento válido y su forma única lo decide
        // el núcleo al verificar.
        cedula = nueva
        verificacion = null
        error = null
    }

    fun cambiarGafete(nuevo: String) {
        gafete = nuevo.filter(Char::isDigit)
    }

    fun cambiarEnVehiculo(vehiculo: Boolean) {
        enVehiculo = vehiculo
    }

    fun cambiarPlaca(nueva: String) {
        placa = nueva
    }

    /// Escrita (al confirmar el teclado) o escaneada.
    fun verificar(texto: String = cedula) {
        if (verificando) return
        cedula = texto
        verificando = true
        error = null
        hecho = null
        val consultada = texto
        viewModelScope.launch {
            try {
                val resultado = withContext(dispatcherIO) {
                    medirNucleo("verificarVisita") { nucleo.verificarVisita(consultada) }
                }
                if (consultada != cedula) return@launch // ya se escribió otra
                verificacion = resultado
                gafete = ""
                // Si el anfitrión anotó la placa, se propone "vehículo".
                val sugerida = (resultado as? VerificacionVisita.Entrada)?.visita?.placaSugerida
                enVehiculo = sugerida != null
                placa = sugerida.orEmpty()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                verificando = false
            }
        }
    }

    /// Gafete vacío = sin gafete. En vehículo se manda la placa tal cual:
    /// si falta o no sirve, lo dice el núcleo (`PlacaRequerida`/`PlacaInvalida`).
    fun registrarEntrada() {
        val visita = (verificacion as? VerificacionVisita.Entrada)?.visita ?: return
        if (registrando) return
        registrando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    // Las reglas (cita vigente, placa, gafete libre aquí y
                    // en el otro equipo, visitante adentro en otra unidad)
                    // las aplica el núcleo en esta llamada.
                    medirNucleo("registrarEntradaVisita") {
                        nucleo.registrarEntradaVisita(
                            visita.cedula,
                            gafete.toLongOrNull(),
                            placa.takeIf { enVehiculo },
                        )
                    }
                }
                CambiosNube.cambioLocal()
                terminar("Entrada registrada · ${visita.nombre}")
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                registrando = false
            }
        }
    }

    fun registrarSalida() {
        val visita = (verificacion as? VerificacionVisita.Salida)?.visita ?: return
        if (registrando) return
        registrando = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    medirNucleo("registrarSalidaVisita") { nucleo.registrarSalidaVisita(visita.movimientoId) }
                }
                CambiosNube.cambioLocal()
                terminar("Salida registrada · ${visita.nombre}")
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                registrando = false
            }
        }
    }

    /// Vuelve a la cédula vacía, lista para la próxima persona.
    fun limpiar() {
        cedula = ""
        verificacion = null
        gafete = ""
        enVehiculo = false
        placa = ""
        error = null
    }

    private fun terminar(confirmacion: String) {
        limpiar()
        hecho = confirmacion
    }

    companion object {
        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { VisitasViewModel(nucleo) }
        }
    }
}

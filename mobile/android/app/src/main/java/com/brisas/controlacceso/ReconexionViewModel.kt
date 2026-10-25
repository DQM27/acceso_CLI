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
import uniffi.control_acceso_mobile.ResultadoReconexion

/// En qué parte de la reconexión está la pantalla.
sealed interface PasoReconexion {
    data object Codigo : PasoReconexion

    /// El núcleo pidió confirmar que se pierden registros al cambiar de unidad.
    data class Confirmar(val pendientes: ULong, val unidad: String) : PasoReconexion

    /// Reconectado en otra unidad: la app tiene que reiniciarse.
    data class OtraUnidad(val unidad: String) : PasoReconexion
}

/// Dueño del estado de [PantallaReconectarEquipo]. Todo lo decide el núcleo
/// (`Nucleo.reconectarEquipo`, ver
/// `docs/features-futuras/plan-reconexion-equipo-retirado.md`): acá sólo se
/// manda el código, se pasa a confirmar o a reiniciar según lo que responda,
/// y se muestra el motivo si falla. Un intento a la vez.
class ReconexionViewModel(
    private val nucleo: Nucleo,
    private val metadata: MetadatosDispositivoLocal,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var paso by mutableStateOf<PasoReconexion>(PasoReconexion.Codigo)
        private set
    var enviando by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    /// Con `pendientesConfirmados`, la persona ya vio cuántos registros se
    /// pierden. En la misma unidad, `onReconectado` recibe la unidad.
    fun reconectar(codigo: String, pendientesConfirmados: ULong? = null, onReconectado: (String) -> Unit) {
        if (enviando || !CodigoVinculacion.completo(codigo)) return
        error = null
        enviando = true
        viewModelScope.launch {
            try {
                val resultado = withContext(dispatcherIO) {
                    medirNucleo("reconectarEquipo") {
                        nucleo.reconectarEquipo(
                            codigo = CodigoVinculacion.normalizar(codigo),
                            pendientesConfirmados = pendientesConfirmados,
                            identificadorHardware = metadata.identificadorHardware,
                            nombreDispositivo = metadata.nombreDispositivo,
                            plataforma = metadata.plataforma,
                            versionBuild = metadata.versionBuild,
                            appVersion = metadata.appVersion,
                        )
                    }
                }
                when (resultado) {
                    is ResultadoReconexion.Reconectado -> onReconectado(resultado.unidad)
                    is ResultadoReconexion.NecesitaConfirmacion ->
                        paso = PasoReconexion.Confirmar(resultado.pendientes, resultado.unidad)
                    is ResultadoReconexion.ReconectadoEnOtraUnidad ->
                        paso = PasoReconexion.OtraUnidad(resultado.unidad)
                }
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                enviando = false
            }
        }
    }

    /// Volver a pedir el código (cancelar el aviso de pérdida).
    fun volverAlCodigo() {
        paso = PasoReconexion.Codigo
        error = null
    }

    companion object {
        fun factory(
            nucleo: Nucleo,
            metadata: MetadatosDispositivoLocal,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { ReconexionViewModel(nucleo, metadata) }
        }
    }
}

/// "3 registros" / "1 registro".
internal fun textoRegistros(cantidad: ULong): String =
    if (cantidad == 1uL) "1 registro" else "$cantidad registros"

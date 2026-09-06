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
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.MovimientoHistorial
import uniffi.control_acceso_mobile.MovimientoHistorialSitio
import uniffi.control_acceso_mobile.ResultadoIngresoRegistrado

import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException

data class FilaHistorial(
    val clave: String,
    val cedula: String,
    val contratistaNombre: String,
    val empresaNombre: String,
    val fechaHoraIngreso: String,
    val fechaHoraSalida: String?,
    val gafeteNumero: Long?,
    val usuarioIngresoNombre: String,
    val usuarioSalidaNombre: String?,
    val advertenciaPraind: Boolean,
    // "pc"/"mobile" (`dispositivos.tipo` en Supabase), o null si vino sin
    // dato (fila remota sincronizada antes de que esto existiera). Una fila
    // local siempre es "mobile": esta pantalla sólo existe en el build de
    // Android, no hace falta leerlo de ningún lado.
    val dispositivoTipo: String?,
) {
    companion object {
        fun local(m: MovimientoHistorial) = FilaHistorial(
            "local:${m.registroId}", m.cedula, m.contratistaNombre, m.empresaNombre,
            m.fechaHoraIngreso, m.fechaHoraSalida, m.gafeteNumero,
            m.usuarioIngresoNombre, m.usuarioSalidaNombre,
            m.resultadoAcceso is ResultadoIngresoRegistrado.PermitidoConAdvertencia,
            dispositivoTipo = "mobile",
        )
        fun remota(m: MovimientoHistorialSitio) = FilaHistorial(
            "nube:${m.uuid}", m.cedula ?: "—", m.contratistaNombre, m.empresaNombre ?: "—",
            m.fechaHoraIngreso, m.fechaHoraSalida, m.gafeteNumero,
            m.usuarioIngresoNombre ?: "—", m.usuarioSalidaNombre,
            m.motivoResultado == "PRAIND_PROXIMO_VENCER",
            dispositivoTipo = m.dispositivoEntradaTipo,
        )
    }
}

/// "pc"/"mobile" → texto corto para mostrar en la fila -- cualquier otro
/// valor (o null) se muestra como "—", nunca se inventa un tipo que no vino.
fun textoDispositivo(tipo: String?): String = when (tipo) {
    "pc" -> "💻 PC"
    "mobile" -> "📱 Celular"
    else -> tipo ?: "—"
}

/// Dueño del estado de [PantallaHistorial] y de la llamada a
/// `Nucleo.buscarHistorial` — ver mobile/app/ARQUITECTURA.md. A diferencia
/// de `PantallaConfirmarIngreso` (que a propósito no tiene ViewModel),
/// Historial es una pestaña que persiste mientras el usuario navega
/// (mismo rol que `ActivosViewModel` para su pestaña), así que sí aplica
/// el patrón completo acá.
class HistorialViewModel(
    private val nucleo: Nucleo,
    // Ver el mismo parámetro en ActivosViewModel — permite tests con
    // tiempo controlado en vez de hilos reales.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.Default,
) : ViewModel() {
    var texto by mutableStateOf("")
        private set
    var movimientos by mutableStateOf<List<FilaHistorial>>(emptyList())
        private set
    var error by mutableStateOf<String?>(null)
        private set

    // Cancela la búsqueda anterior si `texto` cambió antes de que
    // terminara — mismo criterio que `ActivosViewModel.buscar`.
    private var trabajoBusqueda: Job? = null

    init {
        buscar()
    }

    fun cambiarTexto(nuevo: String) {
        texto = nuevo
        buscar()
    }

    private fun buscar() {
        trabajoBusqueda?.cancel()
        trabajoBusqueda = viewModelScope.launch {
            try {
                movimientos = withContext(dispatcherIO) {
                    val locales = nucleo.buscarHistorial(texto).map(FilaHistorial::local)
                    val remotos = nucleo.listarHistorialSitio(texto).map(FilaHistorial::remota)
                    (locales + remotos)
                        .sortedByDescending { java.time.Instant.parse(it.fechaHoraIngreso) }
                        .take(30)
                }
                error = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    fun refrescar() = buscar()

    companion object {
        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { HistorialViewModel(nucleo) }
        }
    }
}

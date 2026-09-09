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

/// "pc"/"mobile" → sólo el ícono para mostrar en la fila -- ícono+palabra
/// ("💻 PC"/"📱 Celular") quedaba desparejo visualmente (una palabra bastante
/// más larga que la otra). Cualquier otro valor (o null) se muestra como
/// "—", nunca se inventa un tipo que no vino.
fun textoDispositivo(tipo: String?): String = when (tipo) {
    "pc" -> "💻"
    "mobile" -> "📱"
    else -> tipo ?: "—"
}

/// Dueño del estado de [PantallaHistorial] y de la llamada a
/// `Nucleo.buscarHistorial` — ver mobile/android/ARQUITECTURA.md. A diferencia
/// de `PantallaConfirmarIngreso` (que a propósito no tiene ViewModel),
/// Historial es una pestaña que persiste mientras el usuario navega
/// (mismo rol que `ActivosViewModel` para su pestaña), así que sí aplica
/// el patrón completo acá.
class HistorialViewModel(
    private val nucleo: Nucleo,
    // Ver el mismo parámetro en ActivosViewModel — permite tests con
    // tiempo controlado en vez de hilos reales.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var texto by mutableStateOf("")
        private set
    var movimientos by mutableStateOf<List<FilaHistorial>>(emptyList())
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var cargando by mutableStateOf(false)
        private set

    // Cancela la búsqueda anterior si `texto` cambió antes de que
    // terminara — mismo criterio que `ActivosViewModel.buscar`.
    private var trabajoBusqueda: Job? = null
    private var versionBusqueda = 0L

    init {
        buscar()
    }

    fun cambiarTexto(nuevo: String) {
        texto = nuevo
        // Con debounce: cada tecla no debe disparar un LIKE sobre 180 días
        // de historial (local + remoto) -- sólo la última pulsación
        // después de una pausa llega a `buscar`. Ver el mismo criterio en
        // `ActivosViewModel.cambiarTexto`.
        buscar(debounce = true)
    }

    private fun buscar(debounce: Boolean = false) {
        trabajoBusqueda?.cancel()
        val version = ++versionBusqueda
        cargando = true
        trabajoBusqueda = viewModelScope.launch {
            try {
                if (debounce) delay(DEBOUNCE_BUSQUEDA_MS)
                movimientos = withContext(dispatcherIO) {
                    val locales = nucleo.buscarHistorial(texto).map(FilaHistorial::local)
                    val remotos = nucleo.listarHistorialSitio(texto).map(FilaHistorial::remota)
                    (locales + remotos)
                        // `Instant.parse` exige el sufijo "Z" -- las fechas
                        // que llegan de Rust (`to_rfc3339()`, o crudas de
                        // Supabase) usan offset numérico ("+00:00"), que
                        // `Instant.parse` rechaza (`DateTimeParseException`,
                        // crasheaba la pantalla apenas había algo que
                        // ordenar). `OffsetDateTime.parse` acepta los dos
                        // formatos.
                        .sortedWith(
                            compareByDescending<FilaHistorial> { instanteFechaHora(it.fechaHoraIngreso) != null }
                                .thenByDescending { instanteFechaHora(it.fechaHoraIngreso) },
                        )
                        .take(30)
                }
                error = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } finally {
                if (version == versionBusqueda) cargando = false
            }
        }
    }

    fun refrescar() = buscar()

    companion object {
        // Ver el mismo comentario en `ActivosViewModel`.
        private const val DEBOUNCE_BUSQUEDA_MS = 300L

        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { HistorialViewModel(nucleo) }
        }
    }
}

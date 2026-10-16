package com.brisas.controlacceso

import android.content.Context
import android.os.SystemClock
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import java.io.IOException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.NovedadActualizacion
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.evaluarManifiestoActualizacion

sealed interface EstadoActualizacion {
    /// Nada que mostrar: build sin actualizador, todavía sin revisar, al día
    /// o aviso pospuesto.
    data object SinAviso : EstadoActualizacion

    /// Los demás estados giran en torno a una versión disponible.
    sealed interface ConNovedad : EstadoActualizacion {
        val novedad: NovedadActualizacion.Disponible
    }

    data class Disponible(override val novedad: NovedadActualizacion.Disponible) : ConNovedad
    data class FaltaPermiso(override val novedad: NovedadActualizacion.Disponible) : ConNovedad
    data class Descargando(override val novedad: NovedadActualizacion.Disponible, val porcentaje: Int) : ConNovedad
    data class Instalando(override val novedad: NovedadActualizacion.Disponible) : ConNovedad
    data class Fallo(override val novedad: NovedadActualizacion.Disponible, val mensaje: String) : ConNovedad
}

/// Dueño del estado del actualizador (ver docs/actualizaciones-android.md).
/// Si hay versión nueva y si es obligatoria lo decide el núcleo
/// (`evaluarManifiestoActualizacion`); [FuenteActualizaciones] sólo descarga,
/// verifica e instala.
class ActualizacionViewModel(
    private val fuente: FuenteActualizaciones,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
    private val relojMs: () -> Long = SystemClock::elapsedRealtime,
) : ViewModel() {
    private val _estado = MutableStateFlow<EstadoActualizacion>(EstadoActualizacion.SinAviso)
    val estado: StateFlow<EstadoActualizacion> = _estado.asStateFlow()

    private var ultimaRevisionMs: Long? = null

    init {
        viewModelScope.launch {
            ResultadosInstalacion.resultados.collect { resultado ->
                val actual = _estado.value as? EstadoActualizacion.Instalando ?: return@collect
                _estado.value = when (resultado) {
                    ResultadoInstalacion.CANCELADA -> EstadoActualizacion.Disponible(actual.novedad)
                    ResultadoInstalacion.RECHAZADA ->
                        EstadoActualizacion.Fallo(actual.novedad, "Android rechazó la actualización.")
                }
            }
        }
    }

    /// Consulta el manifiesto, como mucho una vez por hora: cambia una vez
    /// por release, no vale la pena en cada vuelta a primer plano. Silenciosa
    /// ante fallas de red (no tener señal en la portería no es algo que haya
    /// que avisar) y ante un manifiesto inválido (error de publicación, ya
    /// queda en el registro técnico del núcleo).
    fun revisar() {
        if (fuente.urlManifiesto.isEmpty() || ocupado()) return
        val ahora = relojMs()
        val anterior = ultimaRevisionMs
        if (anterior != null && ahora - anterior < MS_ENTRE_REVISIONES) return
        ultimaRevisionMs = ahora
        viewModelScope.launch {
            val novedad = try {
                withContext(dispatcherIO) {
                    evaluarManifiestoActualizacion(fuente.leerManifiesto(), fuente.urlManifiesto, fuente.versionCodeInstalada())
                }
            } catch (_: IOException) {
                return@launch
            } catch (_: NucleoException) {
                return@launch
            }
            if (ocupado()) return@launch
            _estado.value = when (novedad) {
                is NovedadActualizacion.Disponible -> EstadoActualizacion.Disponible(novedad)
                NovedadActualizacion.AlDia -> EstadoActualizacion.SinAviso
            }
        }
    }

    /// Oculta el aviso hasta la próxima revisión. Una actualización
    /// obligatoria no se puede posponer.
    fun posponer() {
        val actual = _estado.value
        if (actual is EstadoActualizacion.Disponible && !actual.novedad.obligatoria) {
            _estado.value = EstadoActualizacion.SinAviso
        }
    }

    fun actualizar() {
        val novedad = (_estado.value as? EstadoActualizacion.ConNovedad)?.novedad ?: return
        if (ocupado()) return
        if (!fuente.puedeInstalar()) {
            _estado.value = EstadoActualizacion.FaltaPermiso(novedad)
            return
        }
        val manifiesto = novedad.manifiesto
        _estado.value = EstadoActualizacion.Descargando(novedad, porcentaje = 0)
        viewModelScope.launch {
            // Sin asignar nada después de `instalar`: desde ese momento el
            // estado lo mueve sólo el resultado del instalador (ver `init`),
            // que puede llegar antes de que esta corrutina retome.
            val fallo = try {
                withContext(dispatcherIO) {
                    val apk = fuente.descargarApk(manifiesto) { descargados ->
                        val porcentaje = ((descargados * 100) / manifiesto.tamanoBytes).toInt().coerceIn(0, 100)
                        if ((_estado.value as? EstadoActualizacion.Descargando)?.porcentaje != porcentaje) {
                            _estado.value = EstadoActualizacion.Descargando(novedad, porcentaje)
                        }
                    }
                    fuente.verificarApk(apk, manifiesto)
                    _estado.value = EstadoActualizacion.Instalando(novedad)
                    fuente.instalar(apk)
                }
                null
            } catch (e: ApkRechazado) {
                e.message ?: "No se pudo verificar la actualización."
            } catch (_: IOException) {
                // Red o disco al descargar, o `PackageInstaller` al abrir la sesión.
                "No se pudo completar la actualización. Revise la conexión e intente de nuevo."
            } catch (_: SecurityException) {
                "Android no permitió instalar la actualización."
            }
            if (fallo != null) _estado.value = EstadoActualizacion.Fallo(novedad, fallo)
        }
    }

    private fun ocupado(): Boolean =
        _estado.value.let { it is EstadoActualizacion.Descargando || it is EstadoActualizacion.Instalando }

    companion object {
        private const val MS_ENTRE_REVISIONES = 60 * 60 * 1000L

        fun factory(context: Context): ViewModelProvider.Factory = viewModelFactory {
            initializer { ActualizacionViewModel(ServicioActualizacionesAndroid(context)) }
        }
    }
}

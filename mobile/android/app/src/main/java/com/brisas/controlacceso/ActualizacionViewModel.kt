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

    /// `pospuesta`: la persona tocó "Más tarde". La franja se oculta (sigue el
    /// aviso discreto del menú de la cuenta) y no vuelve en cada revisión,
    /// sólo si aparece una versión más nueva. Una obligatoria no se pospone.
    data class Disponible(
        override val novedad: NovedadActualizacion.Disponible,
        val pospuesta: Boolean = false,
    ) : ConNovedad
    data class FaltaPermiso(override val novedad: NovedadActualizacion.Disponible) : ConNovedad
    data class Descargando(override val novedad: NovedadActualizacion.Disponible, val porcentaje: Int) : ConNovedad
    data class Instalando(override val novedad: NovedadActualizacion.Disponible) : ConNovedad
    data class Fallo(override val novedad: NovedadActualizacion.Disponible, val mensaje: String) : ConNovedad
}

/// Resultado de "Buscar actualizaciones" en el menú de la cuenta: la
/// respuesta a una consulta que pidió la persona (no la revisión automática).
enum class EstadoBusqueda { INACTIVA, BUSCANDO, AL_DIA, SIN_RESPUESTA }

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

    private val _busqueda = MutableStateFlow(EstadoBusqueda.INACTIVA)
    val busqueda: StateFlow<EstadoBusqueda> = _busqueda.asStateFlow()

    /// `false` en los builds de prueba (sin dirección del manifiesto): no hay
    /// nada que buscar y el menú no ofrece el botón.
    val activo: Boolean get() = fuente.urlManifiesto.isNotEmpty()

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
        if (!activo || ocupado()) return
        val ahora = relojMs()
        val anterior = ultimaRevisionMs
        if (anterior != null && ahora - anterior < MS_ENTRE_REVISIONES) return
        ultimaRevisionMs = ahora
        viewModelScope.launch {
            val novedad = (consultar() as? Consulta.Respuesta)?.novedad ?: return@launch
            if (ocupado()) return@launch
            aplicar(novedad)
        }
    }

    /// "Buscar actualizaciones" del menú: consulta ya, sin esperar la hora
    /// entre revisiones, y deja dicho qué respondió (al día, o no se pudo).
    /// Si hay versión nueva, queda disponible y a la vista (no pospuesta).
    fun buscarAhora() {
        if (!activo || ocupado() || _busqueda.value == EstadoBusqueda.BUSCANDO) return
        ultimaRevisionMs = relojMs()
        _busqueda.value = EstadoBusqueda.BUSCANDO
        viewModelScope.launch {
            val consulta = consultar()
            if (ocupado()) {
                _busqueda.value = EstadoBusqueda.INACTIVA
                return@launch
            }
            when (consulta) {
                Consulta.SinRespuesta -> _busqueda.value = EstadoBusqueda.SIN_RESPUESTA
                is Consulta.Respuesta -> {
                    aplicar(consulta.novedad, repetirAviso = true)
                    _busqueda.value = when (consulta.novedad) {
                        is NovedadActualizacion.Disponible -> EstadoBusqueda.INACTIVA
                        NovedadActualizacion.AlDia -> EstadoBusqueda.AL_DIA
                    }
                }
            }
        }
    }

    /// Al cerrar el menú: el "Está al día" o "No se pudo revisar" era para
    /// esa vez, la próxima apertura vuelve a ofrecer el botón.
    fun olvidarBusqueda() {
        if (_busqueda.value != EstadoBusqueda.BUSCANDO) _busqueda.value = EstadoBusqueda.INACTIVA
    }

    private sealed interface Consulta {
        data class Respuesta(val novedad: NovedadActualizacion) : Consulta
        data object SinRespuesta : Consulta
    }

    /// Sin red o con un manifiesto inválido no hay respuesta: lo primero no
    /// es algo que haya que avisar en la portería, y lo segundo es un error de
    /// publicación que ya queda en el registro técnico del núcleo.
    private suspend fun consultar(): Consulta = try {
        Consulta.Respuesta(
            withContext(dispatcherIO) {
                evaluarManifiestoActualizacion(fuente.leerManifiesto(), fuente.urlManifiesto, fuente.versionCodeInstalada())
            },
        )
    } catch (_: IOException) {
        Consulta.SinRespuesta
    } catch (_: NucleoException) {
        Consulta.SinRespuesta
    }

    /// `repetirAviso`: una búsqueda pedida a mano vuelve a mostrar la franja
    /// aunque la persona haya dicho "Más tarde" antes.
    private fun aplicar(novedad: NovedadActualizacion, repetirAviso: Boolean = false) {
        _estado.value = when (novedad) {
            is NovedadActualizacion.Disponible -> {
                val actual = _estado.value as? EstadoActualizacion.Disponible
                val yaPospuesta = !repetirAviso && !novedad.obligatoria && actual != null && actual.pospuesta &&
                    actual.novedad.manifiesto.versionCode == novedad.manifiesto.versionCode
                EstadoActualizacion.Disponible(novedad, pospuesta = yaPospuesta)
            }
            NovedadActualizacion.AlDia -> EstadoActualizacion.SinAviso
        }
    }

    /// Oculta la franja pero deja la versión pendiente a la vista en el menú
    /// de la cuenta. Una actualización obligatoria no se puede posponer.
    fun posponer() {
        val actual = _estado.value
        if (actual is EstadoActualizacion.Disponible && !actual.novedad.obligatoria) {
            _estado.value = actual.copy(pospuesta = true)
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

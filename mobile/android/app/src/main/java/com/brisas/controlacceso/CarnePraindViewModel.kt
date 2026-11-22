package com.brisas.controlacceso

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.net.URI
import uniffi.control_acceso_mobile.CargaCarnePraind
import uniffi.control_acceso_mobile.CarnePraind
import uniffi.control_acceso_mobile.EstadoCarnePraind
import uniffi.control_acceso_mobile.GafeteGuardado
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.PermisosCarnePraind
import uniffi.control_acceso_mobile.PreviaGafetePraind
import uniffi.control_acceso_mobile.RevisionGafetePraind

/// Lo que el ViewModel necesita del núcleo (en las pruebas, un doble).
interface FuenteCarnes {
    fun estado(contratistaId: Long): EstadoCarnePraind
    fun ver(contratistaId: Long): CarnePraind
    fun permisos(): PermisosCarnePraind
    fun cargarDesdeQr(contratistaId: Long, enlace: String, soloPrevisualizar: Boolean): CargaCarnePraind
    fun previsualizarConImagen(contratistaId: Long, enlace: String): PreviaGafetePraind
    fun revisar(contratistaId: Long, textoOcr: String, nombreEnOrigen: String?): RevisionGafetePraind
    /// Guarda el gafete y corrige la ficha en una sola llamada: el orden y
    /// las reglas son del núcleo (`guardar_gafete_praind_revisado`).
    fun guardarRevisado(
        contratistaId: Long,
        enlace: String,
        textoOcr: String,
        nombreEnOrigen: String?,
        corregirNombre: Boolean,
        corregirEmpresa: Boolean,
    ): GafeteGuardado
}

private class FuenteCarnesNucleo(private val nucleo: Nucleo) : FuenteCarnes {
    override fun estado(contratistaId: Long) = nucleo.estadoCarnePraind(contratistaId)
    override fun ver(contratistaId: Long) = nucleo.verCarnePraind(contratistaId)
    override fun permisos() = nucleo.permisosCarnePraind()
    override fun cargarDesdeQr(contratistaId: Long, enlace: String, soloPrevisualizar: Boolean) =
        nucleo.cargarCarnePraindDesdeQr(contratistaId, enlace, soloPrevisualizar)
    override fun previsualizarConImagen(contratistaId: Long, enlace: String) =
        nucleo.previsualizarCarnePraindConImagen(contratistaId, enlace)
    override fun revisar(contratistaId: Long, textoOcr: String, nombreEnOrigen: String?) =
        nucleo.revisarGafetePraind(contratistaId, textoOcr, nombreEnOrigen)
    override fun guardarRevisado(
        contratistaId: Long,
        enlace: String,
        textoOcr: String,
        nombreEnOrigen: String?,
        corregirNombre: Boolean,
        corregirEmpresa: Boolean,
    ) = nucleo.guardarGafetePraindRevisado(contratistaId, enlace, textoOcr, nombreEnOrigen, corregirNombre, corregirEmpresa)
}

/// El QR del gafete PRAIND lleva un enlace web (`https://prd.pw/…`). La
/// cámara acepta cualquier enlace http(s) y deja que la nube diga si es de
/// un gafete PRAIND: la lista de sitios válidos vive en un solo lugar (el
/// Worker), no repetida acá. Lo que no es un enlace (el QR de vinculación,
/// otros códigos) se ignora y la cámara sigue mirando.
object EnlaceQrPraind {
    fun desdeQr(contenido: String): String? {
        val texto = contenido.trim()
        val uri = runCatching { URI(texto) }.getOrNull() ?: return null
        val esquema = uri.scheme?.lowercase() ?: return null
        if (esquema != "https" && esquema != "http") return null
        if (uri.host.isNullOrBlank()) return null
        return texto
    }
}

/// Cargar el gafete PRAIND desde el QR: primero se muestra de quién es (sin
/// guardar nada) y recién al confirmar se guarda.
sealed interface CargaCarne {
    data object Inactiva : CargaCarne
    data class Buscando(val nombre: String) : CargaCarne
    /// Lo que encontró la nube, todavía sin guardar. [revision]: el gafete
    /// leído con OCR contra la ficha; `null` si la imagen no se pudo leer
    /// (queda sólo la comparación del nombre de la página).
    data class Previa(
        val contratistaId: Long,
        val nombre: String,
        val enlace: String,
        val nombreEnOrigen: String?,
        val coincide: Boolean,
        val guardando: Boolean = false,
        val revision: RevisionGafetePraind? = null,
        /// El texto que el OCR leyó de la imagen: el núcleo lo vuelve a
        /// revisar contra la ficha al guardar.
        val textoOcr: String = "",
        /// Si el portero eligió corregir la ficha con el nombre o la empresa
        /// del gafete (sólo si difieren). El vencimiento de un gafete más
        /// nuevo se corrige siempre.
        val corregirNombre: Boolean = false,
        val corregirEmpresa: Boolean = false,
    ) : CargaCarne {
        val sePuedeGuardar: Boolean get() = revision?.bloqueaGuardar != true
    }
    data class Error(val nombre: String, val mensaje: String) : CargaCarne
}

sealed interface VistaCarne {
    data object Cerrada : VistaCarne
    data class Cargando(val nombre: String) : VistaCarne
    /// Los bytes de la imagen tal cual (PNG o JPEG); la pantalla los decodifica.
    data class Imagen(val nombre: String, val bytes: ByteArray) : VistaCarne {
        override fun equals(other: Any?) = other is Imagen && other.nombre == nombre && other.bytes.contentEquals(bytes)
        override fun hashCode() = 31 * nombre.hashCode() + bytes.contentHashCode()
    }
    data class Error(val nombre: String, val mensaje: String) : VistaCarne
}

/// Carné PRAIND del contratista que se está por registrar (fase 6 de
/// docs/features-futuras/plan-fotos-praind.md). El carné se baja recién al
/// pedirlo y queda guardado en el teléfono: la segunda vez sale al instante
/// y sin internet. Sin internet y sin haberlo visto antes, el núcleo devuelve
/// el aviso de que hace falta conexión.
class CarnePraindViewModel(
    private val fuente: FuenteCarnes,
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
    private val lector: LectorTextoImagen = LectorTextoMlKit(),
) : ViewModel() {
    /// Si el contratista de la hoja abierta tiene carné (muestra "Ver carné").
    private val _disponible = MutableStateFlow(false)
    val disponible: StateFlow<Boolean> = _disponible.asStateFlow()

    private val _vista = MutableStateFlow<VistaCarne>(VistaCarne.Cerrada)
    val vista: StateFlow<VistaCarne> = _vista.asStateFlow()

    /// Si la nube deja cargar gafetes PRAIND desde este teléfono (lo decide
    /// el panel). Se pregunta una vez; sin conexión se vuelve a intentar al
    /// abrir la próxima hoja.
    private val _puedeCargar = MutableStateFlow(false)
    val puedeCargar: StateFlow<Boolean> = _puedeCargar.asStateFlow()
    private var permisosConsultados = false

    private val _carga = MutableStateFlow<CargaCarne>(CargaCarne.Inactiva)
    val carga: StateFlow<CargaCarne> = _carga.asStateFlow()

    /// Se llena al guardar: la pantalla muestra "Gafete PRAIND de … guardado".
    private val _guardado = MutableStateFlow<String?>(null)
    val guardado: StateFlow<String?> = _guardado.asStateFlow()

    private var consulta: Job? = null
    private var lectura: Job? = null
    private var subida: Job? = null
    private var contratistaHoja: Long? = null

    /// Consulta local (sin red) al abrir la hoja de un contratista.
    fun consultar(contratistaId: Long?) {
        consulta?.cancel()
        contratistaHoja = contratistaId
        _disponible.value = false
        if (contratistaId == null) return
        consultarPermisos()
        consulta = viewModelScope.launch {
            _disponible.value = withContext(dispatcherIO) {
                try {
                    fuente.estado(contratistaId).tieneCarne
                } catch (excepcion: Exception) {
                    excepcion.mensajeDeErrorEsperado()
                    false
                }
            }
        }
    }

    fun abrir(contratistaId: Long, nombre: String) {
        lectura?.cancel()
        _vista.value = VistaCarne.Cargando(nombre)
        lectura = viewModelScope.launch {
            _vista.value = withContext(dispatcherIO) {
                try {
                    VistaCarne.Imagen(nombre, fuente.ver(contratistaId).bytes)
                } catch (excepcion: Exception) {
                    VistaCarne.Error(nombre, excepcion.mensajeDeErrorEsperado() ?: "No se pudo mostrar el gafete PRAIND.")
                }
            }
        }
    }

    fun cerrar() {
        lectura?.cancel()
        _vista.value = VistaCarne.Cerrada
    }

    private fun consultarPermisos() {
        if (permisosConsultados) return
        viewModelScope.launch {
            val permisos = withContext(dispatcherIO) {
                try {
                    fuente.permisos()
                } catch (excepcion: Exception) {
                    excepcion.mensajeDeErrorEsperado()
                    null
                }
            } ?: return@launch
            _puedeCargar.value = permisos.puedeCargar
            // Sin conexión no se sabe: se pregunta otra vez la próxima hoja.
            permisosConsultados = !permisos.sinConexion
        }
    }

    /// Con el enlace leído del QR, baja el gafete sin guardar nada, lo lee
    /// con OCR y lo revisa contra la ficha (cédula, nombre, empresa,
    /// vencimiento).
    fun previsualizar(contratistaId: Long, nombre: String, enlace: String) {
        subida?.cancel()
        _carga.value = CargaCarne.Buscando(nombre)
        subida = viewModelScope.launch {
            val previa = withContext(dispatcherIO) {
                try {
                    fuente.previsualizarConImagen(contratistaId, enlace)
                } catch (excepcion: Exception) {
                    _carga.value = CargaCarne.Error(
                        nombre,
                        excepcion.mensajeDeErrorEsperado() ?: "No se pudo buscar el gafete PRAIND.",
                    )
                    null
                }
            } ?: return@launch
            // Si el OCR o la revisión fallan, se sigue con la comparación del
            // nombre de la página: no impide cargar el gafete.
            val texto = runCatching { lector.leer(previa.imagen) }.getOrNull()
            val revision = texto?.let {
                withContext(dispatcherIO) {
                    try {
                        fuente.revisar(contratistaId, it, previa.nombreEnOrigen)
                    } catch (excepcion: Exception) {
                        excepcion.mensajeDeErrorEsperado()
                        null
                    }
                }
            }
            _carga.value = CargaCarne.Previa(
                contratistaId,
                nombre,
                enlace,
                previa.nombreEnOrigen,
                previa.nombreCoincide,
                revision = revision,
                textoOcr = texto.orEmpty(),
            )
        }
    }

    /// Guarda el gafete que se previsualizó y corrige la ficha con él
    /// (vencimiento más nuevo y, si se marcó, nombre y empresa). Todo lo
    /// decide el núcleo en una llamada; acá sólo se muestra el resultado.
    fun guardar() {
        val previa = _carga.value as? CargaCarne.Previa ?: return
        if (previa.guardando || !previa.sePuedeGuardar) return
        _carga.value = previa.copy(guardando = true)
        subida = viewModelScope.launch {
            val resultado = withContext(dispatcherIO) {
                runCatching {
                    fuente.guardarRevisado(
                        previa.contratistaId,
                        previa.enlace,
                        previa.textoOcr,
                        previa.nombreEnOrigen,
                        corregirNombre = previa.corregirNombre,
                        corregirEmpresa = previa.corregirEmpresa,
                    )
                }
            }
            val guardado = resultado.getOrElse { excepcion ->
                _carga.value = CargaCarne.Error(
                    previa.nombre,
                    (excepcion as? Exception)?.mensajeDeErrorEsperado() ?: "No se pudo guardar el gafete PRAIND.",
                )
                return@launch
            }
            if (contratistaHoja == previa.contratistaId) _disponible.value = true
            // La ficha corregida sube ya, como cualquier cambio de este
            // teléfono (si no, esperaba a la próxima sincronización).
            if (guardado.fichaCambio) CambiosNube.cambioLocal()
            if (guardado.avisoEsError) {
                // El gafete quedó guardado; sólo falló la corrección.
                _carga.value = CargaCarne.Error(previa.nombre, guardado.aviso)
                return@launch
            }
            _carga.value = CargaCarne.Inactiva
            _guardado.value = guardado.aviso
        }
    }

    /// Marcar o desmarcar "corregir la ficha" con el nombre del gafete.
    fun alternarCorregirNombre() {
        val previa = _carga.value as? CargaCarne.Previa ?: return
        if (previa.guardando || previa.revision?.nombreParaFicha == null) return
        _carga.value = previa.copy(corregirNombre = !previa.corregirNombre)
    }

    /// Marcar o desmarcar "corregir la ficha" con la empresa del gafete.
    fun alternarCorregirEmpresa() {
        val previa = _carga.value as? CargaCarne.Previa ?: return
        if (previa.guardando || previa.revision?.empresaParaFicha == null) return
        _carga.value = previa.copy(corregirEmpresa = !previa.corregirEmpresa)
    }

    fun cancelarCarga() {
        subida?.cancel()
        _carga.value = CargaCarne.Inactiva
    }

    fun avisoGuardadoVisto() {
        _guardado.value = null
    }

    companion object {
        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { CarnePraindViewModel(FuenteCarnesNucleo(nucleo), lector = LectorTextoMlKit()) }
        }
    }
}

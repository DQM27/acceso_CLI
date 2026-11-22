package com.brisas.controlacceso

import androidx.compose.runtime.derivedStateOf
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
import uniffi.control_acceso_mobile.AltaConGafete
import uniffi.control_acceso_mobile.ContratistaYaRegistrado
import uniffi.control_acceso_mobile.DatosAltaDesdeGafete
import uniffi.control_acceso_mobile.DatosContratista
import uniffi.control_acceso_mobile.Empresa
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.GafeteGuardado
import uniffi.control_acceso_mobile.PreviaGafeteParaAlta
import uniffi.control_acceso_mobile.TipoIngreso
import uniffi.control_acceso_mobile.gafeteEsDeCedula
import uniffi.control_acceso_mobile.nombreMientrasSeEscribe

/// Lo que el alta por QR necesita del núcleo y de la nube (en las pruebas,
/// un doble para lo que sale a internet).
interface FuenteGafeteAlta {
    /// Si la nube deja cargar gafetes PRAIND desde este teléfono; `false`
    /// también sin conexión.
    fun puedeCargar(): Boolean
    fun previsualizar(enlace: String): PreviaGafeteParaAlta
    fun datos(textoOcr: String, nombreEnOrigen: String?): DatosAltaDesdeGafete

    /// El alta y, si hay QR, el gafete: el orden y la regla de la cédula son
    /// del núcleo (`crear_contratista_con_gafete`).
    fun crearConGafete(datos: DatosContratista, enlace: String?, cedulaGafete: String?): AltaConGafete

    /// Actualizar la ficha que ya existe con el gafete: lo guarda y corrige
    /// la ficha con todo lo que dice (`guardar_gafete_praind_revisado`).
    fun actualizarConGafete(
        contratistaId: Long,
        enlace: String,
        textoOcr: String,
        nombreEnOrigen: String?,
    ): GafeteGuardado
}

private class FuenteGafeteAltaNucleo(private val nucleo: Nucleo) : FuenteGafeteAlta {
    override fun puedeCargar() = nucleo.permisosCarnePraind().puedeCargar
    override fun previsualizar(enlace: String) = nucleo.previsualizarGafetePraindParaAlta(enlace)
    override fun datos(textoOcr: String, nombreEnOrigen: String?) =
        nucleo.datosAltaDesdeGafetePraind(textoOcr, nombreEnOrigen)
    override fun crearConGafete(datos: DatosContratista, enlace: String?, cedulaGafete: String?) =
        medirNucleo("crearContratistaConGafete") { nucleo.crearContratistaConGafete(datos, enlace, cedulaGafete) }
    override fun actualizarConGafete(contratistaId: Long, enlace: String, textoOcr: String, nombreEnOrigen: String?) =
        nucleo.guardarGafetePraindRevisado(
            contratistaId,
            enlace,
            textoOcr,
            nombreEnOrigen,
            corregirNombre = true,
            corregirEmpresa = true,
        )
}

/// Lo que se avisa al volver a la lista tras guardar: si el gafete PRAIND
/// del QR quedó cargado o no.
data class AvisoAlta(val texto: String, val esError: Boolean)

/// Dueño del estado y de las llamadas a [Nucleo] del formulario "Nuevo
/// contratista" (punto M1 de
/// `docs/auditorias/auditoria-movil-arquitectura-2026-09-27.md`) -- mismo
/// criterio que [ActivosViewModel]/[RutasViewModel]: el `@Composable` sólo
/// lee este estado y reporta eventos. Antes la pantalla llamaba al núcleo
/// directo, manejaba sus corrutinas y decidía si el PRAIND estaba vencido.
///
/// Sólo alta, no edición (ver docs/plan-app-movil.md). Este ViewModel no
/// decide ninguna regla de negocio: `requierePraind`, `praindVencido` y
/// `muestraPersonalRuta` le preguntan al núcleo (`domain::contratista`), y
/// al guardar `ContratistaService::crear_en_persona` vuelve a aplicarlas
/// todas (además deja el acceso habilitado). Lo de acá es sólo el aviso
/// inmediato mientras se completa el formulario.
class NuevoContratistaViewModel(
    private val nucleo: Nucleo,
    // Inyectable para tests con un dispatcher de tiempo controlado, mismo
    // motivo que en `ActivosViewModel`.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
    private val fuenteGafete: FuenteGafeteAlta = FuenteGafeteAltaNucleo(nucleo),
    private val lector: LectorTextoImagen = LectorTextoMlKit(),
) : ViewModel() {
    /// Si se ofrece "Escanear QR del gafete PRAIND" (la nube lo permite para
    /// este teléfono y hay conexión).
    var puedeLeerQr by mutableStateOf(false)
        private set

    /// Ya se sabe si se ofrece el QR, o pasó [ESPERA_PERMISO_QR_MS] sin
    /// respuesta (sin conexión la consulta puede tardar): la pantalla decide
    /// entonces qué cámara abrir primero.
    var permisoQrResuelto by mutableStateOf(false)
        private set

    /// Bajando y leyendo el gafete del QR.
    var leyendoQr by mutableStateOf(false)
        private set

    /// El enlace del QR leído: al guardar, el gafete se carga solo.
    private var enlaceGafete by mutableStateOf<String?>(null)

    /// La cédula que dice ese gafete: si la corrigen a otra, el gafete ya no
    /// es de esta persona y no se carga.
    private var cedulaGafete by mutableStateOf<String?>(null)

    /// Si al guardar se cargará el gafete PRAIND del QR (la pantalla lo dice).
    val cargaraGafete: Boolean get() = enlaceGafete != null && gafeteEsDeCedula(cedulaGafete, cedula)

    private var registradoLeido by mutableStateOf<ContratistaYaRegistrado?>(null)

    /// Lo leído del gafete (texto del OCR y nombre de la página del QR),
    /// para revisarlo contra la ficha que ya existe al actualizarla.
    private var textoGafete: String = ""
    private var nombreEnOrigenGafete: String? = null

    /// La cédula del gafete del QR ya tiene ficha: no es un alta. La
    /// pantalla ofrece actualizar esa ficha con el gafete (sólo por QR).
    /// Si se cambia la cédula a mano deja de valer.
    val yaRegistrado: ContratistaYaRegistrado? get() =
        registradoLeido.takeIf { cedulaGafete != null && gafeteEsDeCedula(cedulaGafete, cedula) }

    /// El enlace del QR leído, para revisar el gafete contra la ficha que ya
    /// existe.
    val enlaceGafeteLeido: String? get() = enlaceGafete

    var empresas by mutableStateOf<List<Empresa>>(emptyList())
        private set
    var cedula by mutableStateOf("")
        private set
    var nombre by mutableStateOf("")
        private set
    var empresaSeleccionada by mutableStateOf<Empresa?>(null)
        private set

    /// Nombre de empresa leído del carnet cuando no calzó con ninguna de
    /// [empresas] -- se muestra como pista para elegir a mano, nunca se
    /// manda a Rust (que sólo acepta un `empresa_id` real).
    var empresaSugeridaTexto by mutableStateOf<String?>(null)
        private set
    var tipoIngreso by mutableStateOf(TipoIngreso.PRAIND)
        private set
    var personalRuta by mutableStateOf(false)
        private set
    var fechaPraind by mutableStateOf("")
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var mensaje by mutableStateOf<String?>(null)
        private set
    var enviando by mutableStateOf(false)
        private set

    // Las tres le preguntan al núcleo, pero con `derivedStateOf`: la respuesta
    // queda guardada y sólo se vuelve a preguntar cuando cambia lo que leen
    // (tipo, personal de ruta, fecha). Antes eran getters: cada tecla en el
    // nombre o la cédula recomponía la pantalla y eso eran 3 o 4 cruces al
    // núcleo por JNA (`requierePraind` dos veces).
    val requierePraind: Boolean by derivedStateOf {
        medirNucleo("requierePraindParaFormulario") { nucleo.requierePraindParaFormulario(tipoIngreso, personalRuta) }
    }

    /// Vencido en el instante en que se tipea, no recién al guardar. Una
    /// fecha a medio escribir ("10-09-202") el núcleo la toma como no
    /// vencida. Bloquea el botón Guardar (pedido del usuario 2026-09-20).
    val praindVencido: Boolean by derivedStateOf {
        requierePraind && fechaPraind.isNotBlank() &&
            medirNucleo("praindVencidoParaFormulario") { nucleo.praindVencidoParaFormulario(textoDDMMYYYYaIso(fechaPraind)) }
    }

    val muestraPersonalRuta: Boolean by derivedStateOf {
        medirNucleo("admitePersonalRutaParaFormulario") { nucleo.admitePersonalRutaParaFormulario(tipoIngreso) }
    }

    init {
        viewModelScope.launch {
            try {
                empresas = withContext(dispatcherIO) { medirNucleo("listarEmpresas") { nucleo.listarEmpresas() } }
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            }
        }
        viewModelScope.launch {
            puedeLeerQr = withContext(dispatcherIO) {
                runCatching { fuenteGafete.puedeCargar() }.getOrDefault(false)
            }
            permisoQrResuelto = true
        }
        viewModelScope.launch {
            delay(ESPERA_PERMISO_QR_MS)
            permisoQrResuelto = true
        }
    }

    fun cambiarCedula(texto: String) {
        cedula = texto.filter(Char::isDigit)
    }

    /// Siempre en mayúscula, venga del teclado o del OCR: la regla de todo
    /// nombre la dicta el núcleo (`nombreMientrasSeEscribe`).
    fun cambiarNombre(texto: String) {
        nombre = nombreMientrasSeEscribe(texto)
    }

    private var prellenado = false

    /// Lo que se buscó (o se leyó) en Activos sin encontrar a nadie: una
    /// cédula va a la cédula y cualquier otra cosa al nombre. Una sola vez
    /// por formulario, para que girar la pantalla no pise lo ya corregido.
    fun prellenar(texto: String) {
        if (prellenado) return
        prellenado = true
        val limpio = texto.trim()
        if (limpio.isEmpty()) return
        if (limpio.all { it.isDigit() || it == '-' || it.isWhitespace() }) cambiarCedula(limpio) else cambiarNombre(limpio)
    }

    fun elegirEmpresa(empresa: Empresa) {
        empresaSeleccionada = empresa
        empresaSugeridaTexto = null
    }

    fun cambiarTipoIngreso(tipo: TipoIngreso) {
        tipoIngreso = tipo
        // El check se oculta para estos dos tipos: sin este reseteo un
        // `true` que quedó de un tipo anterior seguía contando para
        // `requierePraind` sin poder destildarse.
        if (!muestraPersonalRuta) personalRuta = false
    }

    fun cambiarPersonalRuta(valor: Boolean) {
        personalRuta = valor
    }

    fun cambiarFechaPraind(texto: String) {
        fechaPraind = formatearFechaDDMMYYYY(texto)
    }

    /// Todo en blanco antes de abrir la cámara: si se escanea un segundo
    /// carnet (persona equivocada, mala lectura) no se mezclan los datos del
    /// primero (pedido del usuario 2026-09-20).
    fun limpiarParaEscanear() {
        cedula = ""
        nombre = ""
        empresaSeleccionada = null
        empresaSugeridaTexto = null
        tipoIngreso = TipoIngreso.PRAIND
        personalRuta = false
        fechaPraind = ""
        error = null
        mensaje = null
        enlaceGafete = null
        cedulaGafete = null
        registradoLeido = null
        textoGafete = ""
        nombreEnOrigenGafete = null
    }

    private var lecturaQr: Job? = null

    /// Con el enlace del QR del gafete PRAIND: baja la imagen (sin guardar
    /// nada), la lee con OCR y rellena el formulario. El gafete se carga
    /// recién al guardar el contratista.
    fun aplicarQrGafete(enlace: String) {
        // Con un guardado o una actualización en curso no se cambian los
        // datos debajo: se terminaría guardando una mezcla de dos gafetes.
        if (enviando) return
        limpiarParaEscanear()
        lecturaQr?.cancel()
        leyendoQr = true
        lecturaQr = viewModelScope.launch {
            try {
                val previa = withContext(dispatcherIO) { fuenteGafete.previsualizar(enlace) }
                // Sin texto (falló el OCR) igual sirve el nombre de la página.
                val texto = runCatching { lector.leer(previa.imagen) }.getOrNull().orEmpty()
                val datos = withContext(dispatcherIO) { fuenteGafete.datos(texto, previa.nombreEnOrigen) }
                aplicarDatosGafete(datos)
                enlaceGafete = enlace
                cedulaGafete = datos.cedula
                registradoLeido = datos.yaRegistrado
                textoGafete = texto
                nombreEnOrigenGafete = previa.nombreEnOrigen
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado() ?: "No se pudo leer el gafete PRAIND."
            } finally {
                leyendoQr = false
            }
        }
    }

    private fun aplicarDatosGafete(datos: DatosAltaDesdeGafete) {
        datos.cedula?.let(::cambiarCedula)
        datos.nombre?.let(::cambiarNombre)
        tipoIngreso = TipoIngreso.PRAIND
        datos.vencimiento?.let { fechaPraind = isoATextoDDMMYYYY(it) }
        val catalogo = datos.empresaCatalogo?.let { propuesta -> empresas.firstOrNull { it.id == propuesta.id } }
        if (catalogo != null) {
            elegirEmpresa(catalogo)
        } else {
            empresaSugeridaTexto = datos.empresa
        }
        // Qué hacer ahora lo dice el núcleo (`mensaje_alta_desde_gafete`).
        if (datos.mensajeEsError) error = datos.mensaje else mensaje = datos.mensaje
    }

    /// Sólo el carnet PRAIND rellena el formulario: es el único documento
    /// que trae, además de cédula y nombre, la empresa y el vencimiento de
    /// la inducción. Cualquier otro se rechaza entero, sin tocar campos
    /// (pedido del usuario 2026-09-20).
    fun aplicarDocumentoEscaneado(documento: DocumentoDetectado) {
        mensaje = null
        if (documento.tipo != TipoDocumento.CARNET_INDUCCION_PRAIND) {
            error = "Documento inválido"
            return
        }
        error = null
        if (documento.numeroDocumento.isNotBlank()) {
            cedula = documento.numeroDocumento.filter(Char::isDigit).ifBlank { documento.numeroDocumento }
        }
        documento.nombre?.let { cambiarNombre(it) }
        tipoIngreso = TipoIngreso.PRAIND
        documento.vencimiento?.let { fechaPraind = it.aTextoDDMMYYYY() }
        val textoEmpresa = documento.empresa?.trim()
        if (!textoEmpresa.isNullOrBlank()) {
            val coincidencia = buscarEmpresaPorNombre(empresas, textoEmpresa)
            if (coincidencia != null) elegirEmpresa(coincidencia) else empresaSugeridaTexto = textoEmpresa
        }
        mensaje = "Gafete PRAIND leído — revise los datos antes de guardar"
    }

    /// Guarda y, si sale bien, llama a [onGuardado] con el contratista nuevo
    /// (la pantalla pasa a darle el ingreso). Corre en
    /// `viewModelScope`: salir de la pantalla a mitad de camino ya no
    /// cancela el aviso a la nube (antes hacía falta `NonCancellable`).
    fun guardar(onGuardado: (contratistaId: Long, aviso: AvisoAlta?) -> Unit) {
        error = null
        mensaje = null
        // Sin empresa no hay `empresa_id` que mandar; el resto (cédula,
        // nombre, PRAIND) lo valida el núcleo y su mensaje se muestra tal cual.
        val empresa = empresaSeleccionada
        if (empresa == null) {
            error = "Elija la empresa"
            return
        }
        if (enviando || leyendoQr) return
        enviando = true
        val datos = DatosContratista(
            cedula = cedula,
            nombre = nombre,
            empresaId = empresa.id,
            tipoIngreso = tipoIngreso,
            fechaVencimientoPraind = fechaPraind.ifBlank { null }?.let(::textoDDMMYYYYaIso),
            esPersonalRuta = personalRuta,
        )
        val enlace = enlaceGafete
        val cedulaDelGafete = cedulaGafete
        viewModelScope.launch {
            try {
                // El alta y el gafete del QR en una llamada al núcleo.
                val alta = withContext(dispatcherIO) { fuenteGafete.crearConGafete(datos, enlace, cedulaDelGafete) }
                CambiosNube.cambioLocal()
                onGuardado(alta.contratistaId, alta.aviso?.let { AvisoAlta(it, alta.avisoEsError) })
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                enviando = false
            }
        }
    }

    /// La cédula del gafete del QR ya tiene ficha: se actualiza directo, sin
    /// otra ventana (pedido del usuario 2026-11-22), y después
    /// [onActualizado] abre su ingreso. Qué se guarda, en qué orden y qué
    /// se corrige lo decide el núcleo (`guardar_gafete_praind_revisado`):
    /// primero el gafete (sin él la ficha no cambia y se puede reintentar),
    /// la fecha sólo si es más nueva y nunca hacia atrás.
    fun actualizarRegistrado(onActualizado: (contratistaId: Long, aviso: AvisoAlta?) -> Unit) {
        val registrado = yaRegistrado ?: return
        val enlace = enlaceGafete ?: return
        if (enviando || leyendoQr) return
        enviando = true
        error = null
        mensaje = null
        val texto = textoGafete
        val nombreEnOrigen = nombreEnOrigenGafete
        viewModelScope.launch {
            try {
                val guardado = withContext(dispatcherIO) {
                    fuenteGafete.actualizarConGafete(registrado.id, enlace, texto, nombreEnOrigen)
                }
                if (guardado.fichaCambio) CambiosNube.cambioLocal()
                onActualizado(registrado.id, AvisoAlta(guardado.aviso, guardado.avisoEsError))
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado() ?: "No se pudo actualizar la ficha."
            } finally {
                enviando = false
            }
        }
    }

    companion object {
        /// Cuánto se espera la respuesta del permiso antes de abrir la
        /// cámara OCR en lugar del lector QR.
        const val ESPERA_PERMISO_QR_MS = 1500L

        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { NuevoContratistaViewModel(nucleo) }
        }
    }
}

/// Máscara DD-MM-AAAA del vencimiento PRAIND: descarta lo que no sea dígito
/// y reinserta los guiones mientras se tipea (pedido del usuario
/// 2026-09-21: sin esto había que tipear los guiones con el teclado
/// completo en vez del numérico).
internal fun formatearFechaDDMMYYYY(texto: String): String {
    val digitos = texto.filter(Char::isDigit).take(8)
    return buildString {
        for (indice in digitos.indices) {
            append(digitos[indice])
            if (indice == 1 || indice == 3) append('-')
        }
    }
}

/// Texto libre del carnet contra los nombres reales de las empresas:
/// primero igualdad exacta sin distinguir mayúsculas, después "una
/// contiene a la otra" (tolera un "S.A." de más o de menos).
internal fun buscarEmpresaPorNombre(empresas: List<Empresa>, textoCarnet: String): Empresa? {
    val normalizado = textoCarnet.trim()
    if (normalizado.isEmpty()) return null
    return empresas.firstOrNull { it.nombre.equals(normalizado, ignoreCase = true) }
        ?: empresas.firstOrNull {
            it.nombre.contains(normalizado, ignoreCase = true) ||
                normalizado.contains(it.nombre, ignoreCase = true)
        }
}

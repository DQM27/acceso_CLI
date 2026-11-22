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
import uniffi.control_acceso_mobile.ContratistaYaRegistrado
import uniffi.control_acceso_mobile.DatosAltaDesdeGafete
import uniffi.control_acceso_mobile.DatosContratista
import uniffi.control_acceso_mobile.Empresa
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.PreviaGafeteParaAlta
import uniffi.control_acceso_mobile.RevisionGafetePraind
import uniffi.control_acceso_mobile.TipoIngreso
import uniffi.control_acceso_mobile.nombreMientrasSeEscribe

/// Lo que el alta por QR necesita del núcleo y de la nube (en las pruebas,
/// un doble para lo que sale a internet).
interface FuenteGafeteAlta {
    /// Si la nube deja cargar gafetes PRAIND desde este teléfono; `false`
    /// también sin conexión.
    fun puedeCargar(): Boolean
    fun previsualizar(enlace: String): PreviaGafeteParaAlta
    fun datos(textoOcr: String, nombreEnOrigen: String?): DatosAltaDesdeGafete
    fun cargar(contratistaId: Long, enlace: String)
    fun revisar(contratistaId: Long, textoOcr: String, nombreEnOrigen: String?): RevisionGafetePraind
    fun corregirFicha(contratistaId: Long, vencimiento: String?, nombre: String?, empresaId: Long?)
}

private class FuenteGafeteAltaNucleo(private val nucleo: Nucleo) : FuenteGafeteAlta {
    override fun puedeCargar() = nucleo.permisosCarnePraind().puedeCargar
    override fun previsualizar(enlace: String) = nucleo.previsualizarGafetePraindParaAlta(enlace)
    override fun datos(textoOcr: String, nombreEnOrigen: String?) =
        nucleo.datosAltaDesdeGafetePraind(textoOcr, nombreEnOrigen)
    override fun cargar(contratistaId: Long, enlace: String) {
        nucleo.cargarGafetePraindDeAlta(contratistaId, enlace)
    }
    override fun revisar(contratistaId: Long, textoOcr: String, nombreEnOrigen: String?) =
        nucleo.revisarGafetePraind(contratistaId, textoOcr, nombreEnOrigen)
    override fun corregirFicha(contratistaId: Long, vencimiento: String?, nombre: String?, empresaId: Long?) =
        nucleo.corregirFichaDesdeGafete(contratistaId, vencimiento, nombre, empresaId)
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
    val cargaraGafete: Boolean get() = enlaceGafete != null && gafeteEsDeEstaCedula()

    private var registradoLeido by mutableStateOf<ContratistaYaRegistrado?>(null)

    /// Lo leído del gafete (texto del OCR y nombre de la página del QR),
    /// para revisarlo contra la ficha que ya existe al actualizarla.
    private var textoGafete: String = ""
    private var nombreEnOrigenGafete: String? = null

    /// La cédula del gafete del QR ya tiene ficha: no es un alta. La
    /// pantalla ofrece actualizar esa ficha con el gafete (sólo por QR).
    /// Si se cambia la cédula a mano deja de valer.
    val yaRegistrado: ContratistaYaRegistrado? get() = registradoLeido.takeIf { cedulaGafete == cedula }

    /// El enlace del QR leído, para revisar el gafete contra la ficha que ya
    /// existe.
    val enlaceGafeteLeido: String? get() = enlaceGafete

    private fun gafeteEsDeEstaCedula(): Boolean = cedulaGafete.let { it == null || it == cedula }

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
        datos.vencimiento?.let { fechaPraind = it.split('-').reversed().joinToString("-") }
        val catalogo = datos.empresaCatalogo?.let { propuesta -> empresas.firstOrNull { it.id == propuesta.id } }
        if (catalogo != null) {
            elegirEmpresa(catalogo)
        } else {
            empresaSugeridaTexto = datos.empresa
        }
        val vence = datos.vencimiento
        val registrado = datos.yaRegistrado
        when {
            datos.vencido && vence != null ->
                error = "El gafete PRAIND venció el ${fechaIsoComoTexto(vence)}: pida a la persona el gafete actualizado."
            registrado != null ->
                mensaje = "Esta cédula ya está registrada (${registrado.nombre}): actualice su ficha con este gafete."
            datos.cedula == null || vence == null ->
                mensaje = "Gafete PRAIND leído: complete lo que falta. Al guardar se carga el gafete."
            else -> mensaje = "Gafete PRAIND leído: revise los datos. Al guardar se carga el gafete."
        }
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
        viewModelScope.launch {
            try {
                val id = withContext(dispatcherIO) {
                    medirNucleo("crearContratista") { nucleo.crearContratista(
                        DatosContratista(
                            cedula = cedula,
                            nombre = nombre,
                            empresaId = empresa.id,
                            tipoIngreso = tipoIngreso,
                            fechaVencimientoPraind = fechaPraind.ifBlank { null }?.let(::textoDDMMYYYYaIso),
                            esPersonalRuta = personalRuta,
                        ),
                    ) }
                }
                CambiosNube.cambioLocal()
                onGuardado(id, cargarGafeteDe(id))
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                enviando = false
            }
        }
    }

    /// La cédula del gafete del QR ya tiene ficha: se actualiza directo, sin
    /// otra ventana (pedido del usuario 2026-11-22). Se guarda el gafete y
    /// la ficha toma su vencimiento si es más nuevo (lo decide la revisión
    /// del núcleo) y el nombre y la empresa que muestra el formulario si
    /// difieren de la ficha. Después [onActualizado] abre su ingreso.
    ///
    /// Primero el gafete: si no se puede guardar (sin conexión), la ficha
    /// no cambia y se puede reintentar. La fecha sólo avanza aunque otro
    /// equipo haya cambiado la ficha mientras tanto (lo cuida el núcleo).
    fun actualizarRegistrado(onActualizado: (contratistaId: Long, aviso: AvisoAlta?) -> Unit) {
        val registrado = yaRegistrado ?: return
        val enlace = enlaceGafete ?: return
        if (enviando || leyendoQr) return
        enviando = true
        error = null
        mensaje = null
        val id = registrado.id
        val nombreFormulario = nombre
        val empresaFormulario = empresaSeleccionada
        viewModelScope.launch {
            try {
                val revision = withContext(dispatcherIO) {
                    fuenteGafete.revisar(id, textoGafete, nombreEnOrigenGafete)
                }
                if (revision.bloqueaGuardar) {
                    error = "La cédula del gafete no coincide con la ficha: no se actualizó."
                    return@launch
                }
                withContext(dispatcherIO) { fuenteGafete.cargar(id, enlace) }
                val fecha = revision.fechaParaFicha
                val nombreNuevo = nombreFormulario.takeIf { revision.nombreParaFicha != null && it.isNotBlank() }
                val empresaNueva = empresaFormulario?.id?.takeIf { revision.empresaParaFicha != null }
                if (fecha != null || nombreNuevo != null || empresaNueva != null) {
                    withContext(dispatcherIO) { fuenteGafete.corregirFicha(id, fecha, nombreNuevo, empresaNueva) }
                    CambiosNube.cambioLocal()
                }
                val quien = nombreNuevo ?: registrado.nombre
                val texto = buildString {
                    append("Gafete PRAIND de $quien guardado")
                    when {
                        fecha != null -> append("; la ficha vence el ${fechaIsoComoTexto(fecha)}")
                        nombreNuevo != null || empresaNueva != null -> append("; ficha corregida")
                        else -> append("; la ficha ya estaba al día")
                    }
                    append('.')
                }
                onActualizado(id, AvisoAlta(texto, esError = false))
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado() ?: "No se pudo actualizar la ficha."
            } finally {
                enviando = false
            }
        }
    }

    /// El gafete del QR, ya con el contratista guardado. Si falla, el alta
    /// queda igual: se avisa y se puede cargar desde su ingreso.
    private suspend fun cargarGafeteDe(contratistaId: Long): AvisoAlta? {
        val enlace = enlaceGafete ?: return null
        if (!gafeteEsDeEstaCedula()) {
            return AvisoAlta("La cédula no es la del gafete PRAIND escaneado: el gafete no se cargó.", esError = true)
        }
        return withContext(dispatcherIO) {
            try {
                fuenteGafete.cargar(contratistaId, enlace)
                AvisoAlta("Gafete PRAIND de $nombre guardado.", esError = false)
            } catch (excepcion: Exception) {
                val motivo = excepcion.mensajeDeErrorEsperado()?.trimEnd('.', ' ')?.let { ": $it" }.orEmpty()
                AvisoAlta(
                    "Contratista guardado, pero el gafete PRAIND no se cargó$motivo. Cárguelo desde su ingreso.",
                    esError = true,
                )
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

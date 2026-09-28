package com.brisas.controlacceso

import uniffi.control_acceso_mobile.ConfiguracionEstabilizador
import uniffi.control_acceso_mobile.ConsensoVotacion
import uniffi.control_acceso_mobile.DatosPdf417Cedula
import uniffi.control_acceso_mobile.EstabilizadorDocumento
import uniffi.control_acceso_mobile.consensoAlcanza
import uniffi.control_acceso_mobile.ResultadoEstabilizacion as ResultadoNucleo

/// Estado central que alimenta los esquineros del visor y el mensaje de la
/// cámara: un solo lugar decide "qué está pasando".
typealias EstadoEscaneo = uniffi.control_acceso_mobile.EstadoEscaneo

typealias ModoEscaneoDocumento = uniffi.control_acceso_mobile.ModoEscaneoDocumento

data class ResultadoEstabilizacion(
    val estado: EstadoEscaneo,
    val documento: DocumentoDetectado? = null,
    val mensaje: String,
    // Sólo con estado CONFIRMADO y fecha de vencimiento disponible; `false`
    // no significa "vigente confirmado".
    val vencido: Boolean = false,
    // 0..1: respaldo del candidato actual sobre el necesario (barra de
    // avance del recuadro: "ya casi, no lo mueva").
    val progreso: Float = 0f,
    // Orientación del recuadro que pide lo leído (`null` = no opina). La
    // pantalla se la pasa a `ControladorEncuadre`.
    val orientacionSugerida: OrientacionEncuadre? = null,
    // Hay un MRZ en cuadro: el lector de códigos no tiene nada que buscar.
    val hayMrz: Boolean = false,
)

/// Decide, frame a frame, si ya hay lectura suficiente para aceptarla. La
/// lógica --votación por carácter ponderada por nitidez, MRZ validado por
/// sus dígitos verificadores, espera del reverso de la cédula, mensajes--
/// vive en el núcleo Rust (`EstabilizadorDocumento`,
/// `lectura_documentos/estabilizador.rs`). Esta clase fija los valores por
/// defecto, pone la fecha de hoy y avisa a Sentry de las correcciones de
/// confusables del MRZ.
///
/// Una instancia por sesión de escaneo; segura entre hilos (el núcleo se
/// sincroniza solo).
class EstabilizadorLectura(
    modo: ModoEscaneoDocumento = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
    // En modo gafete el número no tiene dígito verificador y confirmar
    // registra una salida sin revisión humana: se exige más respaldo.
    framesRequeridos: Int = if (modo == ModoEscaneoDocumento.GAFETE_CONTRATISTA) 3 else 2,
    // Frames recientes (con o sin lectura) que se consideran.
    ventana: Int = framesRequeridos + 2,
    private val obtenerFechaHoy: () -> FechaDocumento = ::fechaDeHoy,
    // Frames que se espera el reverso con el número de la cédula leído del
    // frente pero sin nombre (confirmar cierra la cámara).
    framesEsperaReverso: Int = 20,
    // Frames SEGUIDOS no reconocibles antes de marcar inválido.
    framesParaInvalido: Int = 3,
    // Frames con el tipo reconocido pero sin datos antes de sugerir inclinar.
    framesParaSugerirReflejo: Int = 6,
) {
    init {
        require(framesRequeridos > 0) { "framesRequeridos debe ser mayor que cero" }
        require(ventana >= framesRequeridos) { "ventana debe cubrir los frames requeridos" }
    }

    private val nucleo = EstabilizadorDocumento(
        ConfiguracionEstabilizador(
            modo = modo,
            framesRequeridos = framesRequeridos.toUInt(),
            ventana = ventana.toUInt(),
            framesEsperaReverso = framesEsperaReverso.coerceAtLeast(0).toUInt(),
            framesParaInvalido = framesParaInvalido.coerceAtLeast(0).toUInt(),
            framesParaSugerirReflejo = framesParaSugerirReflejo.coerceAtLeast(0).toUInt(),
        ),
    )

    /// Un frame con un solo texto (ver [procesarTextos]).
    fun procesarFrame(texto: String, peso: Float = 1f, calidad: CalidadFrame? = null): ResultadoEstabilizacion =
        procesarTextos(listOf(texto), peso, calidad)

    /// `textos`: las versiones del texto del frame, en orden de preferencia
    /// (ver `LecturaFrame.textos`). `peso`: calidad relativa del frame en
    /// (0, 1]. `calidad`: medición del recorte, para avisar de un reflejo.
    fun procesarTextos(textos: List<String>, peso: Float = 1f, calidad: CalidadFrame? = null): ResultadoEstabilizacion =
        nucleo.procesarFrame(textos, peso, calidad?.fraccionReflejo, obtenerFechaHoy().aFechaOcr()).aResultado()

    /// Cédula anterior leída de su PDF417 (sólo cédula y nombre, ya
    /// validados por Rust). Confirma en el acto.
    fun procesarPdf417(datos: DatosPdf417Cedula): ResultadoEstabilizacion =
        nucleo.procesarPdf417(datos, obtenerFechaHoy().aFechaOcr()).aResultado()

    fun reiniciar() = nucleo.reiniciar()
}

private fun ResultadoNucleo.aResultado(): ResultadoEstabilizacion {
    // Sólo llegan con un MRZ CONFIRMADO: nunca por frames intermedios.
    if (correccionesMrz.isNotEmpty()) registrarCorreccionesMrzEnSentry(correccionesMrz)
    return ResultadoEstabilizacion(
        estado = estado,
        documento = documento?.aDocumentoDetectado(),
        mensaje = mensaje,
        vencido = vencido,
        progreso = progreso,
        orientacionSugerida = orientacionSugerida,
        hayMrz = hayMrz,
    )
}

/// ¿El consenso alcanza para confirmar con `requeridos` frames? (mismo
/// criterio que el estabilizador de documentos).
internal fun ConsensoVotacion.alcanza(requeridos: Int): Boolean = consensoAlcanza(this, requeridos.toUInt())

/// ¿`nueva` es la lectura de OTRO documento y no el mismo con algún
/// carácter mal leído?
internal fun esOtraLectura(consenso: String, nueva: String): Boolean =
    uniffi.control_acceso_mobile.esOtraLectura(consenso, nueva)

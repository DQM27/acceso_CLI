package com.brisas.controlacceso

import uniffi.control_acceso_mobile.DatosPdf417Cedula
import uniffi.control_acceso_mobile.LecturaPdf417
import uniffi.control_acceso_mobile.LecturaVehiculo
import uniffi.control_acceso_mobile.MotivoPdf417
import uniffi.control_acceso_mobile.largoPrefijoPdf417Cedula
import uniffi.control_acceso_mobile.leerPdf417CedulaConMotivo

/// Lo que produjo un frame, entregado en el hilo del analizador de cámara
/// (ver `analizarFrameOcr`). Sin imagen: sólo texto y datos derivados.
///
/// El texto y el PDF417 se leen EN PARALELO sobre el mismo recorte (dos
/// modelos de ML Kit a la vez), así que un frame puede traer los dos.
class LecturaFrame(
    /// Texto de ML Kit tal cual ("" si el lector de texto falló en este
    /// frame). Sólo para diagnóstico: los lectores usan [textos].
    val texto: String,
    /// Las versiones del texto que prueban los lectores, en orden: los
    /// renglones visuales armados con la geometría de ML Kit y, si difiere,
    /// el texto original (ver `textosDeFrame` en el núcleo). Vacía si no
    /// hubo texto.
    val textos: List<String>,
    /// Cédula anterior leída de su PDF417, si en este frame también se
    /// buscó código y se encontró uno válido.
    val pdf417: DatosPdf417Cedula?,
    /// Medición del recorte (null si no se pudo recortar y se leyó el
    /// frame entero).
    val calidad: CalidadFrame?,
    /// Peso de este frame en la votación (ver `FiltroCalidad`).
    val peso: Float,
    /// Región que se le entregó a ML Kit (null = frame entero).
    val regionLeida: RegionRecorte?,
    /// Cajas de las líneas con forma de MRZ, en fracciones de la imagen
    /// analizada (ver `SeguidorBandaMrz`).
    val lineasMrz: List<FraccionesRect>,
)

/// Extrae cédula y nombre del PDF417 de la cédula anterior entregando a
/// Rust SÓLO los primeros bytes (hasta el final del nombre) y poniendo en
/// cero, al terminar, tanto el prefijo como los bytes crudos recibidos --
/// que incluyen las huellas dactilares. Nada de esto se guarda ni se loguea.
fun extraerPdf417Cedula(crudo: ByteArray): DatosPdf417Cedula? = leerPdf417ConMotivo(crudo).datos

/// Igual que [extraerPdf417Cedula], con el motivo del resultado para la
/// telemetría de diagnóstico (qué validación falló, nunca qué bytes había).
fun leerPdf417ConMotivo(crudo: ByteArray): LecturaPdf417 {
    val largo = largoPrefijoPdf417Cedula().toInt()
    if (crudo.size < largo) {
        crudo.fill(0)
        return LecturaPdf417(datos = null, motivo = MotivoPdf417.PREFIJO_CORTO)
    }
    val prefijo = crudo.copyOfRange(0, largo)
    crudo.fill(0)
    return try {
        leerPdf417CedulaConMotivo(prefijo)
    } finally {
        prefijo.fill(0)
    }
}

/// Lo que pasó con el lector de códigos en un frame que lo usó, sólo con
/// números y motivos (ver [MetricasOcr.registrarBusquedaCodigo]).
class DiagnosticoCodigos(
    /// Códigos PDF417 que ML Kit decodificó en el frame (0 si no vio
    /// ninguno: con poca resolución ni siquiera lo detecta).
    val detectados: Int,
    /// Códigos que llegaron sin bytes crudos (`rawBytes == null`).
    val sinBytes: Int,
    /// Resultado del núcleo para cada código con bytes.
    val motivos: List<MotivoPdf417>,
    /// Largo en bytes de cada código recibido (no su contenido).
    val largosBytes: List<Int>,
    /// Ancho en píxeles de cada código dentro de la imagen analizada.
    val anchosCodigoPx: List<Int>,
    /// Ancho en píxeles de la imagen que analizó ML Kit (el recorte, ya
    /// rotado): cuánta resolución tuvo el código para leerse.
    val anchoImagenPx: Int,
)

/// ¿Esta línea de texto de ML Kit tiene forma de línea de MRZ? Largo de
/// TD1 (30) o TD3 (44) con algo de tolerancia, alfabeto MRZ y al menos un
/// relleno `<<`. Sólo decide DÓNDE está el MRZ para acotar el recorte; su
/// lectura la valida Rust con los dígitos verificadores.
fun esLineaMrzProbable(texto: String): Boolean {
    val normalizada = texto.uppercase().replace(" ", "").replace("«", "<<")
    return normalizada.length in 26..48 &&
        "<<" in normalizada &&
        normalizada.all { it in 'A'..'Z' || it in '0'..'9' || it == '<' }
}

/// Acota el recorte a la banda del MRZ cuando ya se sabe dónde está:
/// ML Kit analiza ~un tercio de los píxeles del recuadro (más rápido) y
/// sin el resto del reverso compitiendo (más exacto).
///
/// Se aprende de las cajas de las líneas MRZ que ML Kit encontró en el
/// frame anterior y se agrega margen para que un leve movimiento no deje
/// el MRZ afuera. Si con la banda se pierden las líneas en
/// [fallosParaSoltar] frames, se vuelve al recuadro completo.
///
/// Sincronizado: lo consulta y alimenta el hilo del analizador; la
/// pantalla sólo lo crea.
class SeguidorBandaMrz(
    private val margenVertical: Float = 0.5f,
    private val margenHorizontal: Float = 0.03f,
    private val fallosParaSoltar: Int = 2,
    private val minimoLineas: Int = 2,
) {
    private var base: RegionRecorte? = null
    private var banda: FraccionesRect? = null
    private var fallos = 0

    /// Región con la que leer el próximo frame si la base es `base`.
    @Synchronized
    fun region(base: RegionRecorte): RegionRecorte {
        val bandaActual = banda
        return if (bandaActual != null && base == this.base) SubregionRecorte(base, bandaActual) else base
    }

    /// Registra las líneas MRZ encontradas al leer con `leidaCon`.
    @Synchronized
    fun registrar(leidaCon: RegionRecorte?, lineasMrz: List<FraccionesRect>) {
        if (leidaCon == null) return
        val (baseLeida, subregion) = when (leidaCon) {
            is SubregionRecorte -> leidaCon.base to leidaCon.fracciones
            else -> leidaCon to null
        }
        if (lineasMrz.size < minimoLineas) {
            if (subregion != null && ++fallos >= fallosParaSoltar) soltar()
            return
        }
        val union = FraccionesRect(
            izquierda = lineasMrz.minOf { it.izquierda },
            arriba = lineasMrz.minOf { it.arriba },
            derecha = lineasMrz.maxOf { it.derecha },
            abajo = lineasMrz.maxOf { it.abajo },
        )
        val enBase = subregion?.let { union.dentroDe(it) } ?: union
        base = baseLeida
        banda = enBase.conMargen(margenHorizontal, margenVertical)
        fallos = 0
    }

    @Synchronized
    fun soltar() {
        base = null
        banda = null
        fallos = 0
    }
}

/// `this` está en fracciones de `contenedor`; se lleva a fracciones de lo
/// que contiene a `contenedor`.
private fun FraccionesRect.dentroDe(contenedor: FraccionesRect) = FraccionesRect(
    izquierda = contenedor.izquierda + izquierda * contenedor.ancho,
    arriba = contenedor.arriba + arriba * contenedor.alto,
    derecha = contenedor.izquierda + derecha * contenedor.ancho,
    abajo = contenedor.arriba + abajo * contenedor.alto,
)

private fun FraccionesRect.conMargen(horizontal: Float, vertical: Float) = FraccionesRect(
    izquierda = (izquierda - horizontal).coerceAtLeast(0f),
    arriba = (arriba - alto * vertical).coerceAtLeast(0f),
    derecha = (derecha + horizontal).coerceAtMost(1f),
    abajo = (abajo + alto * vertical).coerceAtMost(1f),
)

/// Decide, frame a frame, si ADEMÁS del texto se busca el PDF417 de la
/// cédula anterior. Los dos lectores corren en paralelo sobre el mismo
/// recorte, así que buscar el código no le quita frames al texto (antes se
/// alternaban y cada frame de código era uno menos para leer, p. ej., el
/// carnet PRAIND). Buscar el código sí cuesta procesador, por eso:
/// - con un MRZ reciente en cuadro, nunca (la cédula nueva no trae
///   PDF417);
/// - si el texto parece el reverso de la cédula anterior (la cara del
///   PDF417), en cada frame;
/// - si no, uno de cada [periodoSinPista], por si se muestra el reverso sin
///   que el texto alcance a reconocerse.
///
/// Sincronizado: lo usa el hilo del analizador.
class PlanificadorLectores(
    private val habilitado: Boolean,
    private val periodoSinPista: Int = 3,
    private val framesMemoria: Int = 8,
) {
    private var contador = 0
    private var framesConPista = 0
    private var framesConMrz = 0

    init {
        require(periodoSinPista > 0) { "periodoSinPista debe ser positivo" }
    }

    @Synchronized
    fun leerCodigo(): Boolean {
        if (!habilitado || framesConMrz > 0) return false
        if (framesConPista > 0) return true
        contador++
        return contador % periodoSinPista == 0
    }

    /// ¿El texto de los últimos frames dice que se ve el reverso de la
    /// cédula anterior (y no un MRZ)? Ahí vale la pena la foto de alta
    /// resolución (ver `FotografoPdf417`).
    @Synchronized
    fun hayPistaReverso(): Boolean = habilitado && framesConMrz == 0 && framesConPista > 0

    /// Resultado del texto de un frame.
    @Synchronized
    fun registrarTexto(pareceReversoConCodigo: Boolean, hayMrz: Boolean) {
        framesConPista = if (pareceReversoConCodigo) framesMemoria else (framesConPista - 1).coerceAtLeast(0)
        framesConMrz = if (hayMrz) framesMemoria else (framesConMrz - 1).coerceAtLeast(0)
    }
}

/// Tiempos y conteos por sesión de escaneo, sólo para diagnosticar en
/// builds de depuración (`adb logcat -s OcrMetricas`). Nunca incluye texto
/// leído ni datos de la persona: sólo números.
///
/// `reloj` en nanosegundos y `registrar` inyectables para probarlo en la
/// JVM sin `android.util.Log`.
class MetricasOcr(
    private val habilitadas: Boolean,
    private val reloj: () -> Long = System::nanoTime,
    private val registrar: (String) -> Unit = {},
    private val cadaCuantosFrames: Int = 30,
) {
    private val inicio = reloj()
    private var primerFrame = 0L
    private var ultimoFrame = 0L
    private var frames = 0
    private var framesConCodigo = 0
    private var descartadosPorCalidad = 0
    private var fallos = 0
    private var msHastaConfirmar: Long? = null
    private var framesConCodigoDetectado = 0
    private var codigosDetectados = 0
    private var codigosSinBytes = 0
    private var erroresLectorCodigo = 0
    private val motivosPdf417 = sortedMapOf<String, Int>()
    private val largosBytesPdf417 = ArrayDeque<Float>()
    private val anchosCodigoPx = ArrayDeque<Float>()
    private val anchosImagenConCodigoPx = ArrayDeque<Float>()

    // Condiciones de captura (todos los frames medidos, también los
    // descartados): para calibrar el filtro de calidad y el aviso de
    // reflejo con datos reales, de día y de noche.
    private val nitideces = ArrayDeque<Float>()
    private val luminancias = ArrayDeque<Float>()
    private val reflejos = ArrayDeque<Float>()
    private var framesMedidos = 0
    private var framesConLinterna = 0

    // Confianza de las palabras de ML Kit: para calibrar el umbral con que
    // los renglones visuales descartan palabras (`CONFIANZA_MINIMA`, 0,25).
    private var palabras = 0
    private var palabrasConfianzaBaja = 0
    private var framesSinConfianza = 0
    private val confianzas = ArrayDeque<Float>()
    private var framesConTextosDistintos = 0
    private var framesConTexto = 0

    // Placas y números de unidad (nunca el valor leído).
    private var framesSinVehiculo = 0
    private val formatosVehiculo = sortedMapOf<String, Int>()
    private var vehiculosConCorreccion = 0
    private var vehiculosClRestituida = 0
    private val vehiculoPorVersionTexto = sortedMapOf<String, Int>()
    private var reiniciosVotacion = 0

    // Documentos (pantalla de cédula): cómo avanzó y cómo terminó la sesión.
    private var msHastaPrimeraLectura: Long? = null
    private val estadosDocumento = sortedMapOf<String, Int>()
    private var framesConMrz = 0
    private var progresoMaximo = 0f
    private var confirmadoTipo: String? = null
    private var confirmadoFuente: String? = null
    private var confirmadoConNombre: Boolean? = null
    private var confirmadoVencido: Boolean? = null
    private val msRecorte = ArrayDeque<Float>()
    private val msReconocimiento = ArrayDeque<Float>()

    // Fotos de alta resolución para el PDF417 (ver `FotografoPdf417`).
    private val resultadosFotoPdf417 = sortedMapOf<String, Int>()
    private val msFotoPdf417 = ArrayDeque<Float>()
    private val anchosFotoPdf417 = ArrayDeque<Float>()
    private var fotosConCodigoDetectado = 0
    private var msHastaPdf417EnFoto: Long? = null

    /// Un frame que terminó de pasar por ML Kit. `nanosReconocimiento`
    /// cubre los dos lectores si corrieron en paralelo.
    @Synchronized
    fun registrarFrame(conCodigo: Boolean, nanosRecorte: Long, nanosReconocimiento: Long) {
        if (!habilitadas) return
        val ahora = reloj()
        if (frames == 0) primerFrame = ahora
        ultimoFrame = ahora
        frames++
        if (conCodigo) framesConCodigo++
        agregar(msRecorte, nanosRecorte / 1e6f)
        agregar(msReconocimiento, nanosReconocimiento / 1e6f)
        if (frames % cadaCuantosFrames == 0) registrar(resumen())
    }

    /// Resultado del lector de códigos en un frame que lo usó.
    @Synchronized
    fun registrarBusquedaCodigo(diagnostico: DiagnosticoCodigos) {
        if (!habilitadas) return
        if (diagnostico.detectados > 0) framesConCodigoDetectado++
        codigosDetectados += diagnostico.detectados
        codigosSinBytes += diagnostico.sinBytes
        diagnostico.motivos.forEach { motivosPdf417.merge(it.name, 1, Int::plus) }
        diagnostico.largosBytes.forEach { agregar(largosBytesPdf417, it.toFloat()) }
        diagnostico.anchosCodigoPx.forEach { agregar(anchosCodigoPx, it.toFloat()) }
        agregar(anchosImagenConCodigoPx, diagnostico.anchoImagenPx.toFloat())
    }

    /// Calidad medida de un frame (procesado o descartado) y si la linterna
    /// estaba encendida.
    @Synchronized
    fun registrarCalidad(calidad: CalidadFrame, linternaEncendida: Boolean) {
        if (!habilitadas) return
        framesMedidos++
        if (linternaEncendida) framesConLinterna++
        agregar(nitideces, calidad.nitidez, MUESTRAS_CONDICIONES)
        agregar(luminancias, calidad.luminancia, MUESTRAS_CONDICIONES)
        agregar(reflejos, calidad.fraccionReflejo, MUESTRAS_CONDICIONES)
    }

    /// Las palabras que entregó ML Kit en un frame y cuántas versiones de
    /// texto se armaron (2 = los renglones visuales difieren del original).
    @Synchronized
    fun registrarTexto(confianzasPalabras: List<Float>, umbralBajo: Float, versionesTexto: Int) {
        if (!habilitadas) return
        framesConTexto++
        if (versionesTexto > 1) framesConTextosDistintos++
        palabras += confianzasPalabras.size
        // Todas en 0 = el modelo no informa confianza en este frame.
        if (confianzasPalabras.isNotEmpty() && confianzasPalabras.all { it <= 0f }) {
            framesSinConfianza++
            return
        }
        palabrasConfianzaBaja += confianzasPalabras.count { it < umbralBajo }
        confianzasPalabras.forEach { agregar(confianzas, it, MUESTRAS_CONDICIONES) }
    }

    /// Resultado del lector de placas en un frame: `null` si no encontró
    /// nada; `indiceTexto` = versión del texto que lo produjo (0 = renglones
    /// visuales, 1 = texto original de ML Kit).
    @Synchronized
    fun registrarLecturaVehiculo(lectura: LecturaVehiculo?, indiceTexto: Int?) {
        if (!habilitadas) return
        if (lectura == null) {
            framesSinVehiculo++
            return
        }
        formatosVehiculo.merge(lectura.formato.name, 1, Int::plus)
        if (lectura.correcciones > 0u) vehiculosConCorreccion++
        if (lectura.clRestituida) vehiculosClRestituida++
        val version = if (indiceTexto == 0) "visual" else "original"
        vehiculoPorVersionTexto.merge(version, 1, Int::plus)
    }

    /// Primer frame con algo reconocido (una placa, un documento con datos o
    /// un MRZ): separa "tardó en encontrar el documento" de "tardó en
    /// confirmarlo".
    @Synchronized
    fun registrarPrimeraLectura() {
        if (habilitadas && msHastaPrimeraLectura == null) msHastaPrimeraLectura = (reloj() - inicio) / 1_000_000
    }

    /// Resultado del estabilizador de documentos en un frame. Del documento
    /// confirmado sólo se guarda tipo, origen de los datos y si trajo nombre
    /// o está vencido; nunca número ni nombre.
    @Synchronized
    fun registrarResultadoDocumento(resultado: ResultadoEstabilizacion) {
        if (!habilitadas) return
        estadosDocumento.merge(resultado.estado.name, 1, Int::plus)
        if (resultado.hayMrz) framesConMrz++
        progresoMaximo = maxOf(progresoMaximo, resultado.progreso)
        val confirmado = resultado.estado == EstadoEscaneo.CONFIRMADO
        if (resultado.hayMrz || resultado.progreso > 0f || confirmado) registrarPrimeraLectura()
        val documento = resultado.documento
        if (confirmado && documento != null) {
            confirmadoTipo = documento.tipo.name
            confirmadoFuente = documento.fuenteDatos.name
            confirmadoConNombre = documento.nombre != null
            confirmadoVencido = resultado.vencido
        }
    }

    /// La votación se reinició porque un frame leyó OTRO vehículo (o un
    /// valor demasiado distinto).
    @Synchronized
    fun registrarReinicioVotacion() {
        if (habilitadas) reiniciosVotacion++
    }

    /// Una foto de alta resolución para el PDF417: cómo terminó, cuánto
    /// tardó (captura + decodificación + lector) y con qué ancho.
    @Synchronized
    fun registrarFotoPdf417(resultado: ResultadoFotoPdf417, ms: Long, anchoPx: Int?, detectados: Int) {
        if (!habilitadas) return
        resultadosFotoPdf417.merge(resultado.name, 1, Int::plus)
        agregar(msFotoPdf417, ms.toFloat())
        anchoPx?.let { agregar(anchosFotoPdf417, it.toFloat()) }
        if (detectados > 0) fotosConCodigoDetectado++
        if (resultado == ResultadoFotoPdf417.LEIDA && msHastaPdf417EnFoto == null) {
            msHastaPdf417EnFoto = (reloj() - inicio) / 1_000_000
        }
    }

    /// El lector de códigos terminó con error (no "sin códigos").
    @Synchronized
    fun registrarErrorLectorCodigo() {
        if (habilitadas) erroresLectorCodigo++
    }

    @Synchronized
    fun registrarDescarte() {
        if (habilitadas) descartadosPorCalidad++
    }

    @Synchronized
    fun registrarFallo() {
        if (habilitadas) fallos++
    }

    @Synchronized
    fun registrarConfirmacion() {
        if (!habilitadas) return
        val ms = (reloj() - inicio) / 1_000_000
        msHastaConfirmar = ms
        registrar("confirmado en $ms ms; ${resumen()}")
    }

    /// Los mismos números que [resumen], estructurados para la telemetría
    /// de diagnóstico (`Telemetria`, tipo `ocr_sesion`).
    @Synchronized
    fun datos(): Map<String, Any?> = mapOf(
        "ms_sesion" to (reloj() - inicio) / 1_000_000,
        "ms_hasta_confirmar" to msHastaConfirmar,
        "frames" to frames,
        "frames_con_codigo" to framesConCodigo,
        "fps" to framesPorSegundo(),
        "descartados_calidad" to descartadosPorCalidad,
        "fallos" to fallos,
        "recorte_mediana_ms" to mediana(msRecorte),
        "reconocimiento_mediana_ms" to mediana(msReconocimiento),
        // PDF417: dónde se pierde la lectura. Sin códigos detectados con
        // poco ancho de imagen = falta resolución; detectados sin bytes =
        // ML Kit no entrega el binario; con motivos de rechazo = formato.
        "pdf417_frames_con_codigo_detectado" to framesConCodigoDetectado,
        "pdf417_codigos_detectados" to codigosDetectados,
        "pdf417_sin_bytes" to codigosSinBytes,
        "pdf417_errores_lector" to erroresLectorCodigo,
        "pdf417_motivos" to motivosPdf417.toMap(),
        "pdf417_bytes_min" to largosBytesPdf417.minOrNull(),
        "pdf417_bytes_max" to largosBytesPdf417.maxOrNull(),
        "pdf417_ancho_codigo_px_mediana" to mediana(anchosCodigoPx),
        "imagen_ancho_px_mediana_con_codigo" to mediana(anchosImagenConCodigoPx),
        "pdf417_fotos" to resultadosFotoPdf417.toMap(),
        "pdf417_fotos_con_codigo_detectado" to fotosConCodigoDetectado,
        "pdf417_foto_ms_mediana" to mediana(msFotoPdf417),
        "pdf417_foto_ancho_px_mediana" to mediana(anchosFotoPdf417),
        "ms_hasta_pdf417_en_foto" to msHastaPdf417EnFoto,
        // Condiciones de captura: con qué nitidez, luz y reflejo se trabajó.
        "frames_medidos" to framesMedidos,
        "frames_con_linterna" to framesConLinterna,
        "nitidez_p10" to cuantil(nitideces, 0.1),
        "nitidez_p50" to cuantil(nitideces, 0.5),
        "nitidez_p90" to cuantil(nitideces, 0.9),
        "luminancia_p10" to cuantil(luminancias, 0.1),
        "luminancia_p50" to cuantil(luminancias, 0.5),
        "reflejo_p50" to cuantil(reflejos, 0.5),
        "reflejo_p90" to cuantil(reflejos, 0.9),
        // Texto: confianza de ML Kit y cuánto aportan los renglones visuales.
        "frames_con_texto" to framesConTexto,
        "frames_textos_distintos" to framesConTextosDistintos,
        "palabras" to palabras,
        "palabras_confianza_baja" to palabrasConfianzaBaja,
        "frames_sin_confianza" to framesSinConfianza,
        "confianza_p10" to cuantil(confianzas, 0.1),
        "confianza_p50" to cuantil(confianzas, 0.5),
        // Placas (vacío en las demás pantallas).
        "vehiculo_frames_sin_lectura" to framesSinVehiculo,
        "vehiculo_formatos" to formatosVehiculo.toMap(),
        "vehiculo_con_correccion" to vehiculosConCorreccion,
        "vehiculo_cl_restituida" to vehiculosClRestituida,
        "vehiculo_por_version_texto" to vehiculoPorVersionTexto.toMap(),
        "reinicios_votacion" to reiniciosVotacion,
        // Documentos.
        "ms_hasta_primera_lectura" to msHastaPrimeraLectura,
        "documento_estados" to estadosDocumento.toMap(),
        "documento_frames_con_mrz" to framesConMrz,
        "documento_progreso_maximo" to progresoMaximo,
        "documento_confirmado_tipo" to confirmadoTipo,
        "documento_confirmado_fuente" to confirmadoFuente,
        "documento_confirmado_con_nombre" to confirmadoConNombre,
        "documento_confirmado_vencido" to confirmadoVencido,
    )

    @Synchronized
    fun resumen(): String =
        "frames=$frames con_codigo=$framesConCodigo fps=${formato(framesPorSegundo())} " +
            "descartados=$descartadosPorCalidad fallos=$fallos " +
            "recorte_mediana_ms=${formato(mediana(msRecorte))} reconocimiento_mediana_ms=${formato(mediana(msReconocimiento))}"

    /// Frames que terminaron por segundo: lo que de verdad procesa la
    /// tubería (con 2 frames en paralelo puede superar 1/latencia).
    private fun framesPorSegundo(): Float? {
        if (frames < 2 || ultimoFrame <= primerFrame) return null
        return (frames - 1) / ((ultimoFrame - primerFrame) / 1e9f)
    }

    private fun agregar(cola: ArrayDeque<Float>, valor: Float, maximo: Int = MUESTRAS) {
        cola.addLast(valor)
        while (cola.size > maximo) cola.removeFirst()
    }

    private fun cuantil(valores: Collection<Float>, p: Double): Float? =
        percentil(valores.map(Float::toDouble).sorted(), p)?.toFloat()

    private fun mediana(valores: Collection<Float>): Float? {
        if (valores.isEmpty()) return null
        val ordenados = valores.sorted()
        val medio = ordenados.size / 2
        return if (ordenados.size % 2 == 1) ordenados[medio] else (ordenados[medio - 1] + ordenados[medio]) / 2
    }

    private fun formato(valor: Float?) = valor?.let { String.format(java.util.Locale.ROOT, "%.1f", it) } ?: "-"

    private companion object {
        const val MUESTRAS = 60
        // Condiciones y confianzas: más muestras (son baratas y se resumen
        // en cuantiles al cerrar la cámara).
        const val MUESTRAS_CONDICIONES = 2_000
    }
}

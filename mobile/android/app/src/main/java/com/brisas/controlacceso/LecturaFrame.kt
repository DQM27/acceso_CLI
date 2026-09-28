package com.brisas.controlacceso

import uniffi.control_acceso_mobile.DatosPdf417Cedula
import uniffi.control_acceso_mobile.largoPrefijoPdf417Cedula
import uniffi.control_acceso_mobile.leerPdf417Cedula

/// Lo que produjo un frame, entregado en el hilo del analizador de cámara
/// (ver `analizarFrameOcr`). Sin imagen: sólo texto y datos derivados.
///
/// El texto y el PDF417 se leen EN PARALELO sobre el mismo recorte (dos
/// modelos de ML Kit a la vez), así que un frame puede traer los dos.
class LecturaFrame(
    /// Texto de ML Kit ("" si el lector de texto falló en este frame).
    val texto: String,
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
fun extraerPdf417Cedula(crudo: ByteArray): DatosPdf417Cedula? {
    val largo = largoPrefijoPdf417Cedula().toInt()
    if (crudo.size < largo) {
        crudo.fill(0)
        return null
    }
    val prefijo = crudo.copyOfRange(0, largo)
    crudo.fill(0)
    return try {
        leerPdf417Cedula(prefijo)
    } finally {
        prefijo.fill(0)
    }
}

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
    private val msRecorte = ArrayDeque<Float>()
    private val msReconocimiento = ArrayDeque<Float>()

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

    private fun agregar(cola: ArrayDeque<Float>, valor: Float) {
        cola.addLast(valor)
        while (cola.size > MUESTRAS) cola.removeFirst()
    }

    private fun mediana(valores: Collection<Float>): Float? {
        if (valores.isEmpty()) return null
        val ordenados = valores.sorted()
        val medio = ordenados.size / 2
        return if (ordenados.size % 2 == 1) ordenados[medio] else (ordenados[medio - 1] + ordenados[medio]) / 2
    }

    private fun formato(valor: Float?) = valor?.let { String.format(java.util.Locale.ROOT, "%.1f", it) } ?: "-"

    private companion object {
        const val MUESTRAS = 60
    }
}

package com.brisas.controlacceso

/// Calidad de imagen de un frame YA recortado al recuadro guía, medida
/// sobre la luminancia (plano Y del NV21) antes de gastar en ML Kit.
///
/// - `nitidez`: el mínimo entre los cuantiles altos de los gradientes
///   horizontal y vertical -- el estimador de foco que usan los trabajos
///   de MIDV-500 para ponderar frames (ver
///   `docs/auditorias/investigacion-lectura-documentos-2026-09-28.md`).
///   Allí se usa el cuantil 0,95 sobre el recorte de UN campo de texto,
///   donde los bordes dominan; acá se mide la tarjeta entera y el texto
///   ocupa pocos píxeles, así que el 0,95 caería en el fondo y mediría
///   ruido. Se usa el 0,99: sigue a los bordes más fuertes, justo los que
///   el desenfoque y el movimiento suavizan.
///   Es relativa: sólo sirve para comparar frames de la misma sesión, no
///   como umbral absoluto entre teléfonos o iluminaciones.
/// - `fraccionReflejo`: proporción de píxeles saturados (Y >= 250), lo que
///   deja un reflejo sobre el plástico de la tarjeta.
/// - `luminancia`: brillo medio del recorte (0-255). Sólo diagnóstico: dice
///   cuánta luz había (de noche en la portería, con o sin linterna).
data class CalidadFrame(val nitidez: Float, val fraccionReflejo: Float, val luminancia: Float = 0f)

private const val UMBRAL_SATURACION = 250
private const val CUANTIL_NITIDEZ = 0.99

/// Mide [CalidadFrame] recorriendo el plano Y de a `paso` píxeles en cada
/// eje (2 = una cuarta parte de los píxeles: sobra para un cuantil y cuesta
/// poco). Puro, sin Android: `y` es el plano de luminancia de `ancho` x
/// `alto` píxeles contiguos, como lo deja [recortarYuvANv21] al principio
/// del NV21.
fun medirCalidad(y: ByteArray, ancho: Int, alto: Int, paso: Int = 2): CalidadFrame {
    require(ancho > 0 && alto > 0 && y.size >= ancho * alto) { "Plano Y inválido: ${ancho}x$alto en ${y.size} bytes" }
    require(paso > 0) { "paso debe ser positivo" }
    val histogramaH = IntArray(256)
    val histogramaV = IntArray(256)
    var saturados = 0
    var sumaLuminancia = 0L
    var muestras = 0
    var fila = 0
    while (fila < alto - paso) {
        val base = fila * ancho
        var columna = 0
        while (columna < ancho - paso) {
            val actual = y[base + columna].toInt() and 0xFF
            val derecha = y[base + columna + paso].toInt() and 0xFF
            val abajo = y[base + paso * ancho + columna].toInt() and 0xFF
            histogramaH[kotlin.math.abs(derecha - actual)]++
            histogramaV[kotlin.math.abs(abajo - actual)]++
            if (actual >= UMBRAL_SATURACION) saturados++
            sumaLuminancia += actual
            muestras++
            columna += paso
        }
        fila += paso
    }
    if (muestras == 0) return CalidadFrame(nitidez = 0f, fraccionReflejo = 0f)
    val nitidez = minOf(cuantilAlto(histogramaH, muestras), cuantilAlto(histogramaV, muestras))
    return CalidadFrame(
        nitidez = nitidez.toFloat(),
        fraccionReflejo = saturados.toFloat() / muestras,
        luminancia = sumaLuminancia.toFloat() / muestras,
    )
}

private fun cuantilAlto(histograma: IntArray, total: Int): Int {
    val objetivo = (total * CUANTIL_NITIDEZ).toInt()
    var acumulado = 0
    for ((valor, cantidad) in histograma.withIndex()) {
        acumulado += cantidad
        if (acumulado > objetivo) return valor
    }
    return histograma.lastIndex
}

/// Qué hacer con un frame según su nitidez comparada con la de los frames
/// recientes de la misma sesión.
data class DecisionCalidad(
    /// `false` = no vale la pena pasarlo por ML Kit.
    val procesar: Boolean,
    /// Peso de su lectura en la votación entre frames, en (0, 1]: 1 para el
    /// más nítido de la ventana.
    val peso: Float,
)

/// Descarta ANTES de ML Kit (lo más caro de cada frame) los frames
/// claramente más borrosos que los recientes -- típicamente mientras la
/// persona acomoda el documento -- y pondera el resto por nitidez.
///
/// El criterio es relativo al mejor frame reciente, nunca un umbral fijo:
/// la nitidez absoluta cambia con el teléfono, la luz y el documento, y un
/// umbral mal calibrado dejaría de leer por completo. Además, nunca se
/// descartan más de [maximoDescartesSeguidos] frames seguidos: si la
/// cámara no logra nada mejor, se lee lo que hay.
///
/// Pensado para un solo hilo (el del analizador de cámara).
class FiltroCalidad(
    private val ventana: Int = 8,
    private val fraccionMinima: Float = 0.6f,
    private val maximoDescartesSeguidos: Int = 3,
) {
    private val recientes = ArrayDeque<Float>()
    private var descartesSeguidos = 0

    init {
        require(ventana > 0) { "ventana debe ser positiva" }
        require(fraccionMinima in 0f..1f) { "fraccionMinima debe estar entre 0 y 1" }
    }

    fun evaluar(calidad: CalidadFrame): DecisionCalidad {
        recientes.addLast(calidad.nitidez)
        while (recientes.size > ventana) recientes.removeFirst()
        val mejor = recientes.max()
        // Sin bordes significativos en ninguno de los frames recientes
        // (cámara tapada, documento todavía fuera de cuadro) la comparación
        // sería sólo ruido: no se descarta nada.
        val relativa = if (mejor >= NITIDEZ_MINIMA_SIGNIFICATIVA) calidad.nitidez / mejor else 1f
        val procesar = relativa >= fraccionMinima || descartesSeguidos >= maximoDescartesSeguidos
        descartesSeguidos = if (procesar) 0 else descartesSeguidos + 1
        return DecisionCalidad(procesar = procesar, peso = relativa.coerceIn(PESO_MINIMO, 1f))
    }

    fun reiniciar() {
        recientes.clear()
        descartesSeguidos = 0
    }

    private companion object {
        // Un frame que igual se procesó (por el tope de descartes) vota,
        // pero poco.
        const val PESO_MINIMO = 0.2f

        // En niveles de gris (0-255): por debajo, la diferencia entre dos
        // frames es ruido del sensor, no foco.
        const val NITIDEZ_MINIMA_SIGNIFICATIVA = 8f
    }
}

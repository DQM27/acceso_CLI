package com.brisas.controlacceso

import java.lang.ref.WeakReference

// Parte PURA de la telemetría de diagnóstico (ver `Telemetria.kt`): nada de
// Android, para probarla en la JVM. Sólo números y nombres técnicos: nunca
// texto leído por el OCR, cédulas, nombres ni imágenes.

/// JSON mínimo para las filas de telemetría: mapas, listas, cadenas,
/// números, booleanos y null. Suficiente para `jsonb` y sin depender de
/// `org.json` (que en los tests JVM es un stub que tira "not mocked").
fun aJson(valor: Any?): String = StringBuilder().also { escribirJson(it, valor) }.toString()

private fun escribirJson(salida: StringBuilder, valor: Any?) {
    when (valor) {
        null -> salida.append("null")
        is Boolean -> salida.append(valor)
        is Float -> salida.append(if (valor.isFinite()) valor.toString() else "null")
        is Double -> salida.append(if (valor.isFinite()) valor.toString() else "null")
        is Number -> salida.append(valor)
        is Map<*, *> -> {
            salida.append('{')
            valor.entries.forEachIndexed { indice, (clave, contenido) ->
                if (indice > 0) salida.append(',')
                escribirCadena(salida, clave.toString())
                salida.append(':')
                escribirJson(salida, contenido)
            }
            salida.append('}')
        }
        is Iterable<*> -> {
            salida.append('[')
            valor.forEachIndexed { indice, contenido ->
                if (indice > 0) salida.append(',')
                escribirJson(salida, contenido)
            }
            salida.append(']')
        }
        else -> escribirCadena(salida, valor.toString())
    }
}

private fun escribirCadena(salida: StringBuilder, texto: String) {
    salida.append('"')
    for (c in texto) {
        when (c) {
            '"' -> salida.append("\\\"")
            '\\' -> salida.append("\\\\")
            '\n' -> salida.append("\\n")
            '\r' -> salida.append("\\r")
            '\t' -> salida.append("\\t")
            else -> if (c < ' ') salida.append(String.format(java.util.Locale.ROOT, "\\u%04x", c.code)) else salida.append(c)
        }
    }
    salida.append('"')
}

/// Percentil `p` (0..1) por el método del rango más cercano. `null` si no
/// hay valores.
fun percentil(valoresOrdenados: List<Double>, p: Double): Double? {
    if (valoresOrdenados.isEmpty()) return null
    val indice = (kotlin.math.ceil(p * valoresOrdenados.size).toInt() - 1).coerceIn(0, valoresOrdenados.lastIndex)
    return valoresOrdenados[indice]
}

private fun Double.redondeado(decimales: Int = 1): Double {
    val factor = Math.pow(10.0, decimales.toDouble())
    return kotlin.math.round(this * factor) / factor
}

/// Uso de procesador del proceso entre dos muestras. `porcentajeDispositivo`
/// es sobre todos los núcleos (0-100); `porcentajeUnNucleo` puede pasar de
/// 100 si la app usa varios núcleos a la vez (p. ej. ML Kit en paralelo).
data class UsoCpu(val porcentajeDispositivo: Double, val porcentajeUnNucleo: Double)

fun calcularUsoCpu(deltaCpuMs: Long, deltaParedMs: Long, nucleos: Int): UsoCpu? {
    if (deltaParedMs <= 0 || nucleos <= 0 || deltaCpuMs < 0) return null
    val unNucleo = deltaCpuMs * 100.0 / deltaParedMs
    return UsoCpu(porcentajeDispositivo = (unNucleo / nucleos).redondeado(), porcentajeUnNucleo = unNucleo.redondeado())
}

/// Cola de filas ya serializadas, con tope: si el envío falla por mucho
/// tiempo, se descartan las más viejas antes que crecer sin límite (la
/// telemetría no puede ser ella misma una fuga de memoria).
class ColaTelemetria(private val capacidad: Int = 5_000) {
    private val filas = ArrayDeque<String>()
    var descartadas: Long = 0
        private set

    @Synchronized
    fun agregar(fila: String) {
        filas.addLast(fila)
        while (filas.size > capacidad) {
            filas.removeFirst()
            descartadas++
        }
    }

    @Synchronized
    fun agregarTodas(lote: Collection<String>) = lote.forEach(::agregar)

    /// Saca hasta `maximo` filas, las más viejas primero.
    @Synchronized
    fun tomar(maximo: Int): List<String> {
        val lote = ArrayList<String>(minOf(maximo, filas.size))
        while (lote.size < maximo && filas.isNotEmpty()) lote.add(filas.removeFirst())
        return lote
    }

    /// Devuelve un lote que no se pudo enviar al FRENTE de la cola, para
    /// conservar el orden.
    @Synchronized
    fun devolver(lote: List<String>) {
        lote.asReversed().forEach(filas::addFirst)
        while (filas.size > capacidad) {
            filas.removeLast()
            descartadas++
        }
    }

    @Synchronized
    fun tamano(): Int = filas.size
}

/// Duración de las llamadas al núcleo Rust, agregada por nombre entre dos
/// envíos (una fila por operación en vez de una por llamada).
class AgregadorLlamadas {
    private class Acumulado(val duracionesMs: MutableList<Double> = mutableListOf(), var errores: Int = 0)

    private val porNombre = LinkedHashMap<String, Acumulado>()

    @Synchronized
    fun registrar(nombre: String, nanos: Long, ok: Boolean) {
        val acumulado = porNombre.getOrPut(nombre) { Acumulado() }
        acumulado.duracionesMs.add(nanos / 1e6)
        if (!ok) acumulado.errores++
    }

    /// Resumen por nombre y vacía lo acumulado.
    @Synchronized
    fun vaciar(): List<Map<String, Any?>> {
        val resumen = porNombre.map { (nombre, acumulado) ->
            val ordenadas = acumulado.duracionesMs.sorted()
            mapOf(
                "nombre" to nombre,
                "llamadas" to ordenadas.size,
                "errores" to acumulado.errores,
                "p50_ms" to percentil(ordenadas, 0.5)?.redondeado(),
                "p90_ms" to percentil(ordenadas, 0.9)?.redondeado(),
                "max_ms" to ordenadas.lastOrNull()?.redondeado(),
                "total_ms" to ordenadas.sum().redondeado(),
            )
        }
        porNombre.clear()
        return resumen
    }
}

/// Duración de los frames dibujados, agregada por pantalla. Un frame es
/// "trabado" si tardó más del doble del presupuesto de la pantalla (mismo
/// criterio que JankStats de AndroidX) y "congelado" si pasó de 700 ms
/// (criterio de Android vitals).
class AgregadorFrames(private val presupuestoNanos: Long) {
    private val porPantalla = LinkedHashMap<String, MutableList<Double>>()

    init {
        require(presupuestoNanos > 0) { "El presupuesto por frame debe ser positivo" }
    }

    @Synchronized
    fun registrar(pantalla: String, duracionNanos: Long) {
        porPantalla.getOrPut(pantalla) { mutableListOf() }.add(duracionNanos / 1e6)
    }

    @Synchronized
    fun vaciar(): List<Map<String, Any?>> {
        val presupuestoMs = presupuestoNanos / 1e6
        val resumen = porPantalla.filterValues { it.isNotEmpty() }.map { (pantalla, duraciones) ->
            val ordenadas = duraciones.sorted()
            mapOf(
                "pantalla" to pantalla,
                "frames" to ordenadas.size,
                "trabados" to ordenadas.count { it > presupuestoMs * 2 },
                "congelados" to ordenadas.count { it > MS_FRAME_CONGELADO },
                "presupuesto_ms" to presupuestoMs.redondeado(2),
                "p50_ms" to percentil(ordenadas, 0.5)?.redondeado(),
                "p90_ms" to percentil(ordenadas, 0.9)?.redondeado(),
                "p99_ms" to percentil(ordenadas, 0.99)?.redondeado(),
                "max_ms" to ordenadas.last().redondeado(),
            )
        }
        porPantalla.clear()
        return resumen
    }

    private companion object {
        const val MS_FRAME_CONGELADO = 700.0
    }
}

/// Qué pantalla está visible: la última que entró y no salió. Las pantallas
/// anidadas (una cámara abierta desde Activos) se apilan.
class PilaPantallas {
    private val pila = ArrayDeque<String>()

    @Synchronized
    fun entrar(nombre: String) = pila.addLast(nombre)

    @Synchronized
    fun salir(nombre: String) {
        val indice = pila.lastIndexOf(nombre)
        if (indice >= 0) pila.removeAt(indice)
    }

    @Synchronized
    fun actual(): String = pila.lastOrNull() ?: SIN_PANTALLA

    /// La pantalla debajo de la visible: desde dónde se abrió (p. ej. la
    /// cámara abierta desde Proveedores).
    @Synchronized
    fun anterior(): String = pila.getOrNull(pila.size - 2) ?: SIN_PANTALLA

    companion object {
        const val SIN_PANTALLA = "(ninguna)"
    }
}

/// Detector simple de objetos que deberían haberse liberado y siguen vivos
/// (posibles fugas de memoria): se vigila un objeto cuando su dueño lo
/// suelta (una cámara al cerrar, una Activity al destruirse); si pasado
/// [esperaMs] y tras forzar el recolector sigue siendo alcanzable, se
/// informa UNA vez. Es la misma idea que el ObjectWatcher de LeakCanary,
/// sin el volcado de memoria (que exige un build depurable).
class VigilanteRetencion(
    private val esperaMs: Long = 10_000,
    private val reloj: () -> Long,
    private val forzarRecoleccion: () -> Unit,
) {
    private class Vigilado(val referencia: WeakReference<Any>, val descripcion: String, val desdeMs: Long)

    private val vigilados = mutableListOf<Vigilado>()

    @Synchronized
    fun vigilar(objeto: Any, descripcion: String) {
        vigilados.add(Vigilado(WeakReference(objeto), descripcion, reloj()))
    }

    /// Objetos retenidos más allá de la espera. Los liberados y los ya
    /// informados salen de la lista.
    @Synchronized
    fun revisar(): List<Map<String, Any?>> {
        if (vigilados.isEmpty()) return emptyList()
        val ahora = reloj()
        if (vigilados.any { ahora - it.desdeMs >= esperaMs }) forzarRecoleccion()
        val retenidos = mutableListOf<Map<String, Any?>>()
        vigilados.removeAll { vigilado ->
            when {
                vigilado.referencia.get() == null -> true
                ahora - vigilado.desdeMs >= esperaMs -> {
                    retenidos.add(
                        mapOf(
                            "objeto" to vigilado.descripcion,
                            "segundos_retenido" to (ahora - vigilado.desdeMs) / 1000,
                        ),
                    )
                    true
                }
                else -> false
            }
        }
        return retenidos
    }

    @Synchronized
    fun pendientes(): Int = vigilados.size
}

/// Realtime (avisos en vivo de la nube), agregado entre dos envíos: cómo
/// se conecta y se cae el canal y qué llega por él. Sin contenido de los
/// avisos: sólo tabla, operación, tamaño y tiempos.
class AgregadorRealtime {
    private var intentosConexion = 0
    private val msHastaSuscribir = mutableListOf<Double>()
    private val caidas = sortedMapOf<String, Int>()
    private val errores = sortedMapOf<String, Int>()
    private val msConectado = mutableListOf<Double>()
    private val avisosPorTabla = sortedMapOf<String, Int>()
    private var ecosPropios = 0
    private var bytes = 0L
    private var aplicados = 0
    private var noAplicados = 0
    private val msAplicar = mutableListOf<Double>()
    private val latenciasMs = mutableListOf<Double>()
    private var desfaseRelojMs: Long? = null
    private var vacio = true

    /// Desfase del reloj con que se corrigen las latencias (`null` = no se
    /// midió y la latencia incluye el desfase).
    @Synchronized
    fun desfaseReloj(ms: Long?) {
        desfaseRelojMs = ms
    }

    @Synchronized
    fun conectando() {
        intentosConexion++
        vacio = false
    }

    @Synchronized
    fun suscrito(msDesdeIntento: Long) {
        msHastaSuscribir += msDesdeIntento.toDouble()
        vacio = false
    }

    /// El canal dejó de estar suscrito (`motivo`: estado del canal o
    /// "renovacion" del token) tras `msConectado` de conexión.
    @Synchronized
    fun terminado(motivo: String, msConectadoAhora: Long?) {
        caidas.merge(motivo, 1, Int::plus)
        msConectadoAhora?.let { msConectado += it.toDouble() }
        vacio = false
    }

    @Synchronized
    fun error(tipo: String) {
        errores.merge(tipo, 1, Int::plus)
        vacio = false
    }

    /// Un aviso recibido. `latenciaMs`: hora del teléfono (corregida con el
    /// desfase medido, ver [desfaseReloj]) menos la del servidor al escribir
    /// (`changed_at`).
    @Synchronized
    fun aviso(tabla: String?, bytesAviso: Int, ecoPropio: Boolean, latenciaMs: Long?) {
        vacio = false
        bytes += bytesAviso
        latenciaMs?.let { latenciasMs += it.toDouble() }
        if (ecoPropio) {
            ecosPropios++
            return
        }
        avisosPorTabla.merge(tabla ?: "(sin tabla)", 1, Int::plus)
    }

    /// Resultado de guardar en la base local la fila que trajo un aviso.
    @Synchronized
    fun aplicado(ok: Boolean, nanos: Long) {
        if (ok) aplicados++ else noAplicados++
        msAplicar += nanos / 1e6
        vacio = false
    }

    /// Resumen y vacía lo acumulado; `null` si no pasó nada.
    @Synchronized
    fun vaciar(): Map<String, Any?>? {
        if (vacio) return null
        val p = { valores: List<Double>, q: Double -> percentil(valores.sorted(), q)?.redondeado() }
        val resumen = mapOf(
            "intentos_conexion" to intentosConexion,
            "ms_hasta_suscribir_p50" to p(msHastaSuscribir, 0.5),
            "ms_hasta_suscribir_max" to msHastaSuscribir.maxOrNull()?.redondeado(),
            "fin_de_conexion" to caidas.toMap(),
            "errores" to errores.toMap(),
            "minutos_conectado_max" to msConectado.maxOrNull()?.let { (it / 60_000).redondeado() },
            "avisos_por_tabla" to avisosPorTabla.toMap(),
            "avisos" to avisosPorTabla.values.sum(),
            "ecos_propios" to ecosPropios,
            "kb_recibidos" to (bytes / 1024.0).redondeado(),
            "aplicados" to aplicados,
            "no_aplicados" to noAplicados,
            "ms_aplicar_p50" to p(msAplicar, 0.5),
            "ms_aplicar_max" to msAplicar.maxOrNull()?.redondeado(),
            "latencia_ms_p50" to p(latenciasMs, 0.5),
            "latencia_ms_p90" to p(latenciasMs, 0.9),
            "latencia_ms_min" to latenciasMs.minOrNull()?.redondeado(),
            "latencia_ms_max" to latenciasMs.maxOrNull()?.redondeado(),
            "latencia_corregida" to (desfaseRelojMs != null),
            "desfase_reloj_ms" to desfaseRelojMs,
        )
        intentosConexion = 0
        msHastaSuscribir.clear()
        caidas.clear()
        errores.clear()
        msConectado.clear()
        avisosPorTabla.clear()
        ecosPropios = 0
        bytes = 0
        aplicados = 0
        noAplicados = 0
        msAplicar.clear()
        latenciasMs.clear()
        vacio = true
        return resumen
    }
}

/// Resumen de violaciones de StrictMode (acceso a disco o red en el hilo
/// de la pantalla, recursos que no se cerraron, Activities filtradas),
/// agregadas por tipo y por el primer punto del código de la app donde
/// ocurrieron, para no mandar una fila por cada repetición.
class AgregadorViolaciones {
    private val conteos = LinkedHashMap<Pair<String, String>, Int>()

    @Synchronized
    fun registrar(tipo: String, origen: String) {
        val clave = tipo to origen
        conteos[clave] = (conteos[clave] ?: 0) + 1
    }

    @Synchronized
    fun vaciar(): List<Map<String, Any?>> {
        val resumen = conteos.map { (clave, veces) -> mapOf("tipo" to clave.first, "origen" to clave.second, "veces" to veces) }
        conteos.clear()
        return resumen
    }
}

/// Primer marco de la pila que pertenece al código de la app (paquete
/// `com.brisas`): dónde hay que mirar. Si no hay ninguno, el primero.
fun origenEnLaApp(pila: Array<StackTraceElement>, paquete: String = "com.brisas."): String {
    val marco = pila.firstOrNull { it.className.startsWith(paquete) } ?: pila.firstOrNull() ?: return "(desconocido)"
    return "${marco.className.substringAfterLast('.')}.${marco.methodName}:${marco.lineNumber}"
}

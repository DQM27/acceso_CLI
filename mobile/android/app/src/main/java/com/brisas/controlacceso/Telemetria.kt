package com.brisas.controlacceso

import android.app.ActivityManager
import android.app.Application
import android.app.ApplicationExitInfo
import android.content.ComponentCallbacks2
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.res.Configuration
import android.net.TrafficStats
import android.os.BatteryManager
import android.os.Build
import android.os.Debug
import android.os.Handler
import android.os.HandlerThread
import android.os.PowerManager
import android.os.Process
import android.os.StrictMode
import android.os.SystemClock
import android.view.FrameMetrics
import android.view.Window
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.time.Instant
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit

/// Telemetría de rendimiento para DIAGNÓSTICO (sólo el build
/// `diagnostico`, ver app/build.gradle.kts): arranque, pantallas, frames
/// trabados, memoria, posibles fugas, procesador, batería y temperatura,
/// red, duración de las llamadas al núcleo Rust, por qué murió el proceso
/// la vez anterior, violaciones de StrictMode y sesiones de escaneo OCR.
///
/// Todo va a la tabla `telemetria_diagnostico` del SANDBOX (staging, ver
/// `supabase/scripts/telemetria_diagnostico_staging.sql`), en lotes cada
/// minuto y al pasar la app a segundo plano; lo que no se pudo enviar se
/// guarda en un archivo y se reintenta. Sólo números y nombres técnicos:
/// nunca texto leído, cédulas, nombres ni imágenes.
///
/// Con `BuildConfig.TELEMETRIA = false` (release y debug) nada de esto se
/// inicializa y cada punto de medición es un chequeo de un booleano.
object Telemetria {
    @Volatile
    var activa: Boolean = false
        private set

    private var motor: MotorTelemetria? = null

    fun iniciar(app: Application) {
        if (!BuildConfig.TELEMETRIA || activa) return
        motor = MotorTelemetria(app).also { it.arrancar() }
        activa = true
    }

    fun evento(tipo: String, datos: Map<String, Any?> = emptyMap()) {
        motor?.encolar(tipo, datos)
    }

    fun registrarLlamada(nombre: String, nanos: Long, ok: Boolean) {
        motor?.registrarLlamada(nombre, nanos, ok)
    }

    /// La ventana de la Activity, para medir la duración de cada frame.
    fun observarVentana(ventana: Window) {
        motor?.observarVentana(ventana)
    }

    /// El primer frame de la app terminó de dibujarse (arranque completo).
    fun primerFrameDibujado() {
        motor?.primerFrameDibujado()
    }

    fun enPrimerPlano(visible: Boolean) {
        motor?.enPrimerPlano(visible)
    }

    /// Métricas de Realtime (`null` con la telemetría apagada).
    val realtime: AgregadorRealtime? get() = motor?.realtime

    fun pantallaActual(): String = motor?.pantallas?.actual() ?: PilaPantallas.SIN_PANTALLA

    fun pantallaAnterior(): String = motor?.pantallas?.anterior() ?: PilaPantallas.SIN_PANTALLA

    fun entrarPantalla(nombre: String) {
        motor?.pantallas?.entrar(nombre)
    }

    fun salirPantalla(nombre: String) {
        motor?.pantallas?.salir(nombre)
    }

    fun pantallaDibujada(nombre: String, nanosDesdeEntrada: Long) {
        evento("pantalla", mapOf("pantalla" to nombre, "ms_hasta_primer_frame" to nanosDesdeEntrada / 1_000_000))
    }

    /// `objeto` ya debería poder liberarse (su dueño lo soltó): si sigue
    /// vivo en unos segundos, se informa como posible fuga.
    fun vigilarRetencion(objeto: Any, descripcion: String) {
        motor?.vigilante?.vigilar(objeto, descripcion)
    }
}

/// Mide una llamada al núcleo Rust y la suma a la telemetría. Con la
/// telemetría apagada es sólo la llamada.
inline fun <T> medirNucleo(nombre: String, bloque: () -> T): T {
    if (!Telemetria.activa) return bloque()
    val inicio = System.nanoTime()
    var ok = false
    try {
        val resultado = bloque()
        ok = true
        return resultado
    } finally {
        Telemetria.registrarLlamada(nombre, System.nanoTime() - inicio, ok)
    }
}

private class MotorTelemetria(private val app: Application) {
    private val preferencias = app.getSharedPreferences("telemetria_diagnostico", Context.MODE_PRIVATE)
    private val dispositivo: String = preferencias.getString(CLAVE_DISPOSITIVO, null)
        ?: UUID.randomUUID().toString().also { preferencias.edit().putString(CLAVE_DISPOSITIVO, it).apply() }
    private val sesion = UUID.randomUUID().toString()
    private val versionApp = "${BuildConfig.VERSION_NAME} (${BuildConfig.VERSION_CODE})"
    private val ejecutor: ScheduledExecutorService = Executors.newSingleThreadScheduledExecutor { tarea ->
        Thread(tarea, "telemetria").apply { priority = Thread.MIN_PRIORITY }
    }
    private val cola = ColaTelemetria()
    private val archivoPendiente = File(app.filesDir, "telemetria_pendiente.jsonl")
    private val llamadas = AgregadorLlamadas()
    private val violaciones = AgregadorViolaciones()
    val realtime = AgregadorRealtime()
    val pantallas = PilaPantallas()
    val vigilante = VigilanteRetencion(reloj = SystemClock::elapsedRealtime, forzarRecoleccion = { Runtime.getRuntime().gc() })
    private val activityManager = app.getSystemService(ActivityManager::class.java)
    private val nucleosCpu = Runtime.getRuntime().availableProcessors()
    private var frames: AgregadorFrames? = null
    private var hiloFrames: HandlerThread? = null

    @Volatile private var visible = false
    private var primerFrameInformado = false
    private var ultimaCpuMs = Process.getElapsedCpuTime()
    private var ultimaParedMs = SystemClock.elapsedRealtime()
    private var ultimoRx = TrafficStats.getUidRxBytes(Process.myUid())
    private var ultimoTx = TrafficStats.getUidTxBytes(Process.myUid())

    fun arrancar() {
        cargarPendientes()
        encolar("sesion_inicio", datosDispositivo())
        ejecutor.execute { informarSalidasAnteriores() }
        activarStrictMode()
        app.registerComponentCallbacks(object : ComponentCallbacks2 {
            override fun onTrimMemory(nivel: Int) {
                encolar("memoria_baja", mapOf("nivel" to nivel, "pantalla" to pantallas.actual()))
            }

            override fun onConfigurationChanged(configuracion: Configuration) = Unit

            // Obligatorio por la interfaz; `onTrimMemory` ya cubre este caso.
            @Suppress("OVERRIDE_DEPRECATION")
            override fun onLowMemory() = Unit
        })
        ejecutor.scheduleWithFixedDelay(::muestrear, SEGUNDOS_MUESTREO, SEGUNDOS_MUESTREO, TimeUnit.SECONDS)
        ejecutor.scheduleWithFixedDelay(::enviar, SEGUNDOS_ENVIO, SEGUNDOS_ENVIO, TimeUnit.SECONDS)
    }

    fun encolar(tipo: String, datos: Map<String, Any?>) {
        cola.agregar(
            aJson(
                mapOf(
                    "ocurrido_en" to Instant.now().toString(),
                    "dispositivo" to dispositivo,
                    "sesion" to sesion,
                    "version_app" to versionApp,
                    "tipo" to tipo,
                    "datos" to datos,
                ),
            ),
        )
    }

    fun registrarLlamada(nombre: String, nanos: Long, ok: Boolean) = llamadas.registrar(nombre, nanos, ok)

    fun enPrimerPlano(visible: Boolean) {
        this.visible = visible
        // Al irse a segundo plano: resumen de lo acumulado y envío ya, por
        // si el sistema mata el proceso después.
        if (!visible) ejecutor.execute { volcarAgregados(); enviar(); guardarPendientes() }
    }

    fun primerFrameDibujado() {
        if (primerFrameInformado) return
        primerFrameInformado = true
        val datos = mutableMapOf<String, Any?>(
            "ms_desde_inicio_proceso" to SystemClock.uptimeMillis() - Process.getStartUptimeMillis(),
        )
        if (Build.VERSION.SDK_INT >= 35) {
            activityManager.getHistoricalProcessStartReasons(1).firstOrNull()?.let { inicio ->
                datos["tipo_arranque"] = inicio.startType
                datos["motivo_arranque"] = inicio.reason
                datos["estado_arranque"] = inicio.startupState
            }
        }
        encolar("arranque", datos)
    }

    fun observarVentana(ventana: Window) {
        val refresco = app.getSystemService(android.hardware.display.DisplayManager::class.java)
            .getDisplay(android.view.Display.DEFAULT_DISPLAY)?.refreshRate ?: 60f
        val agregador = AgregadorFrames(presupuestoNanos = (1_000_000_000 / refresco).toLong())
        frames = agregador
        val hilo = hiloFrames ?: HandlerThread("telemetria-frames").also { it.start(); hiloFrames = it }
        ventana.addOnFrameMetricsAvailableListener({ _, metricas, _ ->
            agregador.registrar(pantallas.actual(), metricas.getMetric(FrameMetrics.TOTAL_DURATION))
        }, Handler(hilo.looper))
    }

    private fun muestrear() {
        try {
            val vigiladosRetenidos = vigilante.revisar()
            vigiladosRetenidos.forEach { encolar("retencion", it) }
            if (!visible) return
            encolar("muestra_sistema", muestraSistema())
            volcarAgregados()
        } catch (e: RuntimeException) {
            encolar("error_telemetria", mapOf("donde" to "muestrear", "error" to e.javaClass.simpleName))
        }
    }

    private fun volcarAgregados() {
        llamadas.vaciar().forEach { encolar("llamada_nucleo", it) }
        frames?.vaciar()?.forEach { encolar("frames", it) }
        violaciones.vaciar().forEach { encolar("strictmode", it) }
        realtime.vaciar()?.let { encolar("realtime", it) }
    }

    private fun muestraSistema(): Map<String, Any?> {
        val runtime = Runtime.getRuntime()
        val memoria = Debug.MemoryInfo().also(Debug::getMemoryInfo)
        val sistema = ActivityManager.MemoryInfo().also(activityManager::getMemoryInfo)
        val ahoraCpu = Process.getElapsedCpuTime()
        val ahoraPared = SystemClock.elapsedRealtime()
        val cpu = calcularUsoCpu(ahoraCpu - ultimaCpuMs, ahoraPared - ultimaParedMs, nucleosCpu)
        ultimaCpuMs = ahoraCpu
        ultimaParedMs = ahoraPared
        val rx = TrafficStats.getUidRxBytes(Process.myUid())
        val tx = TrafficStats.getUidTxBytes(Process.myUid())
        val datos = mutableMapOf<String, Any?>(
            "pantalla" to pantallas.actual(),
            "heap_java_usada_mb" to mb(runtime.totalMemory() - runtime.freeMemory()),
            "heap_java_max_mb" to mb(runtime.maxMemory()),
            "heap_nativa_mb" to mb(Debug.getNativeHeapAllocatedSize()),
            "pss_total_mb" to kbAMb(memoria.getMemoryStat("summary.total-pss")),
            "pss_java_mb" to kbAMb(memoria.getMemoryStat("summary.java-heap")),
            "pss_nativa_mb" to kbAMb(memoria.getMemoryStat("summary.native-heap")),
            "pss_graficos_mb" to kbAMb(memoria.getMemoryStat("summary.graphics")),
            "pss_codigo_mb" to kbAMb(memoria.getMemoryStat("summary.code")),
            "sistema_disponible_mb" to mb(sistema.availMem),
            "sistema_memoria_baja" to sistema.lowMemory,
            "hilos" to lineaDeEstado("Threads:"),
            "descriptores_abiertos" to File("/proc/self/fd").list()?.size,
            "cpu_dispositivo_pct" to cpu?.porcentajeDispositivo,
            "cpu_un_nucleo_pct" to cpu?.porcentajeUnNucleo,
            "red_rx_kb" to if (rx >= 0 && ultimoRx >= 0) (rx - ultimoRx) / 1024 else null,
            "red_tx_kb" to if (tx >= 0 && ultimoTx >= 0) (tx - ultimoTx) / 1024 else null,
            "cola_telemetria" to cola.tamano(),
            "telemetria_descartada" to cola.descartadas,
            "vigilados_pendientes" to vigilante.pendientes(),
        )
        ultimoRx = rx
        ultimoTx = tx
        datos.putAll(bateriaYTemperatura())
        return datos
    }

    private fun bateriaYTemperatura(): Map<String, Any?> {
        val bateria = app.getSystemService(BatteryManager::class.java)
        val estado: Intent? = app.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
        val energia = app.getSystemService(PowerManager::class.java)
        val datos = mutableMapOf<String, Any?>(
            "bateria_pct" to bateria.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY),
            // µA; el signo depende del fabricante (en Samsung, negativo = descarga).
            "bateria_corriente_ma" to bateria.getIntProperty(BatteryManager.BATTERY_PROPERTY_CURRENT_NOW) / 1000,
            "bateria_temperatura_c" to estado?.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, Int.MIN_VALUE)
                ?.takeIf { it != Int.MIN_VALUE }?.let { it / 10.0 },
            "cargando" to ((estado?.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0) ?: 0) != 0),
            "ahorro_energia" to energia.isPowerSaveMode,
        )
        if (Build.VERSION.SDK_INT >= 29) datos["estado_termico"] = energia.currentThermalStatus
        if (Build.VERSION.SDK_INT >= 30) datos["margen_termico"] = energia.getThermalHeadroom(0).takeIf { it.isFinite() }
        return datos
    }

    private fun datosDispositivo(): Map<String, Any?> {
        val sistema = ActivityManager.MemoryInfo().also(activityManager::getMemoryInfo)
        return mapOf(
            "fabricante" to Build.MANUFACTURER,
            "modelo" to Build.MODEL,
            "android_sdk" to Build.VERSION.SDK_INT,
            "android_version" to Build.VERSION.RELEASE,
            "abi" to Build.SUPPORTED_ABIS.firstOrNull(),
            "nucleos_cpu" to nucleosCpu,
            "ram_total_mb" to mb(sistema.totalMem),
            "clase_memoria_mb" to activityManager.memoryClass,
            "build" to BuildConfig.BUILD_TYPE,
        )
    }

    /// Por qué terminó el proceso las veces anteriores (ANR, falta de
    /// memoria, crash nativo o de Java, cierre por el usuario...). Se
    /// informa cada salida una sola vez.
    private fun informarSalidasAnteriores() {
        if (Build.VERSION.SDK_INT < 30) return
        val ultimaInformada = preferencias.getLong(CLAVE_ULTIMA_SALIDA, 0)
        val salidas = activityManager.getHistoricalProcessExitReasons(app.packageName, 0, 20)
            .filter { it.timestamp > ultimaInformada }
        salidas.sortedBy(ApplicationExitInfo::getTimestamp).forEach { salida ->
            encolar(
                "salida_anterior",
                mapOf(
                    "motivo" to salida.reason,
                    "motivo_nombre" to nombreMotivoSalida(salida.reason),
                    "estado" to salida.status,
                    "importancia" to salida.importance,
                    "pss_mb" to salida.pss / 1024,
                    "rss_mb" to salida.rss / 1024,
                    // Sólo el resumen antes de ":": en un crash lo que sigue
                    // es el mensaje de la excepción, que podría traer datos.
                    "descripcion" to salida.description?.substringBefore(':')?.take(120),
                    "cuando" to Instant.ofEpochMilli(salida.timestamp).toString(),
                ),
            )
        }
        salidas.maxOfOrNull { it.timestamp }?.let { preferencias.edit().putLong(CLAVE_ULTIMA_SALIDA, it).apply() }
    }

    /// Disco o red en el hilo de la pantalla, recursos sin cerrar y
    /// Activities filtradas. Se agregan (ver `AgregadorViolaciones`).
    private fun activarStrictMode() {
        if (Build.VERSION.SDK_INT < 28) return
        val registrar = { violacion: android.os.strictmode.Violation ->
            violaciones.registrar(violacion.javaClass.simpleName, origenEnLaApp(violacion.stackTrace))
        }
        StrictMode.setThreadPolicy(
            StrictMode.ThreadPolicy.Builder()
                .detectDiskReads()
                .detectDiskWrites()
                .detectNetwork()
                .detectCustomSlowCalls()
                .penaltyListener(ejecutor, registrar)
                .build(),
        )
        StrictMode.setVmPolicy(
            StrictMode.VmPolicy.Builder()
                .detectActivityLeaks()
                .detectLeakedClosableObjects()
                .detectLeakedRegistrationObjects()
                .detectLeakedSqlLiteObjects()
                .penaltyListener(ejecutor, registrar)
                .build(),
        )
    }

    /// Una excepción que escape de una tarea periódica cancela en silencio
    /// todas sus ejecuciones siguientes (`scheduleWithFixedDelay`): por eso
    /// ningún error de un envío puede salir de acá.
    private fun enviar() {
        try {
            while (true) {
                val lote = cola.tomar(FILAS_POR_LOTE)
                if (lote.isEmpty()) return
                if (!publicar(lote)) {
                    cola.devolver(lote)
                    guardarPendientes()
                    return
                }
            }
        } catch (e: RuntimeException) {
            encolar("error_telemetria", mapOf("donde" to "enviar", "error" to e.javaClass.simpleName))
        }
    }

    private fun publicar(lote: List<String>): Boolean = try {
        val conexion = URL("${AmbienteStaging.URL}/rest/v1/telemetria_diagnostico").openConnection() as HttpURLConnection
        try {
            conexion.requestMethod = "POST"
            conexion.connectTimeout = TIMEOUT_MS
            conexion.readTimeout = TIMEOUT_MS
            conexion.doOutput = true
            conexion.setRequestProperty("apikey", AmbienteStaging.APIKEY)
            conexion.setRequestProperty("Content-Type", "application/json")
            conexion.setRequestProperty("Prefer", "return=minimal")
            conexion.outputStream.use { it.write(lote.joinToString(",", "[", "]").toByteArray(Charsets.UTF_8)) }
            conexion.responseCode in 200..299
        } finally {
            conexion.disconnect()
        }
    } catch (e: java.io.IOException) {
        false
    }

    private fun guardarPendientes() {
        val pendientes = cola.tomar(Int.MAX_VALUE)
        cola.devolver(pendientes)
        try {
            archivoPendiente.writeText(pendientes.joinToString("\n"))
        } catch (e: java.io.IOException) {
            // Sin espacio o sin permiso: se pierde, es sólo diagnóstico.
        }
    }

    private fun cargarPendientes() {
        try {
            if (archivoPendiente.exists()) {
                cola.agregarTodas(archivoPendiente.readLines().filter { it.isNotBlank() })
                archivoPendiente.delete()
            }
        } catch (e: java.io.IOException) {
            archivoPendiente.delete()
        }
    }

    private fun lineaDeEstado(clave: String): Int? = try {
        File("/proc/self/status").useLines { lineas ->
            lineas.firstOrNull { it.startsWith(clave) }?.substringAfter(clave)?.trim()?.toIntOrNull()
        }
    } catch (e: java.io.IOException) {
        null
    }

    private fun mb(bytes: Long): Long = bytes / (1024 * 1024)
    private fun kbAMb(kb: String?): Long? = kb?.toLongOrNull()?.div(1024)

    private companion object {
        const val CLAVE_DISPOSITIVO = "dispositivo"
        const val CLAVE_ULTIMA_SALIDA = "ultima_salida_informada"
        const val SEGUNDOS_MUESTREO = 30L
        const val SEGUNDOS_ENVIO = 60L
        const val FILAS_POR_LOTE = 200
        const val TIMEOUT_MS = 10_000
    }
}

/// Nombre legible de `ApplicationExitInfo.reason` (las constantes son
/// enteros; así las filas se leen sin buscar en la documentación).
private fun nombreMotivoSalida(motivo: Int): String = when (motivo) {
    ApplicationExitInfo.REASON_ANR -> "ANR"
    ApplicationExitInfo.REASON_CRASH -> "CRASH_JAVA"
    ApplicationExitInfo.REASON_CRASH_NATIVE -> "CRASH_NATIVO"
    ApplicationExitInfo.REASON_EXIT_SELF -> "SALIDA_PROPIA"
    ApplicationExitInfo.REASON_LOW_MEMORY -> "MEMORIA_BAJA"
    ApplicationExitInfo.REASON_EXCESSIVE_RESOURCE_USAGE -> "USO_EXCESIVO"
    ApplicationExitInfo.REASON_USER_REQUESTED -> "USUARIO"
    ApplicationExitInfo.REASON_USER_STOPPED -> "USUARIO_DETUVO"
    ApplicationExitInfo.REASON_SIGNALED -> "SENAL"
    ApplicationExitInfo.REASON_INITIALIZATION_FAILURE -> "FALLA_INICIO"
    ApplicationExitInfo.REASON_PERMISSION_CHANGE -> "CAMBIO_PERMISO"
    ApplicationExitInfo.REASON_DEPENDENCY_DIED -> "DEPENDENCIA"
    ApplicationExitInfo.REASON_OTHER -> "OTRO"
    else -> "DESCONOCIDO_$motivo"
}

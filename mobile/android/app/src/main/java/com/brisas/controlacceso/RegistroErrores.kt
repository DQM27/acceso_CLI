package com.brisas.controlacceso

import android.app.ActivityManager
import android.app.Application
import android.app.ApplicationExitInfo
import android.content.Context
import android.os.Build
import uniffi.control_acceso_mobile.anotarOperacion
import uniffi.control_acceso_mobile.enviarErroresPendientes
import uniffi.control_acceso_mobile.fijarPantallaErrores
import uniffi.control_acceso_mobile.instalarRegistroErrores
import uniffi.control_acceso_mobile.reportarError

/// Registro de errores del teléfono (fase 4 de
/// `docs/features-futuras/plan-telemetria-y-registro-de-errores.md`), sin el
/// SDK de Sentry. Siempre encendido: no depende de la orden de telemetría.
///
/// El registro vive en el núcleo (`control_acceso::errores`): guarda cada
/// error en disco en el momento y lo manda al receptor propio cada minuto.
/// Kotlin sólo le pasa lo que el núcleo no ve:
/// - excepciones que nadie atrapó (`Thread.setDefaultUncaughtExceptionHandler`,
///   que también recibe las de las corrutinas);
/// - ANR y fallos nativos de las veces anteriores (`ApplicationExitInfo`, con
///   el volcado de hilos del ANR);
/// - la pantalla actual y cada llamada al núcleo (`medirNucleo`), que viajan
///   con el próximo error.
///
/// Los pánicos del núcleo y sus errores inesperados los reporta el núcleo
/// mismo. Nada de esto puede tirar la app: cada llamada va en `runCatching`.
object RegistroErrores {
    @Volatile
    private var activo = false

    /// Al crear la `Application`, antes de abrir el núcleo: un fallo al abrir
    /// la base también se reporta.
    fun instalar(app: Application) {
        if (activo) return
        activo = runCatching {
            instalarRegistroErrores(
                directorio = app.filesDir.absolutePath,
                versionApp = "${BuildConfig.VERSION_NAME} (${BuildConfig.VERSION_CODE})",
                dsn = BuildConfig.DSN_ERRORES,
                contexto = mapOf(
                    "os" to "android",
                    "os_version" to Build.VERSION.RELEASE,
                    "android_sdk" to Build.VERSION.SDK_INT.toString(),
                    "fabricante" to Build.MANUFACTURER,
                    "modelo" to Build.MODEL,
                    "arquitectura" to (Build.SUPPORTED_ABIS.firstOrNull() ?: ""),
                    "ambiente" to if (BuildConfig.AMBIENTE_STAGING) "staging" else "produccion",
                ),
            )
        }.getOrDefault(false)
        if (!activo) return
        val anterior = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { hilo, error ->
            // A disco antes de que el proceso muera; sale en el próximo arranque.
            runCatching { reportarError("excepcion", tituloDeExcepcion(error), error.stackTraceToString()) }
            anterior?.uncaughtException(hilo, error)
        }
        // Lee archivos del sistema: fuera del hilo de la pantalla.
        Thread({ runCatching { informarSalidasAnteriores(app) } }, "errores-salidas").start()
    }

    /// Una llamada al núcleo terminó (`medirNucleo`).
    fun operacion(nombre: String, ok: Boolean) {
        if (activo) runCatching { anotarOperacion(nombre, ok) }
    }

    fun pantalla(nombre: String) {
        if (activo) runCatching { fijarPantallaErrores(nombre) }
    }

    /// La app pasó a segundo plano: manda lo pendiente ya, por si el sistema
    /// mata el proceso después.
    fun enviarYa() {
        if (activo) Thread({ runCatching { enviarErroresPendientes() } }, "errores-envio-ya").start()
    }

    /// ANR y fallos nativos que terminaron el proceso las veces anteriores,
    /// una sola vez cada uno. Un crash de Java no: ya lo reportó el manejador
    /// de excepciones antes de morir. La primera vez sólo se toma la marca:
    /// lo de antes de instalar el registro no se manda.
    private fun informarSalidasAnteriores(app: Application) {
        if (Build.VERSION.SDK_INT < 30) return
        val preferencias = app.getSharedPreferences("registro_errores", Context.MODE_PRIVATE)
        val ultima = preferencias.getLong(CLAVE_ULTIMA_SALIDA, -1)
        val salidas = app.getSystemService(ActivityManager::class.java)
            .getHistoricalProcessExitReasons(app.packageName, 0, 20)
        salidas.maxOfOrNull { it.timestamp }?.let { preferencias.edit().putLong(CLAVE_ULTIMA_SALIDA, it).apply() }
        if (ultima < 0) return
        salidas.filter { it.timestamp > ultima }.sortedBy { it.timestamp }.forEach { salida ->
            when (salida.reason) {
                ApplicationExitInfo.REASON_ANR ->
                    reportarError("anr", tituloDeSalida("ANR", salida.description), volcadoDeHilos(salida))
                ApplicationExitInfo.REASON_CRASH_NATIVE ->
                    reportarError("nativo", tituloDeSalida("Fallo nativo", salida.description), "")
            }
        }
    }

    /// El volcado de hilos de un ANR (el núcleo se queda con el principal).
    private fun volcadoDeHilos(salida: ApplicationExitInfo): String = runCatching {
        salida.traceInputStream?.bufferedReader()?.use { lector ->
            val texto = CharArray(MAXIMO_VOLCADO)
            val leidos = lector.read(texto)
            if (leidos > 0) String(texto, 0, leidos) else ""
        } ?: ""
    }.getOrDefault("")

    private const val CLAVE_ULTIMA_SALIDA = "ultima_salida_informada"

    /// El hilo principal está al principio del volcado.
    private const val MAXIMO_VOLCADO = 512 * 1024
}

/// `java.lang.IllegalStateException: mensaje`, o sólo la clase si no trae
/// mensaje. Si envuelve a otra, también la causa de fondo, que suele ser la
/// que explica el error. El núcleo limpia los datos personales.
internal fun tituloDeExcepcion(error: Throwable): String {
    fun describir(e: Throwable) = e.message?.takeIf { it.isNotBlank() }?.let { "${e.javaClass.name}: $it" } ?: e.javaClass.name
    val raiz = generateSequence(error) { it.cause?.takeIf { causa -> causa !== it } }.take(20).last()
    return if (raiz === error) describir(error) else "${describir(error)} (causa: ${describir(raiz)})"
}

/// `ANR: Input dispatching timed out`, `Fallo nativo` sin descripción.
internal fun tituloDeSalida(tipo: String, descripcion: String?): String =
    descripcion?.takeIf { it.isNotBlank() }?.let { "$tipo: $it" } ?: tipo

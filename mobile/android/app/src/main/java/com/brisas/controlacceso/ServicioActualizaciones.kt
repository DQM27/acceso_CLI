package com.brisas.controlacceso

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.IntentCompat
import androidx.core.content.pm.PackageInfoCompat
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URI
import java.security.MessageDigest
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import uniffi.control_acceso_mobile.ManifiestoActualizacion

/// El APK descargado no es el anunciado (huella, paquete, versión o firma).
/// `message` ya viene listo para mostrar.
class ApkRechazado(mensaje: String) : Exception(mensaje)

/// Lo que el actualizador necesita de Android. Ninguna de estas funciones
/// decide si hay versión nueva o si es obligatoria: eso lo resuelve el
/// núcleo (`evaluarManifiestoActualizacion`). Interfaz para poder probar
/// [ActualizacionViewModel] sin Android.
interface FuenteActualizaciones {
    /// Vacía en los builds sin actualizador.
    val urlManifiesto: String
    fun versionCodeInstalada(): Long
    fun puedeInstalar(): Boolean

    /// Texto crudo del manifiesto. [IOException] ante cualquier falla de red.
    fun leerManifiesto(): String

    /// Baja el APK verificando su SHA-256 en el mismo recorrido.
    /// [IOException] ante fallas de red o disco; [ApkRechazado] si no coincide.
    fun descargarApk(manifiesto: ManifiestoActualizacion, alAvanzar: (descargados: Long) -> Unit): File

    /// [ApkRechazado] si no es el mismo paquete, la versión anunciada o la
    /// misma firma que la app instalada.
    fun verificarApk(apk: File, manifiesto: ManifiestoActualizacion)

    /// Entrega el APK al instalador del sistema. El resultado llega después
    /// por [ResultadosInstalacion]. [IOException] o [SecurityException] si
    /// Android no deja abrir la sesión.
    fun instalar(apk: File)
}

/// Implementación real sobre `HttpURLConnection` y `PackageInstaller`.
///
/// La seguridad de fondo la pone Android: sólo instala encima un APK firmado
/// con la misma llave y nunca una versión menor. Las verificaciones de acá
/// existen para fallar antes y con un mensaje claro.
///
/// Desde Android 12, si Lattis tiene el permiso de instalar apps y se está
/// actualizando a sí misma, el sistema no pide confirmación
/// (`USER_ACTION_NOT_REQUIRED`). Si igual la pide, [ReceptorInstalacion]
/// muestra el diálogo del sistema.
class ServicioActualizacionesAndroid(context: Context) : FuenteActualizaciones {
    private val app = context.applicationContext

    override val urlManifiesto: String = BuildConfig.URL_MANIFIESTO_ACTUALIZACION

    override fun versionCodeInstalada(): Long =
        PackageInfoCompat.getLongVersionCode(app.packageManager.getPackageInfo(app.packageName, 0))

    override fun puedeInstalar(): Boolean = app.packageManager.canRequestPackageInstalls()

    override fun leerManifiesto(): String {
        val conexion = abrir(urlManifiesto)
        try {
            exigirOk(conexion)
            return conexion.inputStream.bufferedReader().use { it.readText() }
        } finally {
            conexion.disconnect()
        }
    }

    /// Se escribe en un `.part` y se renombra sólo si la huella coincide:
    /// nunca queda en disco un APK a medias con el nombre final.
    override fun descargarApk(manifiesto: ManifiestoActualizacion, alAvanzar: (Long) -> Unit): File {
        val carpeta = File(app.cacheDir, CARPETA)
        carpeta.deleteRecursively()
        if (!carpeta.mkdirs()) throw IOException("no se pudo crear $carpeta")
        val parcial = File(carpeta, "lattis-${manifiesto.versionCode}.apk.part")
        val digest = MessageDigest.getInstance("SHA-256")
        val conexion = abrir(manifiesto.url)
        try {
            exigirOk(conexion)
            var descargados = 0L
            conexion.inputStream.use { entrada ->
                parcial.outputStream().use { salida ->
                    val bufer = ByteArray(64 * 1024)
                    while (true) {
                        val leidos = entrada.read(bufer)
                        if (leidos < 0) break
                        descargados += leidos
                        if (descargados > manifiesto.tamanoBytes) {
                            throw ApkRechazado("La descarga es más grande de lo anunciado.")
                        }
                        digest.update(bufer, 0, leidos)
                        salida.write(bufer, 0, leidos)
                        alAvanzar(descargados)
                    }
                }
            }
        } finally {
            conexion.disconnect()
        }
        val huella = digest.digest().joinToString("") { "%02x".format(it) }
        if (huella != manifiesto.sha256) {
            parcial.delete()
            throw ApkRechazado("La descarga llegó dañada. Intente de nuevo.")
        }
        val apk = File(carpeta, "lattis-${manifiesto.versionCode}.apk")
        if (!parcial.renameTo(apk)) throw IOException("no se pudo renombrar $parcial")
        return apk
    }

    override fun verificarApk(apk: File, manifiesto: ManifiestoActualizacion) {
        val pm = app.packageManager
        val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) PackageManager.GET_SIGNING_CERTIFICATES else 0
        val info = pm.getPackageArchiveInfo(apk.absolutePath, flags)
            ?: throw ApkRechazado("El archivo descargado no es una app válida.")
        if (info.packageName != app.packageName) throw ApkRechazado("El archivo descargado es de otra app.")
        if (PackageInfoCompat.getLongVersionCode(info) != manifiesto.versionCode) {
            throw ApkRechazado("El archivo descargado no es la versión anunciada.")
        }
        // Android 8 (minSdk 26) no expone la firma de un APK sin instalar; ahí
        // queda sólo la verificación del instalador, que igual la exige.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            val firmantes = info.signingInfo?.apkContentsSigners.orEmpty()
            val mismaFirma = firmantes.isNotEmpty() && firmantes.all {
                pm.hasSigningCertificate(app.packageName, it.toByteArray(), PackageManager.CERT_INPUT_RAW_X509)
            }
            if (!mismaFirma) throw ApkRechazado("El archivo descargado no está firmado por Lattis.")
        }
    }

    override fun instalar(apk: File) {
        val instalador = app.packageManager.packageInstaller
        val parametros = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL).apply {
            setAppPackageName(app.packageName)
            setSize(apk.length())
            setInstallReason(PackageManager.INSTALL_REASON_USER)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED)
            }
        }
        val idSesion = instalador.createSession(parametros)
        var entregada = false
        try {
            instalador.openSession(idSesion).use { sesion ->
                sesion.openWrite("lattis.apk", 0, apk.length()).use { salida ->
                    apk.inputStream().use { it.copyTo(salida) }
                    sesion.fsync(salida)
                }
                // Mutable a propósito: el instalador agrega al intent el
                // estado y, si hace falta, el intent de confirmación.
                val receptor = PendingIntent.getBroadcast(
                    app,
                    idSesion,
                    Intent(app, ReceptorInstalacion::class.java).setPackage(app.packageName),
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
                )
                sesion.commit(receptor.intentSender)
                entregada = true
            }
        } finally {
            // Una sesión que no llegó a entregarse queda ocupando espacio
            // hasta que Android la limpie; se descarta ya.
            if (!entregada) runCatching { instalador.abandonSession(idSesion) }
        }
    }

    private fun abrir(url: String): HttpURLConnection =
        (URI(url).toURL().openConnection() as HttpURLConnection).apply {
            connectTimeout = MS_CONEXION
            readTimeout = MS_LECTURA
            instanceFollowRedirects = false
            useCaches = false
        }

    private fun exigirOk(conexion: HttpURLConnection) {
        if (conexion.responseCode != HttpURLConnection.HTTP_OK) throw IOException("HTTP ${conexion.responseCode}")
    }

    companion object {
        private const val MS_CONEXION = 15_000
        private const val MS_LECTURA = 30_000
        private const val CARPETA = "actualizaciones"

        /// Pantalla de Android donde se permite que Lattis instale apps (se
        /// habilita una sola vez).
        fun intentPermiso(context: Context): Intent =
            Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${context.packageName}"))
    }
}

enum class ResultadoInstalacion { CANCELADA, RECHAZADA }

/// Puente entre [ReceptorInstalacion] (lo invoca Android) y
/// [ActualizacionViewModel]. Con éxito Android reemplaza la app y cierra el
/// proceso, así que sólo viajan los resultados que dejan la app abierta.
object ResultadosInstalacion {
    private val _resultados = MutableSharedFlow<ResultadoInstalacion>(extraBufferCapacity = 4)
    val resultados: SharedFlow<ResultadoInstalacion> = _resultados.asSharedFlow()

    internal fun publicar(resultado: ResultadoInstalacion) {
        _resultados.tryEmit(resultado)
    }
}

/// Recibe el resultado de la sesión de `PackageInstaller`. No exportado: sólo
/// lo dispara el `PendingIntent` propio que se entrega al instalador.
class ReceptorInstalacion : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        when (intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)) {
            PackageInstaller.STATUS_PENDING_USER_ACTION -> {
                val confirmar = IntentCompat.getParcelableExtra(intent, Intent.EXTRA_INTENT, Intent::class.java)
                if (confirmar == null) {
                    ResultadosInstalacion.publicar(ResultadoInstalacion.RECHAZADA)
                } else {
                    context.startActivity(confirmar.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                }
            }
            PackageInstaller.STATUS_SUCCESS -> Unit
            PackageInstaller.STATUS_FAILURE_ABORTED -> ResultadosInstalacion.publicar(ResultadoInstalacion.CANCELADA)
            else -> ResultadosInstalacion.publicar(ResultadoInstalacion.RECHAZADA)
        }
    }
}

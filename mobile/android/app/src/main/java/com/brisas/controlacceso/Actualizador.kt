package com.brisas.controlacceso

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInfo
import android.content.pm.PackageInstaller
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.SystemClock
import android.provider.Settings
import androidx.core.content.IntentCompat
import androidx.core.content.pm.PackageInfoCompat
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URI
import java.security.MessageDigest
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.json.JSONException
import org.json.JSONObject

sealed interface EstadoActualizacion {
    /// Sin novedad que mostrar: build sin actualizador, todavía sin revisar,
    /// al día, o el aviso se pospuso.
    data object SinAviso : EstadoActualizacion
    data class Disponible(val novedad: NovedadActualizacion.Disponible) : EstadoActualizacion
    data class FaltaPermiso(val novedad: NovedadActualizacion.Disponible) : EstadoActualizacion
    data class Descargando(val novedad: NovedadActualizacion.Disponible, val porcentaje: Int?) : EstadoActualizacion
    data class Instalando(val novedad: NovedadActualizacion.Disponible) : EstadoActualizacion
    data class Fallo(val novedad: NovedadActualizacion.Disponible, val mensaje: String) : EstadoActualizacion
}

/// Actualizador de la app fuera de cualquier tienda (ver
/// docs/actualizaciones-android.md): lee el manifiesto que publica cada
/// release, baja el APK, verifica su SHA-256, su paquete y su firma, y se lo
/// pasa al instalador del sistema con `PackageInstaller`.
///
/// La seguridad de fondo la pone Android: sólo instala encima un APK firmado
/// con la misma llave y nunca una versión menor. Las verificaciones de acá
/// existen para fallar antes y con un mensaje claro.
///
/// Desde Android 12, si Lattis tiene el permiso de instalar apps y se está
/// actualizando a sí misma, el sistema no pide confirmación
/// (`USER_ACTION_NOT_REQUIRED`). Si igual la pide, llega
/// `STATUS_PENDING_USER_ACTION` y se muestra el diálogo del sistema.
object Actualizador {
    /// Una revisión por hora como mucho: el manifiesto cambia una vez por
    /// release, no vale la pena consultarlo en cada vuelta a primer plano.
    private const val MS_ENTRE_REVISIONES = 60 * 60 * 1000L
    private const val MS_CONEXION = 15_000
    private const val MS_LECTURA = 30_000
    private const val CARPETA = "actualizaciones"

    private val alcance = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val _estado = MutableStateFlow<EstadoActualizacion>(EstadoActualizacion.SinAviso)
    val estado: StateFlow<EstadoActualizacion> = _estado.asStateFlow()

    @Volatile private var ultimaRevisionMs = 0L

    /// Vacía en los builds de prueba: sólo el release sabe dónde se publica.
    val habilitado: Boolean get() = BuildConfig.URL_MANIFIESTO_ACTUALIZACION.isNotEmpty()

    /// Consulta el manifiesto. Silenciosa ante cualquier falla de red: no
    /// tener señal en la portería no es algo que haya que avisar.
    fun revisar(context: Context, forzar: Boolean = false) {
        if (!habilitado || ocupado()) return
        val ahora = SystemClock.elapsedRealtime()
        if (!forzar && ultimaRevisionMs != 0L && ahora - ultimaRevisionMs < MS_ENTRE_REVISIONES) return
        ultimaRevisionMs = ahora
        val app = context.applicationContext
        alcance.launch {
            val manifiesto = runCatching { descargarManifiesto() }.getOrNull() ?: return@launch
            if (ocupado()) return@launch
            _estado.value = when (val novedad = ActualizacionCalculos.novedad(versionCodeInstalada(app), manifiesto)) {
                is NovedadActualizacion.Disponible -> EstadoActualizacion.Disponible(novedad)
                NovedadActualizacion.AlDia -> EstadoActualizacion.SinAviso
            }
        }
    }

    /// Oculta el aviso hasta la próxima revisión. Una actualización
    /// obligatoria no se puede posponer.
    fun posponer() {
        val actual = _estado.value
        if (actual is EstadoActualizacion.Disponible && !actual.novedad.obligatoria) {
            _estado.value = EstadoActualizacion.SinAviso
        }
    }

    /// Pantalla de Android donde se permite que Lattis instale apps (se
    /// habilita una sola vez).
    fun intentPermiso(context: Context): Intent =
        Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${context.packageName}"))

    fun actualizar(context: Context) {
        val novedad = novedadActual() ?: return
        if (ocupado()) return
        val app = context.applicationContext
        if (!app.packageManager.canRequestPackageInstalls()) {
            _estado.value = EstadoActualizacion.FaltaPermiso(novedad)
            return
        }
        _estado.value = EstadoActualizacion.Descargando(novedad, porcentaje = 0)
        alcance.launch {
            try {
                val apk = descargarApk(app, novedad)
                verificarApk(app, apk, novedad.manifiesto)
                _estado.value = EstadoActualizacion.Instalando(novedad)
                instalar(app, apk)
            } catch (e: FalloActualizacion) {
                _estado.value = EstadoActualizacion.Fallo(novedad, e.message ?: "No se pudo actualizar.")
            } catch (_: IOException) {
                _estado.value = EstadoActualizacion.Fallo(novedad, "No se pudo descargar la actualización. Revise la conexión.")
            } catch (_: SecurityException) {
                _estado.value = EstadoActualizacion.Fallo(novedad, "Android no permitió instalar la actualización.")
            } catch (_: RuntimeException) {
                // `PackageInstaller` puede fallar con IllegalStateException u
                // otras; sin este catch la excepción tumbaría la app entera.
                _estado.value = EstadoActualizacion.Fallo(novedad, "No se pudo instalar la actualización.")
            }
        }
    }

    /// Resultado que entrega el instalador del sistema (ver
    /// [ReceptorInstalacion]). Con éxito Android reemplaza la app y cierra el
    /// proceso, así que no hay nada más que hacer.
    internal fun resultadoInstalacion(context: Context, intent: Intent) {
        // La confirmación se muestra aunque el proceso se haya recreado y el
        // estado ya no recuerde la novedad: la sesión de instalación sigue viva.
        val novedad = novedadActual()
        when (intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)) {
            PackageInstaller.STATUS_PENDING_USER_ACTION -> {
                val confirmar = IntentCompat.getParcelableExtra(intent, Intent.EXTRA_INTENT, Intent::class.java)
                if (confirmar != null) {
                    context.startActivity(confirmar.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                } else if (novedad != null) {
                    _estado.value = EstadoActualizacion.Fallo(novedad, "Android no mostró la confirmación de la instalación.")
                }
            }
            PackageInstaller.STATUS_SUCCESS -> Unit
            PackageInstaller.STATUS_FAILURE_ABORTED ->
                novedad?.let { _estado.value = EstadoActualizacion.Disponible(it) }
            else -> novedad?.let { _estado.value = EstadoActualizacion.Fallo(it, "Android rechazó la actualización.") }
        }
    }

    private fun ocupado(): Boolean =
        _estado.value.let { it is EstadoActualizacion.Descargando || it is EstadoActualizacion.Instalando }

    private fun novedadActual(): NovedadActualizacion.Disponible? = when (val actual = _estado.value) {
        is EstadoActualizacion.Disponible -> actual.novedad
        is EstadoActualizacion.FaltaPermiso -> actual.novedad
        is EstadoActualizacion.Descargando -> actual.novedad
        is EstadoActualizacion.Instalando -> actual.novedad
        is EstadoActualizacion.Fallo -> actual.novedad
        EstadoActualizacion.SinAviso -> null
    }

    private fun versionCodeInstalada(context: Context): Long =
        PackageInfoCompat.getLongVersionCode(context.packageManager.getPackageInfo(context.packageName, 0))

    private fun abrir(url: String): HttpURLConnection =
        (URI(url).toURL().openConnection() as HttpURLConnection).apply {
            connectTimeout = MS_CONEXION
            readTimeout = MS_LECTURA
            instanceFollowRedirects = false
            useCaches = false
        }

    private fun descargarManifiesto(): ManifiestoActualizacion {
        val url = BuildConfig.URL_MANIFIESTO_ACTUALIZACION
        val conexion = abrir(url)
        try {
            if (conexion.responseCode != HttpURLConnection.HTTP_OK) throw IOException("HTTP ${conexion.responseCode}")
            val json = try {
                JSONObject(conexion.inputStream.bufferedReader().use { it.readText() })
            } catch (e: JSONException) {
                throw ManifiestoInvalido("JSON inválido: ${e.message}")
            }
            val manifiesto = ManifiestoActualizacion(
                esquema = json.optInt("esquema", -1),
                versionCode = json.optLong("version_code", -1),
                versionName = json.optString("version_name"),
                url = json.optString("url"),
                sha256 = json.optString("sha256"),
                tamanoBytes = json.optLong("tamano_bytes", -1),
                versionMinimaCode = json.optLong("version_minima_code", 0),
            )
            return ActualizacionCalculos.validar(manifiesto, url)
        } finally {
            conexion.disconnect()
        }
    }

    /// Baja el APK a la caché calculando el SHA-256 en el mismo recorrido. Se
    /// escribe en un `.part` y se renombra sólo si la huella coincide: nunca
    /// queda en disco un APK a medias con el nombre final.
    private fun descargarApk(context: Context, novedad: NovedadActualizacion.Disponible): File {
        val manifiesto = novedad.manifiesto
        val carpeta = File(context.cacheDir, CARPETA)
        carpeta.deleteRecursively()
        if (!carpeta.mkdirs()) throw IOException("no se pudo crear $carpeta")
        val parcial = File(carpeta, "lattis-${manifiesto.versionCode}.apk.part")
        val digest = MessageDigest.getInstance("SHA-256")
        val conexion = abrir(manifiesto.url)
        try {
            if (conexion.responseCode != HttpURLConnection.HTTP_OK) throw IOException("HTTP ${conexion.responseCode}")
            var descargados = 0L
            var ultimoPorcentaje: Int? = 0
            conexion.inputStream.use { entrada ->
                parcial.outputStream().use { salida ->
                    val bufer = ByteArray(64 * 1024)
                    while (true) {
                        val leidos = entrada.read(bufer)
                        if (leidos < 0) break
                        descargados += leidos
                        if (descargados > manifiesto.tamanoBytes) {
                            throw FalloActualizacion("La descarga es más grande de lo anunciado.")
                        }
                        digest.update(bufer, 0, leidos)
                        salida.write(bufer, 0, leidos)
                        val porcentaje = ActualizacionCalculos.porcentaje(descargados, manifiesto.tamanoBytes)
                        if (porcentaje != ultimoPorcentaje) {
                            ultimoPorcentaje = porcentaje
                            _estado.value = EstadoActualizacion.Descargando(novedad, porcentaje)
                        }
                    }
                }
            }
        } finally {
            conexion.disconnect()
        }
        if (ActualizacionCalculos.hexadecimal(digest.digest()) != manifiesto.sha256) {
            parcial.delete()
            throw FalloActualizacion("La descarga llegó dañada. Intente de nuevo.")
        }
        val apk = File(carpeta, "lattis-${manifiesto.versionCode}.apk")
        if (!parcial.renameTo(apk)) throw IOException("no se pudo renombrar $parcial")
        return apk
    }

    private fun verificarApk(context: Context, apk: File, manifiesto: ManifiestoActualizacion) {
        val pm = context.packageManager
        val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) PackageManager.GET_SIGNING_CERTIFICATES else 0
        val info: PackageInfo = pm.getPackageArchiveInfo(apk.absolutePath, flags)
            ?: throw FalloActualizacion("El archivo descargado no es una app válida.")
        if (info.packageName != context.packageName) throw FalloActualizacion("El archivo descargado es de otra app.")
        if (PackageInfoCompat.getLongVersionCode(info) != manifiesto.versionCode) {
            throw FalloActualizacion("El archivo descargado no es la versión anunciada.")
        }
        // Android 8 (minSdk 26) no expone la firma de un APK sin instalar; ahí
        // queda sólo la verificación del instalador, que igual la exige.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            val firmantes = info.signingInfo?.apkContentsSigners.orEmpty()
            val mismaFirma = firmantes.isNotEmpty() && firmantes.all {
                pm.hasSigningCertificate(context.packageName, it.toByteArray(), PackageManager.CERT_INPUT_RAW_X509)
            }
            if (!mismaFirma) throw FalloActualizacion("El archivo descargado no está firmado por Lattis.")
        }
    }

    private fun instalar(context: Context, apk: File) {
        val instalador = context.packageManager.packageInstaller
        val parametros = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL).apply {
            setAppPackageName(context.packageName)
            setSize(apk.length())
            setInstallReason(PackageManager.INSTALL_REASON_USER)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED)
            }
        }
        val idSesion = instalador.createSession(parametros)
        try {
            instalador.openSession(idSesion).use { sesion ->
                sesion.openWrite("lattis.apk", 0, apk.length()).use { salida ->
                    apk.inputStream().use { it.copyTo(salida) }
                    sesion.fsync(salida)
                }
                // Mutable a propósito: el instalador agrega al intent el
                // estado y, si hace falta, el intent de confirmación.
                val receptor = PendingIntent.getBroadcast(
                    context,
                    idSesion,
                    Intent(context, ReceptorInstalacion::class.java).setPackage(context.packageName),
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
                )
                sesion.commit(receptor.intentSender)
            }
        } catch (e: Exception) {
            runCatching { instalador.abandonSession(idSesion) }
            throw e
        }
    }

    private class FalloActualizacion(mensaje: String) : Exception(mensaje)
}

/// Recibe el resultado de la sesión de `PackageInstaller`. No exportado: sólo
/// lo dispara el `PendingIntent` propio que se entrega al instalador.
class ReceptorInstalacion : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        Actualizador.resultadoInstalacion(context, intent)
    }
}

package com.brisas.controlacceso

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.RingtoneManager
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.google.firebase.FirebaseApp
import com.google.firebase.messaging.FirebaseMessaging
import java.net.HttpURLConnection
import java.net.URL
import kotlin.coroutines.resume
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.Nucleo

/// Notificaciones push (Firebase Cloud Messaging): avisos que llegan aunque
/// la app esté cerrada. La conexión Realtime (NubeRealtime.kt) sólo vive con
/// la app en primer plano; esto cubre el resto del tiempo.
///
/// Sin `google-services.json` (CI, clones nuevos -- ver app/build.gradle.kts)
/// Firebase no se inicializa: [disponible] da `false` y todo lo de acá no
/// hace nada en vez de reventar.
object NotificacionesPush {
    const val CANAL_AVISOS = "avisos"
    const val CANAL_EMERGENCIAS = "emergencias"
    private val VIBRACION_EMERGENCIA = longArrayOf(0, 800, 300, 800, 300, 800)
    private const val ETIQUETA_LOG = "LattisPush"
    private const val PREFERENCIAS = "notificaciones_push"
    private const val CLAVE_TOKEN = "token"

    fun disponible(context: Context): Boolean = FirebaseApp.getApps(context).isNotEmpty()

    /// Idempotente: Android ignora la creación de un canal que ya existe.
    /// Se llama al arrancar el proceso (AplicacionControlAcceso) para que el
    /// canal exista antes del primer mensaje, incluso con la app cerrada.
    ///
    /// Sonido y vibración de un canal quedan fijos al crearlo (Android no deja
    /// cambiarlos después desde la app, sólo la persona en Ajustes): cambiar
    /// los de "emergencias" exige un id de canal nuevo.
    fun crearCanales(context: Context) {
        val avisos = NotificationChannel(CANAL_AVISOS, "Avisos", NotificationManager.IMPORTANCE_HIGH).apply {
            description = "Avisos del punto de acceso: equipo retirado, visitas y recordatorios"
        }
        // Emergencias: sonido de alarma (sale por el volumen de alarma, que
        // suele estar alto aunque el timbre esté bajo) y vibración larga.
        val emergencias = NotificationChannel(CANAL_EMERGENCIAS, "Emergencias", NotificationManager.IMPORTANCE_HIGH).apply {
            description = "Avisos urgentes enviados por la administración"
            enableVibration(true)
            vibrationPattern = VIBRACION_EMERGENCIA
            setSound(
                RingtoneManager.getDefaultUri(RingtoneManager.TYPE_ALARM),
                AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_ALARM)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build(),
            )
        }
        context.getSystemService(NotificationManager::class.java).createNotificationChannels(listOf(avisos, emergencias))
    }

    /// Android 13+ pide permiso explícito para mostrar notificaciones; antes
    /// de eso se conceden al instalar.
    fun permisoConcedido(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED

    fun mostrar(context: Context, aviso: AvisoPush) {
        if (!permisoConcedido(context)) return
        val abrirApp = PendingIntent.getActivity(
            context,
            0,
            Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val notificacion = NotificationCompat.Builder(context, if (aviso.emergente) CANAL_EMERGENCIAS else CANAL_AVISOS)
            .setSmallIcon(R.drawable.ic_notificacion)
            .setContentTitle(aviso.titulo)
            .setContentText(aviso.cuerpo)
            .setStyle(NotificationCompat.BigTextStyle().bigText(aviso.cuerpo))
            // Prioridad y categoría sólo cuentan antes de Android 8 (después
            // manda el canal), salvo la categoría ALARM, que además le indica
            // al sistema que es urgente (p. ej. para el modo No molestar).
            .setPriority(if (aviso.emergente) NotificationCompat.PRIORITY_MAX else NotificationCompat.PRIORITY_HIGH)
            .setCategory(if (aviso.emergente) NotificationCompat.CATEGORY_ALARM else NotificationCompat.CATEGORY_MESSAGE)
            .setAutoCancel(true)
            .setContentIntent(abrirApp)
            .build()
        try {
            // Id distinto por aviso: dos avisos seguidos no se pisan.
            NotificationManagerCompat.from(context).notify(System.currentTimeMillis().toInt(), notificacion)
        } catch (e: SecurityException) {
            // El permiso se revocó entre el chequeo y el `notify`.
            Log.w(ETIQUETA_LOG, "Sin permiso para notificar", e)
        }
    }

    /// Token de este teléfono en FCM -- la dirección a la que el servidor
    /// manda los avisos. `null` si Firebase no está disponible o falló.
    suspend fun tokenActual(context: Context): String? {
        if (!disponible(context)) return null
        return suspendCancellableCoroutine { continuacion ->
            FirebaseMessaging.getInstance().token.addOnCompleteListener { tarea ->
                val token = if (tarea.isSuccessful) tarea.result else null
                if (token != null) guardarToken(context, token)
                continuacion.resume(token)
            }
        }
    }

    /// FCM rota el token cada tanto (reinstalación, datos borrados, etc.):
    /// se guarda el último para registrarlo en Supabase al iniciar sesión.
    fun guardarToken(context: Context, token: String) {
        context.getSharedPreferences(PREFERENCIAS, Context.MODE_PRIVATE).edit().putString(CLAVE_TOKEN, token).apply()
        // Sólo en builds de prueba: el token permite mandar un mensaje de
        // prueba a este teléfono desde la consola de Firebase. En release no
        // se escribe en el log.
        if (BuildConfig.DEBUG || BuildConfig.AMBIENTE_STAGING) Log.i(ETIQUETA_LOG, "Token FCM: $token")
    }

    fun tokenGuardado(context: Context): String? =
        context.getSharedPreferences(PREFERENCIAS, Context.MODE_PRIVATE).getString(CLAVE_TOKEN, null)

    /// Registra el token de este teléfono en Supabase
    /// (`registrar_token_push`, migración 20261003150000_tokens_push), con el
    /// JWT del equipo que ya usa Realtime. Se llama al iniciar sesión: el
    /// token que FCM rota con la app cerrada (`onNewToken`) queda guardado y
    /// sube en el próximo inicio de sesión. Sin red o sin nube configurada
    /// no pasa nada: se reintenta la próxima vez.
    suspend fun registrarEnNube(context: Context, nucleo: Nucleo): Boolean = withContext(Dispatchers.IO) {
        val token = tokenGuardado(context) ?: return@withContext false
        try {
            val sesion = nucleo.sesionRealtimeNube()
            val conexion = URL(urlRegistroTokenPush(sesion.baseUrl)).openConnection() as HttpURLConnection
            try {
                conexion.requestMethod = "POST"
                conexion.connectTimeout = 15_000
                conexion.readTimeout = 15_000
                conexion.doOutput = true
                conexion.setRequestProperty("apikey", sesion.apikey)
                conexion.setRequestProperty("Authorization", "Bearer ${sesion.accessToken}")
                conexion.setRequestProperty("Content-Type", "application/json")
                conexion.outputStream.use { it.write(cuerpoRegistroTokenPush(token).toByteArray()) }
                val codigo = conexion.responseCode
                if (codigo !in 200..299) {
                    Log.w(ETIQUETA_LOG, "registrar_token_push respondió $codigo")
                }
                codigo in 200..299
            } finally {
                conexion.disconnect()
            }
        } catch (e: Exception) {
            Log.w(ETIQUETA_LOG, "No se pudo registrar el token push", e)
            false
        }
    }
}

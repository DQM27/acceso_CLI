package com.brisas.controlacceso

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.google.firebase.FirebaseApp
import com.google.firebase.messaging.FirebaseMessaging
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/// Notificaciones push (Firebase Cloud Messaging): avisos que llegan aunque
/// la app esté cerrada. La conexión Realtime (NubeRealtime.kt) sólo vive con
/// la app en primer plano; esto cubre el resto del tiempo.
///
/// Sin `google-services.json` (CI, clones nuevos -- ver app/build.gradle.kts)
/// Firebase no se inicializa: [disponible] da `false` y todo lo de acá no
/// hace nada en vez de reventar.
object NotificacionesPush {
    const val CANAL_AVISOS = "avisos"
    private const val ETIQUETA_LOG = "LattisPush"
    private const val PREFERENCIAS = "notificaciones_push"
    private const val CLAVE_TOKEN = "token"

    fun disponible(context: Context): Boolean = FirebaseApp.getApps(context).isNotEmpty()

    /// Idempotente: Android ignora la creación de un canal que ya existe.
    /// Se llama al arrancar el proceso (AplicacionControlAcceso) para que el
    /// canal exista antes del primer mensaje, incluso con la app cerrada.
    fun crearCanales(context: Context) {
        val canal = NotificationChannel(CANAL_AVISOS, "Avisos", NotificationManager.IMPORTANCE_HIGH).apply {
            description = "Avisos del punto de acceso: equipo retirado, visitas y recordatorios"
        }
        context.getSystemService(NotificationManager::class.java).createNotificationChannel(canal)
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
        val notificacion = NotificationCompat.Builder(context, CANAL_AVISOS)
            .setSmallIcon(R.drawable.ic_notificacion)
            .setContentTitle(aviso.titulo)
            .setContentText(aviso.cuerpo)
            .setStyle(NotificationCompat.BigTextStyle().bigText(aviso.cuerpo))
            .setPriority(NotificationCompat.PRIORITY_HIGH)
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
}

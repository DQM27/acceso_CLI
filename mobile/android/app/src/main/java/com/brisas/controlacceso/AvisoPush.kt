package com.brisas.controlacceso

/// Lo que se muestra en una notificación push. Kotlin puro, sin Android:
/// así se prueba en JVM (ver AvisoPushTest.kt) sin emulador.
data class AvisoPush(val titulo: String, val cuerpo: String)

/// Arma el aviso a partir de un mensaje de Firebase Cloud Messaging.
///
/// El servidor (Edge Function `enviar-push`) manda mensajes de **datos**
/// con `titulo` y `cuerpo` -- un mensaje de datos con prioridad alta llega a
/// `onMessageReceived` aunque la app esté cerrada, y la notificación la arma
/// esta app (canal, ícono, toque que abre la app). Un mensaje con bloque
/// `notification` (p. ej. una prueba desde la consola de Firebase) también
/// se acepta: con la app en primer plano Android no lo muestra solo, así que
/// se usa ese título/cuerpo como respaldo.
///
/// Devuelve `null` si no hay nada que mostrar (mensaje silencioso, p. ej.
/// sólo para despertar una sincronización).
fun avisoDeMensaje(
    datos: Map<String, String>,
    tituloNotificacion: String?,
    cuerpoNotificacion: String?,
): AvisoPush? {
    val titulo = datos["titulo"]?.takeIf { it.isNotBlank() } ?: tituloNotificacion?.takeIf { it.isNotBlank() }
    val cuerpo = datos["cuerpo"]?.takeIf { it.isNotBlank() } ?: cuerpoNotificacion?.takeIf { it.isNotBlank() }
    if (titulo == null && cuerpo == null) return null
    return AvisoPush(titulo = titulo ?: "Lattis", cuerpo = cuerpo.orEmpty())
}

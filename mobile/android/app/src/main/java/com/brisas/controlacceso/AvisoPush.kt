package com.brisas.controlacceso

/// Lo que se muestra en una notificación push. Kotlin puro, sin Android:
/// así se prueba en JVM (ver AvisoPushTest.kt) sin emulador.
///
/// `emergente`: va por el canal "Emergencias" (sonido de alarma, vibración
/// larga) en vez de "Avisos". Lo decide el servidor con `tipo = "emergente"`
/// (ver supabase/functions/_shared/fcm.ts).
data class AvisoPush(val titulo: String, val cuerpo: String, val emergente: Boolean = false)

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
    return AvisoPush(titulo = titulo ?: "Lattis", cuerpo = cuerpo.orEmpty(), emergente = datos["tipo"] == "emergente")
}

/// URL de la función `registrar_token_push` de Supabase (PostgREST RPC).
fun urlRegistroTokenPush(baseUrl: String): String = baseUrl.trimEnd('/') + "/rest/v1/rpc/registrar_token_push"

/// Cuerpo JSON para `registrar_token_push`. Los tokens de FCM sólo traen
/// letras, dígitos, `:`, `-` y `_`, pero igual se escapan comillas y barras
/// para no armar un JSON roto si algún día cambia el formato.
fun cuerpoRegistroTokenPush(token: String): String {
    val escapado = token.replace("\\", "\\\\").replace("\"", "\\\"")
    return "{\"p_token\":\"$escapado\"}"
}

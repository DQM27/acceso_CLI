package com.brisas.controlacceso

/// Registro del token de FCM de este teléfono en Supabase. Kotlin puro, sin
/// Android: así se prueba en JVM (ver RegistroTokenPushTest.kt) sin
/// emulador. Qué dice un aviso que llega lo decide el núcleo
/// (`avisoDeMensaje`, mobile/rust-core/src/avisos.rs).

/// URL de la función `registrar_token_push` de Supabase (PostgREST RPC).
fun urlRegistroTokenPush(baseUrl: String): String = baseUrl.trimEnd('/') + "/rest/v1/rpc/registrar_token_push"

/// Cuerpo JSON para `registrar_token_push`. Los tokens de FCM sólo traen
/// letras, dígitos, `:`, `-` y `_`, pero igual se escapan comillas y barras
/// para no armar un JSON roto si algún día cambia el formato.
fun cuerpoRegistroTokenPush(token: String): String {
    val escapado = token.replace("\\", "\\\\").replace("\"", "\\\"")
    return "{\"p_token\":\"$escapado\"}"
}

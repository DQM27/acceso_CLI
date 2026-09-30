package com.brisas.controlacceso

import java.math.BigInteger
import java.net.URI
import java.util.Base64

/// Código de vinculación que el panel genera para cada equipo (ver
/// `supabase/functions/_shared/dispositivos.ts`): 10 caracteres de un
/// alfabeto sin 0/O/1/I, que el panel muestra como `K7QM-R4XT-2P` y en un
/// QR. Mismo criterio que `CodigoVinculacion.logica.ts` en escritorio: acá
/// sólo se ayuda a escribirlo o leerlo, la validación real la hace el
/// servidor.
object CodigoVinculacion {
    const val LARGO = 10

    /// Esquema del QR del panel: `brisas-acceso://vincular?codigo=K7QMR4XT2P`.
    private const val ESQUEMA_QR = "brisas-acceso"
    private const val ACCION_QR = "vincular"

    /// Mayúsculas y sin separadores ni espacios, como lo compara el servidor.
    fun normalizar(entrada: String): String = entrada.uppercase().filter { it in 'A'..'Z' || it in '0'..'9' }

    /// Agrupado en 4-4-2 como en el panel, y cortado al largo del código.
    fun formatear(entrada: String): String =
        normalizar(entrada).take(LARGO).chunked(4).let { grupos ->
            // El último grupo del panel es de 2: 4-4-2.
            listOfNotNull(
                grupos.getOrNull(0),
                grupos.getOrNull(1),
                grupos.getOrNull(2),
            ).joinToString("-")
        }

    fun completo(entrada: String): Boolean = normalizar(entrada).length == LARGO

    /// El código que trae un QR del panel, o `null` si el QR no es de
    /// vinculación. Acepta también un QR con el código solo.
    fun desdeQr(contenido: String): String? {
        val texto = contenido.trim()
        val codigo = if (texto.startsWith("$ESQUEMA_QR:", ignoreCase = true)) {
            val uri = runCatching { URI(texto) }.getOrNull() ?: return null
            if (!uri.host.equals(ACCION_QR, ignoreCase = true)) return null
            uri.rawQuery
                ?.split('&')
                ?.map { it.split('=', limit = 2) }
                ?.firstOrNull { it.size == 2 && it[0] == "codigo" }
                ?.get(1)
                ?: return null
        } else {
            texto
        }
        return normalizar(codigo).takeIf { it.length == LARGO }
    }
}

/// Clave pública EC P-256 como JWK (`{"kty","crv","x","y"}`), que es lo
/// que espera el núcleo (`ClavePublicaJwk::desde_json`). Las coordenadas van
/// en base64url sin relleno y de 32 bytes exactos: `BigInteger.toByteArray`
/// puede dar 33 (byte de signo) o menos de 32 (ceros a la izquierda).
fun jwkP256(x: BigInteger, y: BigInteger): String {
    val codificador = Base64.getUrlEncoder().withoutPadding()
    fun coordenada(valor: BigInteger): String {
        val bytes = valor.toByteArray()
        val exactos = ByteArray(32)
        val copiar = minOf(bytes.size, 32)
        System.arraycopy(bytes, bytes.size - copiar, exactos, 32 - copiar, copiar)
        return codificador.encodeToString(exactos)
    }
    return """{"kty":"EC","crv":"P-256","x":"${coordenada(x)}","y":"${coordenada(y)}"}"""
}

package com.brisas.controlacceso

import java.net.URI

/// Lo que publica cada release en `android/latest.json` del almacenamiento de
/// descargas (Cloudflare R2, ver docs/actualizaciones-android.md). Lo arma el
/// paso "Publicar el APK en el almacenamiento de descargas" de `release.yml`.
data class ManifiestoActualizacion(
    val esquema: Int,
    val versionCode: Long,
    val versionName: String,
    val url: String,
    val sha256: String,
    val tamanoBytes: Long,
    /// Por debajo de este `versionCode` la actualización es obligatoria (la
    /// versión instalada ya no es compatible con la nube). 0 = nunca.
    val versionMinimaCode: Long,
)

sealed interface NovedadActualizacion {
    data object AlDia : NovedadActualizacion
    data class Disponible(val manifiesto: ManifiestoActualizacion, val obligatoria: Boolean) : NovedadActualizacion
}

class ManifiestoInvalido(motivo: String) : Exception(motivo)

/// Reglas del actualizador sin dependencias de Android, para probarlas en la
/// JVM. La lectura del JSON y la instalación viven en `Actualizador.kt`.
object ActualizacionCalculos {
    const val ESQUEMA_MANIFIESTO = 1

    /// Tope de tamaño del APK: el real ronda los 50 MB. Un manifiesto que
    /// anuncie más es un error de publicación, no una descarga a intentar.
    const val TAMANO_MAXIMO_BYTES = 300L * 1024 * 1024

    private val PATRON_SHA256 = Regex("^[0-9a-f]{64}$")

    /// Rechaza un manifiesto que no se pueda usar con seguridad. El APK sólo
    /// se baja por HTTPS y del mismo servidor que publicó el manifiesto: un
    /// manifiesto alterado no puede mandar al teléfono a bajar de otro lado.
    fun validar(manifiesto: ManifiestoActualizacion, urlManifiesto: String): ManifiestoActualizacion {
        if (manifiesto.esquema != ESQUEMA_MANIFIESTO) {
            throw ManifiestoInvalido("esquema ${manifiesto.esquema} no soportado")
        }
        if (manifiesto.versionCode <= 0) throw ManifiestoInvalido("versionCode inválido")
        if (manifiesto.versionName.isBlank()) throw ManifiestoInvalido("versionName vacío")
        if (!PATRON_SHA256.matches(manifiesto.sha256)) throw ManifiestoInvalido("sha256 inválido")
        if (manifiesto.tamanoBytes !in 1..TAMANO_MAXIMO_BYTES) throw ManifiestoInvalido("tamaño inválido")
        if (manifiesto.versionMinimaCode !in 0..manifiesto.versionCode) {
            throw ManifiestoInvalido("versionMinimaCode fuera de rango")
        }
        val apk = uriSegura(manifiesto.url) ?: throw ManifiestoInvalido("url del APK inválida")
        val origen = uriSegura(urlManifiesto) ?: throw ManifiestoInvalido("url del manifiesto inválida")
        if (apk.scheme != "https") throw ManifiestoInvalido("el APK no se sirve por HTTPS")
        if (!apk.host.equals(origen.host, ignoreCase = true)) {
            throw ManifiestoInvalido("el APK no está en el mismo servidor que el manifiesto")
        }
        return manifiesto
    }

    /// Sólo hay novedad si el manifiesto trae un `versionCode` mayor que el
    /// instalado: Android tampoco deja instalar una versión igual o menor.
    fun novedad(versionCodeInstalada: Long, manifiesto: ManifiestoActualizacion): NovedadActualizacion =
        if (manifiesto.versionCode > versionCodeInstalada) {
            NovedadActualizacion.Disponible(manifiesto, obligatoria = versionCodeInstalada < manifiesto.versionMinimaCode)
        } else {
            NovedadActualizacion.AlDia
        }

    fun hexadecimal(bytes: ByteArray): String = bytes.joinToString("") { "%02x".format(it) }

    /// Porcentaje entero de la descarga, o `null` si no se conoce el total.
    fun porcentaje(descargados: Long, total: Long): Int? =
        if (total <= 0) null else ((descargados * 100) / total).toInt().coerceIn(0, 100)

    private fun uriSegura(texto: String): URI? =
        runCatching { URI(texto) }.getOrNull()?.takeIf { !it.host.isNullOrBlank() && it.userInfo == null }
}

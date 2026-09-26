package com.brisas.controlacceso

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.io.FileOutputStream
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/// MV-03 (auditoría 2026-09-24): clave real de cifrado de
/// `control_acceso.db` (SQLite3MC) -- 32 bytes al azar, protegidos con
/// Android Keystore. Mismo patrón que [SecretoDispositivoStore] (dos
/// capas separadas: Keystore protege la CLAVE, SQLite3MC protege la
/// BASE -- copiar `control_acceso.db` y este archivo juntos a otro
/// dispositivo no sirve de nada, la clave del Keystore no viaja), pero
/// un archivo y un alias propios: mezclar esto con el secreto de
/// dispositivo (que además puede no existir todavía en la primera
/// activación) sería atar dos ciclos de vida que no tienen relación.
interface ClaveBaseDatosStore {
    /// Descifra la clave ya guardada, o genera una nueva (32 bytes de
    /// `SecureRandom`) y la guarda si es la primera vez que se llama --
    /// nunca deja abrir la base sin clave.
    fun obtenerOCrear(): ByteArray
}

class ClaveBaseDatosStoreException(causa: Throwable) :
    IllegalStateException("No se pudo acceder a la clave de la base de datos", causa)

class AndroidKeystoreClaveBaseDatosStore(context: Context) : ClaveBaseDatosStore {
    private val archivo = File(context.filesDir, ARCHIVO_CLAVE)
    private val keyStore = KeyStore.getInstance(PROVEEDOR).apply { load(null) }

    override fun obtenerOCrear(): ByteArray {
        try {
            cargar()?.let { return it }
            val nueva = ByteArray(LONGITUD_CLAVE).also { SecureRandom().nextBytes(it) }
            guardar(nueva)
            return nueva
        } catch (error: Exception) {
            throw ClaveBaseDatosStoreException(error)
        }
    }

    private fun guardar(clave: ByteArray) {
        val cipher = Cipher.getInstance(TRANSFORMACION)
        cipher.init(Cipher.ENCRYPT_MODE, obtenerClaveKeystore())
        val cifrado = cipher.doFinal(clave)
        val contenido = MAGIC + byteArrayOf(cipher.iv.size.toByte()) + cipher.iv + cifrado
        // Escritura atómica -- mismo criterio que
        // AndroidKeystoreSecretoDispositivoStore.guardar: un corte de luz o
        // un crash a mitad de escritura no debe dejar el archivo a medias
        // (eso sí sería perder la base para siempre, sin ninguna clave que
        // la abra).
        val temporal = File(archivo.parentFile, "${archivo.name}.tmp")
        try {
            FileOutputStream(temporal).use { salida ->
                salida.write(contenido)
                salida.flush()
                salida.fd.sync()
            }
            Files.move(
                temporal.toPath(),
                archivo.toPath(),
                StandardCopyOption.ATOMIC_MOVE,
                StandardCopyOption.REPLACE_EXISTING,
            )
        } finally {
            temporal.delete()
        }
    }

    private fun cargar(): ByteArray? {
        val contenido = archivo.takeIf { it.exists() }?.readBytes() ?: return null
        require(contenido.size > MAGIC.size) { "Archivo de clave incompleto" }
        require(contenido.copyOfRange(0, MAGIC.size).contentEquals(MAGIC)) {
            "Formato de clave desconocido"
        }
        val largoIv = contenido[MAGIC.size].toInt() and 0xff
        val inicioIv = MAGIC.size + 1
        val finIv = inicioIv + largoIv
        require(largoIv > 0 && contenido.size > finIv) { "Archivo de clave malformado" }
        val iv = contenido.copyOfRange(inicioIv, finIv)
        val cifrado = contenido.copyOfRange(finIv, contenido.size)

        val cipher = Cipher.getInstance(TRANSFORMACION)
        cipher.init(Cipher.DECRYPT_MODE, obtenerClaveKeystore(), GCMParameterSpec(TAG_BITS, iv))
        return cipher.doFinal(cifrado)
    }

    private fun obtenerClaveKeystore(): SecretKey {
        (keyStore.getEntry(ALIAS, null) as? KeyStore.SecretKeyEntry)?.let { return it.secretKey }

        val keyGenerator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVEEDOR)
        val spec = KeyGenParameterSpec.Builder(
            ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setRandomizedEncryptionRequired(true)
            .build()
        keyGenerator.init(spec)
        return keyGenerator.generateKey()
    }

    private companion object {
        const val PROVEEDOR = "AndroidKeyStore"
        const val ALIAS = "control_acceso.clave_base_datos.v1"
        const val TRANSFORMACION = "AES/GCM/NoPadding"
        const val TAG_BITS = 128
        const val LONGITUD_CLAVE = 32
        const val ARCHIVO_CLAVE = "base-datos.keystore"
        // "BAK2", distinto del "BAK1" (0x31) de SecretoDispositivoStore --
        // dos formatos de archivo independientes, aunque compartan el
        // mismo esquema Keystore por dentro.
        val MAGIC = byteArrayOf(0x42, 0x41, 0x4b, 0x32)
    }
}

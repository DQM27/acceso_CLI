package com.brisas.controlacceso

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import uniffi.control_acceso_mobile.Nucleo

interface SecretoDispositivoStore {
    fun guardar(secreto: String)
    fun cargar(): String?
    fun existe(): Boolean = cargar() != null
}

class SecretoDispositivoNoEncontradoException :
    IllegalStateException("Todavía no se guardó el secreto de este dispositivo")

class SecretoDispositivoStoreException(causa: Throwable) :
    IllegalStateException("No se pudo acceder al secreto seguro de este dispositivo", causa)

class AndroidKeystoreSecretoDispositivoStore(
    context: Context,
    private val nucleo: Nucleo,
    private val directorio: String,
    private val identificadorDispositivo: String,
) : SecretoDispositivoStore {
    private val archivo = File(context.filesDir, ARCHIVO_SECRETO_KEYSTORE)
    private val keyStore = KeyStore.getInstance(PROVEEDOR).apply { load(null) }

    override fun guardar(secreto: String) {
        try {
            val limpio = secreto.trim()
            if (limpio.isEmpty()) return

            val cipher = Cipher.getInstance(TRANSFORMACION)
            cipher.init(Cipher.ENCRYPT_MODE, obtenerClave())
            val cifrado = cipher.doFinal(limpio.toByteArray(Charsets.UTF_8))
            archivo.writeBytes(MAGIC + byteArrayOf(cipher.iv.size.toByte()) + cipher.iv + cifrado)
        } catch (error: Exception) {
            throw SecretoDispositivoStoreException(error)
        }
    }

    override fun cargar(): String? {
        try {
            cargarDesdeKeystore()?.let { return it }

            val legado = nucleo.cargarSecretoDispositivoLegado(directorio, identificadorDispositivo)
                ?: return null
            guardar(legado)
            return legado
        } catch (error: SecretoDispositivoStoreException) {
            throw error
        } catch (error: Exception) {
            throw SecretoDispositivoStoreException(error)
        }
    }

    private fun cargarDesdeKeystore(): String? {
        val contenido = archivo.takeIf { it.exists() }?.readBytes() ?: return null
        if (contenido.size <= MAGIC.size) return null
        if (!contenido.copyOfRange(0, MAGIC.size).contentEquals(MAGIC)) return null
        val largoIv = contenido[MAGIC.size].toInt() and 0xff
        val inicioIv = MAGIC.size + 1
        val finIv = inicioIv + largoIv
        if (largoIv <= 0 || contenido.size <= finIv) return null
        val iv = contenido.copyOfRange(inicioIv, finIv)
        val cifrado = contenido.copyOfRange(finIv, contenido.size)

        val cipher = Cipher.getInstance(TRANSFORMACION)
        cipher.init(Cipher.DECRYPT_MODE, obtenerClave(), GCMParameterSpec(TAG_BITS, iv))
        return String(cipher.doFinal(cifrado), Charsets.UTF_8).trim().ifBlank { null }
    }

    private fun obtenerClave(): SecretKey {
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
        const val ALIAS = "control_acceso.secreto_dispositivo.v1"
        const val TRANSFORMACION = "AES/GCM/NoPadding"
        const val TAG_BITS = 128
        const val ARCHIVO_SECRETO_KEYSTORE = "dispositivo-nube.keystore"
        val MAGIC = byteArrayOf(0x42, 0x41, 0x4b, 0x31)
    }
}

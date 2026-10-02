package com.brisas.controlacceso

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.Signature
import java.security.interfaces.ECPublicKey
import java.security.spec.ECGenParameterSpec
import uniffi.control_acceso_mobile.AlmacenClaveDispositivo
import uniffi.control_acceso_mobile.AlmacenClaveException

/// Identidad de este teléfono ante la nube (ver
/// `docs/features-futuras/propuesta-registro-dispositivos.md` y
/// `mobile/rust-core/src/firmante.rs`): un par EC P-256 generado DENTRO de
/// Android Keystore y marcado sólo para firmar. La clave privada nunca sale
/// del almacén seguro -- ni siquiera esta clase la ve: le pide a Keystore que
/// firme. El núcleo arma la aserción, calcula la huella y habla con el
/// servidor; acá sólo se genera, se entrega la pública y se firma.
///
/// El `dispositivo_id` vinculado vive en preferencias privadas de la app
/// (no es secreto: sólo indica a qué dispositivo del panel quedó atada la
/// clave). La app no hace backup (`allowBackup=false` y
/// `reglas_extraccion.xml`), y aun así se exige que la clave exista: unas
/// preferencias sin su clave no cuentan como vinculadas.
class AlmacenClaveKeystore(context: Context) : AlmacenClaveDispositivo {
    private val preferencias = context.getSharedPreferences(PREFERENCIAS, Context.MODE_PRIVATE)
    private val keyStore = KeyStore.getInstance(PROVEEDOR).apply { load(null) }

    override fun clavePublicaJwk(): String = conErrores {
        synchronized(this) {
            val publica = (entrada() ?: generar()).certificate.publicKey as ECPublicKey
            jwkP256(publica.w.affineX, publica.w.affineY)
        }
    }

    override fun firmarDer(datos: ByteArray): ByteArray = conErrores {
        val privada = entrada()?.privateKey ?: error("Este teléfono todavía no tiene clave")
        Signature.getInstance(ALGORITMO_FIRMA).run {
            initSign(privada)
            update(datos)
            sign()
        }
    }

    override fun regenerar() = conErrores {
        synchronized(this) {
            olvidarVinculacion()
            if (keyStore.containsAlias(ALIAS)) keyStore.deleteEntry(ALIAS)
            generar()
            Unit
        }
    }

    override fun dispositivoVinculado(): String? =
        preferencias.getString(CLAVE_DISPOSITIVO, null)?.takeIf { runCatching { entrada() != null }.getOrDefault(false) }

    override fun marcarVinculada(dispositivoId: String) = conErrores {
        check(preferencias.edit().putString(CLAVE_DISPOSITIVO, dispositivoId).commit()) {
            "No se pudo guardar la vinculación de este teléfono"
        }
    }

    private fun entrada(): KeyStore.PrivateKeyEntry? = keyStore.getEntry(ALIAS, null) as? KeyStore.PrivateKeyEntry

    private fun generar(): KeyStore.PrivateKeyEntry {
        // Una clave nueva nunca está vinculada, aunque quedara un id viejo.
        olvidarVinculacion()
        KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC, PROVEEDOR).apply {
            initialize(
                KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_SIGN)
                    .setAlgorithmParameterSpec(ECGenParameterSpec(CURVA))
                    .setDigests(KeyProperties.DIGEST_SHA256)
                    .build(),
            )
            generateKeyPair()
        }
        return entrada() ?: error("Android Keystore no guardó la clave generada")
    }

    private fun olvidarVinculacion() {
        preferencias.edit().remove(CLAVE_DISPOSITIVO).commit()
    }

    /// Cualquier falla del almacén cruza a Rust como `AlmacenClaveException`
    /// (el único tipo que el callback puede lanzar); el núcleo la convierte
    /// en un error de nube con mensaje para mostrar.
    private inline fun <T> conErrores(operacion: () -> T): T = try {
        operacion()
    } catch (excepcion: AlmacenClaveException) {
        throw excepcion
    } catch (excepcion: Exception) {
        throw AlmacenClaveException.Almacen(excepcion.message ?: excepcion.javaClass.simpleName)
    }

    private companion object {
        const val PROVEEDOR = "AndroidKeyStore"
        const val ALIAS = "control_acceso_identidad_dispositivo"
        const val CURVA = "secp256r1"
        const val ALGORITMO_FIRMA = "SHA256withECDSA"
        const val PREFERENCIAS = "identidad_dispositivo"
        const val CLAVE_DISPOSITIVO = "dispositivo_id"
    }
}

package com.brisas.controlacceso

import android.graphics.BitmapFactory
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/// Lee el texto de una imagen ya bajada (el gafete PRAIND del QR). En las
/// pruebas, un doble.
fun interface LectorTextoImagen {
    /// El texto leído, o `null` si la imagen no se pudo leer.
    suspend fun leer(imagen: ByteArray): String?
}

/// OCR de ML Kit (el mismo reconocedor de las cámaras) sobre la imagen del
/// gafete. Es el original en alta resolución (~2300 x 3600), no una foto: se
/// lee a la mitad, que sobra para el texto y pesa la cuarta parte en memoria.
class LectorTextoMlKit : LectorTextoImagen {
    override suspend fun leer(imagen: ByteArray): String? {
        val tamano = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(imagen, 0, imagen.size, tamano)
        val opciones = BitmapFactory.Options().apply {
            inSampleSize = if (maxOf(tamano.outWidth, tamano.outHeight) > 2500) 2 else 1
        }
        val bitmap = BitmapFactory.decodeByteArray(imagen, 0, imagen.size, opciones) ?: return null
        val cliente = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)
        return try {
            suspendCancellableCoroutine { continuacion ->
                cliente.process(InputImage.fromBitmap(bitmap, 0))
                    .addOnSuccessListener { resultado -> continuacion.resume(resultado.text) }
                    .addOnFailureListener { continuacion.resume(null) }
            }
        } finally {
            cliente.close()
            bitmap.recycle()
        }
    }
}

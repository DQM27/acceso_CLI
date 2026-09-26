package com.brisas.controlacceso

import android.content.Context
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.core.content.ContextCompat
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.TextRecognizer
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.util.concurrent.Executor
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.Job

/// MV-10 (auditoría 2026-09-24): las 4 pantallas de escaneo (Cédula/Gafete,
/// Carnet KOF, Vehículo/Ruta, Comprobante de Ruta) declaraban y liberaban
/// estos mismos 9 recursos por separado -- bloque idéntico byte a byte en
/// las 4, con riesgo de que un cambio futuro en cómo se libera la cámara
/// (o un bug de ciclo de vida) se arregle en una pantalla y se olvide en
/// las otras 3. Agrupa el ejecutor de análisis, el recognizer de ML Kit,
/// las dos banderas atómicas (`detectada`/`sesionActiva`, compartidas
/// entre el hilo del analizador y el principal, ver
/// `construirAnalizadorOcr`/`analizarCedula` en `PantallaEscanearCedula.kt`)
/// y el estado que va llenando `iniciarCamara` a medida que la cámara real
/// queda lista.
///
/// `mutableStateOf` (no propiedades simples) para `cameraProvider`/
/// `vistaPreviaCamara`/`analisisCamara`/`trabajoResultado` -- mismo motivo
/// que antes de este cambio: se asignan DESPUÉS de construir el estado,
/// dentro del `factory` de `AndroidView`, no en la creación.
class EstadoCamaraOcr(contexto: Context) {
    val ejecutor: ExecutorService = Executors.newSingleThreadExecutor()
    val recognizer: TextRecognizer = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)
    val ejecutorPrincipal: Executor = ContextCompat.getMainExecutor(contexto)
    val detectada = AtomicBoolean(false)
    val sesionActiva = AtomicBoolean(true)

    var cameraProvider: ProcessCameraProvider? by mutableStateOf(null)
    var vistaPreviaCamara: Preview? by mutableStateOf(null)
    var analisisCamara: ImageAnalysis? by mutableStateOf(null)
    var trabajoResultado: Job? by mutableStateOf(null)

    /// Mismo cierre que cada pantalla hacía a mano en su propio
    /// `onDispose` -- invalida callbacks de CameraX/ML Kit que terminen
    /// después de salir de la composición, desata la cámara del ciclo de
    /// vida anterior, y libera hilo/recognizer.
    fun liberar() {
        sesionActiva.set(false)
        detectada.set(true)
        trabajoResultado?.cancel()
        analisisCamara?.clearAnalyzer()
        val casos = listOfNotNull(vistaPreviaCamara, analisisCamara).toTypedArray()
        if (casos.isNotEmpty()) cameraProvider?.unbind(*casos)
        ejecutor.shutdown()
        recognizer.close()
    }
}

/// Crea un [EstadoCamaraOcr] atado al ciclo de vida de esta composición --
/// `liberar()` corre una sola vez, al salir, igual que el `onDispose` que
/// reemplaza en las 4 pantallas de escaneo.
@Composable
fun rememberEstadoCamaraOcr(contexto: Context): EstadoCamaraOcr {
    val estado = remember { EstadoCamaraOcr(contexto) }
    DisposableEffect(Unit) {
        estado.sesionActiva.set(true)
        onDispose { estado.liberar() }
    }
    return estado
}

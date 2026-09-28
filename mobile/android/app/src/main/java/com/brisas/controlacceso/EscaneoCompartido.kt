package com.brisas.controlacceso

import android.Manifest
import android.content.pm.PackageManager
import android.media.AudioManager
import android.media.ToneGenerator
import android.os.Handler
import android.os.Looper
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import uniffi.control_acceso_mobile.VotadorPorPosicion

/// Tag común para volcar por `Log.d` el texto crudo que entrega ML Kit en
/// cada frame (sólo en builds debug) -- reemplaza el overlay visible que
/// mostraban Comprobante de ruta y Carnet KOF (pedido explícito del usuario
/// 2026-09-20: "se ve horrible"). Sigue siendo la forma más rápida de
/// diagnosticar por qué un perfil no lee algo bien sin necesitar una foto
/// real del documento (que además sería un dato sensible) -- alcanza con
/// `adb logcat -s $TAG_DEBUG_OCR_LECTURA` mientras se reproduce la falla.
const val TAG_DEBUG_OCR_LECTURA = "OcrLectura"

/// Umbral para distinguir "todavía no hay nada en cuadro" (BUSCANDO, gris)
/// de "hay texto real pero no es el documento que se busca" (INVALIDO, rojo
/// + vibración de error) en las 3 pantallas de escaneo simple (Vehículo,
/// Comprobante, Carnet KOF) -- mismo criterio y mismo número que ya usaba
/// `EstabilizadorLectura` (identidad/gafete) para esa misma distinción.
/// Compartido acá (2026-09-20, pedido explícito del usuario: esas 3
/// pantallas no tenían el efecto de "mal encuadrado" que sí tiene
/// identidad/gafete) para que las 4 pantallas se sientan consistentes.
const val LARGO_MINIMO_TEXTO_INVALIDO = 10

/// Colores de estado del marco/mensaje en cualquiera de las 4 pantallas de
/// escaneo (identidad/gafete, vehículo de ruta, comprobante de ruta, carnet
/// KOF) -- antes cada una declaraba su propia copia de los mismos valores
/// hexadecimales. Fijos, no dependientes del tema (Classic/Brisas/Negro):
/// acá el color comunica significado (buscando/inválido/confirmado/vencido),
/// y ese significado debe leerse igual sin importar qué tema tenga activo
/// quien opera.
val ColorEscaneoBuscando = Color(0xFF9E9E9E)
val ColorEscaneoInvalido = Color(0xFFE53935)
val ColorEscaneoConfirmado = Color(0xFF43A047)
val ColorEscaneoVencido = Color(0xFFFF8F00)

/// Debounce genérico por repetición de frames: exige que el mismo candidato
/// (según `clave`) aparezca `framesRequeridos` veces dentro de la ventana de
/// los últimos frames -- no necesariamente consecutivos, tolera un frame
/// malo salteado por reflejo/ángulo. Reemplaza tres copias casi idénticas
/// que vivían una en cada pantalla de escaneo simple (vehículo de ruta,
/// comprobante de ruta, carnet KOF). `EstabilizadorLectura` (documentos de
/// identidad/gafete) se queda aparte porque además maneja MRZ, PDF417 y
/// el filtro de `TipoDocumento` por modo.
///
/// Con `desdeClave` (auditoría OCR 2026-09-28) en vez de exigir la misma
/// clave entera se VOTA carácter por carácter entre frames, con el mismo
/// criterio que `EstabilizadorLectura` (`VotadorPorPosicion`): sirve para
/// datos cortos sin dígito verificador como una placa, donde cada frame
/// puede errar en un carácter distinto. `desdeClave` reconstruye el valor a
/// partir de la clave ganadora; una lectura que difiere en demasiados
/// caracteres se trata como otro vehículo y empieza de cero.
///
/// Sincronizado: se alimenta desde el hilo del analizador.
class EstabilizadorPorRepeticion<T>(
    private val extraer: (String) -> T?,
    private val clave: (T) -> String,
    private val framesRequeridos: Int = 2,
    // Un poco más grande que `framesRequeridos` -- mismo motivo que en
    // `EstabilizadorLectura`: da lugar a tolerar algún frame malo salteado
    // sin exigir una ventana tan larga que acepte una racha ya abandonada.
    private val ventana: Int = framesRequeridos + 2,
    private val desdeClave: ((String) -> T?)? = null,
) {
    private val candidatosRecientes = ArrayDeque<String>()
    private val votador = desdeClave?.let { VotadorPorPosicion(ventana.toUInt()) }

    @Synchronized
    fun procesarFrame(texto: String, peso: Float = 1f): T? = procesarDetectado(extraer(texto), peso)

    /// Un frame con varias versiones de su texto (ver `LecturaFrame.textos`):
    /// cuenta la primera de la que se extrae algo.
    @Synchronized
    fun procesarTextos(textos: List<String>, peso: Float = 1f): T? =
        procesarDetectado(textos.firstNotNullOfOrNull(extraer), peso)

    private fun procesarDetectado(detectado: T?, peso: Float): T? {
        if (votador != null && desdeClave != null) return votar(votador, desdeClave, detectado, peso)
        val claveActual = detectado?.let(clave) ?: CLAVE_SIN_CANDIDATO
        candidatosRecientes.addLast(claveActual)
        while (candidatosRecientes.size > ventana) candidatosRecientes.removeFirst()
        if (detectado == null) return null
        val repeticiones = candidatosRecientes.count { it == claveActual }
        return if (repeticiones >= framesRequeridos) detectado else null
    }

    private fun votar(votador: VotadorPorPosicion, desdeClave: (String) -> T?, detectado: T?, peso: Float): T? {
        if (detectado == null) {
            votador.agregarVacio()
            return null
        }
        val claveActual = clave(detectado)
        val previo = votador.consenso()
        if (previo != null && esOtraLectura(previo.texto, claveActual)) votador.reiniciar()
        votador.agregar(claveActual, peso)
        val consenso = votador.consenso() ?: return null
        return if (consenso.alcanza(framesRequeridos)) desdeClave(consenso.texto) else null
    }

    companion object {
        private const val CLAVE_SIN_CANDIDATO = " "
    }
}

/// Debounce para la señal "hay texto real pero no es el documento que se
/// busca" (marco rojo + vibración, ver `LARGO_MINIMO_TEXTO_INVALIDO`).
/// Mismo espíritu que [EstabilizadorPorRepeticion] pero para la señal
/// NEGATIVA: exige que la mayoría de una ventana corta de frames confirme
/// el "no reconocido" antes de mostrarlo, en vez de marcarlo apenas UN
/// frame no matchea.
///
/// Necesario porque el clasificador de estos 3 perfiles (`esComprobanteCargaRuta`,
/// `esCarnetKof`) es un regex angosto sobre un documento completo (hoja
/// grande, mucho texto, ángulo/reflejo variable) -- a diferencia de la
/// clasificación de identidad (`clasificarTipoDocumento`), que evalúa
/// contra varios tipos de documento conocidos y casi siempre matchea alguno. Sin este debounce, un
/// solo frame con la palabra clave levemente mal leída (glare, se cortó
/// "Ruta" a la mitad) alcanzaba para poner rojo + vibrar con el documento
/// correcto todavía en cuadro -- hallazgo 2026-09-20, reportado como "se
/// pone muy exigente" contra el comprobante real.
class DetectorTextoNoReconocido(
    private val esTipoEsperado: (String) -> Boolean,
    private val framesRequeridos: Int = 3,
    private val ventana: Int = framesRequeridos + 2,
) {
    private val recientes = ArrayDeque<Boolean>()

    /// `true` sólo cuando la ventana confirma "no reconocido" -- frames sin
    /// texto sustancial (blanco/borroso, ver `LARGO_MINIMO_TEXTO_INVALIDO`)
    /// no cuentan ni a favor ni en contra, igual que un frame sin candidato
    /// no rompe una racha ya acumulada en `EstabilizadorPorRepeticion`.
    fun procesarFrame(texto: String): Boolean = procesarTextos(listOf(texto))

    /// Un frame con varias versiones de su texto: es el tipo esperado si
    /// alguna lo es.
    fun procesarTextos(textos: List<String>): Boolean {
        val sustanciales = textos.filter { it.trim().length >= LARGO_MINIMO_TEXTO_INVALIDO }
        if (sustanciales.isEmpty()) return recientes.count { it } >= framesRequeridos
        recientes.addLast(sustanciales.none(esTipoEsperado))
        while (recientes.size > ventana) recientes.removeFirst()
        return recientes.count { it } >= framesRequeridos
    }
}

/// Envoltorio compartido por las 4 pantallas de escaneo: pide permiso de
/// cámara si hace falta, maneja el botón atrás (`onCerrar`, no el default de
/// Android) y sólo muestra `contenido` (la cámara en sí) una vez concedido.
/// Antes esto era la mitad del código de cada `PantallaEscanear*.kt`,
/// repetido igual en las 4 -- sólo cambiaba el texto de `mensajePermiso`.
///
/// Hubo acá un `forzarHorizontal` que giraba la pantalla sola para el
/// comprobante de carga de ruta -- se sacó (pedido explícito del usuario
/// 2026-09-20: "quedó horrible", la cámara tampoco cerraba bien al
/// capturar). Se resuelve el mismo problema (documento más ancho que una
/// cédula) agrandando la región de recorte en vez de rotar nada -- ver
/// `RegionGuiaOcr.DOCUMENTO_ANCHO`.
@Composable
fun EscanerConPermisoCamara(
    mensajePermiso: String,
    onCerrar: () -> Unit,
    contenido: @Composable () -> Unit,
) {
    BackHandler(onBack = onCerrar)
    val contexto = LocalContext.current
    var permisoConcedido by remember {
        mutableStateOf(ContextCompat.checkSelfPermission(contexto, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED)
    }
    val pedirPermiso = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { concedido ->
        permisoConcedido = concedido
    }

    LaunchedEffect(Unit) {
        if (!permisoConcedido) {
            pedirPermiso.launch(Manifest.permission.CAMERA)
        }
    }

    if (permisoConcedido) {
        contenido()
    } else {
        Column(
            modifier = Modifier.fillMaxSize().padding(16.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(mensajePermiso, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Row(modifier = Modifier.padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                BotonDiscretoBrisas(onClick = onCerrar) { Text("Volver") }
                BotonBrisas(onClick = { pedirPermiso.launch(Manifest.permission.CAMERA) }) { Text("Dar permiso") }
            }
        }
    }
}

/// Sonido corto y discreto de confirmación, compartido por las 4 pantallas
/// de escaneo -- antes sólo lo tenía la de identidad/gafete (hallazgo
/// 2026-09-20: Vehículo, Comprobante y Carnet KOF sólo vibraban, sin
/// ningún sonido, así que con el celular en la mano sin sentir la
/// vibración -- o en un soporte -- no había forma de saber que algo se
/// leyó). Complementa la háptica, no la reemplaza (alguien con el celular
/// en silencio/vibrador no debería quedarse sin ninguna señal, y
/// viceversa). `TONE_PROP_ACK` es literalmente el tono que Android reserva
/// para "confirmación positiva", no un beep genérico. Se reproduce en
/// `STREAM_NOTIFICATION`: ese stream respeta el modo silencioso/No
/// molestar del sistema automáticamente, así que no hace falta consultar
/// `AudioManager.getRingerMode()` a mano. Volumen bajo (`MAX_VOLUME` es
/// 100) y duración corta a propósito: nada de un beep de escáner de
/// supermercado.
fun reproducirSonidoConfirmacion() {
    try {
        val generador = ToneGenerator(AudioManager.STREAM_NOTIFICATION, VOLUMEN_SONIDO_CONFIRMACION)
        generador.startTone(ToneGenerator.TONE_PROP_ACK, DURACION_SONIDO_CONFIRMACION_MS)
        // ToneGenerator reserva un recurso nativo de audio hasta `release()` --
        // sin esto se queda tomado el resto de la vida del proceso. El
        // delay deja que el tono realmente termine de sonar antes de soltarlo.
        Handler(Looper.getMainLooper()).postDelayed(generador::release, DURACION_SONIDO_CONFIRMACION_MS + 50L)
    } catch (e: RuntimeException) {
        // El constructor de ToneGenerator puede fallar si el dispositivo no
        // tiene el recurso de audio disponible en ese momento -- el sonido
        // es un complemento, nunca debe tumbar el flujo de escaneo por esto.
    }
}

private const val VOLUMEN_SONIDO_CONFIRMACION = 40 // sobre 100 -- sutil, no un beep de caja registradora
private const val DURACION_SONIDO_CONFIRMACION_MS = 100

/// `Vibrator`/`VibrationEffect` directo, no el haptic feedback semántico de
/// Compose -- pedido explícito del usuario 2026-09-20 ("aumentale un
/// poquito [la vibración de éxito]", "que vibre más fuerte" en un error).
/// `HapticFeedbackType` no deja elegir amplitud ni patrón, así que para
/// diferenciar éxito de error hace falta esto. Requiere el permiso VIBRATE
/// (normal, sin diálogo en tiempo de ejecución).
private fun obtenerVibrador(contexto: android.content.Context): android.os.Vibrator? =
    if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.S) {
        val gestor = contexto.getSystemService(android.content.Context.VIBRATOR_MANAGER_SERVICE)
            as? android.os.VibratorManager
        gestor?.defaultVibrator
    } else {
        @Suppress("DEPRECATION")
        contexto.getSystemService(android.content.Context.VIBRATOR_SERVICE) as? android.os.Vibrator
    }

/// Vibración corta de éxito -- amplitud media (no al máximo, sigue siendo
/// un "toque" de confirmación, no una alarma).
fun vibrarConfirmacion(contexto: android.content.Context) {
    val vibrador = obtenerVibrador(contexto) ?: return
    if (!vibrador.hasVibrator()) return
    try {
        vibrarComoAlarma(
            vibrador,
            android.os.VibrationEffect.createOneShot(DURACION_VIBRACION_EXITO_MS, AMPLITUD_VIBRACION_EXITO),
        )
    } catch (e: RuntimeException) {
        // Mismo criterio que `reproducirSonidoConfirmacion` -- la
        // vibración es un complemento, nunca debe tumbar el escaneo.
    }
}

/// Vibración de error -- dos golpes cortos a máxima amplitud en vez de un
/// solo toque, para que se sienta claramente distinta de la de éxito sin
/// necesidad de mirar la pantalla (documento no reconocido/no soportado,
/// o -- en el escaneo continuo de gafetes -- un gafete que ya tenía salida
/// registrada u otro error real de la mutación).
fun vibrarError(contexto: android.content.Context) {
    val vibrador = obtenerVibrador(contexto) ?: return
    if (!vibrador.hasVibrator()) return
    try {
        vibrarComoAlarma(
            vibrador,
            android.os.VibrationEffect.createWaveform(longArrayOf(0, 90, 70, 90), intArrayOf(0, 255, 0, 255), -1),
        )
    } catch (e: RuntimeException) {
        // Idem.
    }
}

/// Sin atributos, Android (13+) trata la vibración como de uso
/// "desconocido" y varias capas de la marca la filtran: en Samsung queda
/// atada a "Interacciones táctiles"/intensidad de respuesta táctil, y con
/// el modo silencio o "Vibrar al tocar" apagados no se siente nada -- de
/// ahí que "a veces vibra y a veces no" según el teléfono y su
/// configuración. Se marca como ALARMA: en un puesto de control la
/// confirmación/error del escaneo es un aviso operativo que tiene que
/// sentirse siempre, no una háptica decorativa de la interfaz.
private fun vibrarComoAlarma(vibrador: android.os.Vibrator, efecto: android.os.VibrationEffect) {
    if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
        vibrador.vibrate(
            efecto,
            android.os.VibrationAttributes.createForUsage(android.os.VibrationAttributes.USAGE_ALARM),
        )
    } else {
        @Suppress("DEPRECATION")
        vibrador.vibrate(
            efecto,
            android.media.AudioAttributes.Builder()
                .setUsage(android.media.AudioAttributes.USAGE_ALARM)
                .setContentType(android.media.AudioAttributes.CONTENT_TYPE_SONIFICATION)
                .build(),
        )
    }
}

// 40 ms era imperceptible en motores de vibración de gama baja/media
// (Samsung A, Honor): el motor ni llega a arrancar del todo.
private const val DURACION_VIBRACION_EXITO_MS = 80L
private const val AMPLITUD_VIBRACION_EXITO = 200 // sobre 255 -- un poco más fuerte que el default semántico

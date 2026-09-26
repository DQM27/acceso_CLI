package com.brisas.controlacceso

import android.util.Log
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.launch

/// Escaneo del carnet KOF del encargado de ruta (paso 1 del checklist, ver
/// `PantallaRutas.kt` y `LectorCarnetKof.kt`) -- mismo esqueleto que
/// [PantallaEscanearComprobanteRuta] (cámara genérica + estabilización +
/// perfil de OCR propio), pero sólo confirma con el **nombre** del frente:
/// el campo que llena es "Nombre del encargado" en texto libre, así que si
/// sólo se ve el reverso (código de empleado, sin nombre) el escaneo sigue
/// esperando en vez de confirmar un dato que no serviría para ese campo.
/// El código de empleado se captura cuando aparece en el mismo resultado,
/// pero no es requisito para cerrar el paso.
@Composable
fun PantallaEscanearCarnetKof(
    onCarnetDetectado: suspend (CarnetKofDetectado) -> Unit,
    onCerrar: () -> Unit,
) {
    EscanerConPermisoCamara(
        mensajePermiso = "Se necesita permiso de cámara para escanear el gafete.",
        onCerrar = onCerrar,
    ) {
        VistaCamaraCarnetKof(onCarnetDetectado = onCarnetDetectado, onCerrar = onCerrar)
    }
}

@Composable
private fun VistaCamaraCarnetKof(
    onCarnetDetectado: suspend (CarnetKofDetectado) -> Unit,
    onCerrar: () -> Unit,
) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val alcance = rememberCoroutineScope()
    val onDetectadoActual by rememberUpdatedState(onCarnetDetectado)
    var ultimoMensaje by remember { mutableStateOf(MENSAJE_INICIAL_KOF) }
    var estado by remember { mutableStateOf(EstadoEscaneo.BUSCANDO) }
    val estabilizador = remember {
        EstabilizadorPorRepeticion(
            extraer = { texto -> extraerCarnetKof(texto)?.takeIf { it.nombre != null } },
            clave = { "${it.nombre}:${it.codigoEmpleado}" },
        )
    }
    val detectorInvalido = remember { DetectorTextoNoReconocido(esTipoEsperado = ::esCarnetKof) }
    // MV-10 (auditoría 2026-09-24): agrupa lo que antes eran 9
    // declaraciones + un DisposableEffect idénticos a las otras 3
    // pantallas de escaneo -- ver EstadoCamaraOcr.kt.
    val camara = rememberEstadoCamaraOcr(contexto)

    val colorMarco = when (estado) {
        EstadoEscaneo.CONFIRMADO -> ColorEscaneoConfirmado
        EstadoEscaneo.INVALIDO -> ColorEscaneoInvalido
        EstadoEscaneo.BUSCANDO -> ColorEscaneoBuscando
    }

    Box(modifier = Modifier.fillMaxSize()) {
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx).apply { scaleType = PreviewView.ScaleType.FILL_CENTER }
                val analisis = construirAnalizadorOcr(
                    ejecutorAnalisis = camara.ejecutor,
                    detectada = camara.detectada,
                    sesionActiva = camara.sesionActiva,
                ) { imagen ->
                    analizarCedula(
                        imagen = imagen,
                        recognizer = camara.recognizer,
                        ejecutorPrincipal = camara.ejecutorPrincipal,
                        sesionActiva = camara.sesionActiva,
                        onTexto = { texto ->
                            if (camara.sesionActiva.get()) {
                                // Log, no overlay en pantalla -- pedido
                                // explícito del usuario 2026-09-20 (se
                                // veía mal encima de la cámara). Sigue
                                // disponible por `adb logcat` en un
                                // build debug si hace falta diagnosticar
                                // un perfil que no lee bien.
                                if (BuildConfig.DEBUG) Log.d(TAG_DEBUG_OCR_LECTURA, texto)
                                val resultado = estabilizador.procesarFrame(texto)
                                when {
                                    resultado != null -> {
                                        estado = EstadoEscaneo.CONFIRMADO
                                        ultimoMensaje = "Encargado ${resultado.nombre} confirmado"
                                        if (camara.detectada.compareAndSet(false, true)) {
                                            vibrarConfirmacion(contexto)
                                            reproducirSonidoConfirmacion()
                                            camara.trabajoResultado?.cancel()
                                            camara.trabajoResultado = alcance.launch {
                                                if (camara.sesionActiva.get()) onDetectadoActual(resultado)
                                            }
                                        }
                                    }
                                    // Mismo criterio que Comprobante de
                                    // Ruta -- ver `DetectorTextoNoReconocido`.
                                    // `esCarnetKof` ya excluye el
                                    // comprobante (comparte la marca "Coca
                                    // Cola FEMSA").
                                    detectorInvalido.procesarFrame(texto) -> {
                                        if (estado != EstadoEscaneo.INVALIDO) vibrarError(contexto)
                                        estado = EstadoEscaneo.INVALIDO
                                        ultimoMensaje = "Gafete no reconocido"
                                    }
                                    else -> {
                                        estado = EstadoEscaneo.BUSCANDO
                                        ultimoMensaje = MENSAJE_INICIAL_KOF
                                    }
                                }
                            }
                        },
                        onFallo = {
                            if (camara.sesionActiva.get()) ultimoMensaje = MENSAJE_FALLO_LECTURA_OCR
                        },
                    )
                }
                camara.analisisCamara = analisis
                iniciarCamara(
                    ctx = ctx,
                    previewView = previewView,
                    lifecycleOwner = lifecycleOwner,
                    analisis = analisis,
                    sesionActiva = camara.sesionActiva,
                    onCameraProviderListo = { proveedor, preview ->
                        camara.cameraProvider = proveedor
                        camara.vistaPreviaCamara = preview
                    },
                    onFallo = { mensaje -> if (camara.sesionActiva.get()) ultimoMensaje = mensaje },
                )
                previewView
            },
            modifier = Modifier.fillMaxSize(),
        )
        MarcoGuiaCedula(color = colorMarco, estado = estado, modifier = Modifier.fillMaxSize())
        Text(
            ultimoMensaje,
            color = Color.White,
            style = MaterialTheme.typography.bodyLarge,
            modifier = Modifier
                .fillMaxWidth()
                .align(Alignment.TopCenter)
                .padding(top = 16.dp, start = 16.dp, end = 72.dp)
                .background(Color.Black.copy(alpha = 0.78f), FormaCampoBrisas)
                .padding(12.dp),
        )
        // Mismo botón compartido que las otras 3 pantallas de escaneo
        // (`ControlesBrisas.kt`) -- antes era el texto "Cancelar" (hallazgo
        // 2026-09-19).
        BotonCerrarCamara(onClick = onCerrar, modifier = Modifier.align(Alignment.TopEnd).padding(16.dp))
    }
}

// "gafete KOF", no "carnet KOF" -- el resto de la app llama a este mismo
// documento "gafete KOF" (tab "KOF", encabezado "Encargado (gafete KOF)" en
// PantallaRutas.kt); este mensaje se había quedado con el nombre interno del
// documento en vez del que la persona ya conoce (hallazgo 2026-09-19).
private const val MENSAJE_INICIAL_KOF = "Apunte al frente del gafete KOF"

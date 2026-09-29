package com.brisas.controlacceso

import androidx.camera.core.ImageProxy
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
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
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch


@Composable
fun PantallaEscanearCedula(
    modo: ModoEscaneoDocumento = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
    continuo: Boolean = false,
    // Sólo lo usa el escaneo continuo de gafetes (`PantallaActivos`) -- las
    // otras 3 pantallas de escaneo se quedan con el default (sin tarjeta de
    // resultado) y no cambian en nada. Es una función, no un valor, para
    // leer siempre el estado más reciente del llamador (`ViewModel.mensaje`)
    // en el instante en que cada escaneo termina, sin depender de que
    // Compose ya haya recompuesto con el valor nuevo -- ver el doc-comment
    // en `VistaCamaraCedula` donde se llama.
    resultadoUltimoEscaneo: () -> Pair<String, Boolean>? = { null },
    onDocumentoDetectado: suspend (DocumentoDetectado) -> Unit,
    onCerrar: () -> Unit,
    // Buscar además el PDF417 de la cédula anterior. Sólo tiene sentido
    // donde se acepta una cédula (ingreso de contratistas, proveedores);
    // el alta de contratista sólo acepta el carnet PRAIND y lo apaga.
    lectorPdf417: Boolean = modo == ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
) {
    RegistrarPantalla("escanear_cedula")
    // Antes el mensaje de permiso era fijo ("...para escanear cédulas"),
    // sin importar el modo -- pedía cédulas incluso escaneando un gafete
    // (hallazgo 2026-09-20, al unificar las 4 pantallas de escaneo).
    val mensajePermiso = when (modo) {
        ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Se necesita permiso de cámara para escanear cédulas."
        ModoEscaneoDocumento.GAFETE_CONTRATISTA -> "Se necesita permiso de cámara para escanear gafetes."
    }
    EscanerConPermisoCamara(mensajePermiso = mensajePermiso, onCerrar = onCerrar) {
        VistaCamaraCedula(
            modo = modo,
            continuo = continuo,
            resultadoUltimoEscaneo = resultadoUltimoEscaneo,
            onDocumentoDetectado = onDocumentoDetectado,
            onCerrar = onCerrar,
            lectorPdf417 = lectorPdf417 && modo == ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
        )
    }
}

@Composable
private fun VistaCamaraCedula(
    modo: ModoEscaneoDocumento,
    continuo: Boolean,
    resultadoUltimoEscaneo: () -> Pair<String, Boolean>?,
    onDocumentoDetectado: suspend (DocumentoDetectado) -> Unit,
    onCerrar: () -> Unit,
    lectorPdf417: Boolean,
) {
    val contexto = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val alcance = rememberCoroutineScope()
    val onDocumentoActual by rememberUpdatedState(onDocumentoDetectado)
    val obtenerResultadoActual by rememberUpdatedState(resultadoUltimoEscaneo)
    // `Vibrator`/`VibrationEffect` directo (`vibrarConfirmacion`/
    // `vibrarError` en `EscaneoCompartido.kt`), no la háptica semántica de
    // Compose -- `HapticFeedbackType.Confirm` resultó no vibrar en un
    // Samsung real con "Interacciones táctiles" activado (hallazgo
    // 2026-09-20), y de cualquier forma esa API no deja elegir amplitud ni
    // patrón para poder distinguir éxito de error.
    // MV-10 (auditoría 2026-09-24): agrupa lo que antes eran 9
    // declaraciones + un DisposableEffect idénticos a las otras 3
    // pantallas de escaneo -- ver EstadoCamaraOcr.kt.
    // El PDF417 sólo existe en la cédula anterior: donde no se busca, ni
    // se carga el lector de códigos.
    val camara = rememberEstadoCamaraOcr(contexto, conLectorPdf417 = lectorPdf417)
    var ultimoMensaje by remember { mutableStateOf(mensajeInicialEscaneo(modo)) }
    var estado by remember { mutableStateOf(EstadoEscaneo.BUSCANDO) }
    var vencido by remember { mutableStateOf(false) }
    var progreso by remember { mutableStateOf(0f) }
    val encuadre = remember {
        ControladorEncuadre(
            regionHorizontal = RegionGuiaOcr.TARJETA_ID,
            regionVertical = RegionGuiaOcr.GAFETE_VERTICAL,
            orientacionDelTexto = ::orientacionDeTextoDocumento,
        )
    }
    var orientacionEncuadre by remember { mutableStateOf(encuadre.orientacion) }
    // Resultado real de la última mutación (nombre en éxito, motivo en
    // fallo), no sólo "se leyó el gafete" -- ver `resultadoUltimoEscaneo`.
    // `null` mientras no hay nada que mostrar todavía o el llamador no usa
    // esta función (las otras 3 pantallas de escaneo se quedan con el
    // mensaje de siempre).
    var resultadoMostrado by remember { mutableStateOf<Pair<String, Boolean>?>(null) }
    // Una instancia por apertura de pantalla -- lleva el conteo de frames
    // consistentes del debounce (ver EstabilizadorLectura), no debe
    // compartirse entre sesiones de escaneo distintas.
    val estabilizador = remember(modo) { EstabilizadorLectura(modo = modo) }
    // Acota el recorte a la banda del MRZ cuando ya se sabe dónde está, y
    // reparte los frames entre el lector de texto y el de PDF417 (ver
    // `LecturaFrame.kt`). Ambos los usa sólo el hilo del analizador.
    val seguidorMrz = remember { SeguidorBandaMrz() }
    val planificador = remember(lectorPdf417) { PlanificadorLectores(habilitado = lectorPdf417) }
    // Aviso mientras dura el "modo código" (ver `PlanificadorLectores`).
    var modoCodigoVisible by remember { mutableStateOf(false) }
    // Modo código: el análisis pasa a la mayor resolución mientras dura
    // (las barras del PDF417 de la cédula anterior no alcanzan a verse a
    // 1080p) y vuelve al normal al terminar. Corre en el hilo principal.
    LaunchedEffect(modoCodigoVisible) {
        camara.usarAnalisisDeCodigo(modoCodigoVisible)
    }
    var ultimoValorContinuo by remember { mutableStateOf<String?>(null) }
    var framesSinUltimoValor by remember { mutableStateOf(0) }
    // Colores de estado compartidos por las 4 pantallas de escaneo (ver
    // `EscaneoCompartido.kt`) -- fijos, no dependientes del tema
    // (Classic/Brisas/Negro): acá el color comunica significado
    // (buscando/inválido/confirmado/vencido), y ese significado debe leerse
    // igual sin importar qué tema tenga activo quien opera.
    val colorMarco = when {
        estado == EstadoEscaneo.CONFIRMADO && vencido -> ColorEscaneoVencido
        estado == EstadoEscaneo.CONFIRMADO -> ColorEscaneoConfirmado
        estado == EstadoEscaneo.INVALIDO -> ColorEscaneoInvalido
        else -> ColorEscaneoBuscando
    }

    Box(modifier = Modifier.fillMaxSize()) {
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx).apply {
                    scaleType = PreviewView.ScaleType.FILL_CENTER
                }
                val onResultado: (ResultadoEstabilizacion) -> Unit = onResultado@{ resultado ->
                        if (!camara.sesionActiva.get()) return@onResultado
                        // Un frame que ya estaba en proceso cuando se
                        // confirmó no debe volver a pintar "buscando"
                        // encima del resultado.
                        if (camara.detectada.get() && resultado.estado != EstadoEscaneo.CONFIRMADO) return@onResultado
                        if (continuo && resultado.estado != EstadoEscaneo.CONFIRMADO) {
                            framesSinUltimoValor++
                            if (framesSinUltimoValor >= FRAMES_AUSENCIA_PARA_REPETIR) {
                                ultimoValorContinuo = null
                                framesSinUltimoValor = 0
                            }
                        }
                        // Vibración de error al ENTRAR a inválido, no en
                        // cada frame que se queda ahí -- pedido explícito
                        // del usuario 2026-09-20 ("si el documento no es
                        // correcto que vibre más"), sin repetir el golpe
                        // mientras la persona sigue apuntando mal.
                        if (resultado.estado == EstadoEscaneo.INVALIDO && estado != EstadoEscaneo.INVALIDO) {
                            vibrarError(contexto)
                        }
                        if (resultado.estado == EstadoEscaneo.CONFIRMADO) {
                            planificador.terminarModoCodigo()
                            modoCodigoVisible = false
                        }
                        estado = resultado.estado
                        ultimoMensaje = resultado.mensaje
                        vencido = resultado.vencido
                        progreso = if (resultado.estado == EstadoEscaneo.CONFIRMADO) 1f else resultado.progreso
                        val documento = resultado.documento
                        if (resultado.estado == EstadoEscaneo.CONFIRMADO && documento != null) {
                            if (camara.detectada.compareAndSet(false, true)) {
                                camara.metricas.registrarConfirmacion()
                                val valor = documento.textoBusqueda ?: documento.numeroDocumento
                                val repetidoContinuo = continuo &&
                                    valor == ultimoValorContinuo
                                if (repetidoContinuo) {
                                    camara.detectada.set(false)
                                    estabilizador.reiniciar()
                                } else {
                                    ultimoValorContinuo = valor
                                    framesSinUltimoValor = 0
                                    // En modo continuo el aviso espera al
                                    // resultado real de la mutación (ver más
                                    // abajo) -- pedido explícito del usuario
                                    // 2026-09-20: escanear un gafete que ya
                                    // tenía salida registrada vibraba/sonaba
                                    // exactamente igual que uno que sí salió,
                                    // porque esto disparaba apenas se leía el
                                    // texto, antes de saber si la salida se
                                    // pudo registrar. Sin modo continuo no hay
                                    // "resultado" que esperar -- el llamador
                                    // decide qué hacer con el documento
                                    // después, fuera de esta pantalla.
                                    if (!continuo) {
                                        vibrarConfirmacion(contexto)
                                        reproducirSonidoConfirmacion()
                                    }
                                    camara.trabajoResultado?.cancel()
                                    camara.trabajoResultado = alcance.launch {
                                        // Antes acá se esperaba
                                        // DEMORA_AVISO_VENCIDO_MS si el
                                        // documento estaba vencido, para
                                        // que el operador alcanzara a leer
                                        // el aviso antes de seguir -- se
                                        // saca (pedido explícito del
                                        // usuario, 2026-09-26): la pausa no
                                        // bloqueaba nada de verdad, el
                                        // ingreso se otorgaba igual apenas
                                        // pasaba el segundo. Un mecanismo
                                        // que sí bloquee de verdad (no sólo
                                        // avisar y dejar pasar) queda para
                                        // otra pasada.
                                        if (!camara.sesionActiva.get()) return@launch
                                        // `onDocumentoActual` es suspend: para el
                                        // caso de gafetes ya espera a que la
                                        // mutación en Rust termine (ver
                                        // `ActivosViewModel.registrarSalidaPorGafeteEscaneado`),
                                        // así que al volver de acá el resultado
                                        // que devuelve `obtenerResultadoActual`
                                        // ya es el de ESTE escaneo, no el
                                        // anterior -- se lee directo (una
                                        // función, no un valor recompuesto) para
                                        // no depender de que Compose ya haya
                                        // vuelto a dibujar con el estado nuevo.
                                        onDocumentoActual(documento)
                                        if (continuo && camara.sesionActiva.get()) {
                                            val resultado = obtenerResultadoActual()
                                            if (resultado != null) {
                                                resultadoMostrado = resultado
                                                if (resultado.second) {
                                                    vibrarError(contexto)
                                                } else {
                                                    vibrarConfirmacion(contexto)
                                                    reproducirSonidoConfirmacion()
                                                }
                                            } else {
                                                ultimoMensaje = mensajeProcesadoContinuo(modo, valor)
                                            }
                                            delay(DEMORA_REARMAR_ESCANEO_CONTINUO_MS)
                                            if (camara.sesionActiva.get()) {
                                                camara.detectada.set(false)
                                                estabilizador.reiniciar()
                                                estado = EstadoEscaneo.BUSCANDO
                                                vencido = false
                                                ultimoMensaje = mensajeInicialEscaneo(modo)
                                                resultadoMostrado = null
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                val onFallo: () -> Unit = {
                    if (camara.sesionActiva.get()) {
                        estado = EstadoEscaneo.BUSCANDO
                        vencido = false
                        ultimoMensaje = MENSAJE_FALLO_LECTURA_OCR
                        modoCodigoVisible = planificador.modoCodigo
                    }
                }
                // Encuadre que gira según el documento (ver
                // `ControladorEncuadre`): horizontal para cédula/licencia/
                // DIMEX, vertical para PRAIND, In House y gafete CRC. Todo lo
                // de `onLectura` corre en el hilo del analizador; a la
                // pantalla sólo se publica el resultado.
                val analizarFrame: (ImageProxy) -> Unit = { imagen ->
                    val (regionBase, leidoCon) = encuadre.regionParaFrame()
                    val region = if (leidoCon == OrientacionEncuadre.HORIZONTAL) seguidorMrz.region(regionBase) else regionBase
                    val planCodigo = planificador.planCodigo()
                    analizarFrameOcr(
                        imagen = imagen,
                        camara = camara,
                        region = region,
                        // Texto y PDF417 en paralelo; en modo código el
                        // PDF417 lee todo lo visible, no el recuadro.
                        leerCodigo = planCodigo.leer,
                        modoCodigo = planCodigo.modoCodigo,
                        onLectura = { lectura ->
                            val datosPdf417 = lectura.pdf417
                            if (datosPdf417 != null) {
                                // Corrección de errores propia del código:
                                // gana sobre el texto del mismo frame.
                                val resultado = estabilizador.procesarPdf417(datosPdf417)
                                camara.metricas.registrarResultadoDocumento(resultado)
                                camara.enPrincipal { onResultado(resultado) }
                            } else {
                                val resultado = estabilizador.procesarTextos(lectura.textos, lectura.peso, lectura.calidad)
                                camara.metricas.registrarResultadoDocumento(resultado)
                                if (planificador.registrarTexto(lectura.textos.any(::pareceReversoCedulaAnterior), resultado.hayMrz)) {
                                    camara.metricas.registrarActivacionModoCodigo()
                                }
                                val enModoCodigo = planificador.modoCodigo
                                seguidorMrz.registrar(lectura.regionLeida, lectura.lineasMrz)
                                val giro = encuadre.registrarOrientacion(resultado.orientacionSugerida, leidoCon)
                                camara.enPrincipal {
                                    if (giro) orientacionEncuadre = encuadre.orientacion
                                    modoCodigoVisible = enModoCodigo
                                    onResultado(resultado)
                                }
                            }
                        },
                        onFallo = { camara.enPrincipal(onFallo) },
                    )
                }
                val analisis = construirAnalizadorOcr(
                    ejecutorAnalisis = camara.ejecutor,
                    detectada = camara.detectada,
                    sesionActiva = camara.sesionActiva,
                    hayLugar = camara::hayLugar,
                    onFrameActivo = analizarFrame,
                )
                camara.analisisCamara = analisis
                // El mismo análisis a la mayor resolución, para el modo código
                // (ver el `LaunchedEffect` de `modoCodigoVisible`).
                if (camara.lectorCodigos != null) {
                    camara.analisisCodigo = construirAnalizadorOcr(
                        ejecutorAnalisis = camara.ejecutor,
                        detectada = camara.detectada,
                        sesionActiva = camara.sesionActiva,
                        hayLugar = camara::hayLugar,
                        resolucion = RESOLUCION_ANALISIS_CODIGO,
                        onFrameActivo = analizarFrame,
                    )
                }
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
                    onCamaraLista = { camara.camaraFisica = it },
                    onEnlazado = camara::alEnlazar,
                    onFallo = { mensaje ->
                        if (camara.sesionActiva.get()) {
                            estado = EstadoEscaneo.INVALIDO
                            ultimoMensaje = mensaje
                        }
                    },
                )
                previewView
            },
            modifier = Modifier.fillMaxSize(),
        )
        MarcoGuiaCedula(
            color = colorMarco,
            estado = estado,
            progreso = progreso,
            region = regionAnimada(encuadre.region(orientacionEncuadre)),
            modifier = Modifier.fillMaxSize(),
        )
        // Con resultado real (éxito/fallo de la mutación, no sólo "se leyó
        // el texto"), el mismo mensaje se pinta verde/rojo en vez de negro
        // neutro -- pedido explícito del usuario 2026-09-20: antes, en
        // escaneo continuo, el único aviso de que algo salió mal era el
        // recuadro cambiando de color, fácil de no notar mientras se sigue
        // apuntando la cámara al siguiente gafete.
        val resultado = resultadoMostrado
        val mensajeVisible = when {
            resultado != null -> resultado.first
            modoCodigoVisible && estado != EstadoEscaneo.CONFIRMADO -> MENSAJE_MODO_CODIGO
            else -> ultimoMensaje
        }
        Text(
            mensajeVisible,
            color = Color.White,
            style = MaterialTheme.typography.bodyLarge,
            modifier = Modifier
                .fillMaxWidth()
                .align(Alignment.TopCenter)
                .padding(top = 16.dp, start = 16.dp, end = 72.dp)
                .background(
                    when {
                        resultado == null -> Color.Black.copy(alpha = 0.78f)
                        resultado.second -> ColorEscaneoInvalido.copy(alpha = 0.85f)
                        else -> ColorEscaneoConfirmado.copy(alpha = 0.85f)
                    },
                    FormaCampoBrisas,
                )
                .padding(12.dp),
        )
        // Círculo flotante en vez del texto "Cancelar" que vivía debajo del
        // mensaje -- pedido explícito del usuario (2026-09-19): la posición
        // esquina-superior es la convención de cualquier pantalla de cámara
        // (cerrar sin competir visualmente con el mensaje de estado, y sin
        // el `padding(end = 72.dp)` de arriba el mensaje se le metía debajo).
        // Compartido (`ControlesBrisas.kt`) -- mismo botón en las 4 pantallas
        // de escaneo (Cédula, Carnet KOF, Vehículo/Ruta, Comprobante).
        BotonCerrarCamara(onClick = onCerrar, modifier = Modifier.align(Alignment.TopEnd).padding(16.dp))
        if (camara.tieneLinterna) {
            BotonLinterna(
                encendida = camara.linternaEncendida,
                onClick = camara::alternarLinterna,
                modifier = Modifier.align(Alignment.TopEnd).padding(top = 72.dp, end = 16.dp),
            )
        }
    }
}


// Subido de 900ms -- con el ciclo de salida por gafete ya sin botón de
// confirmar (pedido explícito del usuario 2026-09-20), el mensaje de
// resultado (nombre en verde, motivo en rojo) apenas alcanzaba a leerse
// antes de que la cámara se rearmara para el siguiente. Sigue siendo
// bastante más rápido que tener que confirmar a mano.
private const val DEMORA_REARMAR_ESCANEO_CONTINUO_MS = 1600L
private const val FRAMES_AUSENCIA_PARA_REPETIR = 3

// Mientras dura el modo código (ver `PlanificadorLectores`): el PDF417 de la
// cédula anterior necesita muchos más píxeles que el recuadro guía. De lado
// queda a lo largo de los 1920 px del frame, la mayor ganancia.
private const val MENSAJE_MODO_CODIGO = "Acerque el código de barras hasta que llene la pantalla (de lado funciona mejor)"

// El reverso (con el MRZ -- las líneas de texto tipo código de barras) trae
// nombre Y cédula en un solo escaneo con checksum verificado; el frente
// (con la foto) sólo trae el número. Guiar hacia el reverso desde el
// mensaje inicial evita que quien opera tenga que enterarse por su cuenta
// (ver el aviso en `EstabilizadorLectura.mensajeDeConfirmacion` para cuando
// igual termina mostrando el frente).
private fun mensajeInicialEscaneo(modo: ModoEscaneoDocumento): String =
    when (modo) {
        ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Coloque el documento dentro del recuadro"
        ModoEscaneoDocumento.GAFETE_CONTRATISTA -> "Coloque el gafete dentro del recuadro"
    }

private fun mensajeProcesadoContinuo(modo: ModoEscaneoDocumento, valor: String): String =
    when (modo) {
        ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Documento $valor procesado"
        ModoEscaneoDocumento.GAFETE_CONTRATISTA -> "Gafete $valor procesado"
    }

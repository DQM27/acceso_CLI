package com.brisas.controlacceso

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue

/// Orientación del recuadro guía de una pantalla de cámara.
typealias OrientacionEncuadre = uniffi.control_acceso_mobile.OrientacionEncuadre

/// Qué orientación pide cada tipo de documento: el carnet PRAIND, el gafete
/// In House y el gafete CRC se sostienen VERTICALES; el resto son tarjetas
/// horizontales. `null` para lo que no se reconoce.
fun TipoDocumento.orientacionEncuadre(): OrientacionEncuadre? =
    uniffi.control_acceso_mobile.orientacionEncuadreDeTipo(this)

/// Mecanismo GENÉRICO de encuadre que se adapta al documento, para que cada
/// pantalla de cámara lo reuse con su propio comportamiento: cada una elige
/// sus dos regiones, cuál es la de arranque y cómo deduce la orientación
/// del texto leído (`orientacionDelTexto`). No sabe nada de cédulas ni de
/// ML Kit -- es una máquina de estados pura, probada en
/// `EncuadreAdaptativoTest`.
///
/// Reglas:
/// 1. Arranca en `inicial` (lo que ve quien opera).
/// 2. Si el texto leído pide la OTRA orientación en [framesParaGirar]
///    frames seguidos, el recuadro gira (se dibuja y se recorta así).
///    Ej.: se lee "No. de cédula:" + "inducción" (PRAIND) con el recuadro
///    horizontal -> gira a vertical.
/// 3. Sonda: con la orientación inicial, un frame que no reconoce NADA
///    hace que el siguiente se lea con la otra región SIN dibujarla -- un
///    gafete vertical puede no reconocerse nunca con el recuadro horizontal
///    (su identificación queda fuera del recorte). Si la sonda reconoce un
///    documento de esa orientación, cuenta para girar (regla 2).
/// 4. Si ya giró y pasan [framesSinDocumentoParaVolver] frames sin ningún
///    documento, vuelve a la inicial -- la persona retiró el gafete y va a
///    mostrar otra cosa.
///
/// Hilos: [regionParaFrame] lo llama el hilo del analizador de CameraX y
/// [registrarTexto] el hilo principal (callback de ML Kit); por eso los
/// dos son `@Synchronized`.
class ControladorEncuadre(
    private val regionHorizontal: RegionGuiaOcr,
    private val regionVertical: RegionGuiaOcr,
    private val inicial: OrientacionEncuadre = OrientacionEncuadre.HORIZONTAL,
    private val orientacionDelTexto: (String) -> OrientacionEncuadre?,
    private val framesParaGirar: Int = 2,
    private val framesSinDocumentoParaVolver: Int = 15,
) {
    var orientacion: OrientacionEncuadre = inicial
        private set

    private var sondearProximo = false
    private var candidata: OrientacionEncuadre? = null
    private var conteoCandidata = 0
    private var framesSinDocumento = 0

    fun region(orientacion: OrientacionEncuadre): RegionGuiaOcr =
        if (orientacion == OrientacionEncuadre.HORIZONTAL) regionHorizontal else regionVertical

    /// Orientación con la que leer el PRÓXIMO frame: la actual, o la otra
    /// si toca una sonda (regla 3).
    @Synchronized
    fun orientacionParaFrame(): OrientacionEncuadre {
        if (sondearProximo) {
            sondearProximo = false
            return otra(orientacion)
        }
        return orientacion
    }

    fun regionParaFrame(): Pair<RegionGuiaOcr, OrientacionEncuadre> {
        val o = orientacionParaFrame()
        return region(o) to o
    }

    /// Registra el texto de un frame leído con `leidoCon`. Devuelve `true`
    /// si la orientación del recuadro cambió (la pantalla tiene que
    /// redibujarlo).
    @Synchronized
    fun registrarTexto(texto: String, leidoCon: OrientacionEncuadre): Boolean =
        registrarOrientacion(orientacionDelTexto(texto), leidoCon)

    /// Igual que [registrarTexto], con la orientación ya deducida por quien
    /// llama (p. ej. `ResultadoEstabilizacion.orientacionSugerida`): evita
    /// clasificar dos veces el mismo texto en el mismo frame.
    @Synchronized
    fun registrarOrientacion(pedida: OrientacionEncuadre?, leidoCon: OrientacionEncuadre): Boolean {
        if (pedida == null) {
            if (leidoCon != orientacion) return false // sonda sin resultado
            framesSinDocumento++
            if (orientacion != inicial && framesSinDocumento >= framesSinDocumentoParaVolver) {
                cambiarA(inicial)
                return true
            }
            if (orientacion == inicial) sondearProximo = true
            return false
        }
        framesSinDocumento = 0
        if (pedida == orientacion) {
            candidata = null
            conteoCandidata = 0
            return false
        }
        if (candidata == pedida) conteoCandidata++ else {
            candidata = pedida
            conteoCandidata = 1
        }
        if (conteoCandidata >= framesParaGirar) {
            cambiarA(pedida)
            return true
        }
        return false
    }

    @Synchronized
    fun reiniciar() = cambiarA(inicial)

    private fun cambiarA(nueva: OrientacionEncuadre) {
        orientacion = nueva
        sondearProximo = false
        candidata = null
        conteoCandidata = 0
        framesSinDocumento = 0
    }

    private fun otra(o: OrientacionEncuadre) =
        if (o == OrientacionEncuadre.HORIZONTAL) OrientacionEncuadre.VERTICAL else OrientacionEncuadre.HORIZONTAL
}

/// Comportamiento de la pantalla de escaneo general (documentos de
/// contratista y gafetes): el MRZ del reverso de una cédula/DIMEX no se
/// clasifica por palabras clave, pero es claramente horizontal.
fun orientacionDeTextoDocumento(texto: String): OrientacionEncuadre? =
    uniffi.control_acceso_mobile.orientacionDeTextoDocumento(texto)

/// Región que se dibuja con transición suave al girar el encuadre (el
/// recorte real cambia en el acto; esto es sólo lo que ve quien opera).
@Composable
fun regionAnimada(destino: RegionGuiaOcr): RegionGuiaOcr {
    val ancho by animateFloatAsState(destino.fraccionAncho, tween(280), label = "anchoEncuadre")
    val proporcion by animateFloatAsState(destino.proporcionAnchoAlto, tween(280), label = "proporcionEncuadre")
    val centro by animateFloatAsState(destino.fraccionTopCentro, tween(280), label = "centroEncuadre")
    return RegionGuiaOcr(fraccionAncho = ancho, proporcionAnchoAlto = proporcion, fraccionTopCentro = centro)
}

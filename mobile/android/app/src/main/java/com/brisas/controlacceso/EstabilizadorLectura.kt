package com.brisas.controlacceso

/// Estado central que alimenta tanto los esquineros del viewfinder como el
/// mensaje in-cámara (ver plan, secciones 7 y 8) -- un solo lugar decide
/// "qué está pasando", el resto de la UI sólo reacciona a este valor.
enum class EstadoEscaneo { BUSCANDO, INVALIDO, CONFIRMADO }

data class ResultadoEstabilizacion(
    val estado: EstadoEscaneo,
    val documento: DocumentoDetectado? = null,
    val mensaje: String,
)

/// Decide, frame a frame, si ya hay lectura suficiente para aceptarla.
///
/// Regla (plan, sección 5): si el MRZ trae checksum válido, se acepta en el
/// mismo frame -- no hace falta esperar repeticiones porque el dígito
/// verificador ya es la prueba de que la lectura es correcta. Sin checksum
/// (extracción del frente por regex), se exige que el mismo resultado se
/// repita en `framesRequeridos` frames consecutivos antes de aceptarlo, para
/// no disparar sobre una lectura a medio acomodar el documento.
///
/// Con instancia por sesión de escaneo: crear una nueva por cada vez que se
/// abre la pantalla de cámara, no reusar entre escaneos distintos.
class EstabilizadorLectura(private val framesRequeridos: Int = 3) {
    private var ultimoCandidato: DocumentoDetectado? = null
    private var repeticiones = 0

    fun procesarFrame(texto: String): ResultadoEstabilizacion {
        if (texto.isBlank() || texto.trim().length < 10) {
            reiniciar()
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "Acerque el documento")
        }

        val mrz = leerMrzDeTexto(texto)
        if (mrz != null) {
            reiniciar() // el MRZ no depende del debounce por candidato repetido
            return when {
                mrz.numeroDocumentoExtendidoSinSoporte ->
                    ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no reconocido")
                mrz.checksumsValidos -> {
                    val documento = mrz.aDocumentoDetectado()
                    ResultadoEstabilizacion(
                        EstadoEscaneo.CONFIRMADO,
                        documento = documento,
                        mensaje = "${documento.tipo.nombreLegible()} confirmado",
                    )
                }
                else ->
                    ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no reconocido")
            }
        }

        val tipo = clasificarTipoDocumento(texto)
        if (tipo == TipoDocumento.DESCONOCIDO) {
            reiniciar()
            return ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no reconocido")
        }

        val documento = leerDocumentoDeTexto(texto)
        if (documento == null) {
            // Tipo reconocible por palabras clave, pero todavía no se pudo
            // extraer el número -- lectura parcial (glare, ángulo, foco), no
            // un documento inválido. Ya se sabe qué es: se lo decimos a
            // quien opera en vez de un "mantenga firme" genérico.
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "${tipo.nombreLegible()} detectado — mantenga firme")
        }

        if (documento == ultimoCandidato) {
            repeticiones++
        } else {
            ultimoCandidato = documento
            repeticiones = 1
        }

        return if (repeticiones >= framesRequeridos) {
            ResultadoEstabilizacion(
                EstadoEscaneo.CONFIRMADO,
                documento = documento,
                mensaje = "${documento.tipo.nombreLegible()} confirmado",
            )
        } else {
            ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "${tipo.nombreLegible()} detectado — mantenga firme")
        }
    }

    private fun reiniciar() {
        ultimoCandidato = null
        repeticiones = 0
    }
}

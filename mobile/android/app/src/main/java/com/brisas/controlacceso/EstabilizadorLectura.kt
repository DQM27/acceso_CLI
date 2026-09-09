package com.brisas.controlacceso

/// Estado central que alimenta tanto los esquineros del viewfinder como el
/// mensaje in-cámara (ver plan, secciones 7 y 8) -- un solo lugar decide
/// "qué está pasando", el resto de la UI sólo reacciona a este valor.
enum class EstadoEscaneo { BUSCANDO, INVALIDO, CONFIRMADO }

enum class ModoEscaneoDocumento { DOCUMENTO_CONTRATISTA, GAFETE_CONTRATISTA }

data class ResultadoEstabilizacion(
    val estado: EstadoEscaneo,
    val documento: DocumentoDetectado? = null,
    val mensaje: String,
    // Sólo tiene sentido cuando estado == CONFIRMADO y el documento trae
    // fecha de vencimiento. `false` no significa "vigente confirmado", sólo
    // "no se detectó como vencido" (puede ser vigente, o simplemente no
    // haber fecha de vencimiento disponible para ese tipo de documento).
    val vencido: Boolean = false,
)

/// Decide, frame a frame, si ya hay lectura suficiente para aceptarla.
///
/// Regla (plan, sección 5): si el MRZ trae checksum válido, se acepta en el
/// mismo frame -- no hace falta esperar repeticiones porque el dígito
/// verificador ya es la prueba de que la lectura es correcta. Sin checksum
/// (extracción del frente por regex), se exige que el mismo resultado
/// aparezca `framesRequeridos` veces dentro de los últimos `ventana` frames
/// -- no necesariamente consecutivos.
///
/// Antes exigía que fueran consecutivos (un candidato distinto reiniciaba
/// el conteo a cero). Eso resultó ser demasiado frágil con reflejos: un
/// solo frame afectado por un reflejo (que hace que ML Kit lea mal un
/// dígito) tiraba todo el progreso acumulado, y con reflejos intermitentes
/// el conteo nunca llegaba a completarse. La ventana deslizante tolera
/// algún frame malo salteado entre medio sin perder lo ya leído bien.
///
/// Con instancia por sesión de escaneo: crear una nueva por cada vez que se
/// abre la pantalla de cámara, no reusar entre escaneos distintos.
///
/// **No es thread-safe** -- `candidatosRecientes` se lee y escribe sin
/// sincronización. Sólo es seguro porque `procesarFrame` se llama
/// exclusivamente desde el hilo principal (el callback de ML Kit se entrega
/// ahí explícitamente, ver `PantallaEscanearCedula.analizarCedula`). Si
/// algún día se llama desde el hilo del analizador de CameraX en vez del
/// principal, hay que agregar sincronización acá.
class EstabilizadorLectura(
    private val modo: ModoEscaneoDocumento = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
    private val framesRequeridos: Int = 2,
    // Un poco más grande que `framesRequeridos` -- da lugar a tolerar algún
    // frame malo salteado sin exigir tampoco una ventana tan larga que
    // acepte una racha vieja de candidatos ya abandonados.
    private val ventana: Int = framesRequeridos + 2,
    // Inyectable para poder fijar la fecha en tests sin depender del reloj
    // del sistema -- ver `fechaDeHoy()`.
    private val obtenerFechaHoy: () -> FechaDocumento = ::fechaDeHoy,
) {
    // La ventana guarda una clave estable, no el objeto entero. Nombre,
    // fecha u otros campos opcionales pueden aparecer y desaparecer entre
    // frames aunque el número reconocido sea el mismo.
    private val candidatosRecientes = ArrayDeque<String>()

    init {
        require(framesRequeridos > 0) { "framesRequeridos debe ser mayor que cero" }
        require(ventana >= framesRequeridos) { "ventana debe cubrir los frames requeridos" }
    }

    fun procesarFrame(texto: String): ResultadoEstabilizacion {
        if (texto.isBlank() || texto.trim().length < 10) {
            registrarFrameSinCandidato()
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "Acerque el documento")
        }

        val hoy = obtenerFechaHoy()
        val hoyLocal = java.time.LocalDate.of(hoy.anio, hoy.mes, hoy.dia)
        val mrz = if (modo == ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA) leerMrzDeTexto(texto, hoyLocal) else null
        if (mrz != null) {
            reiniciar() // el MRZ no depende del debounce por candidato repetido
            return when {
                mrz.numeroDocumentoExtendidoSinSoporte ->
                    ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no reconocido")
                mrz.checksumsValidos -> {
                    val documento = mrz.aDocumentoDetectado().reclasificarPorEdad(hoy)
                    if (!documento.tipo.esValidoParaModo(modo)) {
                        ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no soportado")
                    } else {
                        val (mensaje, vencido) = mensajeDeConfirmacion(documento, hoy)
                        ResultadoEstabilizacion(
                            EstadoEscaneo.CONFIRMADO,
                            documento = documento,
                            mensaje = mensaje,
                            vencido = vencido,
                        )
                    }
                }
                else ->
                    ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no reconocido")
            }
        }

        val tipo = clasificarTipoDocumento(texto)
        if (tipo == TipoDocumento.DESCONOCIDO) {
            registrarFrameSinCandidato()
            return ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = mensajeNoReconocido())
        }
        if (!tipo.esValidoParaModo(modo)) {
            registrarFrameSinCandidato()
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = mensajeApuntar())
        }

        val documento = leerDocumentoDeTexto(texto)
        if (documento == null) {
            // Tipo reconocible por palabras clave, pero todavía no se pudo
            // extraer el número -- lectura parcial (glare, ángulo, foco), no
            // un documento inválido. Ya se sabe qué es: se lo decimos a
            // quien opera en vez de un "mantenga firme" genérico.
            registrarFrameSinCandidato()
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "${tipo.nombreLegible()} detectado — mantenga firme")
        }

        val clave = documento.claveEstabilizacion()
        registrarClave(clave)
        val repeticiones = candidatosRecientes.count { it == clave }

        return if (repeticiones >= framesRequeridos) {
            val (mensaje, vencido) = mensajeDeConfirmacion(documento, hoy)
            ResultadoEstabilizacion(EstadoEscaneo.CONFIRMADO, documento = documento, mensaje = mensaje, vencido = vencido)
        } else {
            ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "${tipo.nombreLegible()} detectado — mantenga firme")
        }
    }

    /// Anuncia vencimiento en el mismo mensaje de confirmación -- un
    /// documento vencido igual se identificó correctamente (por eso sigue
    /// siendo CONFIRMADO, no INVALIDO), pero quien opera necesita saberlo de
    /// inmediato sin tener que leer la fecha en la pantalla por su cuenta.
    private fun mensajeDeConfirmacion(
        documento: DocumentoDetectado,
        hoy: FechaDocumento,
    ): Pair<String, Boolean> {
        val nombre = documento.tipo.nombreLegible()
        val vencimiento = documento.vencimiento
        val vencido = vencimiento != null && vencimiento.estaVencida(hoy)
        val mensaje = if (vencido) "$nombre confirmado — DOCUMENTO VENCIDO" else "$nombre confirmado"
        return mensaje to vencido
    }

    fun reiniciar() {
        candidatosRecientes.clear()
    }

    private fun registrarFrameSinCandidato() = registrarClave(CLAVE_SIN_CANDIDATO)

    private fun registrarClave(clave: String) {
        candidatosRecientes.addLast(clave)
        while (candidatosRecientes.size > ventana) candidatosRecientes.removeFirst()
    }

    private fun mensajeApuntar(): String =
        when (modo) {
            ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Apunte al documento del contratista"
            ModoEscaneoDocumento.GAFETE_CONTRATISTA -> "Apunte al gafete de contratista"
        }

    private fun mensajeNoReconocido(): String =
        when (modo) {
            ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Documento no reconocido"
            ModoEscaneoDocumento.GAFETE_CONTRATISTA -> "Gafete no reconocido"
        }
}

private const val CLAVE_SIN_CANDIDATO = "\u0000"

private fun DocumentoDetectado.claveEstabilizacion(): String =
    "${tipo.name}:${(textoBusqueda ?: numeroDocumento).trim().uppercase()}"

private fun TipoDocumento.esValidoParaModo(modo: ModoEscaneoDocumento): Boolean =
    when (modo) {
        ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA ->
            this != TipoDocumento.GAFETE_CONTRATISTA && this != TipoDocumento.DESCONOCIDO
        ModoEscaneoDocumento.GAFETE_CONTRATISTA ->
            this == TipoDocumento.GAFETE_CONTRATISTA
    }

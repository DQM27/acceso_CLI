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
    // MV-06 (auditoría 2026-09-24): en modo gafete, el número sale de una
    // regex sin dígito verificador (a diferencia del MRZ, que si trae
    // checksum válido se acepta en el mismo frame más arriba en
    // `procesarFrame`) y dispara una mutación real sin confirmación humana
    // (salida automática, ver `PantallaActivos`/`ActivosViewModel`). Con
    // sólo 2 repeticiones alcanzaba para que una confusión de OCR
    // (3/8, 1/7, 0/6) que se repite en el mismo reflejo cerrara el ingreso
    // de la persona equivocada. 3 en vez de 2 no elimina el riesgo (sigue
    // sin dígito verificador -- eso queda para una mejora aparte, QR o
    // checksum en el gafete), pero exige una confusión más consistente
    // para colarse. DOCUMENTO_CONTRATISTA se queda en 2: ese modo sí abre
    // un formulario para que el operador revise antes de guardar, no
    // dispara nada solo.
    private val framesRequeridos: Int = if (modo == ModoEscaneoDocumento.GAFETE_CONTRATISTA) 3 else 2,
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

    // Acumula campos opcionales (nombre, apellidos...) vistos en distintos
    // frames para el MISMO candidato (misma clave) -- hallazgo 2026-09-20
    // contra una cédula nacional real: "Nombre:", "1° Apellido:" y "2°
    // Apellido:" son tres bloques de texto separados en la tarjeta (a
    // diferencia de DIMEX, donde "Apellidos:" es un solo bloque), así que
    // rara vez ML Kit los lee los tres juntos en el mismo frame -- sin
    // esto, el documento que se confirmaba era el de ESE frame puntual
    // (a veces con nombre, a veces sólo con apellidos, a veces ninguno),
    // en vez de la unión de todo lo ya visto para ese número mientras se
    // sostiene el documento. Se reinicia sólo cuando cambia la clave (otro
    // número), nunca por un frame suelto sin candidato -- un frame borroso
    // de por medio no debe tirar lo ya leído bien.
    private var claveAcumulada: String? = null
    private var documentoAcumulado: DocumentoDetectado? = null

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
                    if (mrz.correcciones.isNotEmpty()) registrarCorreccionesMrzEnSentry(mrz.correcciones)
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

        val documentoDeEsteFrame = leerDocumentoDeTexto(texto)
        if (documentoDeEsteFrame == null) {
            // Tipo reconocible por palabras clave, pero todavía no se pudo
            // extraer el número -- lectura parcial (glare, ángulo, foco), no
            // un documento inválido. Ya se sabe qué es: se lo decimos a
            // quien opera en vez de un "mantenga firme" genérico.
            registrarFrameSinCandidato()
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "${tipo.nombreLegible()} detectado — mantenga firme")
        }

        val clave = documentoDeEsteFrame.claveEstabilizacion()
        val documento = if (clave == claveAcumulada) {
            documentoAcumulado!!.fusionarCamposOpcionales(documentoDeEsteFrame)
        } else {
            documentoDeEsteFrame
        }
        claveAcumulada = clave
        documentoAcumulado = documento

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
    ///
    /// Caso especial: cédula nacional leída del FRENTE (sin MRZ). Desde
    /// 2026-09-20 `extraerCedulaNacionalFrente` también intenta leer
    /// "Nombre:"/"1° Apellido:"/"2° Apellido:" del frente, pero sigue
    /// siendo una lectura por regex sin checksum (a diferencia del MRZ del
    /// reverso) -- por ángulo/reflejo puede confirmarse el número sin haber
    /// alcanzado a leer el nombre todavía. Este aviso sigue existiendo para
    /// ese caso (`documento.nombre == null`), no porque el frente nunca
    /// pueda traer nombre -- bug original reportado en pruebas reales,
    /// 2026-09-17: sin este aviso, quien operaba no tenía forma de saber
    /// que le faltaba el nombre hasta llenar el formulario a mano.
    private fun mensajeDeConfirmacion(
        documento: DocumentoDetectado,
        hoy: FechaDocumento,
    ): Pair<String, Boolean> {
        val nombreTipo = documento.tipo.nombreLegible()
        val vencimiento = documento.vencimiento
        val vencido = vencimiento != null && vencimiento.estaVencida(hoy)
        val faltaNombrePorFrente = documento.tipo == TipoDocumento.CEDULA_NACIONAL &&
            documento.fuenteDatos == FuenteDatos.OCR_FRENTE &&
            documento.nombre == null
        val mensaje = when {
            vencido -> "$nombreTipo confirmado — DOCUMENTO VENCIDO"
            faltaNombrePorFrente -> "Ya tengo el número — muéstreme el reverso para el nombre"
            else -> "$nombreTipo confirmado"
        }
        return mensaje to vencido
    }

    fun reiniciar() {
        candidatosRecientes.clear()
        claveAcumulada = null
        documentoAcumulado = null
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

/// Combina este documento (ya acumulado de frames anteriores para la misma
/// clave) con la lectura de un frame nuevo -- cada campo opcional se
/// actualiza sólo si el frame nuevo trae un valor no nulo, si no conserva
/// el que ya se tenía. Así un campo que un frame puntual no alcanzó a leer
/// (nombre, un apellido...) no borra lo que sí se leyó bien en un frame
/// anterior del mismo candidato. `numeroDocumento`/`tipo`/`fuenteDatos`
/// vienen siempre del frame nuevo -- son la clave, no un campo opcional, y
/// ya se validó que corresponde al mismo candidato antes de llamar a esto.
private fun DocumentoDetectado.fusionarCamposOpcionales(nuevo: DocumentoDetectado): DocumentoDetectado = nuevo.copy(
    textoBusqueda = nuevo.textoBusqueda ?: textoBusqueda,
    nombre = nuevo.nombre ?: nombre,
    apellidos = nuevo.apellidos ?: apellidos,
    nacionalidad = nuevo.nacionalidad ?: nacionalidad,
    empresa = nuevo.empresa ?: empresa,
    vencimiento = nuevo.vencimiento ?: vencimiento,
    fechaNacimiento = nuevo.fechaNacimiento ?: fechaNacimiento,
    sexo = nuevo.sexo ?: sexo,
)

private fun TipoDocumento.esValidoParaModo(modo: ModoEscaneoDocumento): Boolean =
    when (modo) {
        ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA ->
            this != TipoDocumento.GAFETE_CONTRATISTA && this != TipoDocumento.DESCONOCIDO
        ModoEscaneoDocumento.GAFETE_CONTRATISTA ->
            this == TipoDocumento.GAFETE_CONTRATISTA
    }

package com.brisas.controlacceso

import uniffi.control_acceso_mobile.RegistroMrz

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
    // 0..1: cuántas lecturas coincidentes lleva el candidato actual sobre
    // las que hacen falta para confirmar. La pantalla lo dibuja como barra
    // de avance en el recuadro ("ya casi, no lo mueva").
    val progreso: Float = 0f,
)

/// Decide, frame a frame, si ya hay lectura suficiente para aceptarla.
///
/// Regla (plan, sección 5): si el MRZ trae checksum válido, número y fechas
/// ya están probados por el dígito verificador -- sólo se pide que la
/// lectura (incluidos los nombres, que el MRZ no protege con checksum)
/// coincida en 2 frames, ver `procesarMrzValido`. Sin checksum
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
    // Cuántos frames se espera, con el número de una cédula ya leído del
    // frente pero sin nombre, a que la persona muestre el reverso (MRZ,
    // que sí trae el nombre) antes de confirmar sólo con el número. ~3 s al
    // ritmo real de análisis (150 ms entre frames, ver
    // `INTERVALO_MINIMO_ENTRE_FRAMES_MS`). Antes se confirmaba al instante
    // con el mensaje "muéstreme el reverso", pero confirmar cierra la
    // cámara: el reverso nunca llegaba a leerse.
    private val framesEsperaReverso: Int = 20,
    // Frames SEGUIDOS con texto no reconocible antes de mostrarlo como
    // inválido (marco rojo + vibración de error). Con 1, cualquier frame
    // de transición -- la mano moviéndose, un cartel de fondo, el
    // documento entrando al cuadro -- hacía parpadear el rojo y vibrar.
    private val framesParaInvalido: Int = 3,
    // Frames en que el tipo se reconoce pero sus datos no terminan de
    // leerse (típico de un reflejo sobre el número) antes de sugerir
    // inclinar el documento.
    private val framesParaSugerirReflejo: Int = 6,
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

    // Lecturas de MRZ con checksum válido de los últimos frames -- la línea
    // de nombres del MRZ NO tiene dígito verificador, así que un nombre mal
    // leído en un único frame se confirmaba igual. Ahora número + nombres
    // tienen que coincidir en 2 frames (ver `procesarMrzValido`).
    private val lecturasMrzRecientes = ArrayDeque<RegistroMrzEnVentana>()

    // `-1` = no se está esperando el reverso. Ver `framesEsperaReverso`.
    private var framesEsperandoReverso = -1
    private var framesDesconocidosSeguidos = 0
    private var framesTipoSinDatosSeguidos = 0

    init {
        require(framesRequeridos > 0) { "framesRequeridos debe ser mayor que cero" }
        require(ventana >= framesRequeridos) { "ventana debe cubrir los frames requeridos" }
    }

    fun procesarFrame(texto: String): ResultadoEstabilizacion {
        val hoy = obtenerFechaHoy()
        // Cuenta TODO frame (también los vacíos mientras se voltea la
        // tarjeta) para que la espera del reverso tenga un fin real.
        if (framesEsperandoReverso >= 0) framesEsperandoReverso++
        val esperaAgotada = framesEsperandoReverso >= framesEsperaReverso

        if (texto.isBlank() || texto.trim().length < 10) {
            framesDesconocidosSeguidos = 0
            registrarFrameSinCandidato()
            if (esperaAgotada) confirmarSinReverso(hoy)?.let { return it }
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "Acerque el documento")
        }

        val hoyLocal = java.time.LocalDate.of(hoy.anio, hoy.mes, hoy.dia)
        val mrz = if (modo == ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA) leerMrzDeTexto(texto, hoyLocal) else null
        if (mrz != null && mrz.checksumsValidos && !mrz.numeroDocumentoExtendidoSinSoporte) {
            return procesarMrzValido(mrz, hoy)
        }
        if (esperaAgotada) confirmarSinReverso(hoy)?.let { return it }
        if (mrz != null) {
            return if (mrz.numeroDocumentoExtendidoSinSoporte) {
                ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no reconocido")
            } else {
                // Hay un MRZ en cuadro pero todavía mal leído (foco, reflejo,
                // ángulo) -- es lectura parcial, no un documento inválido.
                // Antes era INVALIDO: marco rojo + vibración de error cada
                // vez que se entraba en ese estado mientras la persona
                // apenas acomodaba la tarjeta. Tampoco borra lo acumulado
                // del frente.
                ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = MENSAJE_LEYENDO_REVERSO)
            }
        }

        val tipo = clasificarTipoDocumento(texto)
        if (tipo == TipoDocumento.DESCONOCIDO) {
            registrarFrameSinCandidato()
            framesDesconocidosSeguidos++
            return if (framesDesconocidosSeguidos >= framesParaInvalido) {
                ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = mensajeNoReconocido())
            } else {
                ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = MENSAJE_BUSCANDO)
            }
        }
        framesDesconocidosSeguidos = 0
        if (!tipo.esValidoParaModo(modo)) {
            registrarFrameSinCandidato()
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = mensajeTipoEquivocado(tipo))
        }

        val documentoDeEsteFrame = leerDocumentoDeTexto(texto)
        if (documentoDeEsteFrame == null) {
            // Tipo reconocible por palabras clave, pero todavía no se pudo
            // extraer el número -- lectura parcial (glare, ángulo, foco), no
            // un documento inválido. Ya se sabe qué es: se lo decimos a
            // quien opera en vez de un "mantenga firme" genérico.
            registrarFrameSinCandidato()
            framesTipoSinDatosSeguidos++
            val mensaje = if (framesTipoSinDatosSeguidos >= framesParaSugerirReflejo) {
                "${tipo.nombreLegible()} — incline un poco para quitar el reflejo"
            } else {
                "${tipo.nombreLegible()} — no lo mueva"
            }
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = mensaje)
        }
        framesTipoSinDatosSeguidos = 0

        val clave = documentoDeEsteFrame.claveEstabilizacion()
        val documento = if (clave == claveAcumulada) {
            documentoAcumulado!!.fusionarCamposOpcionales(documentoDeEsteFrame)
        } else {
            documentoDeEsteFrame
        }
        if (clave != claveAcumulada) framesEsperandoReverso = -1
        claveAcumulada = clave
        documentoAcumulado = documento

        registrarClave(clave)
        val repeticiones = candidatosRecientes.count { it == clave }

        if (repeticiones >= framesRequeridos && faltaNombrePorFrente(documento)) {
            if (framesEsperandoReverso < 0) framesEsperandoReverso = 0
            if (framesEsperandoReverso < framesEsperaReverso) {
                return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = MENSAJE_FALTA_REVERSO)
            }
        }
        return if (repeticiones >= framesRequeridos) {
            framesEsperandoReverso = -1
            val (mensaje, vencido) = mensajeDeConfirmacion(documento, hoy)
            ResultadoEstabilizacion(EstadoEscaneo.CONFIRMADO, documento = documento, mensaje = mensaje, vencido = vencido)
        } else {
            ResultadoEstabilizacion(
                EstadoEscaneo.BUSCANDO,
                mensaje = "${tipo.nombreLegible()} — no lo mueva",
                progreso = repeticiones.toFloat() / framesRequeridos,
            )
        }
    }

    /// Anuncia vencimiento en el mismo mensaje de confirmación -- un
    /// documento vencido igual se identificó correctamente (por eso sigue
    /// siendo CONFIRMADO, no INVALIDO), pero quien opera necesita saberlo de
    /// inmediato sin tener que leer la fecha en la pantalla por su cuenta.
    ///
    /// Caso especial: cédula nacional leída del FRENTE sin nombre. Antes se
    /// confirmaba al instante con "muéstreme el reverso para el nombre",
    /// pero confirmar cierra la cámara, así que el reverso nunca se llegaba
    /// a leer. Ahora `procesarFrame` espera [framesEsperaReverso] frames al
    /// MRZ (mensaje [MENSAJE_FALTA_REVERSO], estado BUSCANDO); sólo si no
    /// llega se confirma con el número, avisando que el nombre falta.
    private fun mensajeDeConfirmacion(
        documento: DocumentoDetectado,
        hoy: FechaDocumento,
    ): Pair<String, Boolean> {
        val nombreTipo = documento.tipo.nombreLegible()
        val vencimiento = documento.vencimiento
        val vencido = vencimiento != null && vencimiento.estaVencida(hoy)
        // "Listo: <tipo>" en vez de "<tipo> confirmado": la mayoría de los
        // tipos son femeninos ("Cédula...", "Licencia...") y el participio
        // no concordaba. El vencido dice CUÁNDO venció, que es lo que quien
        // opera necesita para decidir.
        val mensaje = when {
            vencido -> "Listo: $nombreTipo — VENCIDO el ${vencimiento!!.aTextoDDMMYYYY()}"
            faltaNombrePorFrente(documento) -> "Listo: $nombreTipo — sin nombre, complételo a mano"
            else -> "Listo: $nombreTipo"
        }
        return mensaje to vencido
    }

    fun reiniciar() {
        candidatosRecientes.clear()
        claveAcumulada = null
        documentoAcumulado = null
        lecturasMrzRecientes.clear()
        framesEsperandoReverso = -1
        framesDesconocidosSeguidos = 0
        framesTipoSinDatosSeguidos = 0
    }

    /// Se agotó la espera del reverso sin que llegara un MRZ válido: se
    /// confirma la cédula con lo que se leyó del frente (el número).
    private fun confirmarSinReverso(hoy: FechaDocumento): ResultadoEstabilizacion? {
        val documento = documentoAcumulado ?: return null
        framesEsperandoReverso = -1
        val (mensaje, vencido) = mensajeDeConfirmacion(documento, hoy)
        return ResultadoEstabilizacion(EstadoEscaneo.CONFIRMADO, documento = documento, mensaje = mensaje, vencido = vencido)
    }

    /// MRZ con checksums válidos: número, fechas y el resto ya están
    /// verificados por dígito verificador, pero la línea de nombres no --
    /// se confirma cuando la misma lectura completa (número + nombres)
    /// aparece en 2 de los últimos frames. Si el número se repite
    /// [LECTURAS_MRZ_PARA_DESEMPATAR] veces con nombres que no terminan de
    /// coincidir (reflejo sobre esa línea), se confirma con los nombres más
    /// repetidos en vez de trabarse para siempre.
    private fun procesarMrzValido(mrz: RegistroMrz, hoy: FechaDocumento): ResultadoEstabilizacion {
        val documento = mrz.aDocumentoDetectado().reclasificarPorEdad(hoy)
        if (!documento.tipo.esValidoParaModo(modo)) {
            return ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no soportado")
        }
        val lectura = RegistroMrzEnVentana(mrz, documento)
        lecturasMrzRecientes.addLast(lectura)
        while (lecturasMrzRecientes.size > VENTANA_LECTURAS_MRZ) lecturasMrzRecientes.removeFirst()

        val mismoNumero = lecturasMrzRecientes.filter { it.documento.numeroDocumento == documento.numeroDocumento }
        val masRepetida = mismoNumero.groupBy { it.claveNombres }.maxBy { it.value.size }.value
        val elegida = when {
            masRepetida.size >= LECTURAS_MRZ_COINCIDENTES -> masRepetida.last()
            mismoNumero.size >= LECTURAS_MRZ_PARA_DESEMPATAR -> masRepetida.last()
            else -> return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = MENSAJE_LEYENDO_REVERSO)
        }
        reiniciar()
        if (elegida.mrz.correcciones.isNotEmpty()) registrarCorreccionesMrzEnSentry(elegida.mrz.correcciones)
        val (mensaje, vencido) = mensajeDeConfirmacion(elegida.documento, hoy)
        return ResultadoEstabilizacion(
            EstadoEscaneo.CONFIRMADO,
            documento = elegida.documento,
            mensaje = mensaje,
            vencido = vencido,
        )
    }

    private fun registrarFrameSinCandidato() = registrarClave(CLAVE_SIN_CANDIDATO)

    private fun registrarClave(clave: String) {
        candidatosRecientes.addLast(clave)
        while (candidatosRecientes.size > ventana) candidatosRecientes.removeFirst()
    }

    /// Dice QUÉ vio la cámara, no sólo "apunte a...": así quien opera sabe
    /// que tiene en la mano el objeto equivocado (p. ej. el gafete en vez
    /// de la cédula) y no que la cámara "no lee".
    private fun mensajeTipoEquivocado(tipo: TipoDocumento): String =
        when (modo) {
            ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA ->
                "Detecté ${tipo.nombreLegible()} — aquí va el documento del contratista"
            ModoEscaneoDocumento.GAFETE_CONTRATISTA ->
                "Detecté ${tipo.nombreLegible()} — aquí va el gafete de contratista"
        }

    private fun mensajeNoReconocido(): String =
        when (modo) {
            ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA -> "Documento no reconocido"
            ModoEscaneoDocumento.GAFETE_CONTRATISTA -> "Gafete no reconocido"
        }
}

private const val CLAVE_SIN_CANDIDATO = "\u0000"
private const val VENTANA_LECTURAS_MRZ = 6
private const val LECTURAS_MRZ_COINCIDENTES = 2
private const val LECTURAS_MRZ_PARA_DESEMPATAR = 4
private const val MENSAJE_BUSCANDO = "Buscando un documento…"
private const val MENSAJE_LEYENDO_REVERSO = "Leyendo el reverso — mantenga firme"
private const val MENSAJE_FALTA_REVERSO = "Ya tengo el número — muéstreme la otra cara para el nombre"

private class RegistroMrzEnVentana(val mrz: RegistroMrz, val documento: DocumentoDetectado) {
    val claveNombres: String = "${documento.apellidos}|${documento.nombre}"
}

/// Cédula nacional leída sólo del frente, todavía sin nombre -- el nombre
/// está garantizado en el reverso (MRZ), por eso se espera a que la
/// persona la voltee (ver `framesEsperaReverso`).
private fun faltaNombrePorFrente(documento: DocumentoDetectado): Boolean =
    documento.tipo == TipoDocumento.CEDULA_NACIONAL &&
        documento.fuenteDatos == FuenteDatos.OCR_FRENTE &&
        documento.nombre == null

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

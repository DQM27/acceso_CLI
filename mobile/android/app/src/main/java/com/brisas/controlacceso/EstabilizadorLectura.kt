package com.brisas.controlacceso

import uniffi.control_acceso_mobile.ConsensoVotacion
import uniffi.control_acceso_mobile.DatosPdf417Cedula
import uniffi.control_acceso_mobile.VotadorPorPosicion
import uniffi.control_acceso_mobile.leerMrz

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
    // 0..1: cuánto respaldo lleva el candidato actual sobre el que hace
    // falta para confirmar. La pantalla lo dibuja como barra de avance en
    // el recuadro ("ya casi, no lo mueva").
    val progreso: Float = 0f,
    // Orientación del recuadro que pide lo leído en este frame (`null` =
    // no opina). La pantalla se la pasa a `ControladorEncuadre` en vez de
    // que éste vuelva a clasificar el mismo texto.
    val orientacionSugerida: OrientacionEncuadre? = null,
    // Hay un MRZ en cuadro: el lector de códigos no tiene nada que buscar
    // (la cédula nueva cambió el PDF417 por el MRZ).
    val hayMrz: Boolean = false,
)

/// Decide, frame a frame, si ya hay lectura suficiente para aceptarla.
///
/// **Votación por carácter** (auditoría OCR 2026-09-28). Antes se exigía
/// que la misma cadena completa se repitiera N veces; con reflejos cada
/// frame erraba en un carácter distinto y ninguna lectura coincidía entera
/// con otra. Ahora cada frame vota posición por posición
/// (`VotadorPorPosicion`, en Rust) con un peso según su nitidez, y se
/// confirma cuando, en TODAS las posiciones, el carácter ganador:
/// - tiene respaldo de al menos `framesRequeridos` frames (descontando
///   [TOLERANCIA_SOPORTE] para frames apenas menos nítidos), y
/// - le saca a la segunda opción al menos [MARGEN_MINIMO] de ventaja.
///
/// Con frames de peso 1 esto nunca es más permisivo que la regla anterior
/// (N lecturas del mismo valor), incluido el modo gafete, que dispara una
/// salida sin revisión humana. Si un frame difiere del consenso en más de
/// un par de caracteres se trata como OTRO documento y se empieza de cero:
/// la votación corrige errores de lectura, nunca mezcla dos personas.
///
/// El MRZ se vota igual (sus líneas completas) y el consenso se valida con
/// sus dígitos verificadores: la línea de nombres no los tiene, por eso
/// también ahí se exige respaldo de 2 frames.
///
/// El PDF417 de la cédula anterior ([procesarPdf417]) confirma en una
/// sola lectura: el código trae su propia corrección de errores y Rust
/// valida la forma de los datos.
///
/// Una instancia por sesión de escaneo. Es seguro llamarla desde el hilo
/// del analizador y reiniciarla desde el principal: los métodos públicos
/// están sincronizados.
class EstabilizadorLectura(
    private val modo: ModoEscaneoDocumento = ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA,
    // MV-06 (auditoría 2026-09-24): en modo gafete el número no tiene
    // dígito verificador y confirmar dispara una salida real sin revisión
    // humana, así que se exige más respaldo. DOCUMENTO_CONTRATISTA abre un
    // formulario que el operador revisa antes de guardar.
    private val framesRequeridos: Int = if (modo == ModoEscaneoDocumento.GAFETE_CONTRATISTA) 3 else 2,
    // Frames recientes (con o sin lectura) que se consideran: tolera algún
    // frame malo salteado sin aceptar una racha vieja ya abandonada.
    private val ventana: Int = framesRequeridos + 2,
    // Inyectable para poder fijar la fecha en tests sin depender del reloj
    // del sistema -- ver `fechaDeHoy()`.
    private val obtenerFechaHoy: () -> FechaDocumento = ::fechaDeHoy,
    // Cuántos frames se espera, con el número de una cédula ya leído del
    // frente pero sin nombre, a que la persona muestre el reverso antes de
    // confirmar sólo con el número. Confirmar cierra la cámara: sin esta
    // espera el reverso nunca llegaba a leerse.
    private val framesEsperaReverso: Int = 20,
    // Frames SEGUIDOS con texto no reconocible antes de mostrarlo como
    // inválido (marco rojo + vibración de error): con 1, cualquier frame de
    // transición hacía parpadear el rojo.
    private val framesParaInvalido: Int = 3,
    // Frames en que el tipo se reconoce pero sus datos no terminan de
    // leerse (típico de un reflejo sobre el número) antes de sugerir
    // inclinar el documento aunque la medición no vea reflejo.
    private val framesParaSugerirReflejo: Int = 6,
) {
    private val votadorNumero = VotadorPorPosicion(ventana.toUInt())
    private val votadorMrz = VotadorPorPosicion(VENTANA_LECTURAS_MRZ.toUInt())

    // Campos opcionales (nombre, apellidos...) vistos en distintos frames
    // del MISMO documento: en la cédula, "Nombre:" y los apellidos son
    // bloques separados que ML Kit rara vez lee juntos en un frame.
    private var tipoAcumulado: TipoDocumento? = null
    private var documentoAcumulado: DocumentoDetectado? = null

    // `-1` = no se está esperando el reverso. Ver `framesEsperaReverso`.
    private var framesEsperandoReverso = -1
    private var framesDesconocidosSeguidos = 0
    private var framesTipoSinDatosSeguidos = 0
    private var framesConReflejoSeguidos = 0

    init {
        require(framesRequeridos > 0) { "framesRequeridos debe ser mayor que cero" }
        require(ventana >= framesRequeridos) { "ventana debe cubrir los frames requeridos" }
    }

    /// `peso`: calidad relativa del frame en (0, 1] (ver `FiltroCalidad`).
    /// `calidad`: medición del frame, para avisar de un reflejo real.
    @Synchronized
    fun procesarFrame(texto: String, peso: Float = 1f, calidad: CalidadFrame? = null): ResultadoEstabilizacion {
        val hoy = obtenerFechaHoy()
        // Cuenta TODO frame (también los vacíos mientras se voltea la
        // tarjeta) para que la espera del reverso tenga un fin real.
        if (framesEsperandoReverso >= 0) framesEsperandoReverso++
        val esperaAgotada = framesEsperandoReverso >= framesEsperaReverso
        framesConReflejoSeguidos = if (calidad != null && calidad.fraccionReflejo >= UMBRAL_REFLEJO) {
            framesConReflejoSeguidos + 1
        } else {
            0
        }

        if (texto.isBlank() || texto.trim().length < 10) {
            framesDesconocidosSeguidos = 0
            registrarFrameSinCandidato()
            votadorMrz.agregarVacio()
            if (esperaAgotada) confirmarSinReverso(hoy)?.let { return it }
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = "Acerque el documento")
        }

        val hoyLocal = java.time.LocalDate.of(hoy.anio, hoy.mes, hoy.dia)
        val lecturaMrz = if (modo == ModoEscaneoDocumento.DOCUMENTO_CONTRATISTA) leerLecturaMrz(texto, hoyLocal) else null
        if (lecturaMrz != null) return procesarLecturaMrz(lecturaMrz, peso, hoy, esperaAgotada)
        votadorMrz.agregarVacio()
        if (esperaAgotada) confirmarSinReverso(hoy)?.let { return it }

        val orientacionMrz = if ("<<" in texto) OrientacionEncuadre.HORIZONTAL else null
        val tipo = clasificarTipoDocumento(texto)
        if (tipo == TipoDocumento.DESCONOCIDO) {
            registrarFrameSinCandidato()
            framesDesconocidosSeguidos++
            return if (framesDesconocidosSeguidos >= framesParaInvalido) {
                ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = mensajeNoReconocido(), orientacionSugerida = orientacionMrz)
            } else {
                ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = MENSAJE_BUSCANDO, orientacionSugerida = orientacionMrz)
            }
        }
        framesDesconocidosSeguidos = 0
        val orientacion = orientacionMrz ?: tipo.orientacionEncuadre()
        if (!tipo.esValidoParaModo(modo)) {
            registrarFrameSinCandidato()
            return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = mensajeTipoEquivocado(tipo), orientacionSugerida = orientacion)
        }

        val documentoDeEsteFrame = leerDocumentoDeTexto(texto, tipo)
        if (documentoDeEsteFrame == null) {
            // Tipo reconocible por palabras clave, pero todavía no se pudo
            // extraer el número -- lectura parcial (reflejo, ángulo, foco).
            registrarFrameSinCandidato()
            framesTipoSinDatosSeguidos++
            val sugerirReflejo = framesTipoSinDatosSeguidos >= framesParaSugerirReflejo
            return ResultadoEstabilizacion(
                EstadoEscaneo.BUSCANDO,
                mensaje = mensajeLecturaParcial("${tipo.nombreLegible()} — no lo mueva", tipo, sugerirReflejo),
                orientacionSugerida = orientacion,
            )
        }
        framesTipoSinDatosSeguidos = 0
        return votarNumero(documentoDeEsteFrame, peso, hoy, orientacion)
    }

    /// Cédula anterior leída de su PDF417 (sólo cédula y nombre, ya
    /// validados por Rust). Confirma en el acto.
    @Synchronized
    fun procesarPdf417(datos: DatosPdf417Cedula): ResultadoEstabilizacion {
        if (!TipoDocumento.CEDULA_NACIONAL.esValidoParaModo(modo)) {
            return ResultadoEstabilizacion(
                EstadoEscaneo.BUSCANDO,
                mensaje = mensajeTipoEquivocado(TipoDocumento.CEDULA_NACIONAL),
            )
        }
        val documento = DocumentoDetectado(
            tipo = TipoDocumento.CEDULA_NACIONAL,
            numeroDocumento = datos.cedula,
            nombre = datos.nombre,
            apellidos = datos.apellidos,
            fuenteDatos = FuenteDatos.PDF417,
            checksumValido = true,
        )
        reiniciarEstado()
        val (mensaje, vencido) = mensajeDeConfirmacion(documento, obtenerFechaHoy())
        return ResultadoEstabilizacion(EstadoEscaneo.CONFIRMADO, documento = documento, mensaje = mensaje, vencido = vencido)
    }

    @Synchronized
    fun reiniciar() = reiniciarEstado()

    private fun reiniciarEstado() {
        votadorNumero.reiniciar()
        votadorMrz.reiniciar()
        tipoAcumulado = null
        documentoAcumulado = null
        framesEsperandoReverso = -1
        framesDesconocidosSeguidos = 0
        framesTipoSinDatosSeguidos = 0
        framesConReflejoSeguidos = 0
    }

    private fun votarNumero(
        documentoDeEsteFrame: DocumentoDetectado,
        peso: Float,
        hoy: FechaDocumento,
        orientacion: OrientacionEncuadre?,
    ): ResultadoEstabilizacion {
        val valor = documentoDeEsteFrame.valorVotado()
        val consensoPrevio = votadorNumero.consenso()
        val otroDocumento = documentoDeEsteFrame.tipo != tipoAcumulado ||
            (consensoPrevio != null && esOtraLectura(consensoPrevio.texto, valor))
        if (otroDocumento) {
            votadorNumero.reiniciar()
            documentoAcumulado = null
            framesEsperandoReverso = -1
        }
        tipoAcumulado = documentoDeEsteFrame.tipo
        documentoAcumulado = documentoAcumulado?.fusionarCamposOpcionales(documentoDeEsteFrame) ?: documentoDeEsteFrame
        votadorNumero.agregar(valor, peso)

        val consenso = votadorNumero.consenso() ?: return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = MENSAJE_BUSCANDO)
        val documento = documentoAcumulado!!.conValorVotado(consenso.texto)
        documentoAcumulado = documento
        val listo = consenso.alcanza(framesRequeridos)

        if (listo && faltaNombrePorFrente(documento)) {
            if (framesEsperandoReverso < 0) framesEsperandoReverso = 0
            if (framesEsperandoReverso < framesEsperaReverso) {
                return ResultadoEstabilizacion(EstadoEscaneo.BUSCANDO, mensaje = MENSAJE_FALTA_REVERSO, orientacionSugerida = orientacion)
            }
        }
        if (listo) {
            framesEsperandoReverso = -1
            val (mensaje, vencido) = mensajeDeConfirmacion(documento, hoy)
            return ResultadoEstabilizacion(
                EstadoEscaneo.CONFIRMADO,
                documento = documento,
                mensaje = mensaje,
                vencido = vencido,
                orientacionSugerida = orientacion,
            )
        }
        return ResultadoEstabilizacion(
            EstadoEscaneo.BUSCANDO,
            mensaje = mensajeLecturaParcial("${documento.tipo.nombreLegible()} — no lo mueva", documento.tipo, sugerirPorTiempo = false),
            progreso = (consenso.soporteMinimo / framesRequeridos).coerceIn(0f, 1f),
            orientacionSugerida = orientacion,
        )
    }

    private fun procesarLecturaMrz(
        lectura: LecturaMrz,
        peso: Float,
        hoy: FechaDocumento,
        esperaAgotada: Boolean,
    ): ResultadoEstabilizacion {
        val registro = lectura.registro
        if (registro.numeroDocumentoExtendidoSinSoporte) {
            return ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no reconocido", hayMrz = true)
        }
        // Un MRZ válido de un tipo no soportado es definitivo: no depende
        // de más lecturas.
        if (registro.checksumsValidos && !registro.aDocumentoDetectado().reclasificarPorEdad(hoy).tipo.esValidoParaModo(modo)) {
            return ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no soportado", hayMrz = true)
        }
        val lineas = lectura.lineas.joinToString(SEPARADOR_LINEAS_MRZ)
        val consensoPrevio = votadorMrz.consenso()
        if (consensoPrevio != null && esOtraLecturaMrz(consensoPrevio.texto, lineas)) votadorMrz.reiniciar()
        votadorMrz.agregar(lineas, peso)
        confirmarPorConsensoMrz(hoy)?.let { return it }
        if (esperaAgotada) confirmarSinReverso(hoy)?.let { return it }
        // MRZ en cuadro pero todavía sin consenso válido (foco, reflejo,
        // ángulo): lectura parcial, no un documento inválido -- sin marco
        // rojo ni vibración de error, y sin borrar lo leído del frente.
        return ResultadoEstabilizacion(
            EstadoEscaneo.BUSCANDO,
            mensaje = mensajeLecturaParcial(MENSAJE_LEYENDO_REVERSO, tipo = null, sugerirPorTiempo = false),
            orientacionSugerida = OrientacionEncuadre.HORIZONTAL,
            hayMrz = true,
        )
    }

    private fun confirmarPorConsensoMrz(hoy: FechaDocumento): ResultadoEstabilizacion? {
        val consenso = votadorMrz.consenso() ?: return null
        if (!consenso.alcanza(LECTURAS_MRZ_COINCIDENTES)) return null
        val registro = leerMrz(consenso.texto.split(SEPARADOR_LINEAS_MRZ), hoy.anio)
        if (!registro.formatoReconocido || !registro.checksumsValidos || registro.numeroDocumentoExtendidoSinSoporte) return null
        val documento = registro.aDocumentoDetectado().reclasificarPorEdad(hoy)
        if (!documento.tipo.esValidoParaModo(modo)) {
            return ResultadoEstabilizacion(EstadoEscaneo.INVALIDO, mensaje = "Documento no soportado", hayMrz = true)
        }
        reiniciarEstado()
        if (registro.correcciones.isNotEmpty()) registrarCorreccionesMrzEnSentry(registro.correcciones)
        val (mensaje, vencido) = mensajeDeConfirmacion(documento, hoy)
        return ResultadoEstabilizacion(
            EstadoEscaneo.CONFIRMADO,
            documento = documento,
            mensaje = mensaje,
            vencido = vencido,
            orientacionSugerida = OrientacionEncuadre.HORIZONTAL,
            hayMrz = true,
        )
    }

    /// Anuncia vencimiento en el mismo mensaje de confirmación -- un
    /// documento vencido igual se identificó correctamente (por eso sigue
    /// siendo CONFIRMADO, no INVALIDO), pero quien opera necesita saberlo de
    /// inmediato sin tener que leer la fecha en la pantalla por su cuenta.
    private fun mensajeDeConfirmacion(
        documento: DocumentoDetectado,
        hoy: FechaDocumento,
    ): Pair<String, Boolean> {
        val nombreTipo = documento.tipo.nombreLegible()
        val vencimiento = documento.vencimiento
        val vencido = vencimiento != null && vencimiento.estaVencida(hoy)
        // "Listo: <tipo>" en vez de "<tipo> confirmado": la mayoría de los
        // tipos son femeninos ("Cédula...", "Licencia...") y el participio
        // no concordaba.
        val mensaje = when {
            vencido -> "Listo: $nombreTipo — VENCIDO el ${vencimiento!!.aTextoDDMMYYYY()}"
            faltaNombrePorFrente(documento) -> "Listo: $nombreTipo — sin nombre, complételo a mano"
            else -> "Listo: $nombreTipo"
        }
        return mensaje to vencido
    }

    /// Se agotó la espera del reverso sin que llegara un MRZ válido: se
    /// confirma la cédula con lo que se leyó del frente (el número).
    private fun confirmarSinReverso(hoy: FechaDocumento): ResultadoEstabilizacion? {
        val documento = documentoAcumulado ?: return null
        framesEsperandoReverso = -1
        val (mensaje, vencido) = mensajeDeConfirmacion(documento, hoy)
        return ResultadoEstabilizacion(EstadoEscaneo.CONFIRMADO, documento = documento, mensaje = mensaje, vencido = vencido)
    }

    /// Mensaje mientras se lee: si la medición del frame ve reflejo
    /// sostenido, lo dice (antes sólo se sospechaba por tiempo, cuando el
    /// tipo se reconocía pero los datos no terminaban de salir).
    private fun mensajeLecturaParcial(base: String, tipo: TipoDocumento?, sugerirPorTiempo: Boolean): String {
        val hayReflejo = framesConReflejoSeguidos >= FRAMES_REFLEJO_PARA_AVISAR
        return when {
            hayReflejo -> MENSAJE_REFLEJO_MEDIDO
            sugerirPorTiempo && tipo != null -> "${tipo.nombreLegible()} — incline un poco para quitar el reflejo"
            else -> base
        }
    }

    private fun registrarFrameSinCandidato() = votadorNumero.agregarVacio()

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

private const val VENTANA_LECTURAS_MRZ = 6
private const val LECTURAS_MRZ_COINCIDENTES = 2
private const val SEPARADOR_LINEAS_MRZ = "\n"

// Un frame apenas menos nítido que el mejor pesa algo menos de 1: sin esta
// tolerancia, 2 frames buenos (p. ej. 1,0 + 0,9) no alcanzarían "2". Con
// frames claramente peores sí hace falta uno más -- esa es la idea de
// ponderar por nitidez.
private const val TOLERANCIA_SOPORTE = 0.5f

// Ventaja mínima del carácter ganador sobre la segunda opción, en "frames
// completos": un empate (0) nunca confirma.
private const val MARGEN_MINIMO = 0.5f

// Proporción de píxeles saturados del recuadro a partir de la cual el
// reflejo tapa texto (ver `CalidadFrame`), y cuántos frames seguidos antes
// de decirlo -- un destello suelto no merece aviso.
private const val UMBRAL_REFLEJO = 0.03f
private const val FRAMES_REFLEJO_PARA_AVISAR = 3

private const val MENSAJE_BUSCANDO = "Buscando un documento…"
private const val MENSAJE_LEYENDO_REVERSO = "Leyendo el reverso — mantenga firme"
private const val MENSAJE_FALTA_REVERSO = "Ya tengo el número — muéstreme la otra cara para el nombre"
private const val MENSAJE_REFLEJO_MEDIDO = "Hay reflejo — incline un poco el documento"

internal fun ConsensoVotacion.alcanza(requeridos: Int): Boolean =
    soporteMinimo >= requeridos - TOLERANCIA_SOPORTE && margenMinimo >= MARGEN_MINIMO

/// ¿`nueva` es la lectura de OTRO documento y no el mismo con algún
/// carácter mal leído? Distinto largo, o más diferencias de las que
/// explica un error de lectura (un par de caracteres; en números cortos,
/// como el de un gafete, uno solo).
internal fun esOtraLectura(consenso: String, nueva: String): Boolean {
    if (consenso.length != nueva.length) return true
    val diferencias = consenso.zip(nueva).count { (a, b) -> a != b }
    return diferencias > maxOf(1, consenso.length / 4)
}

/// Mismo criterio para las 2-3 líneas del MRZ (72-92 caracteres): otra
/// persona difiere en número, fechas y nombres a la vez.
private fun esOtraLecturaMrz(consenso: String, nueva: String): Boolean {
    if (consenso.length != nueva.length) return true
    return consenso.zip(nueva).count { (a, b) -> a != b } > consenso.length / 5
}

/// Cédula nacional leída sólo del frente, todavía sin nombre -- el nombre
/// está garantizado en el reverso (MRZ), por eso se espera a que la
/// persona la voltee (ver `framesEsperaReverso`).
private fun faltaNombrePorFrente(documento: DocumentoDetectado): Boolean =
    documento.tipo == TipoDocumento.CEDULA_NACIONAL &&
        documento.fuenteDatos == FuenteDatos.OCR_FRENTE &&
        documento.nombre == null

/// Lo que se vota entre frames: el número (o, para el gafete In House sin
/// cédula, el nombre con que se busca).
private fun DocumentoDetectado.valorVotado(): String = (textoBusqueda ?: numeroDocumento).trim()

/// El documento acumulado con el valor que ganó la votación en vez del de
/// un frame puntual.
private fun DocumentoDetectado.conValorVotado(valor: String): DocumentoDetectado =
    if (textoBusqueda != null) {
        copy(
            textoBusqueda = valor,
            numeroDocumento = if (numeroDocumento.trim() == textoBusqueda.trim()) valor else numeroDocumento,
        )
    } else {
        copy(numeroDocumento = valor)
    }

/// Combina este documento (ya acumulado de frames anteriores del mismo
/// documento) con la lectura de un frame nuevo -- cada campo opcional se
/// actualiza sólo si el frame nuevo trae un valor no nulo, si no conserva
/// el que ya se tenía. `numeroDocumento`/`tipo`/`fuenteDatos` vienen del
/// frame nuevo; el número definitivo lo pone después la votación.
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

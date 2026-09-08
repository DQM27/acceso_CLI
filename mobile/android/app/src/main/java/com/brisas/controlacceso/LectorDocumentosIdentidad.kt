package com.brisas.controlacceso

/// Tipos de documento que el lector sabe clasificar. `DESCONOCIDO` es el
/// resultado cuando el texto no calza ninguna señal conocida -- la pantalla
/// de escaneo debe seguir buscando, no tratarlo como error terminal.
enum class TipoDocumento {
    CEDULA_NACIONAL,
    CEDULA_RESIDENCIA,
    LICENCIA_NACIONAL,
    LICENCIA_EXTRANJERO,
    PASAPORTE,
    DESCONOCIDO,
}

/// Resultado normalizado de leer un documento, sin importar cuál extractor
/// lo produjo (frente vía regex, o MRZ vía checksum). Los campos que un
/// tipo de documento no trae (ej. nacionalidad en una licencia nacional)
/// quedan en `null` en vez de forzar un valor -- ver sección 0.2 del plan.
data class DocumentoDetectado(
    val tipo: TipoDocumento,
    val numeroDocumento: String,
    val nombre: String? = null,
    val apellidos: String? = null,
    val nacionalidad: String? = null,
    val esExtranjero: Boolean = false,
    val vencimiento: FechaDocumento? = null,
    val fechaNacimiento: FechaDocumento? = null,
    val sexo: Char? = null,
    val fuenteDatos: FuenteDatos = FuenteDatos.OCR_FRENTE,
    val checksumValido: Boolean? = null,
)

/// Nombre para mostrar en el feedback in-cámara -- quien opera nunca elige
/// el tipo de documento a mano, así que esto es lo único que le confirma
/// qué detectó el lector (sección 8 del plan, mensajes in-cámara).
fun TipoDocumento.nombreLegible(): String = when (this) {
    TipoDocumento.CEDULA_NACIONAL -> "Cédula de identidad"
    TipoDocumento.CEDULA_RESIDENCIA -> "Cédula de residencia (DIMEX)"
    TipoDocumento.LICENCIA_NACIONAL -> "Licencia de conducir"
    TipoDocumento.LICENCIA_EXTRANJERO -> "Licencia de conducir de extranjero"
    TipoDocumento.PASAPORTE -> "Pasaporte"
    TipoDocumento.DESCONOCIDO -> "Documento"
}

/// Traduce un MRZ ya parseado al modelo normalizado. TD1 se etiqueta como
/// `CEDULA_RESIDENCIA` por ahora -- la cédula nacional 2025+ también usa
/// TD1 y hoy no hay forma de distinguirlas sólo por el MRZ (el campo de
/// tipo de documento no está estandarizado entre emisores); si esto
/// importa en la práctica, resolver combinando con el frente.
fun ResultadoMrz.aDocumentoDetectado(): DocumentoDetectado = DocumentoDetectado(
    tipo = if (formato == "TD3") TipoDocumento.PASAPORTE else TipoDocumento.CEDULA_RESIDENCIA,
    numeroDocumento = numeroDocumento,
    nombre = nombres.ifBlank { null },
    apellidos = apellidos.ifBlank { null },
    nacionalidad = nacionalidad.ifBlank { null },
    vencimiento = fechaVencimiento,
    fechaNacimiento = fechaNacimiento,
    sexo = sexo,
    fuenteDatos = FuenteDatos.MRZ,
    checksumValido = checksumsValidos,
)

data class FechaDocumento(val dia: Int, val mes: Int, val anio: Int) {
    fun estaVencida(hoy: FechaDocumento): Boolean {
        val propia = anio * 10000 + mes * 100 + dia
        val actual = hoy.anio * 10000 + hoy.mes * 100 + hoy.dia
        return propia < actual
    }
}

/// La fecha real del dispositivo, envuelta en `FechaDocumento` -- factorizada
/// en su propia función (en vez de llamar `LocalDate.now()` directo donde se
/// necesite) para poder inyectar una fecha fija en tests y no depender del
/// reloj del sistema al probar vigencia/vencimiento.
fun fechaDeHoy(): FechaDocumento {
    val hoy = java.time.LocalDate.now()
    return FechaDocumento(hoy.dayOfMonth, hoy.monthValue, hoy.year)
}

/// Clasifica el tipo de documento a partir del texto crudo de ML Kit, antes
/// de intentar extraer ningún campo -- este orden importa porque DIMEX y
/// licencia de extranjero comparten el mismo rango de número de documento,
/// y sólo el contexto (palabras clave) los distingue de forma confiable.
fun clasificarTipoDocumento(texto: String): TipoDocumento {
    val mayus = texto.uppercase()
    return when {
        "LICENCIA DE CONDUCIR" in mayus && Regex("""N[º9O]?[:.]?\s*DM[- ]""").containsMatchIn(mayus) ->
            TipoDocumento.LICENCIA_EXTRANJERO
        "LICENCIA DE CONDUCIR" in mayus ->
            TipoDocumento.LICENCIA_NACIONAL
        "DGME" in mayus || "MIGRACIÓN Y EXTRANJERÍA" in mayus || "MIGRACION Y EXTRANJERIA" in mayus ||
            "RESIDENTE PERMANENTE" in mayus || "RESIDENTE TEMPORAL" in mayus ->
            TipoDocumento.CEDULA_RESIDENCIA
        "TRIBUNAL SUPREMO DE ELECCIONES" in mayus || extraerCedulaDeTexto(texto) != null ->
            TipoDocumento.CEDULA_NACIONAL
        else -> TipoDocumento.DESCONOCIDO
    }
}

/// Punto de entrada único: clasifica y extrae en un solo paso. Devuelve
/// `null` cuando no hay suficiente información todavía (la pantalla de
/// escaneo debe seguir esperando más frames, no tratarlo como fallo).
fun leerDocumentoDeTexto(texto: String): DocumentoDetectado? {
    return when (clasificarTipoDocumento(texto)) {
        TipoDocumento.CEDULA_NACIONAL -> extraerCedulaDeTexto(texto)?.let {
            DocumentoDetectado(tipo = TipoDocumento.CEDULA_NACIONAL, numeroDocumento = it)
        }
        TipoDocumento.CEDULA_RESIDENCIA -> extraerDimex(texto)
        TipoDocumento.LICENCIA_NACIONAL -> extraerLicencia(texto, esExtranjero = false)
        TipoDocumento.LICENCIA_EXTRANJERO -> extraerLicencia(texto, esExtranjero = true)
        // El clasificador por palabras clave del frente no distingue
        // pasaporte todavía -- llega sólo vía MRZ (ver ResultadoMrz.aDocumentoDetectado).
        TipoDocumento.PASAPORTE -> null
        TipoDocumento.DESCONOCIDO -> null
    }
}

/// Extrae el número de "Documento No.:", nunca el de "Expediente No.:" --
/// este es el caso que motivó todo el refinamiento (ver plan, sección 1):
/// ambos son números de longitud similar en el mismo bloque de texto, y una
/// regex genérica sin contexto de etiqueta puede agarrar el equivocado.
private fun extraerDimex(texto: String): DocumentoDetectado? {
    val numero = Regex("""DOCUMENTO\s*NO\.?:?\s*(\d{6,15})""", RegexOption.IGNORE_CASE)
        .find(texto)?.groupValues?.get(1)
        ?: return null

    val nombre = Regex("""Nombre:\s*\n?\s*([A-ZÁÉÍÓÚÑ ]+)""", RegexOption.IGNORE_CASE)
        .find(texto)?.groupValues?.get(1)?.trim()
    val apellidos = Regex("""Apellidos:\s*\n?\s*([A-ZÁÉÍÓÚÑ ]+)""", RegexOption.IGNORE_CASE)
        .find(texto)?.groupValues?.get(1)?.trim()
    val nacionalidad = Regex("""Nacionalidad:\s*\n?\s*([A-ZÁÉÍÓÚÑ ]+)""", RegexOption.IGNORE_CASE)
        .find(texto)?.groupValues?.get(1)?.trim()
    val vencimiento = extraerFecha(texto, etiqueta = "Vence")

    return DocumentoDetectado(
        tipo = TipoDocumento.CEDULA_RESIDENCIA,
        numeroDocumento = numero,
        nombre = nombre,
        apellidos = apellidos,
        nacionalidad = nacionalidad,
        vencimiento = vencimiento,
    )
}

/// El prefijo "DM-" es la señal de que es una licencia de extranjero (ver
/// plan, sección 3) -- se remueve del número final pero ya se usó para
/// clasificar, así que `esExtranjero` llega como parámetro ya decidido.
private fun extraerLicencia(texto: String, esExtranjero: Boolean): DocumentoDetectado? {
    val numero = Regex("""N[º9O]?[:.]?\s*(?:DM[- ])?(\d{6,15})""", RegexOption.IGNORE_CASE)
        .find(texto)?.groupValues?.get(1)
        ?: return null

    val vencimiento = extraerFecha(texto, etiqueta = "Vencimiento")

    return DocumentoDetectado(
        tipo = if (esExtranjero) TipoDocumento.LICENCIA_EXTRANJERO else TipoDocumento.LICENCIA_NACIONAL,
        numeroDocumento = numero,
        esExtranjero = esExtranjero,
        vencimiento = vencimiento,
    )
}

private fun extraerFecha(texto: String, etiqueta: String): FechaDocumento? {
    val match = Regex("""$etiqueta[:.]?\s*(\d{1,2})[-\s](\d{1,2})[-\s](\d{4})""", RegexOption.IGNORE_CASE)
        .find(texto) ?: return null
    val (dia, mes, anio) = match.destructured
    return FechaDocumento(dia.toInt(), mes.toInt(), anio.toInt())
}

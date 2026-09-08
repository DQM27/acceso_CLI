package com.brisas.controlacceso

/// Tipos de documento que el lector sabe clasificar. `DESCONOCIDO` es el
/// resultado cuando el texto no calza ninguna señal conocida -- la pantalla
/// de escaneo debe seguir buscando, no tratarlo como error terminal.
enum class TipoDocumento {
    CEDULA_NACIONAL,
    // TIM: Tarjeta de Identidad de Menores -- mismo código de documento MRZ
    // que la cédula nacional (`IDCRI`, ver ResultadoMrz.aDocumentoDetectado),
    // se distingue por edad calculada desde `fechaNacimiento`, no por el
    // código -- ver `reclasificarPorEdad`.
    TARJETA_IDENTIDAD_MENOR,
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
    TipoDocumento.TARJETA_IDENTIDAD_MENOR -> "Tarjeta de Identidad de Menores"
    TipoDocumento.CEDULA_RESIDENCIA -> "Cédula de residencia (DIMEX)"
    TipoDocumento.LICENCIA_NACIONAL -> "Licencia de conducir"
    TipoDocumento.LICENCIA_EXTRANJERO -> "Licencia de conducir de extranjero"
    TipoDocumento.PASAPORTE -> "Pasaporte"
    TipoDocumento.DESCONOCIDO -> "Documento"
}

/// Traduce un MRZ ya parseado al modelo normalizado. La distinción entre
/// cédula nacional y DIMEX (ambas TD1) es una regla de Costa Rica, no algo
/// que ICAO estandarice -- por eso vive acá, no en `MrzParser.kt`: el
/// código de documento (ICAO 9303 permite A/C/I como primer carácter, el
/// segundo a discreción del emisor) usado por Costa Rica es `ID` para la
/// cédula nacional (Decreto TSE n.° 22-2025, vigente desde oct-2025) y `C<`
/// para el DIMEX/residencia de DGME (confirmado contra un documento real).
/// Sin especificación pública oficial que lo documente con este nivel de
/// detalle -- basado en las imágenes de las circulares/decreto del TSE.
/// Un TD1 de otro país (o de Costa Rica con un código distinto de estos
/// dos) queda como `DESCONOCIDO`: no hay regla verificada para él todavía,
/// mejor eso que asumir uno de los dos casos costarricenses sin fundamento.
fun ResultadoMrz.aDocumentoDetectado(): DocumentoDetectado = DocumentoDetectado(
    tipo = when {
        formato == "TD3" -> TipoDocumento.PASAPORTE
        formato == "TD1" && paisEmisor == "CRI" && codigoDocumento == "ID" -> TipoDocumento.CEDULA_NACIONAL
        formato == "TD1" && paisEmisor == "CRI" && codigoDocumento == "C<" -> TipoDocumento.CEDULA_RESIDENCIA
        else -> TipoDocumento.DESCONOCIDO
    },
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

    /// Edad en años cumplidos a la fecha `hoy` -- resta los años, y le quita
    /// uno más si el cumpleaños todavía no pasó este año (comparando
    /// mes/día directamente, sin pasar por fechas reales de calendario).
    fun edadEnAnios(hoy: FechaDocumento): Int {
        val cumpleañosYaPaso = (hoy.mes > mes) || (hoy.mes == mes && hoy.dia >= dia)
        return hoy.anio - anio - if (cumpleañosYaPaso) 0 else 1
    }
}

private const val EDAD_MAYORIA_DE_EDAD = 18

/// La TIM (Tarjeta de Identidad de Menores) usa el mismo código de MRZ que
/// la cédula nacional de adulto (`IDCRI`, ver sección 4.3 de
/// fixtures-ocr-sinteticos.md) -- el código de documento por sí solo no
/// alcanza para distinguirlas, pero la fecha de nacimiento sí. Sin efecto
/// sobre otros tipos (DIMEX, licencias, pasaporte) ni si no hay fecha de
/// nacimiento disponible.
fun DocumentoDetectado.reclasificarPorEdad(hoy: FechaDocumento): DocumentoDetectado {
    val nacimiento = fechaNacimiento ?: return this
    if (tipo != TipoDocumento.CEDULA_NACIONAL) return this
    return if (nacimiento.edadEnAnios(hoy) < EDAD_MAYORIA_DE_EDAD) {
        copy(tipo = TipoDocumento.TARJETA_IDENTIDAD_MENOR)
    } else {
        this
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
        // Nunca lo produce el clasificador del frente -- sólo aparece vía
        // reclasificación por edad después de leer el MRZ (reclasificarPorEdad).
        TipoDocumento.TARJETA_IDENTIDAD_MENOR -> null
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

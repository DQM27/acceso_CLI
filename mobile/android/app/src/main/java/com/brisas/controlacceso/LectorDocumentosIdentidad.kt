package com.brisas.controlacceso

import uniffi.control_acceso_mobile.FechaMrz
import uniffi.control_acceso_mobile.FormatoMrz
import uniffi.control_acceso_mobile.RegistroMrz

/// Origen de los datos de un documento leído: el frente (texto libre, vía
/// regex por etiqueta) o el MRZ del reverso (con checksum verificable) --
/// vivía en `MrzParser.kt` antes de la migración a Rust del 2026-09-25, se
/// mueve acá porque describe el origen de un `DocumentoDetectado` (este
/// archivo), no es parte del parseo de MRZ en sí.
enum class FuenteDatos { OCR_FRENTE, MRZ }

/// Tipos de documento que el lector sabe clasificar. `DESCONOCIDO` es el
/// resultado cuando el texto no calza ninguna señal conocida -- la pantalla
/// de escaneo debe seguir buscando, no tratarlo como error terminal.
enum class TipoDocumento {
    CEDULA_NACIONAL,
    // TIM: Tarjeta de Identidad de Menores -- mismo código de documento MRZ
    // que la cédula nacional (`IDCRI`, ver RegistroMrz.aDocumentoDetectado),
    // se distingue por edad calculada desde `fechaNacimiento`, no por el
    // código -- ver `reclasificarPorEdad`.
    TARJETA_IDENTIDAD_MENOR,
    CEDULA_RESIDENCIA,
    LICENCIA_NACIONAL,
    LICENCIA_EXTRANJERO,
    PASAPORTE,
    // Carnet de inducción al sitio (PRAIND) -- corresponde al mismo campo
    // `fecha_vencimiento_praind` que ya existe en `Contratista`. Visto en
    // dos variantes de diseño distintas (encabezado/pie de página
    // distintos), pero ambas comparten las mismas etiquetas de campo
    // ("Nombre:", "No. de cédula:", "Fecha de inducción:", "Fecha de
    // vencimiento de inducción:") y la frase "CARNET DE INDUCCIÓN", que es
    // la señal de clasificación.
    CARNET_INDUCCION_PRAIND,
    CARNET_IN_HOUSE,
    CARNET_BAC,
    GAFETE_CONTRATISTA,
    DESCONOCIDO,
}

/// Resultado normalizado de leer un documento, sin importar cuál extractor
/// lo produjo (frente vía regex, o MRZ vía checksum). Los campos que un
/// tipo de documento no trae (ej. nacionalidad en una licencia nacional)
/// quedan en `null` en vez de forzar un valor -- ver sección 0.2 del plan.
data class DocumentoDetectado(
    val tipo: TipoDocumento,
    val numeroDocumento: String,
    val textoBusqueda: String? = null,
    val nombre: String? = null,
    val apellidos: String? = null,
    val nacionalidad: String? = null,
    // Sólo la trae el carnet PRAIND (`extraerPraind`) -- texto crudo tal
    // cual lo imprime el carnet, no un `empresa_id`: quien consume esto
    // (`PantallaNuevoContratista`) decide cómo emparejarlo contra
    // `Nucleo.listarEmpresas()`.
    val empresa: String? = null,
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
    TipoDocumento.CARNET_INDUCCION_PRAIND -> "Carnet de inducción (PRAIND)"
    TipoDocumento.CARNET_IN_HOUSE -> "Carnet in-house"
    TipoDocumento.CARNET_BAC -> "Carnet BAC"
    TipoDocumento.GAFETE_CONTRATISTA -> "Gafete de contratista"
    TipoDocumento.DESCONOCIDO -> "Documento"
}

/// `FechaMrz` (generada por UniFFI desde `mobile/rust-core/src/mrz.rs`) al
/// `FechaDocumento` que ya usa el resto de la app (lectores no-MRZ
/// incluidos) -- conversión mecánica, `dia`/`mes` llegan como `UByte`.
private fun FechaMrz.aFechaDocumento(): FechaDocumento = FechaDocumento(dia.toInt(), mes.toInt(), anio)

/// Traduce un MRZ ya parseado (por Rust, ver `leerMrz` en `MrzParser.kt`) al
/// modelo normalizado. La distinción entre cédula nacional y DIMEX (ambas
/// TD1) es una regla de Costa Rica, no algo que ICAO estandarice -- por eso
/// vive acá, no en el parser de Rust: el código de documento (ICAO 9303
/// permite A/C/I como primer carácter, el segundo a discreción del emisor)
/// usado por Costa Rica es `ID` para la cédula nacional (Decreto TSE
/// n.° 22-2025, vigente desde oct-2025) y `C<` para el DIMEX/residencia de
/// DGME (confirmado contra un documento real). Sin especificación pública
/// oficial que lo documente con este nivel de detalle -- basado en las
/// imágenes de las circulares/decreto del TSE. Un TD1 de otro país (o de
/// Costa Rica con un código distinto de estos dos) queda como
/// `DESCONOCIDO`: no hay regla verificada para él todavía, mejor eso que
/// asumir uno de los dos casos costarricenses sin fundamento.
fun RegistroMrz.aDocumentoDetectado(): DocumentoDetectado = DocumentoDetectado(
    tipo = when {
        formato == FormatoMrz.TD3 -> TipoDocumento.PASAPORTE
        formato == FormatoMrz.TD1 && paisEmisor == "CRI" && codigoDocumento == "ID" -> TipoDocumento.CEDULA_NACIONAL
        formato == FormatoMrz.TD1 && paisEmisor == "CRI" && codigoDocumento == "C<" -> TipoDocumento.CEDULA_RESIDENCIA
        else -> TipoDocumento.DESCONOCIDO
    },
    numeroDocumento = numeroDocumento,
    nombre = nombres.ifBlank { null },
    apellidos = apellidos.ifBlank { null },
    nacionalidad = nacionalidad.ifBlank { null },
    vencimiento = fechaVencimiento?.aFechaDocumento(),
    fechaNacimiento = fechaNacimiento?.aFechaDocumento(),
    sexo = sexo.firstOrNull(),
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

    companion object {
        fun crearValida(dia: Int, mes: Int, anio: Int): FechaDocumento? =
            runCatching { java.time.LocalDate.of(anio, mes, dia) }
                .getOrNull()
                ?.let { FechaDocumento(it.dayOfMonth, it.monthValue, it.year) }
    }
}

private const val EDAD_MAYORIA_DE_EDAD = 18

/// La TIM (Tarjeta de Identidad de Menores) usa el mismo código de MRZ que
/// la cédula nacional de adulto (`IDCRI`, ver sección 4.3 de
/// docs/arquitectura/fixtures-ocr-sinteticos.md) -- el código de documento por sí solo no
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

// Compiladas una sola vez a nivel de archivo, no dentro de la función --
// `clasificarTipoDocumento` y los extractores de abajo corren en CADA frame
// mientras la cámara está escaneando (varias veces por segundo). Un `Regex(...)`
// dentro de una función recompila el patrón en cada llamada; a este ritmo eso
// es trabajo de CPU/batería desperdiciado sin ninguna razón, ya que el patrón
// nunca cambia entre llamadas.
// (Las de cédula nacional -- guiones/espacios, pegada, caracteres a limpiar --
// viven en PantallaEscanearCedula.kt junto a extraerCedulaDeTexto, que es
// donde se usan; `private` a nivel de archivo en Kotlin no cruza archivos.)
private val REGEX_LICENCIA_EXTRANJERO = Regex("""N[º9O]?[:.]?\s*DM[- ]""")
private val REGEX_LICENCIA_DM_DIRECTO = Regex("""\bDM[- ]?(\d{6,15})\b""", RegexOption.IGNORE_CASE)
private val REGEX_DIMEX_NUMERO = Regex("""DOCUMENTO\s*NO\.?:?\s*(\d{6,15})""", RegexOption.IGNORE_CASE)
private val REGEX_DIMEX_NUMERO_PROVISIONAL = Regex("""N[°ºO]?\s*DOCUMENTO\s*:?\s*(\d{6,15})""", RegexOption.IGNORE_CASE)
// Compartida con la cédula nacional de frente (misma etiqueta "Nombre:"
// exacta en ambos documentos) -- ver `extraerCedulaNacionalFrente`.
private val REGEX_NOMBRE_ETIQUETA = Regex("""Nombre:\s*\n?\s*([A-ZÁÉÍÓÚÑ ]+)""", RegexOption.IGNORE_CASE)
// Cédula nacional de frente: el apellido viene partido en dos campos, no
// uno solo como en DIMEX -- "1°Apellido:"/"1° Apellido:" según el diseño
// (formato nuevo con orquídeas vs. el azul anterior, dos fotos reales del
// 2026-09-20), y lo mismo para el segundo. `\D{0,4}` entre el dígito y
// "Apellido" tolera el símbolo de grado, el espacio, o ambos, sin
// necesitar saber cuál de los dos diseños es.
private val REGEX_CEDULA_APELLIDO1 = Regex("""1\D{0,4}Apellido:?[ \t]*\n?[ \t]*([A-ZÁÉÍÓÚÑ ]+)""", RegexOption.IGNORE_CASE)
private val REGEX_CEDULA_APELLIDO2 = Regex("""2\D{0,4}Apellido:?[ \t]*\n?[ \t]*([A-ZÁÉÍÓÚÑ ]+)""", RegexOption.IGNORE_CASE)
private val REGEX_LICENCIA_NUMERO = Regex("""N[º°9O]?[:.]?\s*(?:DM|CI)?[- ]?(\d{6,15})""", RegexOption.IGNORE_CASE)
// El nombre completo en la licencia NO trae ninguna etiqueta ("Nombre:")
// a diferencia de cédula/DIMEX -- aparece como una línea suelta en
// mayúsculas, sin más (verificado contra una licencia real, 2026-09-26:
// antes esto no se intentaba leer para nada, sólo el número). Una línea
// candidata es puro texto en mayúsculas de al menos 3 palabras.
private val REGEX_LICENCIA_NOMBRE_COMPLETO = Regex("""^[A-ZÁÉÍÓÚÑ]+(?:[ \t]+[A-ZÁÉÍÓÚÑ]+){2,}$""")
// Palabras que SÍ aparecen impresas en mayúsculas en el resto del diseño
// de la licencia (encabezado, sello de fondo del MOPT) -- una línea
// candidata que contenga alguna de éstas se descarta, para no confundir
// "REPUBLICA DE COSTA RICA" o el sello "DIRECCION GENERAL EDUCACION
// VIAL" con el nombre real de la persona.
private val PALABRAS_NO_NOMBRE_LICENCIA = setOf(
    "REPUBLICA", "REPÚBLICA", "COSTA", "RICA", "LICENCIA", "CONDUCIR",
    "EXPEDICION", "EXPEDICIÓN", "NACIMIENTO", "VENCIMIENTO", "TIPO", "DONADOR",
    "DIRECCION", "DIRECCIÓN", "GENERAL", "EDUCACION", "EDUCACIÓN", "VIAL", "MOPT",
)
private val REGEX_PRAIND_CEDULA = Regex("""No\.?\s*de\s*c[ée]dula:?\s*(\d{6,15})""", RegexOption.IGNORE_CASE)
private val REGEX_PRAIND_NOMBRE = Regex("""Nombre:?[ \t]*\n?[ \t]*([^\n]+)""", RegexOption.IGNORE_CASE)
private val REGEX_PRAIND_EMPRESA = Regex("""Empresa:?[ \t]*\n?[ \t]*([^\n]+)""", RegexOption.IGNORE_CASE)
// `vencimiento` va antes que `induccion` en el orden de declaración
// solamente por legibilidad -- lo que importa es que la frase completa de
// cada regex es literal, así que "Fecha de inducción" nunca matchea dentro
// de "Fecha de vencimiento de inducción" (tiene "vencimiento de" injertado
// en el medio) ni al revés. `\s+` entre "de" e "inducción" en la de
// vencimiento tolera el salto de línea que trae uno de los dos diseños de
// carnet vistos ("Fecha de vencimiento de\ninducción:").
private val REGEX_PRAIND_FECHA_VENCIMIENTO = Regex(
    """Fecha\s+de\s+vencimiento\s+de\s+inducci[oó]n:?\s*(\d{1,2})[/-](\d{1,2})[/-](\d{4})""",
    RegexOption.IGNORE_CASE,
)
private val REGEX_INHOUSE_CEDULA = Regex("""C[ÉE]DULA:?\s*\n?\s*(\d{6,15})""", RegexOption.IGNORE_CASE)
private val REGEX_DIGITOS_BAC = Regex("""\b\d{9,15}\b""")
private val REGEX_GAFETE_CONTRATISTA = Regex("""\bCRC\s*[-:]?\s*(\d{1,4})\b""", RegexOption.IGNORE_CASE)

/// Clasifica el tipo de documento a partir del texto crudo de ML Kit, antes
/// de intentar extraer ningún campo -- este orden importa porque DIMEX y
/// licencia de extranjero comparten el mismo rango de número de documento,
/// y sólo el contexto (palabras clave) los distingue de forma confiable.
fun clasificarTipoDocumento(texto: String): TipoDocumento {
    val mayus = texto.uppercase()
    return when {
        "LICENCIA DE CONDUCIR" in mayus && REGEX_LICENCIA_EXTRANJERO.containsMatchIn(mayus) ->
            TipoDocumento.LICENCIA_EXTRANJERO
        "LICENCIA DE CONDUCIR" in mayus && REGEX_LICENCIA_DM_DIRECTO.containsMatchIn(mayus) ->
            TipoDocumento.LICENCIA_EXTRANJERO
        "LICENCIA DE CONDUCIR" in mayus ->
            TipoDocumento.LICENCIA_NACIONAL
        "DGME" in mayus || "MIGRACIÓN Y EXTRANJERÍA" in mayus || "MIGRACION Y EXTRANJERIA" in mayus ||
            "CÉDULA DE RESIDENCIA" in mayus || "CEDULA DE RESIDENCIA" in mayus ||
            "RESIDENTE PERMANENTE" in mayus || "RESIDENTE TEMPORAL" in mayus ||
            "CARNE PROVISIONAL" in mayus || "CARNÉ PROVISIONAL" in mayus || "PERMISO LABORAL" in mayus ->
            TipoDocumento.CEDULA_RESIDENCIA
        // Antes que la cédula nacional a propósito: un carnet PRAIND trae su
        // propio "No. de cédula: 123456789" de 9 dígitos, que si no se
        // clasificara primero acá calzaría con `extraerCedulaDeTexto` de
        // abajo y el carnet se leería como si fuera la cédula misma.
        "CARNET DE INDUCCIÓN" in mayus || "CARNET DE INDUCCION" in mayus ->
            TipoDocumento.CARNET_INDUCCION_PRAIND
        "BAC" in mayus && REGEX_DIGITOS_BAC.containsMatchIn(mayus) ->
            TipoDocumento.CARNET_BAC
        "CONTRATISTAS" in mayus && "COSTA RICA" in mayus && REGEX_GAFETE_CONTRATISTA.containsMatchIn(mayus) ->
            TipoDocumento.GAFETE_CONTRATISTA
        "CONTRATISTA" in mayus && "COSTA RICA" in mayus && ("EMPRESA" in mayus || REGEX_INHOUSE_CEDULA.containsMatchIn(texto)) ->
            TipoDocumento.CARNET_IN_HOUSE
        "CONTRATISTA" in mayus && "COSTA RICA" in mayus && extraerNombreInHouseFrente(texto) != null ->
            TipoDocumento.CARNET_IN_HOUSE
        ("TRIBUNAL SUPREMO DE ELECCIONES" in mayus ||
            "CÉDULA DE IDENTIDAD" in mayus || "CEDULA DE IDENTIDAD" in mayus) &&
            extraerCedulaDeTexto(texto) != null ->
            TipoDocumento.CEDULA_NACIONAL
        else -> TipoDocumento.DESCONOCIDO
    }
}

/// Punto de entrada único: clasifica y extrae en un solo paso. Devuelve
/// `null` cuando no hay suficiente información todavía (la pantalla de
/// escaneo debe seguir esperando más frames, no tratarlo como fallo).
fun leerDocumentoDeTexto(texto: String): DocumentoDetectado? {
    return when (clasificarTipoDocumento(texto)) {
        TipoDocumento.CEDULA_NACIONAL -> extraerCedulaNacionalFrente(texto)
        TipoDocumento.CEDULA_RESIDENCIA -> extraerDimex(texto)
        TipoDocumento.LICENCIA_NACIONAL -> extraerLicencia(texto, esExtranjero = false)
        TipoDocumento.LICENCIA_EXTRANJERO -> extraerLicencia(texto, esExtranjero = true)
        TipoDocumento.CARNET_INDUCCION_PRAIND -> extraerPraind(texto)
        TipoDocumento.CARNET_IN_HOUSE -> extraerInHouse(texto)
        TipoDocumento.CARNET_BAC -> extraerBac(texto)
        TipoDocumento.GAFETE_CONTRATISTA -> extraerGafeteContratista(texto)
        // El clasificador por palabras clave del frente no distingue
        // pasaporte todavía -- llega sólo vía MRZ (ver RegistroMrz.aDocumentoDetectado).
        TipoDocumento.PASAPORTE -> null
        // Nunca lo produce el clasificador del frente -- sólo aparece vía
        // reclasificación por edad después de leer el MRZ (reclasificarPorEdad).
        TipoDocumento.TARJETA_IDENTIDAD_MENOR -> null
        TipoDocumento.DESCONOCIDO -> null
    }
}

// Red de seguridad adicional al esquema de abajo: si ML Kit ordena las
// columnas de forma que el valor de Sexo/Género queda sólo en el renglón
// que sigue a "Nombre:" o "Apellidos:" (hallazgo 2026-09-20 con una DIMEX
// real: el nombre salía "MASCULINO"), ese valor se descarta y se sigue
// buscando en el renglón siguiente.
private val VALORES_SEXO_DIMEX = setOf("M", "F", "MASCULINO", "FEMENINO")

// Esquema del frente del DIMEX (Documento de Identidad Migratorio para
// Extranjeros, DGME), tal como está impreso -- dos columnas de texto a la
// derecha de la foto (valores de ejemplo, ficticios):
//
//   RESIDENTE PERMANENTE / LIBRE CONDICIÓN        <- categoría (2 renglones)
//   Apellidos:
//   PEREZ MORA
//   Nombre:
//   JUAN CARLOS                    Género: M      <- mismo renglón visual
//   Nacionalidad:
//   NICARAGUA                      F.nac.: 01 01 1990
//   Documento No.: 155800000000    Emitido: 01 01 2023
//   Expediente No.: 135 - 000000   Vence:   01 01 2026
//
// ML Kit suele devolver cada renglón visual como UNA línea, así que el
// valor de la columna izquierda llega pegado al campo de la derecha
// ("JUAN CARLOS Género: M", "NICARAGUA F.nac.: ..."). Por eso cada valor
// se toma del resto del renglón de su etiqueta (o del renglón siguiente si
// la etiqueta quedó sola) y se corta en la primera etiqueta vecina.
//
// Sólo se extraen 4 campos: nombre, apellidos, "Documento No." (ES el
// número de la cédula de residencia) y "Vence". Todo lo demás (categoría,
// nacionalidad, género, fecha de nacimiento, emisión y "Expediente No.",
// que es el número de trámite de la DGME) se descarta a propósito: cuanto
// menos se lee, menos riesgo de que un campo contamine a otro del mismo
// renglón. Sus etiquetas sólo sirven como límite para cortar los valores.
private val PATRON_ETIQUETAS_IZQ_DIMEX = """APELLIDOS|NOMBRE|NACIONALIDAD|DOCUMENTO|EXPEDIENTE"""
// `\S{0,3}` tolera la tilde de "Género" leída como otra cosa ("GÉNERO",
// "Genero", "Gènero", "Gnero").
private val PATRON_ETIQUETAS_DER_DIMEX = """G\S{0,3}NERO|SEXO|F\.?\s*NAC|EMITIDO|VENCE"""
private const val LETRA = """A-Za-zÁÉÍÓÚÑáéíóúñ"""
// Etiqueta vecina en cualquier parte del renglón: se corta desde ahí. El
// lookahead negativo evita cortar un nombre real que empiece igual
// ("GENEROSO", "VENCESLAO").
private val REGEX_CORTE_ETIQUETA_DIMEX = Regex(
    """(?:^|\s)(?:$PATRON_ETIQUETAS_IZQ_DIMEX|$PATRON_ETIQUETAS_DER_DIMEX)(?![$LETRA]).*$""",
    RegexOption.IGNORE_CASE,
)
private val REGEX_EMPIEZA_CON_ETIQUETA_IZQ_DIMEX = Regex(
    """^\s*(?:$PATRON_ETIQUETAS_IZQ_DIMEX)(?![$LETRA])""",
    RegexOption.IGNORE_CASE,
)
private val REGEX_DIMEX_ETIQUETA_APELLIDOS = Regex("""APELLIDOS\s*:?""", RegexOption.IGNORE_CASE)
private val REGEX_DIMEX_ETIQUETA_NOMBRE = Regex("""(?<![$LETRA])NOMBRE\s*:?""", RegexOption.IGNORE_CASE)
private val REGEX_PREFIJO_LETRAS = Regex("""^[$LETRA ]+""")
// Respaldo cuando "Documento No.:" y su número quedaron separados por el
// orden de lectura: el número DIMEX tiene 11-12 dígitos seguidos; el de
// expediente ("135 - 453544") nunca, y su renglón se descarta igual.
private val REGEX_DIMEX_NUMERO_SUELTO = Regex("""(?<!\d)\d{11,12}(?!\d)""")

/// Valor de texto de un campo del DIMEX: el resto del renglón de la
/// etiqueta o, si ahí no queda nada (etiqueta sola, o sólo el campo de la
/// columna derecha), los renglones siguientes -- hasta toparse con la
/// siguiente etiqueta de la columna izquierda.
private fun valorTextoDimex(texto: String, etiqueta: Regex): String? {
    val match = etiqueta.find(texto) ?: return null
    val renglones = texto.substring(match.range.last + 1).split('\n')
    for ((indice, renglon) in renglones.take(3).withIndex()) {
        if (indice > 0 && REGEX_EMPIEZA_CON_ETIQUETA_IZQ_DIMEX.containsMatchIn(renglon)) return null
        val sinVecino = renglon.replace(REGEX_CORTE_ETIQUETA_DIMEX, "").trim()
        val valor = REGEX_PREFIJO_LETRAS.find(sinVecino)?.value?.trim()
        if (!valor.isNullOrBlank() && valor.uppercase() !in VALORES_SEXO_DIMEX) return valor
    }
    return null
}

private fun numeroDimex(texto: String): String? =
    REGEX_DIMEX_NUMERO.find(texto)?.groupValues?.get(1)
        ?: REGEX_DIMEX_NUMERO_PROVISIONAL.find(texto)?.groupValues?.get(1)
        ?: texto.lineSequence()
            .filterNot { "EXPEDIENTE" in it.uppercase() }
            .firstNotNullOfOrNull { REGEX_DIMEX_NUMERO_SUELTO.find(it)?.value }

/// Extrae el número de "Documento No.:", nunca el de "Expediente No.:" --
/// ambos son números en el mismo bloque de texto, y una regex genérica sin
/// contexto de etiqueta puede agarrar el equivocado. Ver el esquema arriba.
private fun extraerDimex(texto: String): DocumentoDetectado? {
    val numero = numeroDimex(texto) ?: return null

    val nombre = valorTextoDimex(texto, REGEX_DIMEX_ETIQUETA_NOMBRE)
    val apellidos = valorTextoDimex(texto, REGEX_DIMEX_ETIQUETA_APELLIDOS)
    val vencimiento = extraerFecha(texto, etiqueta = "Vence")
        ?: extraerFecha(texto, etiqueta = "Fecha Vencimiento")

    return DocumentoDetectado(
        tipo = TipoDocumento.CEDULA_RESIDENCIA,
        numeroDocumento = numero,
        nombre = nombre,
        apellidos = apellidos,
        vencimiento = vencimiento,
    )
}

/// Frente de la cédula nacional (sin voltear al MRZ del reverso). Hasta acá
/// esta rama sólo devolvía el número -- pedido explícito del usuario
/// 2026-09-20 tras probar contra dos cédulas reales (formato nuevo con
/// orquídeas y el formato azul anterior): en ambas el nombre se veía en
/// pantalla pero nunca se guardaba porque este extractor no intentaba leer
/// nada más que `numeroDocumento`. Las dos variantes de diseño comparten
/// las mismas etiquetas de campo ("Nombre:", "1°/2° Apellido:"), así que
/// un solo extractor cubre ambas sin necesitar distinguir cuál es cuál.
/// `numeroDocumento` sigue siendo el único campo obligatorio -- nombre y
/// apellidos quedan en `null` si por ángulo/reflejo no se leyeron todavía,
/// nunca deben bloquear que se acepte la lectura por el número.
private fun extraerCedulaNacionalFrente(texto: String): DocumentoDetectado? {
    val numero = extraerCedulaDeTexto(texto) ?: return null
    val nombre = REGEX_NOMBRE_ETIQUETA.find(texto)?.groupValues?.get(1)?.trim()
    val apellido1 = REGEX_CEDULA_APELLIDO1.find(texto)?.groupValues?.get(1)?.trim()
    val apellido2 = REGEX_CEDULA_APELLIDO2.find(texto)?.groupValues?.get(1)?.trim()
    val apellidos = listOfNotNull(apellido1, apellido2).filter { it.isNotBlank() }.joinToString(" ").ifBlank { null }

    return DocumentoDetectado(
        tipo = TipoDocumento.CEDULA_NACIONAL,
        numeroDocumento = numero,
        nombre = nombre,
        apellidos = apellidos,
    )
}

/// El prefijo "DM-" es la señal de que es una licencia de extranjero (ver
/// plan, sección 3) -- se remueve del número final pero ya se usó para
/// clasificar, así que `esExtranjero` llega como parámetro ya decidido.
private fun extraerLicencia(texto: String, esExtranjero: Boolean): DocumentoDetectado? {
    val numero = REGEX_LICENCIA_NUMERO.find(texto)?.groupValues?.get(1)
        ?: REGEX_LICENCIA_DM_DIRECTO.find(texto)?.groupValues?.get(1)
        ?: return null

    val vencimiento = extraerFecha(texto, etiqueta = "Vencimiento")
    val nombreYApellidos = extraerNombreCompletoLicencia(texto)

    return DocumentoDetectado(
        tipo = if (esExtranjero) TipoDocumento.LICENCIA_EXTRANJERO else TipoDocumento.LICENCIA_NACIONAL,
        numeroDocumento = numero,
        esExtranjero = esExtranjero,
        vencimiento = vencimiento,
        nombre = nombreYApellidos?.first,
        apellidos = nombreYApellidos?.second,
    )
}

/// Orden legal costarricense en el nombre completo impreso: 1er apellido,
/// 2do apellido, nombre(s) -- por eso las primeras dos palabras son
/// siempre los apellidos y el resto es el nombre, sin importar cuántas
/// palabras tenga (un nombre compuesto como "Daniel de Jesús", tres
/// palabras, es tan válido como uno de una sola). Se toma la ÚLTIMA línea
/// candidata, no la primera: "REPUBLICA DE COSTA RICA" en el encabezado
/// también es puro texto en mayúsculas de varias palabras, así que
/// filtrar por [PALABRAS_NO_NOMBRE_LICENCIA] no alcanza sola si ML Kit
/// llega a leer el encabezado y el nombre en un orden inesperado dentro
/// del mismo bloque de texto.
private fun extraerNombreCompletoLicencia(texto: String): Pair<String, String>? {
    val candidato = texto.lines()
        .map { it.trim().uppercase() }
        .filter { linea ->
            REGEX_LICENCIA_NOMBRE_COMPLETO.matches(linea) &&
                linea.split(" ").none { it in PALABRAS_NO_NOMBRE_LICENCIA }
        }
        .lastOrNull() ?: return null

    val palabras = candidato.split(" ").filter { it.isNotBlank() }
    if (palabras.size < 3) return null
    val apellidos = palabras.take(2).joinToString(" ")
    val nombre = palabras.drop(2).joinToString(" ")
    return nombre to apellidos
}

/// El carnet PRAIND identifica a la persona por cédula (`numeroDocumento`),
/// no es un documento de identidad en sí mismo -- lo que de verdad importa
/// leer es `vencimiento`, que corresponde 1:1 al campo
/// `fecha_vencimiento_praind` que ya existe en `Contratista`. `empresa` es
/// texto libre tal cual lo imprime el carnet -- no hay forma de saber acá
/// si calza con alguna fila de `Nucleo.listarEmpresas()`, eso lo resuelve
/// quien consuma este resultado (alta de contratista con OCR). Se probó
/// contra las dos variantes de diseño reales (encabezado/pie de página
/// distintos, mismas etiquetas de campo) -- ver `LectorDocumentosIdentidadTest`.
private fun extraerPraind(texto: String): DocumentoDetectado? {
    val numero = REGEX_PRAIND_CEDULA.find(texto)?.groupValues?.get(1) ?: return null

    val nombre = REGEX_PRAIND_NOMBRE.find(texto)?.groupValues?.get(1)?.trim()
    val empresa = REGEX_PRAIND_EMPRESA.find(texto)?.groupValues?.get(1)?.trim()
    val vencimiento = REGEX_PRAIND_FECHA_VENCIMIENTO.find(texto)?.let { match ->
        val (dia, mes, anio) = match.destructured
        FechaDocumento.crearValida(dia.toInt(), mes.toInt(), anio.toInt())
    }

    return DocumentoDetectado(
        tipo = TipoDocumento.CARNET_INDUCCION_PRAIND,
        numeroDocumento = numero,
        nombre = nombre,
        empresa = empresa,
        vencimiento = vencimiento,
    )
}

private fun extraerInHouse(texto: String): DocumentoDetectado? {
    val cedula = REGEX_INHOUSE_CEDULA.find(texto)?.groupValues?.get(1)
    if (cedula != null) {
        return DocumentoDetectado(
            tipo = TipoDocumento.CARNET_IN_HOUSE,
            numeroDocumento = cedula,
        )
    }

    val nombre = extraerNombreInHouseFrente(texto) ?: return null
    return DocumentoDetectado(
        tipo = TipoDocumento.CARNET_IN_HOUSE,
        numeroDocumento = nombre,
        textoBusqueda = nombre,
        nombre = nombre,
    )
}

private fun extraerBac(texto: String): DocumentoDetectado? {
    val numero = REGEX_DIGITOS_BAC.find(texto)?.value ?: return null
    val nombre = texto.lines()
        .map { it.trim() }
        .firstOrNull { linea ->
            linea.length >= 6 &&
                linea.any(Char::isLetter) &&
                "BAC" !in linea.uppercase()
        }

    return DocumentoDetectado(
        tipo = TipoDocumento.CARNET_BAC,
        numeroDocumento = numero,
        nombre = nombre,
    )
}

private fun extraerGafeteContratista(texto: String): DocumentoDetectado? {
    val numero = REGEX_GAFETE_CONTRATISTA.find(texto)?.groupValues?.get(1) ?: return null
    return DocumentoDetectado(
        tipo = TipoDocumento.GAFETE_CONTRATISTA,
        numeroDocumento = numero,
        textoBusqueda = numero,
    )
}

private fun extraerNombreInHouseFrente(texto: String): String? {
    val lineas = texto.lines()
        .map { it.trim() }
        .filter { it.isNotEmpty() }

    val indiceContratista = lineas.indexOfFirst { "CONTRATISTA" in it.uppercase() }
    if (indiceContratista <= 0) return null

    val candidatas = lineas
        .take(indiceContratista)
        .filter { linea ->
            val mayus = linea.uppercase()
            linea.any(Char::isLetter) &&
                "EMPRESA" !in mayus &&
                "CÉDULA" !in mayus &&
                "CEDULA" !in mayus &&
                "COSTA RICA" !in mayus &&
                !linea.any(Char::isDigit)
        }
        .takeLast(2)

    val nombre = candidatas.joinToString(" ").replace(Regex("""\s+"""), " ").trim()
    return nombre.takeIf { it.length >= 6 }
}

// Una entrada por etiqueta usada ("Vence", "Vencimiento"), compilada la
// primera vez que se pide y reusada después -- mismo motivo que las demás
// constantes de arriba (esto corre en cada frame). `Regex.escape(etiqueta)`
// evita que un carácter especial de regex en la etiqueta (ninguna de las
// actuales lo tiene, pero nada garantiza que una futura no lo tenga) rompa
// el patrón o cambie su significado en vez de buscarse literal.
private val regexesPorEtiquetaFecha = listOf("Vence", "Fecha Vencimiento", "Vencimiento")
    .associateWith { etiqueta ->
        Regex(
            """${Regex.escape(etiqueta)}[:.]?\s*(\d{1,2})[-/\s](\d{1,2})[-/\s](\d{4})""",
            RegexOption.IGNORE_CASE,
        )
    }

private fun extraerFecha(texto: String, etiqueta: String): FechaDocumento? {
    val regex = regexesPorEtiquetaFecha[etiqueta] ?: return null
    val match = regex.find(texto) ?: return null
    val (dia, mes, anio) = match.destructured
    return FechaDocumento.crearValida(dia.toInt(), mes.toInt(), anio.toInt())
}

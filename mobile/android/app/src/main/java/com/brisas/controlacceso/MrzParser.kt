package com.brisas.controlacceso

/// Origen de los datos de un documento leído: el frente (texto libre, vía
/// regex por etiqueta) o el MRZ del reverso (con checksum verificable).
enum class FuenteDatos { OCR_FRENTE, MRZ }

/// Valor numérico de un carácter de MRZ según ICAO 9303: '<' = 0, dígitos
/// tal cual, letras A-Z = 10-35.
private fun valorCaracterMrz(c: Char): Int = when {
    c == '<' -> 0
    c in '0'..'9' -> c - '0'
    c in 'A'..'Z' -> c - 'A' + 10
    else -> throw IllegalArgumentException("Carácter fuera de alfabeto MRZ: '$c'")
}

/// Dígito verificador ICAO 9303: módulo 10 con pesos 7,3,1 repetidos.
fun digitoVerificadorMrz(datos: String): Int {
    var suma = 0
    for ((i, c) in datos.withIndex()) {
        val peso = intArrayOf(7, 3, 1)[i % 3]
        suma += valorCaracterMrz(c) * peso
    }
    return suma % 10
}

private fun checksumValido(datos: String, esperado: Char): Boolean =
    esperado in '0'..'9' && digitoVerificadorMrz(datos) == esperado - '0'

data class ResultadoMrz(
    val formato: String, // "TD1" | "TD3"
    // Posiciones 1-2 de la línea 1 ("código de documento" según ICAO 9303 --
    // el primer carácter puede ser A/C/I, el segundo queda a discreción del
    // Estado emisor). Deliberadamente NO se interpreta acá qué significa
    // cada valor para cada país -- eso es una regla de país, vive en
    // `ResultadoMrz.aDocumentoDetectado()`, no en este parser genérico.
    val codigoDocumento: String,
    val paisEmisor: String,
    val numeroDocumento: String,
    val apellidos: String,
    val nombres: String,
    val nacionalidad: String,
    val fechaNacimiento: FechaDocumento?,
    val sexo: Char?,
    val fechaVencimiento: FechaDocumento?,
    val checksumsValidos: Boolean,
    // true cuando la posición 15 trae un dígito (no relleno) y hay más
    // dígitos en el campo opcional, pero no calza ni el mecanismo estándar
    // de ICAO ni ninguna convención de país verificada (ver
    // `CR_DIMEX_TD1_2023` en `parsearMrzTd1`) -- en ese caso `numeroDocumento`
    // sólo trae los primeros 9 caracteres y NO debe usarse como número
    // completo.
    val numeroDocumentoExtendidoSinSoporte: Boolean = false,
)

// El estándar ICAO no fija el siglo. La decisión se calcula contra una fecha
// inyectable, nunca contra una constante anual que envejezca dentro del APK.
private fun anioCompleto(yy: Int, esNacimiento: Boolean, hoy: java.time.LocalDate): Int {
    if (!esNacimiento) return 2000 + yy
    return if (2000 + yy <= hoy.year) 2000 + yy else 1900 + yy
}

private fun parsearFechaMrz(
    yymmdd: String,
    esNacimiento: Boolean,
    hoy: java.time.LocalDate,
): FechaDocumento? {
    if (yymmdd.length != 6 || !yymmdd.all { it in '0'..'9' }) return null
    val yy = yymmdd.substring(0, 2).toInt()
    val mm = yymmdd.substring(2, 4).toInt()
    val dd = yymmdd.substring(4, 6).toInt()
    val anio = anioCompleto(yy, esNacimiento, hoy)
    return FechaDocumento.crearValida(dd, mm, anio)
}

// Compilado una sola vez -- ver nota equivalente en LectorDocumentosIdentidad.kt,
// esto también corre en cada frame mientras se busca un MRZ.
private val REGEX_ESPACIOS_MULTIPLES = Regex(" +")

private fun separarNombres(campoNombres: String): Pair<String, String> {
    val partes = campoNombres.split("<<", limit = 2)
    fun limpiar(s: String) = s.replace('<', ' ').trim().replace(REGEX_ESPACIOS_MULTIPLES, " ")
    return limpiar(partes.getOrElse(0) { "" }) to limpiar(partes.getOrElse(1) { "" })
}

/// Busca `cantidad` líneas consecutivas de exactamente `longitud` caracteres
/// del alfabeto MRZ (A-Z, 0-9, '<') dentro del texto crudo de ML Kit. El OCR
/// a veces mete espacios dentro de la MRZ -- se remueven antes de medir
/// longitud, nunca dentro de otra línea del documento (esas simplemente no
/// van a calzar con el alfabeto o la longitud esperada y quedan descartadas).
private fun buscarLineasMrz(texto: String, longitud: Int, cantidad: Int): List<String>? {
    val normalizadas = texto.lines()
        .map { it.uppercase().replace(" ", "") }
    fun esLineaMrz(linea: String): Boolean =
        linea.length == longitud && linea.all { c -> c in 'A'..'Z' || c in '0'..'9' || c == '<' }

    // Deben ser consecutivas. Filtrar primero y tomar las primeras podía
    // juntar líneas de regiones distintas del documento y fabricar un MRZ.
    return normalizadas.windowed(cantidad)
        .firstOrNull { bloque -> bloque.all(::esLineaMrz) }
}

/// TD1 (3 líneas x 30): cédula nacional 2025+, cédula de residencia (DIMEX).
fun parsearMrzTd1(texto: String, hoy: java.time.LocalDate = java.time.LocalDate.now()): ResultadoMrz? {
    val lineas = buscarLineasMrz(texto, longitud = 30, cantidad = 3) ?: return null
    val (l1, l2, l3) = Triple(lineas[0], lineas[1], lineas[2])

    val codigoDocumento = l1.substring(0, 2)
    val paisEmisor = l1.substring(2, 5)
    val bloqueNumero = l1.substring(5, 14) // 9 caracteres
    val checkNumero = l1[14]
    val opcional1 = l1.substring(15, 30) // 15 caracteres

    // Mecanismo de número extendido (ICAO 9303, TD1): cuando el número real
    // supera 9 caracteres, la posición 15 se reemplaza por el relleno '<' y
    // el resto del número continúa en el campo opcional, seguido de un
    // check digit para el número completo. Confirmado contra un caso real
    // documentado (cédulas belgas, ver issue github.com/Arg0s1080/mrz/issues/4):
    // el check digit de la extensión se calcula sobre
    // "bloque base + '<' de la posición 15 + continuación", y el número
    // completo resulta de bloque base + continuación (sin el '<').
    var numeroDocumento = bloqueNumero
    val numeroValido: Boolean
    if (checkNumero == '<') {
        val trasRelleno = opcional1.trimEnd('<')
        if (trasRelleno.isEmpty()) {
            // Extensión declarada (posición 15 = '<') pero sin continuación
            // ni check digit legibles -- MRZ incompleto, no un número normal.
            return ResultadoMrz(
                formato = "TD1", codigoDocumento = codigoDocumento, paisEmisor = paisEmisor, numeroDocumento = bloqueNumero,
                apellidos = "", nombres = "", nacionalidad = "", fechaNacimiento = null, sexo = null,
                fechaVencimiento = null, checksumsValidos = false, numeroDocumentoExtendidoSinSoporte = true,
            )
        }
        val continuacion = trasRelleno.dropLast(1)
        val checkExtendido = trasRelleno.last()
        numeroDocumento = bloqueNumero + continuacion
        numeroValido = checksumValido(bloqueNumero + checkNumero + continuacion, checkExtendido)
    } else if (paisEmisor == "CRI" && checksumValido(bloqueNumero, checkNumero) &&
        opcional1.takeWhile { it != '<' }.let { it.isNotEmpty() && it.all(Char::isDigit) }
    ) {
        // Convención costarricense del DIMEX (perfil CR_DIMEX_TD1_2023),
        // distinta del mecanismo "long document number" de ICAO: la posición
        // 15 SÍ es el check digit normal de las 9 posiciones base (no '<'),
        // y los dígitos nacionales que faltan del DIMEX de 11-12 dígitos
        // continúan sin check digit propio en el campo opcional. Verificado
        // contra un DIMEX real (155824395 + check 6 + continuación 105 =
        // 155824395105) con las 4 validaciones ICAO calzando: check del
        // bloque base, nacimiento, vencimiento y el compuesto final -- este
        // último ya cubre el campo opcional completo tal cual viene
        // impreso, así que no necesita tratamiento aparte acá.
        val continuacion = opcional1.takeWhile { it != '<' }
        numeroDocumento = bloqueNumero + continuacion
        numeroValido = true
    } else {
        // Dígito (no '<') en la posición 15 con más dígitos en el campo
        // opcional, pero sin calzar ni el mecanismo estándar de ICAO ni la
        // convención de DIMEX verificada (país distinto de CRI, o el check
        // digit del bloque base no valida) -- no se arriesga un algoritmo
        // sin poder confirmarlo.
        val pareceExtendidoNoEstandar = opcional1.takeWhile { it != '<' }.any { it.isDigit() }
        if (pareceExtendidoNoEstandar) {
            return ResultadoMrz(
                formato = "TD1", codigoDocumento = codigoDocumento, paisEmisor = paisEmisor, numeroDocumento = bloqueNumero,
                apellidos = "", nombres = "", nacionalidad = "", fechaNacimiento = null, sexo = null,
                fechaVencimiento = null, checksumsValidos = false, numeroDocumentoExtendidoSinSoporte = true,
            )
        }
        numeroValido = checksumValido(bloqueNumero, checkNumero)
    }

    val nacimientoStr = l2.substring(0, 6)
    val checkNacimiento = l2[6]
    val sexo = l2[7]
    val vencimientoStr = l2.substring(8, 14)
    val checkVencimiento = l2[14]
    val nacionalidad = l2.substring(15, 18)
    val opcional2 = l2.substring(18, 29)
    val checkCompuesto = l2[29]

    // El checksum compuesto cubre TODO el campo de línea 1 desde el número
    // hasta el final (bloque + check + opcional1 completo, 25 caracteres),
    // no sólo bloque+check -- con relleno puro esto no cambia el resultado
    // (aporta valor 0), pero con datos reales en el opcional (como en el
    // número extendido) sí importa y antes se omitía por error.
    val compuestoInput = bloqueNumero + checkNumero + opcional1 +
        nacimientoStr + checkNacimiento + vencimientoStr + checkVencimiento + opcional2

    val checksumsValidos = numeroValido &&
        checksumValido(nacimientoStr, checkNacimiento) &&
        checksumValido(vencimientoStr, checkVencimiento) &&
        checksumValido(compuestoInput, checkCompuesto)

    val (apellidos, nombres) = separarNombres(l3)

    return ResultadoMrz(
        formato = "TD1",
        codigoDocumento = codigoDocumento,
        paisEmisor = paisEmisor,
        numeroDocumento = numeroDocumento,
        apellidos = apellidos,
        nombres = nombres,
        nacionalidad = nacionalidad,
        fechaNacimiento = parsearFechaMrz(nacimientoStr, esNacimiento = true, hoy = hoy),
        sexo = sexo,
        fechaVencimiento = parsearFechaMrz(vencimientoStr, esNacimiento = false, hoy = hoy),
        checksumsValidos = checksumsValidos,
    )
}

/// TD3 (2 líneas x 44): pasaporte, formato futuro/referencia (sección 0.4
/// del plan). Los números de pasaporte rara vez exceden 9 caracteres, así
/// que el problema de número extendido de TD1 no aplica acá.
fun parsearMrzTd3(texto: String, hoy: java.time.LocalDate = java.time.LocalDate.now()): ResultadoMrz? {
    val lineas = buscarLineasMrz(texto, longitud = 44, cantidad = 2) ?: return null
    val (l1, l2) = lineas[0] to lineas[1]

    val codigoDocumento = l1.substring(0, 2)
    val paisEmisor = l1.substring(2, 5)
    val (apellidos, nombres) = separarNombres(l1.substring(5))

    val bloqueNumero = l2.substring(0, 9)
    val checkNumero = l2[9]
    val nacionalidad = l2.substring(10, 13)
    val nacimientoStr = l2.substring(13, 19)
    val checkNacimiento = l2[19]
    val sexo = l2[20]
    val vencimientoStr = l2.substring(21, 27)
    val checkVencimiento = l2[27]
    val datosPersonales = l2.substring(28, 42)
    val checkDatosPersonales = l2[42]
    val checkCompuesto = l2[43]

    val compuestoInput = bloqueNumero + checkNumero + nacimientoStr + checkNacimiento +
        vencimientoStr + checkVencimiento + datosPersonales + checkDatosPersonales

    val checksumsValidos = checksumValido(bloqueNumero, checkNumero) &&
        checksumValido(nacimientoStr, checkNacimiento) &&
        checksumValido(vencimientoStr, checkVencimiento) &&
        checksumValido(datosPersonales, checkDatosPersonales) &&
        checksumValido(compuestoInput, checkCompuesto)

    return ResultadoMrz(
        formato = "TD3",
        codigoDocumento = codigoDocumento,
        paisEmisor = paisEmisor,
        numeroDocumento = bloqueNumero.trimEnd('<'),
        apellidos = apellidos,
        nombres = nombres,
        nacionalidad = nacionalidad,
        fechaNacimiento = parsearFechaMrz(nacimientoStr, esNacimiento = true, hoy = hoy),
        sexo = sexo,
        fechaVencimiento = parsearFechaMrz(vencimientoStr, esNacimiento = false, hoy = hoy),
        checksumsValidos = checksumsValidos,
    )
}

/// Punto de entrada: intenta TD1 (cédula/DIMEX) y luego TD3 (pasaporte).
/// Devuelve `null` si no hay líneas MRZ reconocibles todavía en el texto --
/// la pantalla de escaneo debe seguir esperando más frames, no tratarlo
/// como error (ver sección 5 del plan, estabilidad de lectura).
fun leerMrzDeTexto(texto: String, hoy: java.time.LocalDate = java.time.LocalDate.now()): ResultadoMrz? =
    parsearMrzTd1(texto, hoy) ?: parsearMrzTd3(texto, hoy)

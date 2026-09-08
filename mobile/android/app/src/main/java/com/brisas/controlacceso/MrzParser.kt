package com.brisas.controlacceso

/// Origen de los datos de un documento leído: el frente (texto libre, vía
/// regex por etiqueta) o el MRZ del reverso (con checksum verificable).
enum class FuenteDatos { OCR_FRENTE, MRZ }

/// Valor numérico de un carácter de MRZ según ICAO 9303: '<' = 0, dígitos
/// tal cual, letras A-Z = 10-35.
private fun valorCaracterMrz(c: Char): Int = when {
    c == '<' -> 0
    c.isDigit() -> c - '0'
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
    esperado.isDigit() && digitoVerificadorMrz(datos) == esperado - '0'

data class ResultadoMrz(
    val formato: String, // "TD1" | "TD3"
    val paisEmisor: String,
    val numeroDocumento: String,
    val apellidos: String,
    val nombres: String,
    val nacionalidad: String,
    val fechaNacimiento: FechaDocumento?,
    val sexo: Char?,
    val fechaVencimiento: FechaDocumento?,
    val checksumsValidos: Boolean,
    // Ver nota junto a `pareceNumeroExtendido` en parsearMrzTd1: cuando es
    // true, `numeroDocumento` sólo trae los primeros 9 caracteres y NO debe
    // usarse como número completo -- falta implementar el mecanismo de
    // número extendido de ICAO 9303 antes de confiar en este campo.
    val numeroDocumentoExtendidoSinSoporte: Boolean = false,
)

// El estándar ICAO no fija el siglo de una fecha de 2 dígitos -- cada
// aplicación decide el corte. Para nacimiento asumimos que nadie escaneado
// nació en el futuro respecto al año corto actual; para vencimiento,
// siempre 20XX (ningún documento de identidad vigente vence en 19XX).
// Ajustar `ANIO_CORTO_ACTUAL` si esta heurística empieza a fallar por
// desactualización.
private const val ANIO_CORTO_ACTUAL = 26

private fun anioCompleto(yy: Int, esNacimiento: Boolean): Int {
    if (!esNacimiento) return 2000 + yy
    return if (yy > ANIO_CORTO_ACTUAL) 1900 + yy else 2000 + yy
}

private fun parsearFechaMrz(yymmdd: String, esNacimiento: Boolean): FechaDocumento? {
    if (yymmdd.length != 6 || !yymmdd.all { it.isDigit() }) return null
    val yy = yymmdd.substring(0, 2).toInt()
    val mm = yymmdd.substring(2, 4).toInt()
    val dd = yymmdd.substring(4, 6).toInt()
    if (mm !in 1..12 || dd !in 1..31) return null
    return FechaDocumento(dd, mm, anioCompleto(yy, esNacimiento))
}

private fun separarNombres(campoNombres: String): Pair<String, String> {
    val partes = campoNombres.split("<<", limit = 2)
    fun limpiar(s: String) = s.replace('<', ' ').trim().replace(Regex(" +"), " ")
    return limpiar(partes.getOrElse(0) { "" }) to limpiar(partes.getOrElse(1) { "" })
}

/// Busca `cantidad` líneas consecutivas de exactamente `longitud` caracteres
/// del alfabeto MRZ (A-Z, 0-9, '<') dentro del texto crudo de ML Kit. El OCR
/// a veces mete espacios dentro de la MRZ -- se remueven antes de medir
/// longitud, nunca dentro de otra línea del documento (esas simplemente no
/// van a calzar con el alfabeto o la longitud esperada y quedan descartadas).
private fun buscarLineasMrz(texto: String, longitud: Int, cantidad: Int): List<String>? {
    val candidatas = texto.lines()
        .map { it.uppercase().replace(" ", "") }
        .filter { it.length == longitud && it.all { c -> c in 'A'..'Z' || c.isDigit() || c == '<' } }
    if (candidatas.size < cantidad) return null
    return candidatas.take(cantidad)
}

/// TD1 (3 líneas x 30): cédula nacional 2025+, cédula de residencia (DIMEX).
fun parsearMrzTd1(texto: String): ResultadoMrz? {
    val lineas = buscarLineasMrz(texto, longitud = 30, cantidad = 3) ?: return null
    val (l1, l2, l3) = Triple(lineas[0], lineas[1], lineas[2])

    val paisEmisor = l1.substring(2, 5)
    val bloqueNumero = l1.substring(5, 14) // 9 caracteres
    val checkNumero = l1[14]
    val opcional1 = l1.substring(15, 30)

    // Mecanismo de número extendido (ICAO 9303, TD1): cuando el número real
    // supera 9 caracteres -- el caso real de las cédulas/DIMEX
    // costarricenses, con 12-13 dígitos -- la posición 15 deja de ser un
    // check digit simple y el resto del número continúa en el campo
    // opcional con su propio checksum. Ese mecanismo extendido NO está
    // implementado todavía: se detecta (heurística: aparecen dígitos justo
    // después de la posición 15, antes del primer relleno '<') y se marca
    // explícitamente en vez de calcular un checksum o un número
    // incompletos con apariencia de válidos.
    val pareceNumeroExtendido = opcional1.takeWhile { it != '<' }.any { it.isDigit() }
    if (pareceNumeroExtendido) {
        return ResultadoMrz(
            formato = "TD1", paisEmisor = paisEmisor, numeroDocumento = bloqueNumero,
            apellidos = "", nombres = "", nacionalidad = "", fechaNacimiento = null, sexo = null,
            fechaVencimiento = null, checksumsValidos = false, numeroDocumentoExtendidoSinSoporte = true,
        )
    }

    val nacimientoStr = l2.substring(0, 6)
    val checkNacimiento = l2[6]
    val sexo = l2[7]
    val vencimientoStr = l2.substring(8, 14)
    val checkVencimiento = l2[14]
    val nacionalidad = l2.substring(15, 18)
    val opcional2 = l2.substring(18, 29)
    val checkCompuesto = l2[29]

    val compuestoInput = bloqueNumero + checkNumero + nacimientoStr + checkNacimiento +
        vencimientoStr + checkVencimiento + opcional2

    val checksumsValidos = checksumValido(bloqueNumero, checkNumero) &&
        checksumValido(nacimientoStr, checkNacimiento) &&
        checksumValido(vencimientoStr, checkVencimiento) &&
        checksumValido(compuestoInput, checkCompuesto)

    val (apellidos, nombres) = separarNombres(l3)

    return ResultadoMrz(
        formato = "TD1",
        paisEmisor = paisEmisor,
        numeroDocumento = bloqueNumero,
        apellidos = apellidos,
        nombres = nombres,
        nacionalidad = nacionalidad,
        fechaNacimiento = parsearFechaMrz(nacimientoStr, esNacimiento = true),
        sexo = sexo,
        fechaVencimiento = parsearFechaMrz(vencimientoStr, esNacimiento = false),
        checksumsValidos = checksumsValidos,
    )
}

/// TD3 (2 líneas x 44): pasaporte, formato futuro/referencia (sección 0.4
/// del plan). Los números de pasaporte rara vez exceden 9 caracteres, así
/// que el problema de número extendido de TD1 no aplica acá.
fun parsearMrzTd3(texto: String): ResultadoMrz? {
    val lineas = buscarLineasMrz(texto, longitud = 44, cantidad = 2) ?: return null
    val (l1, l2) = lineas[0] to lineas[1]

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
        paisEmisor = paisEmisor,
        numeroDocumento = bloqueNumero.trimEnd('<'),
        apellidos = apellidos,
        nombres = nombres,
        nacionalidad = nacionalidad,
        fechaNacimiento = parsearFechaMrz(nacimientoStr, esNacimiento = true),
        sexo = sexo,
        fechaVencimiento = parsearFechaMrz(vencimientoStr, esNacimiento = false),
        checksumsValidos = checksumsValidos,
    )
}

/// Punto de entrada: intenta TD1 (cédula/DIMEX) y luego TD3 (pasaporte).
/// Devuelve `null` si no hay líneas MRZ reconocibles todavía en el texto --
/// la pantalla de escaneo debe seguir esperando más frames, no tratarlo
/// como error (ver sección 5 del plan, estabilidad de lectura).
fun leerMrzDeTexto(texto: String): ResultadoMrz? =
    parsearMrzTd1(texto) ?: parsearMrzTd3(texto)

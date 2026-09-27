package com.brisas.controlacceso

// Número de cédula nacional en texto crudo de OCR (sacado de
// `PantallaEscanearCedula.kt`, punto M4 de la auditoría móvil). Lógica
// pura, sin cámara: la usan los extractores de documentos.

// Compiladas una sola vez, no dentro de la función -- se llaman en cada
// frame mientras la cámara escanea (ver nota equivalente en
// LectorDocumentosIdentidad.kt).
private val REGEX_CARACTERES_NO_CEDULA = Regex("[^0-9\\n -]")
private val REGEX_CEDULA_CON_GUIONES_O_ESPACIOS = Regex("""\b\d[- ]?\d{4}[- ]?\d{4}\b""")
private val REGEX_CEDULA_PEGADA = Regex("""\b\d{9}\b""")
private val REGEX_LINEA_NUMERO_AJENO = Regex(
    """\bEXPEDIENTE\s*(?:N[O°º.]*)?""",
    RegexOption.IGNORE_CASE,
)

fun extraerCedulaDeTexto(texto: String): String? {
    val lineasCandidatas = texto
        .lines()
        .filterNot { REGEX_LINEA_NUMERO_AJENO.containsMatchIn(it) }

    for (linea in lineasCandidatas) {
        val normalizada = linea.replace(REGEX_CARACTERES_NO_CEDULA, " ")
        REGEX_CEDULA_CON_GUIONES_O_ESPACIOS
            .find(normalizada)
            ?.let { return it.value.filter(Char::isDigit) }
    }

    for (linea in lineasCandidatas) {
        val normalizada = linea.replace(REGEX_CARACTERES_NO_CEDULA, " ")
        REGEX_CEDULA_PEGADA
            .find(normalizada)
            ?.let { return it.value }
    }

    return null
}

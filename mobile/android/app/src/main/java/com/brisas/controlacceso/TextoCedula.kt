package com.brisas.controlacceso

// Número de cédula nacional en texto crudo de OCR (sacado de
// `PantallaEscanearCedula.kt`, punto M4 de la auditoría móvil). Lógica
// pura, sin cámara: la usan los extractores de documentos.

// Compiladas una sola vez, no dentro de la función -- se llaman en cada
// frame mientras la cámara escanea (ver nota equivalente en
// LectorDocumentosIdentidad.kt).
private val REGEX_CARACTERES_NO_CEDULA = Regex("[^0-9\\n -]")
// El primer dígito de una cédula es la provincia (1-9), nunca 0: así un
// número de control impreso junto a la cédula ("001234567" bajo el PDF417
// del reverso anterior) no se toma por cédula, y si hay ambos en el mismo
// texto se sigue buscando hasta la cédula real en vez de rendirse.
private val REGEX_CEDULA_CON_GUIONES_O_ESPACIOS = Regex("""\b[1-9][- ]?\d{4}[- ]?\d{4}\b""")
private val REGEX_CEDULA_PEGADA = Regex("""\b[1-9]\d{8}\b""")
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

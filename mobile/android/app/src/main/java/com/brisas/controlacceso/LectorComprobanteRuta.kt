package com.brisas.controlacceso

/// Perfil de OCR aislado para el "Comprobante de Carga de Ruta" (Coca-Cola
/// FEMSA) -- ver `docs/planes-implementados/plan-control-rutas.md`. Perfil
/// separado de `LectorDocumentosIdentidad.kt` a propósito: este documento
/// no identifica a una persona, y su forma (ruta/documento/fecha) no tiene
/// nada en común con `DocumentoDetectado` -- forzarlo dentro de ese modelo
/// obligaría a rellenar de `null` la mitad de sus campos (nombre,
/// nacionalidad, vencimiento...) que acá no aplican. Basado en 4 fotos
/// reales de comprobantes distintos compartidas por el usuario
/// (2026-09-15): mismo número de ruta, sub-número y "Transporte" (número
/// de documento) distintos por página -- ver [ComprobanteRutaDetectado].
data class ComprobanteRutaDetectado(
    /// El código antes de la barra en "Ruta / No.de Carga:" (ej. `CRR079`).
    val numeroRuta: String,
    /// El sub-número tras la barra, sin ceros a la izquierda -- `1` es la
    /// ruta PRINCIPAL, `2`/`3`/`4`... son H2/H3/H4 (recargas). Ver
    /// `DocumentoRuta.etiquetaTipo` en `RutasViewModel.kt`, mismo cómputo.
    val subNumero: Int,
    /// El campo "Transporte:" del comprobante -- es el número de documento
    /// de esa página/carga en particular, no un dato de transporte físico.
    val numeroDocumento: String,
    /// "Fecha de Entrega:" -- la fecha que dispara el bloqueo transitorio
    /// si no coincide con hoy (ver el plan). `null` si el comprobante no
    /// trae una fecha reconocible; quien llama decide si eso bloquea o no.
    val fecha: FechaDocumento?,
)

// Compiladas una sola vez a nivel de archivo -- mismo motivo que las
// constantes equivalentes en LectorDocumentosIdentidad.kt (este parser
// también puede correr una vez por frame de cámara).
//
// "Ruta / No.de Carga:" es la señal de clasificación: es la única frase
// que trae este comprobante y ningún perfil de identidad. El número de
// ruta real visto (`CRR079`) es 3 letras + 3 dígitos, pero se admite un
// rango algo más amplio (2-6 letras, 2-6 dígitos) para no atarse a un
// único formato de ruta sin haber visto más variantes reales.
private val REGEX_RUTA_NUMERO_CARGA = Regex(
    """Ruta\s*/\s*No\.?\s*de\s*Carga:?\s*([A-Z]{2,6}\d{2,6})\s*/\s*0*(\d+)""",
    RegexOption.IGNORE_CASE,
)
private val REGEX_TRANSPORTE = Regex("""Transporte:?\s*(\d{6,15})""", RegexOption.IGNORE_CASE)
// El comprobante real usa puntos como separador ("15.09.2026"), pero se
// toleran también guion/barra por si una foto futura trae otro formato.
private val REGEX_FECHA_ENTREGA = Regex(
    """Fecha\s+de\s+Entrega:?\s*(\d{1,2})[.\-/](\d{1,2})[.\-/](\d{4})""",
    RegexOption.IGNORE_CASE,
)

/// Clasificación: ¿este texto es un Comprobante de Carga de Ruta? Separado
/// de [extraerComprobanteRuta] para que la pantalla de escaneo pueda seguir
/// buscando ("¿es este tipo de documento?") sin tratar cada frame parcial
/// como un intento fallido de extracción.
fun esComprobanteCargaRuta(texto: String): Boolean = REGEX_RUTA_NUMERO_CARGA.containsMatchIn(texto)

/// Punto de entrada del perfil. Ruta y documento son obligatorios -- sin
/// alguno de los dos no hay nada útil que registrar. La fecha es la única
/// pieza opcional: su ausencia no invalida la lectura, la decisión de
/// bloquear por fecha faltante/vencida vive en la pantalla, no acá.
fun extraerComprobanteRuta(texto: String): ComprobanteRutaDetectado? {
    val matchRuta = REGEX_RUTA_NUMERO_CARGA.find(texto) ?: return null
    val (numeroRuta, subNumeroTexto) = matchRuta.destructured
    val numeroDocumento = REGEX_TRANSPORTE.find(texto)?.groupValues?.get(1) ?: return null
    val fecha = REGEX_FECHA_ENTREGA.find(texto)?.let { match ->
        val (dia, mes, anio) = match.destructured
        FechaDocumento.crearValida(dia.toInt(), mes.toInt(), anio.toInt())
    }

    return ComprobanteRutaDetectado(
        numeroRuta = numeroRuta.uppercase(),
        subNumero = subNumeroTexto.toIntOrNull() ?: 1,
        numeroDocumento = numeroDocumento,
        fecha = fecha,
    )
}

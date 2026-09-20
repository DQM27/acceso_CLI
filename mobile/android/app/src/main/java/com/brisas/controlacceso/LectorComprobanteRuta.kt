package com.brisas.controlacceso

/// Perfil de OCR aislado para el "Comprobante de Carga de Ruta" (Coca-Cola
/// FEMSA) -- ver `docs/planes-implementados/plan-control-rutas.md`. Perfil
/// separado de `LectorDocumentosIdentidad.kt` a propósito: este documento
/// no identifica a una persona, y su forma (ruta/documento) no tiene nada
/// en común con `DocumentoDetectado` -- forzarlo dentro de ese modelo
/// obligaría a rellenar de `null` la mitad de sus campos (nombre,
/// nacionalidad, vencimiento...) que acá no aplican. Basado en 4 fotos
/// reales de comprobantes distintos compartidas por el usuario
/// (2026-09-15): mismo número de ruta, sub-número y "Transporte" (número
/// de documento) distintos por página -- ver [ComprobanteRutaDetectado].
/// "Fecha de Entrega" ya no se lee acá (pedido explícito del usuario
/// 2026-09-20): sigue siendo un campo del formulario, pero se completa a
/// mano, no por OCR.
data class ComprobanteRutaDetectado(
    /// El código antes de la barra en "Ruta / No.de Carga:" (ej. `CRR079`).
    val numeroRuta: String,
    /// El sub-número tras la barra, sin ceros a la izquierda -- `1` es la
    /// ruta PRINCIPAL, `2`/`3`/`4`... son H2/H3/H4 (recargas). Ver la
    /// función privada `etiquetaSubNumero` en `PantallaRutas.kt`, mismo
    /// cómputo.
    val subNumero: Int,
    /// El campo "Transporte:" del comprobante -- es el número de documento
    /// de esa página/carga en particular, no un dato de transporte físico.
    val numeroDocumento: String,
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
// Tolera como máximo UN salto de línea entre la etiqueta y el número --
// ni "cero" (`[ \t]*` a secas resultó demasiado estricto: en el texto que
// de verdad entrega ML Kit en el A25, "Transporte:" y su valor no siempre
// caen en la misma línea reconocida, y con esa versión el escáner no
// reconocía nada) ni "cualquier cantidad" (`\s*` original: saltaba varias
// líneas hasta la tabla de materiales de abajo y agarraba el primer SKU
// de 6 dígitos, ej. `164145`, en vez del transporte real). Un salto cubre
// el caso real observado sin llegar tan lejos como la tabla, que queda
// varias líneas después (pasando primero por "Fecha de Entrega:" y
// "Camión:").
//
// El mínimo de dígitos subió de 6 a 7 (2026-09-15, tercera ronda): en las
// 4 fotos reales, el código de material (columna "Material" de la tabla,
// ej. `164145`, `167251`, `163966`) es *siempre* de exactamente 6 dígitos,
// mientras que "Transporte:" es *siempre* de 9 (`700101452`-`455`). Con
// mínimo 6 el regex seguía pudiendo confirmar un SKU como transporte
// cuando ML Kit entregaba el salto de línea real justo antes de la tabla
// en vez de antes de "Fecha de Entrega:" (layout de foto de celular, no
// de escaneo plano -- el orden de líneas que arma ML Kit no es fijo).
// Subir a 7 excluye estructuralmente todo código de material observado
// sin dejar de aceptar variantes de transporte más cortas que las 4
// vistas hasta ahora.
private val REGEX_TRANSPORTE = Regex(
    """Transporte:?[ \t]*\r?\n?[ \t]*(\d{7,15})""",
    RegexOption.IGNORE_CASE,
)

/// Clasificación: ¿este texto es un Comprobante de Carga de Ruta? Separado
/// de [extraerComprobanteRuta] para que la pantalla de escaneo pueda seguir
/// buscando ("¿es este tipo de documento?") sin tratar cada frame parcial
/// como un intento fallido de extracción.
fun esComprobanteCargaRuta(texto: String): Boolean = REGEX_RUTA_NUMERO_CARGA.containsMatchIn(texto)

/// Punto de entrada del perfil. Ruta y documento son obligatorios -- sin
/// alguno de los dos no hay nada útil que registrar. Ya no intenta leer
/// "Fecha de Entrega:" -- pedido explícito del usuario 2026-09-20: la
/// fecha se sigue completando/editando a mano en el formulario, dejar de
/// pedírsela al OCR achica lo que puede fallar en un documento que ya es
/// difícil de leer completo (mucho más grande que una tarjeta).
fun extraerComprobanteRuta(texto: String): ComprobanteRutaDetectado? {
    val matchRuta = REGEX_RUTA_NUMERO_CARGA.find(texto) ?: return null
    val (numeroRuta, subNumeroTexto) = matchRuta.destructured
    val numeroDocumento = REGEX_TRANSPORTE.find(texto)?.groupValues?.get(1) ?: return null

    return ComprobanteRutaDetectado(
        numeroRuta = numeroRuta.uppercase(),
        subNumero = subNumeroTexto.toIntOrNull() ?: 1,
        numeroDocumento = numeroDocumento,
    )
}

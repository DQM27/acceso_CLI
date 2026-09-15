package com.brisas.controlacceso

/// Perfil de OCR aislado para el paso "Placa o número de unidad" del
/// checklist de rutas (ver `docs/planes-implementados/plan-control-rutas.md`
/// y `PantallaRutas.kt`). Un solo campo de texto cubre dos datos distintos
/// a propósito -- un camión de la flota roja trae **número de unidad**
/// (calcomanía roja con dígitos blancos pegada a la carrocería, ej.
/// `22906`, foto real 2026-09-15), mientras que un camión de apoyo
/// (particular) sólo trae **placa** -- por eso [extraerVehiculo] intenta
/// ambos patrones sobre el mismo texto y devuelve el primero que calce, en
/// vez de forzar dos pantallas de escaneo separadas.
data class VehiculoRutaDetectado(
    val valor: String,
    val tipo: TipoVehiculoDetectado,
)

enum class TipoVehiculoDetectado { PLACA, NUMERO_UNIDAD }

// Formato de placa investigado (2026-09-15), NO confirmado todavía contra
// una foto real de placa -- a diferencia de LectorComprobanteRuta.kt (4
// fotos reales transcritas), esto sale de fuentes públicas sobre matrícula
// costarricense:
// - Vehículos de carga/comerciales: prefijo `C` (también existe `CL` para
//   carga liviana) + una tanda de dígitos -- ninguna fuente oficial
//   consultada especifica el largo exacto, así que se admite un rango
//   (4-6) en vez de un número fijo. PENDIENTE: validar contra una foto
//   real de la placa de un camión de la flota y ajustar el rango si hace
//   falta (mismo criterio que ya se aplicó con REGEX_TRANSPORTE).
// - Vehículos particulares (los camiones de apoyo/H, que no tienen número
//   de unidad): 3 letras + 3 dígitos, ej. `BPH485` -- este sí es el
//   formato estándar documentado para placas particulares.
// Ambas variantes toleran guion o espacio entre letras y dígitos porque
// así se ven pintadas/grabadas en la práctica.
private val REGEX_PLACA_CARGA = Regex(
    """\b(C[L]?)[\s-]?(\d{4,6})\b""",
    RegexOption.IGNORE_CASE,
)
private val REGEX_PLACA_PARTICULAR = Regex(
    """\b([A-Z]{3})[\s-]?(\d{3})\b""",
    RegexOption.IGNORE_CASE,
)

// Número de unidad: calcomanía roja con dígitos blancos pegada a la
// carrocería del camión (no un documento con etiquetas de texto) -- el
// texto que entrega ML Kit del recuadro guía es, en la práctica, casi
// sólo ese número. Única muestra real vista hasta ahora: `22906` (5
// dígitos) -- se admite 4-6 por si otra unidad trae uno más corto o más
// largo, mismo criterio que el resto de los campos numéricos de este
// perfil mientras no haya más muestras.
private val REGEX_NUMERO_UNIDAD = Regex("""\b(\d{4,6})\b""")

/// Punto de entrada del perfil. Intenta placa primero -- es el patrón más
/// específico (exige letras) -- y sólo cae a número de unidad (sólo
/// dígitos) si no hay placa reconocible, para no leer la parte numérica de
/// una placa como si fuera un número de unidad.
fun extraerVehiculo(texto: String): VehiculoRutaDetectado? {
    val textoNormalizado = texto.uppercase()
    REGEX_PLACA_CARGA.find(textoNormalizado)?.let { match ->
        val (prefijo, digitos) = match.destructured
        return VehiculoRutaDetectado("$prefijo$digitos", TipoVehiculoDetectado.PLACA)
    }
    REGEX_PLACA_PARTICULAR.find(textoNormalizado)?.let { match ->
        val (letras, digitos) = match.destructured
        return VehiculoRutaDetectado("$letras$digitos", TipoVehiculoDetectado.PLACA)
    }
    REGEX_NUMERO_UNIDAD.find(textoNormalizado)?.let { match ->
        return VehiculoRutaDetectado(match.groupValues[1], TipoVehiculoDetectado.NUMERO_UNIDAD)
    }
    return null
}

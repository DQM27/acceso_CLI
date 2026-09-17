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

// Formato de placa de carga/comercial (`C`/`CL` + dígitos) confirmado contra
// una foto real (2026-09-17, camioneta con placa `CL371931`), pero el texto
// crudo que entrega ML Kit de esa misma foto fue:
//   FIAT
//   E37 1931
//   COSTA RICA
//   CENTROAMERICA
// Dos problemas reales, no teóricos:
// 1. El prefijo `C`/`CL` va apilado en la placa física (una letra encima de
//    la otra), y ML Kit lo funde en UN solo carácter que ni siquiera es
//    consistentemente `C` -- acá salió `E`. Exigir literalmente `C`/`CL`
//    (como antes) hace que este caso nunca matchee. Se relajó a 1-2 letras
//    cualquiera -- el valor final puede traer el prefijo mal leído (queda
//    en un campo de texto editable, se corrige a mano), pero al menos ya no
//    se pierde el intento completo.
// 2. Los 6 dígitos salieron partidos "37 1931" (2+4) en vez de pegados --
//    aparentemente un artefacto de cómo ML Kit agrupa palabras, no algo
//    impreso en la placa. Antes esto hacía fallar el patrón por completo y
//    caía al respaldo de `NUMERO_UNIDAD` (que exige 4-6 dígitos *pegados*),
//    perdiendo los primeros 2 dígitos -- de ahí el reporte real de "solo me
//    reconoce los últimos números". Ahora tolera un separador opcional
//    entre dos tandas de dígitos y las junta.
private val REGEX_PLACA_CARGA = Regex(
    """\b([A-Z]{1,2})[\s-]?(\d{2,3})[\s-]?(\d{2,4})\b""",
    RegexOption.IGNORE_CASE,
)
// Vehículos particulares (los camiones de apoyo/H, que no tienen número de
// unidad): 3 letras + 3 dígitos, ej. `BPH485` -- formato estándar
// documentado, confirmado sin problemas de lectura.
private val REGEX_PLACA_PARTICULAR = Regex(
    """\b([A-Z]{3})[\s-]?(\d{3})\b""",
    RegexOption.IGNORE_CASE,
)

// Placa de moto: dos grupos de 3 caracteres (dígitos, o dígitos+letras) en
// líneas separadas de la placa física, con un prefijo `M` que a veces
// también aparece como línea propia y a veces se pierde en el ruido de
// "COSTA RICA"/"CENTROAMERICA" mal leídos. Dos fotos reales (2026-09-17):
//   CoSmCA          ->  M947369  (texto: "947" / "369" / "M" en líneas propias)
//   947
//   369
//   CENTROAMERIGA
//   M
//
//   09TA RICA       ->  807ACL  ("M" no se detectó esta vez, pero "807" y
//   807                 "ACL" sí salieron perfectos -- confirma que basta
//   ACL                 con los dos grupos de 3, el prefijo es un extra)
//   NOANA
// "COSTA RICA"/"CENTROAMERICA" salen garabateados de forma distinta cada
// vez (nunca calzan `\b[A-Z0-9]{3}\b` por ser una sola palabra larga sin
// espacio interno), así que no hace falta filtrarlos aparte -- el límite de
// palabra ya los descarta solos.
private val REGEX_GRUPO_TRIPLE_MOTO = Regex("""\b([A-Z0-9]{3})\b""")
private val REGEX_PREFIJO_MOTO = Regex("""\bM\b""")

// Número de unidad: calcomanía roja con dígitos blancos pegada a la
// carrocería del camión (no un documento con etiquetas de texto) -- el
// texto que entrega ML Kit del recuadro guía es, en la práctica, casi
// sólo ese número. Única muestra real vista hasta ahora: `22906` (5
// dígitos) -- se admite 4-6 por si otra unidad trae uno más corto o más
// largo, mismo criterio que el resto de los campos numéricos de este
// perfil mientras no haya más muestras.
private val REGEX_NUMERO_UNIDAD = Regex("""\b(\d{4,6})\b""")

/// Espejo de la explicación en `REGEX_GRUPO_TRIPLE_MOTO` -- toma los dos
/// primeros grupos de 3 caracteres que aparezcan en el texto (en orden) y
/// les antepone `M` sólo si esa letra apareció suelta en algún lado. Se
/// intenta DESPUÉS de placa de carga/particular (que exigen letras + más
/// dígitos, no calzan con esto) y ANTES de número de unidad (que por sí
/// solo nunca matchea un grupo de sólo 3 dígitos, mínimo son 4).
private fun extraerMoto(textoNormalizado: String): VehiculoRutaDetectado? {
    val grupos = REGEX_GRUPO_TRIPLE_MOTO.findAll(textoNormalizado).map { it.value }.toList()
    if (grupos.size < 2) return null
    val prefijo = if (REGEX_PREFIJO_MOTO.containsMatchIn(textoNormalizado)) "M" else ""
    return VehiculoRutaDetectado("$prefijo${grupos[0]}${grupos[1]}", TipoVehiculoDetectado.PLACA)
}

/// Punto de entrada del perfil. Intenta placa primero -- es el patrón más
/// específico (exige letras) -- y sólo cae a número de unidad (sólo
/// dígitos) si no hay placa reconocible, para no leer la parte numérica de
/// una placa como si fuera un número de unidad.
fun extraerVehiculo(texto: String): VehiculoRutaDetectado? {
    val textoNormalizado = texto.uppercase()
    REGEX_PLACA_CARGA.find(textoNormalizado)?.let { match ->
        val (prefijo, primeraTanda, segundaTanda) = match.destructured
        val digitos = primeraTanda + segundaTanda
        if (digitos.length in 4..6) {
            return VehiculoRutaDetectado("$prefijo$digitos", TipoVehiculoDetectado.PLACA)
        }
    }
    REGEX_PLACA_PARTICULAR.find(textoNormalizado)?.let { match ->
        val (letras, digitos) = match.destructured
        return VehiculoRutaDetectado("$letras$digitos", TipoVehiculoDetectado.PLACA)
    }
    extraerMoto(textoNormalizado)?.let { return it }
    REGEX_NUMERO_UNIDAD.find(textoNormalizado)?.let { match ->
        return VehiculoRutaDetectado(match.groupValues[1], TipoVehiculoDetectado.NUMERO_UNIDAD)
    }
    return null
}

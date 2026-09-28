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

/// Clave estable de un vehículo leído (lo que se vota entre frames, ver
/// `EstabilizadorPorRepeticion`) y su inversa.
fun VehiculoRutaDetectado.claveVotacion(): String = "${tipo.name}:$valor"

fun vehiculoDesdeClave(clave: String): VehiculoRutaDetectado? {
    val (tipo, valor) = clave.split(':', limit = 2).takeIf { it.size == 2 } ?: return null
    val tipoDetectado = TipoVehiculoDetectado.entries.firstOrNull { it.name == tipo } ?: return null
    return VehiculoRutaDetectado(valor, tipoDetectado).takeIf { valor.isNotBlank() }
}

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
//
// Las posiciones de dígito aceptan también las letras que el OCR confunde
// con un dígito (ver [DIGITO_POR_LETRA]); cuáles se aceptan de verdad lo
// decide [corregirGrupo], con un máximo de [MAXIMO_CORRECCIONES_POR_PLACA].
private val REGEX_PLACA_CARGA = Regex(
    """\b([A-Z]{1,2})[\s-]?([0-9ODQILZSGB]{2,3})[\s-]?([0-9ODQILZSGB]{2,4})\b""",
    RegexOption.IGNORE_CASE,
)
// Vehículos particulares (los camiones de apoyo/H, que no tienen número de
// unidad): 3 letras + 3 dígitos, ej. `BPH485` -- formato estándar
// documentado, confirmado sin problemas de lectura. Mismo criterio de
// confusiones que la de carga, en las dos direcciones: `8PH485` -> `BPH485`
// y `BPH48S` -> `BPH485`.
private val REGEX_PLACA_PARTICULAR = Regex(
    """\b([A-Z0-2568]{3})[\s-]?([0-9ODQILZSGB]{3})\b""",
    RegexOption.IGNORE_CASE,
)

// Confusiones de forma típicas del OCR entre letras y dígitos (auditoría
// OCR 2026-09-28). Una placa tiene formato fijo -- letras primero, dígitos
// después -- así que la posición dice cuál de los dos era el carácter real
// y se puede corregir sin adivinar, igual que el MRZ corrige confusables
// guiado por su checksum.
private val DIGITO_POR_LETRA = mapOf(
    'O' to '0', 'D' to '0', 'Q' to '0', 'I' to '1', 'L' to '1',
    'Z' to '2', 'S' to '5', 'G' to '6', 'B' to '8',
)
private val LETRA_POR_DIGITO = mapOf('0' to 'O', '1' to 'I', '2' to 'Z', '5' to 'S', '6' to 'G', '8' to 'B')

// Sin dígito verificador no hay forma de confirmar una corrección, así que
// se admite UNA por placa: suficiente para el carácter suelto mal leído y
// lo bastante estricto como para que una palabra rotulada o un número de
// unidad (`228051` -> "ZZB051" serían 3) nunca pase por placa.
private const val MAXIMO_CORRECCIONES_POR_PLACA = 1

/// Placa candidata y cuántos caracteres hubo que corregir para leerla.
private class PlacaCandidata(val valor: String, val correcciones: Int)

/// Lleva cada carácter de `grupo` a la clase que pide su posición
/// (`esValido`), usando `reemplazos` para las confusiones conocidas.
/// Devuelve el grupo corregido y cuántos cambios hizo, o `null` si algún
/// carácter no es válido ni tiene reemplazo.
private fun corregirGrupo(grupo: String, esValido: (Char) -> Boolean, reemplazos: Map<Char, Char>): Pair<String, Int>? {
    var correcciones = 0
    val corregido = buildString {
        for (c in grupo) {
            when {
                esValido(c) -> append(c)
                c in reemplazos -> {
                    append(reemplazos.getValue(c))
                    correcciones++
                }
                else -> return null
            }
        }
    }
    return corregido to correcciones
}

private fun esDigito(c: Char) = c in '0'..'9'
private fun esLetra(c: Char) = c in 'A'..'Z'

private fun placaDeCarga(textoNormalizado: String): PlacaCandidata? =
    REGEX_PLACA_CARGA.findAll(textoNormalizado).mapNotNull { match ->
        val (prefijo, primeraTanda, segundaTanda) = match.destructured
        val (digitos, correcciones) = corregirGrupo(primeraTanda + segundaTanda, ::esDigito, DIGITO_POR_LETRA)
            ?: return@mapNotNull null
        PlacaCandidata("$prefijo$digitos", correcciones)
            .takeIf { digitos.length in 4..6 && correcciones <= MAXIMO_CORRECCIONES_POR_PLACA }
    }.minByOrNull { it.correcciones }

private fun placaParticular(textoNormalizado: String): PlacaCandidata? =
    REGEX_PLACA_PARTICULAR.findAll(textoNormalizado).mapNotNull { match ->
        val (letrasLeidas, digitosLeidos) = match.destructured
        val (letras, correccionesLetras) = corregirGrupo(letrasLeidas, ::esLetra, LETRA_POR_DIGITO)
            ?: return@mapNotNull null
        val (digitos, correccionesDigitos) = corregirGrupo(digitosLeidos, ::esDigito, DIGITO_POR_LETRA)
            ?: return@mapNotNull null
        val correcciones = correccionesLetras + correccionesDigitos
        PlacaCandidata("$letras$digitos", correcciones).takeIf { correcciones <= MAXIMO_CORRECCIONES_POR_PLACA }
    }.minByOrNull { it.correcciones }

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
///
/// Sólo cuenta un par de grupos seguidos donde AL MENOS uno trae dígitos:
/// dos palabras de 3 letras cualesquiera rotuladas en el camión (ej. "KOF"
/// y "CRC") se tomaban antes como placa de moto -- y como esto se intenta
/// antes que el número de unidad, tapaban la calcomanía real (ej. `22906`).
/// Las dos motos reales vistas ("947"/"369" y "807"/"ACL") tienen dígitos
/// en al menos un grupo.
private fun extraerMoto(textoNormalizado: String): VehiculoRutaDetectado? {
    val grupos = REGEX_GRUPO_TRIPLE_MOTO.findAll(textoNormalizado).map { it.value }.toList()
    val (primero, segundo) = grupos.zipWithNext()
        .firstOrNull { (a, b) -> a.any(Char::isDigit) || b.any(Char::isDigit) }
        ?: return null
    val prefijo = if (REGEX_PREFIJO_MOTO.containsMatchIn(textoNormalizado)) "M" else ""
    return VehiculoRutaDetectado("$prefijo$primero$segundo", TipoVehiculoDetectado.PLACA)
}

/// Punto de entrada del perfil. Intenta placa primero -- es el patrón más
/// específico (exige letras) -- y sólo cae a número de unidad (sólo
/// dígitos) si no hay placa reconocible, para no leer la parte numérica de
/// una placa como si fuera un número de unidad.
///
/// Entre placa de carga y particular gana la que necesitó MENOS
/// correcciones (a igualdad, la de carga, como antes): así `SGB123`, leída
/// tal cual como particular, no se convierte en la de carga `SG8123`
/// (que requeriría cambiar la `B`).
fun extraerVehiculo(texto: String): VehiculoRutaDetectado? {
    val textoNormalizado = texto.uppercase()
    listOfNotNull(placaDeCarga(textoNormalizado), placaParticular(textoNormalizado))
        .minByOrNull { it.correcciones }
        ?.let { return VehiculoRutaDetectado(it.valor, TipoVehiculoDetectado.PLACA) }
    extraerMoto(textoNormalizado)?.let { return it }
    REGEX_NUMERO_UNIDAD.find(textoNormalizado)?.let { match ->
        return VehiculoRutaDetectado(match.groupValues[1], TipoVehiculoDetectado.NUMERO_UNIDAD)
    }
    return null
}

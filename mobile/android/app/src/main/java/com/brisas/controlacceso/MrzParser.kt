package com.brisas.controlacceso

import io.sentry.Sentry
import uniffi.control_acceso_mobile.CorreccionAplicada
import uniffi.control_acceso_mobile.RegistroMrz
import uniffi.control_acceso_mobile.leerMrz

/// Cuántos caracteres de más o de menos se toleran en una línea de MRZ
/// antes de descartarla. ML Kit se come o duplica `<` con frecuencia en las
/// corridas largas de relleno -- con largo exacto (como antes), UN `<`
/// perdido alcanzaba para no leer el documento en ese frame.
private const val TOLERANCIA_LARGO_MRZ = 2

/// Símbolos que ML Kit devuelve en lugar de `<` (el MRZ nunca los trae de
/// verdad). `«` es dos `<` fundidos en un glifo.
private val SUSTITUTOS_RELLENO = mapOf("«" to "<<", "»" to "<<", "‹" to "<", "›" to "<", "(" to "<", "[" to "<", "{" to "<")

/// Corridas de 2+ `K` -- lectura típica de ML Kit para `<<` (mismo trazo
/// en ángulo). OJO: el checksum NO puede distinguirlas -- `K` vale 20 y
/// `<` vale 0, iguales módulo 10 -- así que una línea con `KK` en vez de
/// `<<` valida igual y dejaba "PEREZKKMARIA" como apellido. Por eso la
/// variante con relleno se prueba PRIMERO (ver [variantesMrz]); un nombre
/// o número real con "KK" no aparece en documentos costarricenses.
private val REGEX_K_COMO_RELLENO = Regex("K{2,}")

private fun esAlfabetoMrz(c: Char): Boolean = c in 'A'..'Z' || c in '0'..'9' || c == '<'

private fun normalizarLineaMrz(linea: String): String {
    var normalizada = linea.uppercase().replace(" ", "")
    for ((sustituto, relleno) in SUSTITUTOS_RELLENO) normalizada = normalizada.replace(sustituto, relleno)
    return normalizada
}

/// Lleva `linea` a exactamente `longitud` caracteres agregando o quitando
/// `<` en su corrida de relleno más larga -- el lugar donde ML Kit pierde o
/// duplica rellenos. `conservarUltimo`: el último carácter es un dígito
/// verificador (línea 2 de TD1 y de TD3), nunca se toca. `null` si no hay
/// relleno donde ajustar. Seguro para las líneas con checksum (1 y 2 de
/// TD1, 2 de TD3): un ajuste mal ubicado no inventa un dato, hace fallar el
/// checksum compuesto.
private fun ajustarLargoMrz(linea: String, longitud: Int, conservarUltimo: Boolean): String? {
    val diferencia = longitud - linea.length
    if (diferencia == 0) return linea
    val limite = if (conservarUltimo) linea.length - 1 else linea.length
    val corrida = Regex("<+").findAll(linea.substring(0, limite)).maxByOrNull { it.value.length }
    return when {
        diferencia > 0 && corrida != null ->
            linea.substring(0, corrida.range.last + 1) + "<".repeat(diferencia) + linea.substring(corrida.range.last + 1)
        diferencia > 0 && !conservarUltimo -> linea + "<".repeat(diferencia)
        diferencia < 0 && corrida != null && corrida.value.length > -diferencia ->
            linea.removeRange(corrida.range.first, corrida.range.first - diferencia)
        else -> null
    }
}

/// Busca `cantidad` líneas CONSECUTIVAS con forma de MRZ (alfabeto A-Z,
/// 0-9, `<`, largo `longitud` ± [TOLERANCIA_LARGO_MRZ]) dentro del texto
/// crudo de ML Kit y las devuelve normalizadas a su largo exacto. Deben
/// ser consecutivas: filtrar primero y tomar las primeras podía juntar
/// líneas de regiones distintas del documento y fabricar un MRZ.
///
/// Extracción puramente mecánica ("¿cuáles líneas TIENEN forma de MRZ?"),
/// no decide nada sobre su contenido -- por eso se queda en Kotlin mientras
/// el resto (parseo, checksum, corrección de confusables) vive en Rust
/// (`leerMrz`, `mobile/rust-core/src/mrz.rs`, migración del 2026-09-25, ver
/// `docs/auditorias/auditoria-separacion-kotlin-rust-2026-09-25.md`).
private fun buscarBloquesMrz(texto: String, longitud: Int, cantidad: Int): List<List<String>> {
    val normalizadas = texto.lines().map(::normalizarLineaMrz)
    fun esCandidata(linea: String): Boolean =
        linea.length in (longitud - TOLERANCIA_LARGO_MRZ)..(longitud + TOLERANCIA_LARGO_MRZ) && linea.all(::esAlfabetoMrz)
    return normalizadas.windowed(cantidad)
        .filter { bloque -> bloque.all(::esCandidata) }
        .mapNotNull { bloque ->
            // La línea con el dígito verificador compuesto al final es la
            // segunda en ambos formatos (TD1: 2 de 3; TD3: 2 de 2).
            val ajustadas = bloque.mapIndexed { indice, linea -> ajustarLargoMrz(linea, longitud, conservarUltimo = indice == 1) }
            if (ajustadas.any { it == null }) null else ajustadas.filterNotNull()
        }
}

/// Variantes a probar de un bloque, en orden: con las corridas de `K`
/// convertidas en relleno (ver [REGEX_K_COMO_RELLENO] por qué va primero),
/// y tal cual.
private fun variantesMrz(bloque: List<String>): List<List<String>> {
    val conRelleno = bloque.map { it.replace(REGEX_K_COMO_RELLENO) { m -> "<".repeat(m.value.length) } }
    return if (conRelleno == bloque) listOf(bloque) else listOf(conRelleno, bloque)
}

/// Punto de entrada: aísla las líneas de MRZ (TD1 -- cédula/DIMEX -- antes
/// que TD3 -- pasaporte) y se las manda a Rust (`leerMrz`), que decide
/// formato, parsea, valida por checksum y corrige caracteres ambiguos (ver
/// `leer_mrz`/`corregir_campo` en `mobile/rust-core/src/mrz.rs`). Prueba
/// cada bloque candidato y sus variantes (ver [variantesMrz]) y se queda
/// con la primera lectura que valida los checksums; si ninguna valida,
/// devuelve la primera reconocida (para que la pantalla sepa que HAY un
/// MRZ en cuadro, todavía mal leído).
///
/// `null` si no hay ninguna combinación de líneas reconocible todavía -- la
/// pantalla de escaneo debe seguir esperando más frames, no tratarlo como
/// error (ver sección 5 del plan, estabilidad de lectura).
fun leerMrzDeTexto(texto: String, hoy: java.time.LocalDate = java.time.LocalDate.now()): RegistroMrz? {
    var primeraReconocida: RegistroMrz? = null
    val bloques = buscarBloquesMrz(texto, longitud = 30, cantidad = 3) + buscarBloquesMrz(texto, longitud = 44, cantidad = 2)
    for (bloque in bloques) {
        for (variante in variantesMrz(bloque)) {
            val registro = leerMrz(variante, hoy.year)
            if (!registro.formatoReconocido) continue
            if (registro.checksumsValidos) return registro
            if (primeraReconocida == null) primeraReconocida = registro
        }
    }
    return primeraReconocida
}

/// Único punto donde una corrección de confusables llega a Sentry
/// (`io.sentry:sentry-android`, ya inicializado por `AndroidManifest.xml`
/// vía meta-data -- primer uso explícito del SDK en código de esta app).
/// Rust nunca llama a Sentry directamente: no hay SDK de Sentry para Rust
/// en `mobile/rust-core` (agregarlo sería una segunda inicialización del
/// mismo DSN, sin necesidad -- ver diseño 2026-09-25), así que le entrega el
/// resultado a Kotlin y Kotlin decide qué loguear.
///
/// Se llama sólo cuando el documento terminó CONFIRMADO con al menos una
/// corrección aplicada (ver `EstabilizadorLectura.procesarFrame`) -- nunca
/// por cada frame intermedio descartado, para no generar ruido por cada
/// intento fallido mientras la persona todavía está acomodando el
/// documento frente a la cámara.
fun registrarCorreccionesMrzEnSentry(correcciones: List<CorreccionAplicada>) {
    for (correccion in correcciones) {
        Sentry.captureMessage(
            "Corrección MRZ: campo=${correccion.campo} posición=${correccion.posicion} " +
                "leído='${correccion.caracterLeido}' corregido='${correccion.caracterCorregido}'",
        )
    }
}

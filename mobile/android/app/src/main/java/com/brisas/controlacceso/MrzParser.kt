package com.brisas.controlacceso

import io.sentry.Sentry
import uniffi.control_acceso_mobile.CorreccionAplicada
import uniffi.control_acceso_mobile.RegistroMrz
import uniffi.control_acceso_mobile.leerMrz

/// Busca `cantidad` líneas consecutivas de exactamente `longitud` caracteres
/// del alfabeto MRZ (A-Z, 0-9, '<') dentro del texto crudo de ML Kit. El OCR
/// a veces mete espacios dentro de la MRZ -- se remueven antes de medir
/// longitud, nunca dentro de otra línea del documento (esas simplemente no
/// van a calzar con el alfabeto o la longitud esperada y quedan descartadas).
///
/// Extracción puramente mecánica ("¿cuáles líneas TIENEN forma de MRZ?"),
/// no decide nada sobre su contenido -- por eso se queda en Kotlin mientras
/// el resto (parseo, checksum, corrección de confusables) pasó a Rust
/// (`leerMrz`, `mobile/rust-core/src/mrz.rs`) en la migración del
/// 2026-09-25 (ver `docs/auditorias/auditoria-separacion-kotlin-rust-2026-09-25.md`).
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

/// Punto de entrada: aísla las líneas de MRZ (TD1 -- cédula/DIMEX -- antes
/// que TD3 -- pasaporte) y le manda sólo esas 2-3 líneas ya aisladas a Rust
/// (`leerMrz`), sin lógica adicional antes ni después de la llamada --
/// Rust decide formato, parsea, valida por checksum y corrige caracteres
/// ambiguos acotado (ver doc-comment de `leer_mrz`/`corregir_campo` en
/// `mobile/rust-core/src/mrz.rs`).
///
/// `null` si no hay ninguna combinación de líneas reconocible todavía -- la
/// pantalla de escaneo debe seguir esperando más frames, no tratarlo como
/// error (ver sección 5 del plan, estabilidad de lectura).
fun leerMrzDeTexto(texto: String, hoy: java.time.LocalDate = java.time.LocalDate.now()): RegistroMrz? {
    val lineas = buscarLineasMrz(texto, longitud = 30, cantidad = 3)
        ?: buscarLineasMrz(texto, longitud = 44, cantidad = 2)
        ?: return null
    return leerMrz(lineas, hoy.year).takeIf { it.formatoReconocido }
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

package com.brisas.controlacceso

import io.sentry.Sentry
import uniffi.control_acceso_mobile.CorreccionAplicada
import uniffi.control_acceso_mobile.RegistroMrz

// Aislar las líneas de MRZ dentro del texto, tolerar rellenos perdidos y
// sustitutos, parsear, validar los dígitos verificadores y corregir
// confusables vive en el núcleo Rust (`lectura_documentos/mrz_texto.rs` y
// `mrz.rs`). Acá quedan los puntos de entrada de siempre y el aviso a
// Sentry, que sólo existe del lado de Android.

/// Un MRZ encontrado en el texto de un frame: las líneas normalizadas a su
/// largo exacto (lo que se vota entre frames) y lo que se leyó de ellas.
typealias LecturaMrz = uniffi.control_acceso_mobile.LecturaMrz

/// El MRZ del texto (TD1 antes que TD3): la primera lectura que valida los
/// dígitos verificadores o, si ninguna, la primera reconocida. `null` si no
/// hay nada con forma de MRZ.
fun leerMrzDeTexto(texto: String, hoy: java.time.LocalDate = java.time.LocalDate.now()): RegistroMrz? =
    leerLecturaMrz(texto, hoy)?.registro

/// Igual que [leerMrzDeTexto], conservando también las líneas elegidas.
fun leerLecturaMrz(texto: String, hoy: java.time.LocalDate = java.time.LocalDate.now()): LecturaMrz? =
    uniffi.control_acceso_mobile.leerLecturaMrz(texto, hoy.year)

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

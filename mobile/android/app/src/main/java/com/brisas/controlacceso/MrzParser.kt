package com.brisas.controlacceso

import uniffi.control_acceso_mobile.CorreccionAplicada
import uniffi.control_acceso_mobile.RegistroMrz

// Aislar las líneas de MRZ dentro del texto, tolerar rellenos perdidos y
// sustitutos, parsear, validar los dígitos verificadores y corregir
// confusables vive en el núcleo Rust (`lectura_documentos/mrz_texto.rs` y
// `mrz.rs`). Acá quedan los puntos de entrada de siempre y el aviso de
// las correcciones a la telemetría.

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

/// Cada corrección de confusables del MRZ va a la telemetría
/// (`correccion_mrz`, grupo `escaneo`: sólo cuando el panel lo enciende para
/// calibrar el escáner). No es un error: antes iba a Sentry como mensaje.
/// Sólo el campo, la posición y el carácter leído y el corregido: un
/// carácter suelto no identifica a nadie.
///
/// Se llama sólo cuando el documento terminó CONFIRMADO con al menos una
/// corrección aplicada (ver `EstabilizadorLectura.procesarFrame`) -- nunca
/// por cada frame intermedio descartado, para no generar ruido por cada
/// intento fallido mientras la persona todavía está acomodando el
/// documento frente a la cámara.
fun registrarCorreccionesMrz(correcciones: List<CorreccionAplicada>) {
    for (correccion in correcciones) {
        Telemetria.evento(
            "correccion_mrz",
            mapOf(
                "campo" to correccion.campo.name,
                "posicion" to correccion.posicion.toInt(),
                "leido" to correccion.caracterLeido,
                "corregido" to correccion.caracterCorregido,
            ),
        )
    }
}

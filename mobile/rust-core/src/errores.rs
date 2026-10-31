//! Registro de errores del teléfono: lo que Kotlin necesita del registro del
//! núcleo (`control_acceso::errores`, siempre encendido, sin el SDK de
//! Sentry). Ver `docs/features-futuras/plan-telemetria-y-registro-de-errores.md`,
//! fase 4.
//!
//! - Pánicos del núcleo: los atrapa el gancho del registro, a disco en el
//!   momento (después `UniFFI` los convierte en una excepción de Kotlin).
//! - Errores inesperados del núcleo: `error::interno` pasa por
//!   `mensajes::mensaje_generico`, que los reporta; `error::apertura` reporta
//!   el suyo.
//! - Excepciones de Kotlin sin atrapar, ANR y fallos nativos:
//!   [`reportar_error`], desde `RegistroErrores.kt`.
//! - Cada llamada al núcleo (`medirNucleo`) queda en el rastro de operaciones
//!   ([`anotar_operacion`]) y la pantalla actual viaja con cada error
//!   ([`fijar_pantalla_errores`]).

use std::collections::{BTreeMap, HashMap};

use control_acceso::errores::{self, Configuracion, Origen};

/// Largo máximo de lo que manda Kotlin: un mensaje gigante no llena el disco.
const MAXIMO_TITULO: usize = 2_000;
/// Un volcado de ANR trae todos los hilos; el núcleo se queda con el
/// principal y guarda sólo sus marcos.
const MAXIMO_TRAZA: usize = 1024 * 1024;

/// Recorta un texto a `maximo` bytes sin partir un carácter.
fn recortar(texto: &str, maximo: usize) -> &str {
    if texto.len() <= maximo {
        return texto;
    }
    let mut fin = maximo;
    while !texto.is_char_boundary(fin) {
        fin -= 1;
    }
    &texto[..fin]
}

/// Los orígenes que puede reportar Kotlin: los pánicos y los errores del
/// núcleo los reporta el núcleo mismo.
fn origen_de_kotlin(nombre: &str) -> Option<Origen> {
    Origen::desde_nombre(nombre)
        .filter(|origen| matches!(origen, Origen::Excepcion | Origen::Anr | Origen::Nativo))
}

/// Pone en marcha el registro. Se llama al crear la `Application`, antes de
/// abrir la base. `dsn`: el de siempre (`BuildConfig.DSN_ERRORES`); vacío =
/// los errores quedan en el teléfono. Devuelve `false` si ya estaba.
#[uniffi::export]
pub fn instalar_registro_errores(
    directorio: String,
    version_app: String,
    dsn: String,
    contexto: HashMap<String, String>,
) -> bool {
    errores::instalar(Configuracion {
        directorio: directorio.into(),
        plataforma: "movil".into(),
        version_app,
        dsn,
        // Kotlin de la app, los bindings y el núcleo en Rust.
        propios: vec![
            "com.brisas.controlacceso".into(),
            "uniffi.control_acceso_mobile".into(),
            "control_acceso".into(),
        ],
        contexto: contexto.into_iter().collect::<BTreeMap<_, _>>(),
    })
}

/// Reporta un error que vio Kotlin. `origen`: `excepcion`, `anr` o `nativo`
/// (otro valor se descarta y devuelve `false`). `traza`: el texto de la traza
/// (`stackTraceToString()` o el volcado del ANR).
#[uniffi::export]
pub fn reportar_error(origen: String, titulo: String, traza: String) -> bool {
    let Some(origen) = origen_de_kotlin(&origen) else {
        return false;
    };
    errores::reportar_texto(
        origen,
        recortar(&titulo, MAXIMO_TITULO),
        recortar(&traza, MAXIMO_TRAZA),
    )
}

/// Anota una llamada al núcleo en el rastro que viaja con el próximo error.
#[uniffi::export]
pub fn anotar_operacion(nombre: String, ok: bool) {
    errores::anotar_operacion(&nombre, Some(ok));
}

/// La pantalla que está viendo quien opera.
#[uniffi::export]
pub fn fijar_pantalla_errores(nombre: String) {
    errores::fijar_pantalla(recortar(&nombre, 120));
}

/// Manda ya lo pendiente (bloqueante, fuera del hilo de la pantalla). El
/// registro lo hace solo cada minuto; Kotlin lo pide al pasar a segundo
/// plano, por si el sistema mata el proceso. Devuelve cuántos salieron.
#[uniffi::export]
pub fn enviar_errores_pendientes() -> u32 {
    errores::enviar_pendientes().map_or(0, |enviados| u32::try_from(enviados).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use control_acceso::errores::Origen;

    use super::{origen_de_kotlin, recortar};

    #[test]
    fn recorta_sin_partir_un_caracter() {
        assert_eq!(recortar("abc", 10), "abc");
        assert_eq!(recortar("cédula", 2), "c");
    }

    #[test]
    fn kotlin_solo_reporta_sus_propios_origenes() {
        assert_eq!(origen_de_kotlin("excepcion"), Some(Origen::Excepcion));
        assert_eq!(origen_de_kotlin("anr"), Some(Origen::Anr));
        assert_eq!(origen_de_kotlin("nativo"), Some(Origen::Nativo));
        assert_eq!(origen_de_kotlin("panico"), None);
        assert_eq!(origen_de_kotlin("js"), None);
        assert_eq!(origen_de_kotlin("nucleo"), None);
        assert_eq!(origen_de_kotlin("cualquiera"), None);
    }
}

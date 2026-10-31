//! Registro de errores del escritorio: instala el del núcleo
//! (`control_acceso::errores`, siempre encendido, sin el SDK de Sentry) y
//! recibe lo que manda el frontend. Ver
//! `docs/features-futuras/plan-telemetria-y-registro-de-errores.md`, fase 4.
//!
//! - Pánicos de Rust: los atrapa el gancho del núcleo, a disco en el momento.
//! - Errores inesperados del núcleo: `mensajes::mensaje_generico`.
//! - Fallos de arranque, de la sincronización automática y del splash:
//!   [`reportar`], desde `lib.rs`.
//! - Errores de JavaScript (`window.onerror`, promesas sin atrapar y los que
//!   atrapa un `ErrorBoundary`): [`reportar_error_js`], desde
//!   `desktop/src/errores.ts`.
//! - Cada comando que invoca el frontend queda en el rastro de operaciones
//!   ([`anotar_comando`], desde el despachador de `lib.rs`).

use std::collections::BTreeMap;
use std::path::Path;

use control_acceso::errores::{self, Configuracion, Origen};

/// A dónde van los errores: el receptor propio
/// (`supabase/functions-staging/receptor-errores`). Llega por la variable
/// `SENTRY_DSN` al compilar (el CI la toma del secreto del mismo nombre; el
/// nombre quedó del SDK de Sentry). No es un secreto: va dentro de la app.
///
/// Sin variable, o vacía, no se manda nada: los errores quedan en disco
/// (`errores_pendientes/`) y en el registro local.
const DSN: &str = match option_env!("SENTRY_DSN") {
    Some(dsn) => dsn,
    None => "",
};

/// Largo máximo de lo que manda el frontend (título y traza de un error de
/// JavaScript): un mensaje gigante no llena el disco.
const MAXIMO_TITULO_JS: usize = 2_000;
const MAXIMO_TRAZA_JS: usize = 16 * 1024;

/// Pone en marcha el registro. `directorio` es donde vive la base de datos
/// local; se llama antes de abrirla, así un fallo al abrir también se reporta.
pub fn instalar(directorio: &Path) {
    let ambiente = if control_acceso::nube::base_url() == crate::telemetria::URL_STAGING {
        "staging"
    } else if cfg!(debug_assertions) {
        "desarrollo"
    } else {
        "produccion"
    };
    errores::instalar(Configuracion {
        directorio: directorio.to_path_buf(),
        plataforma: "escritorio".into(),
        version_app: crate::telemetria::version_app(),
        dsn: DSN.into(),
        // Rust (`control_acceso`, `control_acceso_desktop_lib`) y el frontend
        // empaquetado (`http://tauri.localhost/assets/…`).
        propios: vec!["control_acceso".into(), "tauri.localhost".into()],
        contexto: BTreeMap::from([
            ("os".into(), std::env::consts::OS.into()),
            ("arquitectura".into(), std::env::consts::ARCH.into()),
            ("ambiente".into(), ambiente.into()),
        ]),
    });
}

/// Un error del lado Rust que no es un pánico ni pasa por
/// `mensaje_generico` (arranque, sincronización automática, splash).
pub fn reportar(titulo: &str) {
    errores::reportar(Origen::Nucleo, titulo, Vec::new());
}

/// Comandos que no se anotan: llegan cada pocos segundos (telemetría) o son
/// el propio reporte, y taparían las operaciones que importan.
fn anotable(nombre: &str) -> bool {
    !nombre.starts_with("telemetria_")
        && !matches!(nombre, "reportar_error_js" | "errores_pantalla")
}

/// Anota un comando invocado por el frontend en el rastro de operaciones.
pub fn anotar_comando(nombre: &str) {
    if anotable(nombre) {
        errores::anotar_operacion(nombre, None);
    }
}

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

/// Un error de JavaScript: título (`TypeError: …`) y la traza del navegador
/// (`error.stack`). El núcleo limpia los datos personales.
#[tauri::command]
pub fn reportar_error_js(titulo: String, traza: String) {
    errores::reportar_texto(
        Origen::Js,
        recortar(&titulo, MAXIMO_TITULO_JS),
        recortar(&traza, MAXIMO_TRAZA_JS),
    );
}

/// La pantalla que está viendo quien opera: viaja con el próximo error.
#[tauri::command]
pub fn errores_pantalla(nombre: String) {
    errores::fijar_pantalla(recortar(&nombre, 120));
}

#[cfg(test)]
mod tests {
    use super::{anotable, recortar};

    #[test]
    fn la_telemetria_y_el_propio_reporte_no_van_al_rastro() {
        assert!(anotable("registrar_ingreso"));
        assert!(!anotable("telemetria_eventos"));
        assert!(!anotable("telemetria_grupos"));
        assert!(!anotable("reportar_error_js"));
        assert!(!anotable("errores_pantalla"));
    }

    #[test]
    fn recorta_sin_partir_un_caracter() {
        assert_eq!(recortar("abc", 10), "abc");
        assert_eq!(recortar("cédula", 2), "c", "la é ocupa dos bytes");
        assert_eq!(recortar("cédula", 3), "cé");
    }
}

//! Lee las trazas que llegan como texto y las convierte en [`Marco`]s, del
//! más nuevo (donde ocurrió el error) al más viejo.
//!
//! - [`marcos_de_backtrace`]: `std::backtrace::Backtrace` de Rust.
//! - [`marcos_de_texto`]: las líneas `at …` de Java/Kotlin, de JavaScript
//!   (formato de Chromium, el del `WebView2`) y de un volcado de ANR.

use super::formato::Marco;
use super::limpieza::limpiar_ruta;

/// Marcos que se guardan por error: los más nuevos.
pub const MAXIMO_MARCOS: usize = 30;

/// Los marcos de arriba de la traza que son la maquinaria del pánico o de
/// este mismo módulo, no el lugar del error.
const RUIDO_INICIAL: [&str; 9] = [
    "std::backtrace",
    "backtrace_rs",
    "std::panicking",
    "core::panicking",
    "std::panic::",
    "rust_begin_unwind",
    "__rust",
    "control_acceso::errores",
    "control_acceso::mensajes::mensaje_generico",
];

/// ¿El marco es código nuestro? `propios` son prefijos de módulo o de
/// paquete (`control_acceso`, `com.brisas.controlacceso`…).
fn es_propio(funcion: Option<&str>, archivo: Option<&str>, propios: &[&str]) -> bool {
    propios.iter().any(|prefijo| {
        funcion.is_some_and(|f| f.starts_with(prefijo) || f.contains(&format!("<{prefijo}")))
            || archivo.is_some_and(|a| a.contains(prefijo))
    })
}

/// `ruta:linea:columna`, `ruta:linea` o `ruta` sola.
fn separar_lugar(lugar: &str) -> (String, Option<u32>) {
    let mut partes = lugar.rsplitn(3, ':');
    let ultima = partes.next().unwrap_or_default();
    let penultima = partes.next();
    let resto = partes.next();
    match (
        ultima.parse::<u32>(),
        penultima.map(str::parse::<u32>),
        resto,
    ) {
        // ruta:linea:columna
        (Ok(_), Some(Ok(linea)), Some(ruta)) => (ruta.to_owned(), Some(linea)),
        // ruta:linea
        (Ok(linea), Some(_), _) => (
            lugar
                .rsplit_once(':')
                .map_or(lugar, |(ruta, _)| ruta)
                .to_owned(),
            Some(linea),
        ),
        _ => (lugar.to_owned(), None),
    }
}

/// Saca el hash que Rust agrega al final de cada símbolo (`::h1a2b3c…`).
fn sin_hash(funcion: &str) -> &str {
    match funcion.rsplit_once("::h") {
        Some((base, hash)) if hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit()) => {
            base
        }
        _ => funcion,
    }
}

/// Quita del principio los marcos de [`RUIDO_INICIAL`], descarta los que no
/// dicen nada (un release sin símbolos da `<unknown>`) y deja los más nuevos.
fn depurar(marcos: Vec<Marco>) -> Vec<Marco> {
    marcos
        .into_iter()
        .skip_while(|marco| {
            marco
                .funcion
                .as_deref()
                .is_some_and(|f| RUIDO_INICIAL.iter().any(|ruido| f.contains(ruido)))
        })
        .filter(|marco| marco.funcion.is_some() || marco.archivo.is_some())
        .take(MAXIMO_MARCOS)
        .collect()
}

/// Los marcos de un `std::backtrace::Backtrace` impreso:
///
/// ```text
///    3: control_acceso::nube::sincronizar
///              at ./src/nube/mod.rs:120:9
/// ```
pub fn marcos_de_backtrace(texto: &str, propios: &[&str]) -> Vec<Marco> {
    let mut marcos: Vec<Marco> = Vec::new();
    for linea in texto.lines().map(str::trim) {
        if let Some(lugar) = linea.strip_prefix("at ") {
            if let Some(ultimo) = marcos.last_mut()
                && ultimo.archivo.is_none()
            {
                let (archivo, numero) = separar_lugar(lugar);
                ultimo.archivo = Some(limpiar_ruta(&archivo));
                ultimo.linea = numero;
            }
        } else if let Some((indice, funcion)) = linea.split_once(": ")
            && indice.chars().all(|c| c.is_ascii_digit())
            && !indice.is_empty()
        {
            let funcion = sin_hash(funcion.trim());
            marcos.push(Marco {
                funcion: (funcion != "<unknown>").then(|| funcion.to_owned()),
                archivo: None,
                linea: None,
                propio: false,
            });
        }
    }
    for marco in &mut marcos {
        marco.propio = es_propio(marco.funcion.as_deref(), marco.archivo.as_deref(), propios);
    }
    depurar(marcos)
}

/// Un marco `at …` de Java/Kotlin (`at pkg.Clase.metodo(Archivo.kt:12)`) o de
/// JavaScript (`at funcion (url:1:200)` o `at url:1:200`).
fn marco_de_linea(resto: &str, propios: &[&str]) -> Marco {
    let (funcion, lugar) = match resto.strip_suffix(')').and_then(|r| r.split_once('(')) {
        Some((funcion, lugar)) => (Some(funcion.trim()), Some(lugar)),
        None => (None, Some(resto)),
    };
    let (archivo, linea) = lugar
        .filter(|l| !l.is_empty() && *l != "Native Method" && *l != "Unknown Source")
        .map_or((None, None), |l| {
            let (archivo, linea) = separar_lugar(l);
            (Some(limpiar_ruta(&archivo)), linea)
        });
    let funcion = funcion.filter(|f| !f.is_empty()).map(str::to_owned);
    Marco {
        propio: es_propio(funcion.as_deref(), archivo.as_deref(), propios),
        funcion,
        archivo,
        linea,
    }
}

/// Los marcos de una traza de Java/Kotlin o de JavaScript: las líneas que
/// empiezan con `at `, hasta la primera causa (`Caused by:`), que repite
/// marcos de más abajo.
pub fn marcos_de_texto(texto: &str, propios: &[&str]) -> Vec<Marco> {
    let marcos = texto
        .lines()
        .map(str::trim)
        .take_while(|linea| !linea.starts_with("Caused by:"))
        .filter_map(|linea| linea.strip_prefix("at "))
        .map(|resto| marco_de_linea(resto, propios))
        .collect();
    depurar(marcos)
}

/// De un volcado de ANR, sólo el hilo principal: es el que estaba trabado.
/// Va desde la línea `"main" …` hasta la siguiente línea en blanco. Sin ese
/// hilo, el volcado entero.
pub fn hilo_principal_de_anr(volcado: &str) -> &str {
    let Some(inicio) = volcado
        .find("\n\"main\"")
        .map(|i| i + 1)
        .or_else(|| volcado.starts_with("\"main\"").then_some(0))
    else {
        return volcado;
    };
    let resto = &volcado[inicio..];
    resto.find("\n\n").map_or(resto, |fin| &resto[..fin])
}

#[cfg(test)]
mod tests {
    use super::{hilo_principal_de_anr, marcos_de_backtrace, marcos_de_texto};
    use crate::errores::formato::Marco;

    const PROPIOS: [&str; 3] = [
        "control_acceso",
        "com.brisas.controlacceso",
        "tauri.localhost",
    ];

    fn marco(
        funcion: Option<&str>,
        archivo: Option<&str>,
        linea: Option<u32>,
        propio: bool,
    ) -> Marco {
        Marco {
            funcion: funcion.map(str::to_owned),
            archivo: archivo.map(str::to_owned),
            linea,
            propio,
        }
    }

    #[test]
    fn el_backtrace_de_rust_sin_la_maquinaria_del_panico() {
        let texto = "   0: std::backtrace_rs::backtrace::libunwind::trace
             at /rustc/abc/library/std/src/../../backtrace/src/backtrace/libunwind.rs:116:5
   1: std::panicking::begin_panic_handler::{{closure}}
             at /rustc/abc/library/std/src/panicking.rs:665:13
   2: control_acceso::nube::sincronizar::h0123456789abcdef
             at ./src/nube/mod.rs:120:9
   3: tokio::runtime::task::harness::poll
             at C:\\Users\\daniel\\.cargo\\registry\\src\\tokio-1.53.3\\src\\runtime\\task\\harness.rs:20:5
   4: <unknown>";
        assert_eq!(
            marcos_de_backtrace(texto, &PROPIOS),
            vec![
                marco(
                    Some("control_acceso::nube::sincronizar"),
                    Some("./src/nube/mod.rs"),
                    Some(120),
                    true
                ),
                marco(
                    Some("tokio::runtime::task::harness::poll"),
                    Some(
                        r"C:\Users\<usuario>\.cargo\registry\src\tokio-1.53.3\src\runtime\task\harness.rs"
                    ),
                    Some(20),
                    false
                ),
            ]
        );
    }

    #[test]
    fn un_release_sin_simbolos_no_deja_marcos() {
        assert!(marcos_de_backtrace("   0: <unknown>\n   1: <unknown>", &PROPIOS).is_empty());
    }

    #[test]
    fn la_traza_de_kotlin_hasta_la_primera_causa() {
        let texto = "java.lang.IllegalStateException: sin cámara
	at com.brisas.controlacceso.Escaner.abrir(Escaner.kt:42)
	at android.os.Handler.dispatchMessage(Handler.java:106)
	at java.lang.reflect.Method.invoke(Native Method)
Caused by: java.io.IOException: algo
	at com.brisas.controlacceso.Otra.cosa(Otra.kt:9)";
        assert_eq!(
            marcos_de_texto(texto, &PROPIOS),
            vec![
                marco(
                    Some("com.brisas.controlacceso.Escaner.abrir"),
                    Some("Escaner.kt"),
                    Some(42),
                    true
                ),
                marco(
                    Some("android.os.Handler.dispatchMessage"),
                    Some("Handler.java"),
                    Some(106),
                    false
                ),
                marco(Some("java.lang.reflect.Method.invoke"), None, None, false),
            ]
        );
    }

    #[test]
    fn la_traza_de_javascript_con_y_sin_nombre_de_funcion() {
        let texto = "TypeError: x is undefined
    at Formulario (http://tauri.localhost/assets/index-abc.js:12:3456)
    at http://tauri.localhost/assets/vendor-def.js:1:200";
        assert_eq!(
            marcos_de_texto(texto, &PROPIOS),
            vec![
                marco(
                    Some("Formulario"),
                    Some("http://tauri.localhost/assets/index-abc.js"),
                    Some(12),
                    true
                ),
                marco(
                    None,
                    Some("http://tauri.localhost/assets/vendor-def.js"),
                    Some(1),
                    true
                ),
            ]
        );
    }

    #[test]
    fn del_volcado_de_anr_solo_el_hilo_principal() {
        let volcado = "----- pid 123 -----
\"Signal Catcher\" daemon prio=10
  at dalvik.system.Otra.cosa(Otra.java:1)

\"main\" prio=5 tid=1 Blocked
  | group=\"main\" sCount=1
  at com.brisas.controlacceso.Base.leer(Base.kt:77)
  - waiting to lock <0x0abc> held by thread 12
  native: #00 pc 000000000004e8b8  /system/lib64/libc.so
  at android.os.Looper.loop(Looper.java:288)

\"hilo-2\" prio=5
  at x.Y.z(Y.kt:1)";
        let principal = hilo_principal_de_anr(volcado);
        assert!(principal.starts_with("\"main\""));
        assert_eq!(
            marcos_de_texto(principal, &PROPIOS),
            vec![
                marco(
                    Some("com.brisas.controlacceso.Base.leer"),
                    Some("Base.kt"),
                    Some(77),
                    true
                ),
                marco(
                    Some("android.os.Looper.loop"),
                    Some("Looper.java"),
                    Some(288),
                    false
                ),
            ]
        );
    }

    #[test]
    fn un_volcado_sin_hilo_principal_queda_entero() {
        assert_eq!(hilo_principal_de_anr("sin hilos"), "sin hilos");
    }

    #[test]
    fn se_guardan_como_mucho_treinta_marcos() {
        let texto = (0..50)
            .map(|i| format!("at com.brisas.controlacceso.A.m{i}(A.kt:{i})"))
            .collect::<Vec<_>>()
            .join("\n");
        let marcos = marcos_de_texto(&texto, &PROPIOS);
        assert_eq!(marcos.len(), 30);
        assert_eq!(marcos[0].linea, Some(0), "los más nuevos");
    }
}

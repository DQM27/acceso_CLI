//! El gancho de pánicos: arma el reporte y lo escribe a disco en el momento,
//! antes de seguir con el gancho anterior (que imprime el pánico y, en el
//! escritorio, anota la telemetría). No manda nada por la red: el pánico
//! puede ocurrir dentro de un hilo de tokio, y el proceso puede morir
//! enseguida. Sale en la siguiente vuelta de envío o en el próximo arranque.

use std::any::Any;
use std::backtrace::Backtrace;
use std::panic::PanicHookInfo;
use std::path::Path;

use super::formato::{Marco, Origen};
use super::limpieza::limpiar_ruta;
use super::traza::marcos_de_backtrace;

pub fn instalar_gancho() {
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        reportar(info);
        anterior(info);
    }));
}

fn reportar(info: &PanicHookInfo<'_>) {
    let Some(registro) = super::REGISTRO.get() else {
        return;
    };
    let lugar = info.location().map(|l| (l.file(), l.line()));
    let mensaje = mensaje_del_panico(info.payload());
    registro.reportar_sin_bloquear(Origen::Panico, &format!("panic: {mensaje}"), || {
        let backtrace = Backtrace::force_capture().to_string();
        marcos_del_panico(lugar, &backtrace, &registro.propios())
    });
}

/// El texto con que se llamó a `panic!` (o el de un `unwrap`/`expect`).
pub fn mensaje_del_panico(carga: &(dyn Any + Send)) -> &str {
    carga
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| carga.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("(sin mensaje)")
}

/// ¿El archivo es de este repositorio? Rust da las rutas de los crates
/// propios relativas (`src/nube/mod.rs`) y las de las dependencias y la
/// librería estándar absolutas (`.cargo/registry/…`, `/rustc/…`).
fn lugar_propio(archivo: &str) -> bool {
    !Path::new(archivo).is_absolute()
        && !archivo.starts_with('/')
        && !archivo.contains(".cargo")
        && !archivo.contains("rustc")
}

/// Los marcos de un pánico: primero el lugar exacto donde ocurrió (siempre
/// está, aunque el release no tenga símbolos), después la traza.
pub fn marcos_del_panico(
    lugar: Option<(&str, u32)>,
    backtrace: &str,
    propios: &[&str],
) -> Vec<Marco> {
    let mut marcos = marcos_de_backtrace(backtrace, propios);
    if let Some((archivo, linea)) = lugar {
        let archivo = limpiar_ruta(archivo);
        let repetido = marcos.first().is_some_and(|m| {
            m.linea == Some(linea)
                && m.archivo
                    .as_deref()
                    .is_some_and(|a| a.ends_with(archivo.as_str()))
        });
        if !repetido {
            marcos.insert(
                0,
                Marco {
                    funcion: None,
                    propio: lugar_propio(&archivo),
                    archivo: Some(archivo),
                    linea: Some(linea),
                },
            );
            marcos.truncate(super::traza::MAXIMO_MARCOS);
        }
    }
    marcos
}

#[cfg(test)]
mod tests {
    use super::{marcos_del_panico, mensaje_del_panico};
    use crate::errores::formato::Marco;

    #[test]
    fn el_mensaje_sale_de_un_str_o_de_un_string() {
        assert_eq!(mensaje_del_panico(&"se rompió"), "se rompió");
        assert_eq!(mensaje_del_panico(&String::from("índice 3")), "índice 3");
        assert_eq!(mensaje_del_panico(&5_u8), "(sin mensaje)");
    }

    #[test]
    fn sin_simbolos_queda_al_menos_el_lugar() {
        let marcos = marcos_del_panico(
            Some(("src/nube/mod.rs", 120)),
            "   0: <unknown>",
            &["control_acceso"],
        );
        assert_eq!(
            marcos,
            vec![Marco {
                funcion: None,
                archivo: Some("src/nube/mod.rs".into()),
                linea: Some(120),
                propio: true,
            }]
        );
    }

    #[test]
    fn el_lugar_de_una_dependencia_no_es_propio() {
        let marcos = marcos_del_panico(
            Some((
                r"C:\Users\daniel\.cargo\registry\src\tokio-1.53.3\src\lib.rs",
                51,
            )),
            "",
            &["control_acceso"],
        );
        assert!(!marcos[0].propio);
        assert_eq!(
            marcos[0].archivo.as_deref(),
            Some(r"C:\Users\<usuario>\.cargo\registry\src\tokio-1.53.3\src\lib.rs")
        );
    }

    #[test]
    fn el_lugar_no_se_repite_si_la_traza_ya_empieza_ahi() {
        let traza = "   0: control_acceso::nube::sincronizar
             at ./src/nube/mod.rs:120:9";
        let marcos = marcos_del_panico(Some(("src/nube/mod.rs", 120)), traza, &["control_acceso"]);
        assert_eq!(marcos.len(), 1);
        assert_eq!(
            marcos[0].funcion.as_deref(),
            Some("control_acceso::nube::sincronizar")
        );
    }
}

//! Quita de un texto lo que puede identificar a una persona o dar acceso.
//! Las mismas reglas que `limpiarTexto` del receptor
//! (`supabase/functions-staging/receptor-errores/logica.ts`): dos defensas,
//! porque un mensaje de error puede traer una cédula sin que nadie lo decida.

use std::sync::LazyLock;

use regex::Regex;

static REGLAS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    [
        // Tokens: JWT y cadenas largas de base64/hex (también se lleva los UUID).
        (r"eyJ[\w-]+\.[\w-]+\.[\w-]+", "<token>"),
        (r"[A-Za-z0-9+/_-]{32,}={0,2}", "<token>"),
        (r"[\w.+-]+@[\w-]+(\.[\w-]+)+", "<correo>"),
        // Rutas con el usuario del sistema.
        (
            r#"(?i)([A-Z]:[\\/]+Users[\\/]+)[^\\/\s"']+"#,
            "${1}<usuario>",
        ),
        (r#"(/(?:home|Users)/)[^/\s"']+"#, "${1}<usuario>"),
        // Cédula con guiones (1-0123-0456) y números largos (cédula, DIMEX, tarjetas).
        (r"\b\d-\d{4}-\d{4}\b", "<num>"),
        (r"\b\d{7,}\b", "<num>"),
        // Teléfonos de 8 dígitos (8888-8888, 8888 8888).
        (r"\b\d{4}[- ]\d{4}\b", "<num>"),
        // Placas (ABC123, ABC-1234).
        (r"\b[A-Z]{3}[- ]?\d{3,4}\b", "<placa>"),
        // Los datos de una URL van en la query.
        (r#"\?[^\s"')]+"#, "?<datos>"),
    ]
    .into_iter()
    .filter_map(|(patron, reemplazo)| Regex::new(patron).ok().map(|regla| (regla, reemplazo)))
    .collect()
});

/// Las reglas de las rutas con el usuario del sistema, dentro de [`REGLAS`].
const REGLAS_DE_RUTA: std::ops::Range<usize> = 3..5;

fn aplicar(texto: &str, reglas: &[(Regex, &'static str)]) -> String {
    reglas
        .iter()
        .fold(texto.to_owned(), |actual, (regla, reemplazo)| {
            regla.replace_all(&actual, *reemplazo).into_owned()
        })
}

/// El texto sin tokens, correos, usuarios del sistema en las rutas, cédulas,
/// teléfonos, placas ni datos de URL.
pub fn limpiar_texto(texto: &str) -> String {
    aplicar(texto, &REGLAS)
}

/// La ruta de un archivo de la traza sin el usuario del sistema. Sólo eso:
/// la regla de tokens se comería las rutas largas del código.
pub fn limpiar_ruta(ruta: &str) -> String {
    aplicar(ruta, REGLAS.get(REGLAS_DE_RUTA).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::{REGLAS, limpiar_ruta, limpiar_texto};

    #[test]
    fn todas_las_reglas_compilan() {
        assert_eq!(REGLAS.len(), 10);
    }

    #[test]
    fn quita_cedulas_telefonos_y_numeros_largos() {
        assert_eq!(
            limpiar_texto("cédula 1-0123-0456 y 112340567"),
            "cédula <num> y <num>"
        );
        assert_eq!(limpiar_texto("llamar al 8888-1234"), "llamar al <num>");
    }

    #[test]
    fn quita_correos_tokens_y_placas() {
        assert_eq!(
            limpiar_texto("de ana@correo.com con placa ABC-123"),
            "de <correo> con placa <placa>"
        );
        assert_eq!(
            limpiar_texto("Bearer eyJhbGciOi.eyJzdWIiOi.firma_x"),
            "Bearer <token>"
        );
        assert_eq!(
            limpiar_texto("id 550e8400e29b41d4a716446655440000 fin"),
            "id <token> fin"
        );
    }

    #[test]
    fn quita_el_usuario_de_las_rutas_y_los_datos_de_la_url() {
        assert_eq!(
            limpiar_texto(r"C:\Users\daniel\AppData\x.db"),
            r"C:\Users\<usuario>\AppData\x.db"
        );
        assert_eq!(
            limpiar_texto("/home/daniel/.cargo/x.rs"),
            "/home/<usuario>/.cargo/x.rs"
        );
        assert_eq!(
            limpiar_texto("GET https://x.co/rest/v1/ingresos?cedula=eq.1 falló"),
            "GET https://x.co/rest/v1/ingresos?<datos> falló"
        );
    }

    #[test]
    fn la_ruta_solo_pierde_el_usuario() {
        assert_eq!(
            limpiar_ruta(r"C:\Users\daniel\.cargo\registry\src\tokio-1.53.3\src\lib.rs"),
            r"C:\Users\<usuario>\.cargo\registry\src\tokio-1.53.3\src\lib.rs"
        );
        let propia = "desktop/src-tauri/src/comandos/nube.rs";
        assert_eq!(
            limpiar_ruta(propia),
            propia,
            "la regla de tokens no se aplica"
        );
    }

    #[test]
    fn deja_el_resto_como_esta() {
        let texto = "called `Option::unwrap()` on a `None` value at src/nube/mod.rs:120";
        assert_eq!(limpiar_texto(texto), texto);
    }
}

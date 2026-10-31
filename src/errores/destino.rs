//! A dónde se mandan los errores. Sale del mismo DSN que usaba el SDK de
//! Sentry (variable `SENTRY_DSN` al compilar, secreto del mismo nombre en el
//! CI), así no hace falta un secreto nuevo:
//!
//! ```text
//! https://<clave>@<proyecto>.supabase.co/functions/v1/receptor-errores/1
//! ```
//!
//! da la dirección `https://<proyecto>.supabase.co/functions/v1/receptor-errores/v1/errores`
//! y la clave `<clave>`, que viaja en `x-clave-errores`. La clave no es un
//! secreto (va dentro de las apps): sólo identifica al emisor.

use reqwest::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destino {
    pub url: String,
    pub clave: String,
}

impl Destino {
    /// El destino de ese DSN; `None` si está vacío o no tiene la forma
    /// esperada (sin destino, los errores quedan en disco y en el registro
    /// local).
    pub fn desde_dsn(dsn: &str) -> Option<Self> {
        let mut url = Url::parse(dsn.trim()).ok()?;
        if !matches!(url.scheme(), "https" | "http") || url.host_str().is_none() {
            return None;
        }
        let clave = url.username().to_owned();
        if clave.is_empty() {
            return None;
        }
        // El último tramo del camino es el número de proyecto de Sentry.
        let mut tramos: Vec<String> = url.path_segments()?.map(str::to_owned).collect();
        tramos.pop();
        tramos.retain(|tramo| !tramo.is_empty());
        url.set_username("").ok()?;
        url.set_password(None).ok()?;
        url.set_query(None);
        url.set_path(&format!("{}/v1/errores", tramos.join("/")));
        Some(Self {
            url: url.into(),
            clave,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Destino;

    #[test]
    fn el_dsn_del_receptor_da_su_direccion_y_su_clave() {
        assert_eq!(
            Destino::desde_dsn("https://abc123@xyz.supabase.co/functions/v1/receptor-errores/1"),
            Some(Destino {
                url: "https://xyz.supabase.co/functions/v1/receptor-errores/v1/errores".into(),
                clave: "abc123".into(),
            })
        );
    }

    #[test]
    fn sin_dsn_o_sin_clave_no_hay_destino() {
        assert_eq!(Destino::desde_dsn(""), None);
        assert_eq!(Destino::desde_dsn("   "), None);
        assert_eq!(Destino::desde_dsn("https://xyz.supabase.co/x/1"), None);
        assert_eq!(Destino::desde_dsn("ftp://abc@xyz.co/1"), None);
        assert_eq!(Destino::desde_dsn("no es un dsn"), None);
    }
}

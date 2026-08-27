//! Preferencia persistente de qué interfaz abrir por defecto al arrancar sin
//! flags — la lee `main.rs` antes de decidir la ruta de arranque, y la
//! escriben los dos gestos que cambian de interfaz "para siempre": el
//! comando `/clasico` de `--comandos` y la opción "Modo comandos" del Menú
//! Principal de la TUI clásica. Mismo directorio de datos que
//! `comandos::preferencias` (`%LOCALAPPDATA%\ControlAcceso`), archivo propio.
//!
//! Los flags `--tui-clasica`/`--comandos` siguen siendo overrides puntuales
//! de un solo arranque (ver `main.rs`): ni leen ni escriben este archivo, así
//! que probar la otra interfaz una vez no cambia el default sin querer.

use std::{fs, path::PathBuf};

const FILE_NAME: &str = "interfaz-preferida.conf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interfaz {
    Clasica,
    Comandos,
}

impl Interfaz {
    fn como_texto(self) -> &'static str {
        match self {
            Self::Clasica => "clasica",
            Self::Comandos => "comandos",
        }
    }

    fn desde_texto(texto: &str) -> Option<Self> {
        match texto.trim() {
            "clasica" => Some(Self::Clasica),
            "comandos" => Some(Self::Comandos),
            _ => None,
        }
    }
}

fn ruta() -> Option<PathBuf> {
    let root = std::env::var_os(crate::database::connection::LOCAL_APP_DATA_ENV)?;
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return None;
    }
    Some(root.join("ControlAcceso").join(FILE_NAME))
}

/// `None` sin preferencia guardada (archivo ausente, contenido irreconocible
/// o sin `%LOCALAPPDATA%` disponible) — el llamador cae al default vigente
/// (hoy, `--comandos`).
pub fn leer() -> Option<Interfaz> {
    Interfaz::desde_texto(&fs::read_to_string(ruta()?).ok()?)
}

/// Nunca falla el flujo que la llama (mismo criterio que
/// `comandos::preferencias`): sin `%LOCALAPPDATA%` o sin permiso de
/// escritura, sigue como si no se hubiera llamado — el peor caso es que la
/// próxima vez arranque en la interfaz de siempre en vez de la recién
/// elegida, nunca un error visible a mitad de un reinicio.
pub fn guardar(interfaz: Interfaz) {
    let Some(ruta) = ruta() else {
        return;
    };
    if let Some(padre) = ruta.parent() {
        let _ = fs::create_dir_all(padre);
    }
    let _ = fs::write(ruta, interfaz.como_texto());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desde_texto_reconoce_los_dos_valores() {
        assert_eq!(Interfaz::desde_texto("clasica"), Some(Interfaz::Clasica));
        assert_eq!(Interfaz::desde_texto("comandos"), Some(Interfaz::Comandos));
    }

    #[test]
    fn desde_texto_recorta_espacios_y_saltos_de_linea() {
        assert_eq!(
            Interfaz::desde_texto("  clasica\n"),
            Some(Interfaz::Clasica)
        );
    }

    #[test]
    fn desde_texto_rechaza_contenido_desconocido() {
        assert_eq!(Interfaz::desde_texto("otracosa"), None);
        assert_eq!(Interfaz::desde_texto(""), None);
    }
}

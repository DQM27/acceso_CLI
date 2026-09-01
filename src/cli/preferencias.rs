//! Persistencia de preferencias de `--cli` (hoy sólo columnas visibles
//! por tabla). Archivo propio (`cli-preferencias.conf`), independiente
//! de `src/tui/preferences.rs` — mismo directorio de datos de la app
//! (`%LOCALAPPDATA%\ControlAcceso`, vía `database::connection`, que no es
//! exclusivo de la TUI clásica) pero sin tocar ni importar ese módulo
//! (DEC-002/DEC-014).

use std::{fs, io, path::PathBuf};

const FILE_NAME: &str = "cli-preferencias.conf";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preferencias {
    pub columnas_busqueda: String,
    pub columnas_activos: String,
    pub columnas_historial: String,
}

impl Preferencias {
    fn parse(content: &str) -> Self {
        let mut preferencias = Self::default();
        for line in content.lines() {
            let Some((clave, valor)) = line.split_once('=') else {
                continue;
            };
            match clave.trim() {
                "columnas_busqueda" => preferencias.columnas_busqueda = valor.trim().to_owned(),
                "columnas_activos" => preferencias.columnas_activos = valor.trim().to_owned(),
                "columnas_historial" => preferencias.columnas_historial = valor.trim().to_owned(),
                _ => {}
            }
        }
        preferencias
    }

    fn serialize(&self) -> String {
        format!(
            "version=1\ncolumnas_busqueda={}\ncolumnas_activos={}\ncolumnas_historial={}\n",
            self.columnas_busqueda, self.columnas_activos, self.columnas_historial
        )
    }
}

#[derive(Debug)]
pub struct PreferenciasStore {
    path: PathBuf,
    actual: Preferencias,
}

impl PreferenciasStore {
    pub fn load_default() -> Option<Self> {
        let root = std::env::var_os(crate::database::connection::LOCAL_APP_DATA_ENV)?;
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return None;
        }
        Some(Self::load(root.join("ControlAcceso").join(FILE_NAME)))
    }

    fn load(path: PathBuf) -> Self {
        let actual = fs::read_to_string(&path)
            .map(|contenido| Preferencias::parse(&contenido))
            .unwrap_or_default();
        Self { path, actual }
    }

    pub fn actual(&self) -> &Preferencias {
        &self.actual
    }

    /// Escribe sólo cuando cambió algo. Un archivo incompleto o ausente
    /// nunca impide arrancar: el lector ignora líneas inválidas y conserva
    /// los defaults (todas las columnas visibles).
    pub fn guardar_si_cambio(&mut self, preferencias: Preferencias) -> io::Result<bool> {
        if preferencias == self.actual {
            return Ok(false);
        }
        if let Some(padre) = self.path.parent() {
            fs::create_dir_all(padre)?;
        }
        fs::write(&self.path, preferencias.serialize())?;
        self.actual = preferencias;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ruta_temporal(sufijo: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "cli-preferencias-test-{}-{sufijo}.conf",
            std::process::id()
        ))
    }

    #[test]
    fn archivo_ausente_carga_defaults() {
        let store = PreferenciasStore::load(ruta_temporal("ausente"));
        assert_eq!(store.actual(), &Preferencias::default());
    }

    #[test]
    fn guardar_y_recargar_conserva_los_valores() {
        let ruta = ruta_temporal("roundtrip");
        let _ = fs::remove_file(&ruta);
        let mut store = PreferenciasStore::load(ruta.clone());
        let cambiado = store
            .guardar_si_cambio(Preferencias {
                columnas_busqueda: "nombre,tipo".to_string(),
                columnas_activos: "gafete,nombre".to_string(),
                columnas_historial: "ingreso_col,nombre".to_string(),
            })
            .unwrap();
        assert!(cambiado);

        let recargado = PreferenciasStore::load(ruta.clone());
        assert_eq!(recargado.actual().columnas_busqueda, "nombre,tipo");
        assert_eq!(recargado.actual().columnas_activos, "gafete,nombre");
        assert_eq!(recargado.actual().columnas_historial, "ingreso_col,nombre");
        let _ = fs::remove_file(&ruta);
    }

    #[test]
    fn guardar_sin_cambios_no_reescribe() {
        let ruta = ruta_temporal("sin-cambios");
        let _ = fs::remove_file(&ruta);
        let mut store = PreferenciasStore::load(ruta.clone());
        let actual = store.actual().clone();
        assert!(!store.guardar_si_cambio(actual).unwrap());
        assert!(!ruta.exists());
    }

    #[test]
    fn contenido_corrupto_cae_a_defaults_sin_panico() {
        let ruta = ruta_temporal("corrupto");
        fs::write(&ruta, "esto no es clave=valor\n\n===").unwrap();
        let store = PreferenciasStore::load(ruta.clone());
        assert_eq!(store.actual(), &Preferencias::default());
        let _ = fs::remove_file(&ruta);
    }
}

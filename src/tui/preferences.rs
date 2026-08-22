use std::{fs, io, path::PathBuf};

use super::ui_kit::ThemePreset;

const FILE_NAME: &str = "ui-preferences.conf";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPreferences {
    pub theme: ThemePreset,
    pub activos_columns: String,
    pub contratistas_columns: String,
    pub historial_view: String,
    pub historial_columns: String,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            theme: ThemePreset::Brisas,
            activos_columns: String::new(),
            contratistas_columns: String::new(),
            historial_view: "timeline".to_owned(),
            historial_columns: String::new(),
        }
    }
}

impl UiPreferences {
    fn parse(content: &str) -> Self {
        let mut preferences = Self::default();
        for line in content.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "theme" => {
                    if let Some(theme) = ThemePreset::from_key(value.trim()) {
                        preferences.theme = theme;
                    }
                }
                "activos_columns" => preferences.activos_columns = value.trim().to_owned(),
                "contratistas_columns" => {
                    preferences.contratistas_columns = value.trim().to_owned()
                }
                "historial_view" if matches!(value.trim(), "timeline" | "classic") => {
                    preferences.historial_view = value.trim().to_owned()
                }
                "historial_columns" => preferences.historial_columns = value.trim().to_owned(),
                _ => {}
            }
        }
        preferences
    }

    fn serialize(&self) -> String {
        format!(
            "version=1\ntheme={}\nactivos_columns={}\ncontratistas_columns={}\nhistorial_view={}\nhistorial_columns={}\n",
            self.theme.key(),
            self.activos_columns,
            self.contratistas_columns,
            self.historial_view,
            self.historial_columns,
        )
    }
}

#[derive(Debug)]
pub struct PreferencesStore {
    path: PathBuf,
    current: UiPreferences,
}

impl PreferencesStore {
    pub fn load_default() -> Option<Self> {
        let root = std::env::var_os(crate::database::connection::LOCAL_APP_DATA_ENV)?;
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return None;
        }
        Some(Self::load(root.join("ControlAcceso").join(FILE_NAME)))
    }

    fn load(path: PathBuf) -> Self {
        let current = fs::read_to_string(&path)
            .map(|content| UiPreferences::parse(&content))
            .unwrap_or_default();
        Self { path, current }
    }

    pub fn current(&self) -> &UiPreferences {
        &self.current
    }

    /// Escribe sólo cuando cambió alguna preferencia. Un archivo incompleto
    /// nunca impide arrancar: el lector ignora líneas inválidas y conserva
    /// valores predeterminados.
    pub fn save_if_changed(&mut self, preferences: UiPreferences) -> io::Result<bool> {
        if preferences == self.current {
            return Ok(false);
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, preferences.serialize())?;
        self.current = preferences;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "brisas-ui-preferences-{}-{}.conf",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("reloj válido")
                .as_nanos()
        ))
    }

    #[test]
    fn persiste_y_recupera_preferencias_validas() {
        let path = path();
        let mut store = PreferencesStore::load(path.clone());
        let preferences = UiPreferences {
            theme: ThemePreset::Negro,
            activos_columns: "nombre,gafete".into(),
            contratistas_columns: "nombre,empresa".into(),
            historial_view: "classic".into(),
            historial_columns: "fecha,nombre".into(),
        };

        assert!(store.save_if_changed(preferences.clone()).unwrap());
        assert!(!store.save_if_changed(preferences.clone()).unwrap());
        assert_eq!(PreferencesStore::load(path.clone()).current, preferences);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn archivo_invalido_cae_a_valores_seguros() {
        let parsed = UiPreferences::parse("theme=desconocido\nhistorial_view=otra\n");
        assert_eq!(parsed, UiPreferences::default());
    }

    #[test]
    fn preferencia_antigua_de_heatmap_migra_a_timeline() {
        let parsed = UiPreferences::parse("historial_view=heatmap\n");
        assert_eq!(parsed.historial_view, "timeline");
    }
}

//! Almacenamiento local del secreto de este dispositivo (ver
//! `docs/plan-persistencia-nube.md`).
//!
//! En texto plano a propósito por ahora: es el mismo criterio que ya rige el
//! resto de la base local, que tampoco tiene cifrado en reposo todavía (ver
//! memoria del proyecto "Cifrado en reposo" — `SQLCipher` se descartó, la
//! decisión sigue pendiente). Cuando se resuelva eso para el resto de los
//! datos, este archivo entra en el mismo paquete, no antes por separado.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

const FILE_NAME: &str = "dispositivo-nube.secret";

/// Resuelve `%LOCALAPPDATA%\ControlAcceso`. `None` si la variable de
/// entorno no está definida o no es una ruta absoluta — mismo criterio de
/// `PreferencesStore::load_default`.
fn directorio_default() -> Option<PathBuf> {
    let root = std::env::var_os(crate::database::connection::LOCAL_APP_DATA_ENV)?;
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return None;
    }
    Some(root.join("ControlAcceso"))
}

/// Sólo válido en escritorio, donde `%LOCALAPPDATA%` existe. En Android no
/// hay esa variable de entorno — el lado móvil usa
/// [`guardar_secreto_en`]/[`cargar_secreto_en`] con el directorio que ya le
/// pasa Kotlin para abrir la base `SQLite` (mismo criterio, misma carpeta).
pub fn guardar_secreto(secreto: &str) -> io::Result<()> {
    let directorio = directorio_default()
        .ok_or_else(|| io::Error::other("no se pudo resolver %LOCALAPPDATA%"))?;
    guardar_secreto_en(&directorio, secreto)
}

/// Ver [`guardar_secreto`] sobre por qué esta versión es sólo de escritorio.
#[must_use]
pub fn cargar_secreto() -> Option<String> {
    cargar_secreto_en(&directorio_default()?)
}

/// Guarda el secreto en `<directorio>/dispositivo-nube.secret`. Es la
/// primitiva que usan tanto [`guardar_secreto`] (escritorio, resuelve el
/// directorio solo) como el lado móvil (recibe el directorio de Kotlin).
pub fn guardar_secreto_en(directorio: &Path, secreto: &str) -> io::Result<()> {
    fs::create_dir_all(directorio)?;
    fs::write(directorio.join(FILE_NAME), secreto.trim())
}

/// Ver [`guardar_secreto_en`].
#[must_use]
pub fn cargar_secreto_en(directorio: &Path) -> Option<String> {
    let ruta = directorio.join(FILE_NAME);
    let contenido = fs::read_to_string(ruta).ok()?;
    let secreto = contenido.trim();
    if secreto.is_empty() {
        None
    } else {
        Some(secreto.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn directorio_temporal() -> PathBuf {
        std::env::temp_dir().join(format!(
            "control-acceso-dispositivo-nube-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("reloj válido")
                .as_nanos()
        ))
    }

    #[test]
    fn guarda_y_recupera_el_secreto() {
        let directorio = directorio_temporal();

        guardar_secreto_en(&directorio, "s3cr3t0-de-prueba").expect("se guarda");
        let recuperado = cargar_secreto_en(&directorio);

        assert_eq!(recuperado.as_deref(), Some("s3cr3t0-de-prueba"));
        let _ = fs::remove_dir_all(directorio);
    }

    #[test]
    fn recorta_espacios_y_saltos_de_linea_al_guardar() {
        let directorio = directorio_temporal();

        guardar_secreto_en(&directorio, "  s3cr3t0  \n").expect("se guarda");
        let recuperado = cargar_secreto_en(&directorio);

        assert_eq!(recuperado.as_deref(), Some("s3cr3t0"));
        let _ = fs::remove_dir_all(directorio);
    }

    #[test]
    fn ausente_es_none_no_error() {
        let directorio = directorio_temporal();

        assert_eq!(cargar_secreto_en(&directorio), None);
    }
}

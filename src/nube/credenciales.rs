//! Almacenamiento local del secreto de este dispositivo (ver
//! `docs/plan-persistencia-nube.md`).
//!
//! Sin el feature `cifrado-secreto-dispositivo-portable` sigue en texto
//! plano a propósito, mismo criterio que ya rige el resto de la base local
//! (ver memoria del proyecto "Cifrado en reposo" — `SQLCipher` se descartó,
//! esa decisión general sigue pendiente y es aparte de esta). Con el feature
//! activo el archivo se cifra con una clave derivada de un identificador de
//! dispositivo -- ver `docs/plan-panel-administrativo-web.md`, "Protección
//! del secreto del dispositivo en reposo". Cada plataforma resuelve ese
//! identificador a su manera: escritorio solo, vía el Machine GUID de
//! Windows (feature `cifrado-secreto-dispositivo`, ver [`guardar_secreto`]);
//! móvil lo recibe de Kotlin (`Settings.Secure.ANDROID_ID`, ver
//! [`guardar_secreto_en_con_identificador`]) porque Android no tiene un
//! equivalente al registro de Windows.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

const FILE_NAME: &str = "dispositivo-nube.secret";

#[cfg(feature = "cifrado-secreto-dispositivo-portable")]
mod cifrado {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use rand_core::{OsRng, RngCore};
    use sha2::{Digest, Sha256};

    /// Antepuesto al contenido cifrado -- distingue un archivo ya cifrado de
    /// uno viejo en texto plano (de antes de este feature, o escrito por un
    /// dispositivo que todavía no lo activa) sin necesitar un
    /// intento-y-error de descifrado para saberlo.
    const MAGIC: &[u8; 4] = b"BAE1";
    const LARGO_NONCE: usize = 12;

    fn clave(identificador: &str) -> Key<Aes256Gcm> {
        let hash = Sha256::digest(identificador.as_bytes());
        Key::<Aes256Gcm>::try_from(hash.as_slice()).expect("Sha256 produce 32 bytes exactos")
    }

    /// `None` sólo si el cifrado en sí falla (no debería, con una clave de
    /// 32 bytes válida) -- quien llama cae de vuelta a guardar en texto
    /// plano antes que perder el secreto.
    pub(super) fn cifrar(secreto: &str, identificador: &str) -> Option<Vec<u8>> {
        let cipher = Aes256Gcm::new(&clave(identificador));
        let mut nonce_bytes = [0u8; LARGO_NONCE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::try_from(nonce_bytes.as_slice())
            .expect("LARGO_NONCE es el tamaño exacto del nonce");
        let cifrado = cipher.encrypt(&nonce, secreto.as_bytes()).ok()?;

        let mut salida = Vec::with_capacity(MAGIC.len() + LARGO_NONCE + cifrado.len());
        salida.extend_from_slice(MAGIC);
        salida.extend_from_slice(&nonce_bytes);
        salida.extend_from_slice(&cifrado);
        Some(salida)
    }

    /// `true` si `contenido` viene de [`cifrar`] -- lo que no matchea (texto
    /// plano legado, o un archivo de otro dispositivo que todavía no activa
    /// este feature) queda para el camino de texto plano de siempre.
    pub(super) fn es_cifrado(contenido: &[u8]) -> bool {
        contenido.starts_with(MAGIC)
    }

    /// `None` si el archivo está corrupto, o (el caso que motiva todo esto)
    /// `contenido` se cifró con un identificador distinto -- la clave
    /// derivada no calza y el `tag` de GCM no valida, sin distinción posible
    /// entre esos dos casos ni falta que hace.
    pub(super) fn descifrar(contenido: &[u8], identificador: &str) -> Option<String> {
        let resto = contenido.strip_prefix(MAGIC)?;
        if resto.len() < LARGO_NONCE {
            return None;
        }
        let (nonce_bytes, cifrado) = resto.split_at(LARGO_NONCE);
        let nonce = Nonce::try_from(nonce_bytes).ok()?;
        let cipher = Aes256Gcm::new(&clave(identificador));
        let plano = cipher.decrypt(&nonce, cifrado).ok()?;
        String::from_utf8(plano).ok()
    }
}

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

/// `None` si no se pudo leer el Machine GUID de Windows (registro
/// inaccesible, permisos) -- [`guardar_secreto_en`]/[`cargar_secreto_en`]
/// caen a texto plano en ese caso antes que perder el secreto; un
/// dispositivo real con Windows corrupto al punto de no poder leer su
/// propio registro ya tiene problemas más grandes que éste.
#[cfg(feature = "cifrado-secreto-dispositivo")]
fn identificador_de_esta_maquina() -> Option<String> {
    machine_uid::get().ok()
}

/// Sólo válido en escritorio, donde `%LOCALAPPDATA%` existe. En Android no
/// hay esa variable de entorno — el lado móvil usa
/// [`guardar_secreto_en_con_identificador`]/[`cargar_secreto_en_con_identificador`]
/// con el directorio que ya le pasa Kotlin para abrir la base `SQLite`
/// (mismo criterio, misma carpeta) y con `Settings.Secure.ANDROID_ID` como
/// identificador.
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

/// Guarda el secreto en `<directorio>/dispositivo-nube.secret`, cifrado con
/// una clave derivada del Machine GUID de esta máquina (feature
/// `cifrado-secreto-dispositivo`, sólo escritorio -- ver
/// [`guardar_secreto_en_con_identificador`] para el equivalente móvil).
pub fn guardar_secreto_en(directorio: &Path, secreto: &str) -> io::Result<()> {
    #[cfg(feature = "cifrado-secreto-dispositivo")]
    if let Some(identificador) = identificador_de_esta_maquina() {
        return guardar_secreto_en_con_identificador(directorio, secreto, &identificador);
    }
    guardar_secreto_sin_cifrar(directorio, secreto)
}

/// Ver [`guardar_secreto_en`]. Lee tanto un archivo cifrado por esta misma
/// máquina (feature activo) como uno en texto plano legado.
#[must_use]
pub fn cargar_secreto_en(directorio: &Path) -> Option<String> {
    #[cfg(feature = "cifrado-secreto-dispositivo")]
    if let Some(identificador) = identificador_de_esta_maquina() {
        return cargar_secreto_en_con_identificador(directorio, &identificador);
    }
    cargar_secreto_sin_cifrar(directorio)
}

/// Igual que [`guardar_secreto_en`], pero con un identificador de
/// dispositivo explícito en vez de resolverlo internamente -- lo que usa el
/// lado móvil, donde ese identificador (`Settings.Secure.ANDROID_ID`) sólo
/// Kotlin puede leerlo. Sin el feature `cifrado-secreto-dispositivo-portable`
/// activo, `identificador` se ignora y el comportamiento es texto plano de
/// siempre -- misma lógica que ya tenía [`guardar_secreto_en`], para que
/// quien llama (`application::nube`) no tenga que conocer el feature.
pub fn guardar_secreto_en_con_identificador(
    directorio: &Path,
    secreto: &str,
    #[cfg_attr(
        not(feature = "cifrado-secreto-dispositivo-portable"),
        allow(unused_variables)
    )]
    identificador: &str,
) -> io::Result<()> {
    fs::create_dir_all(directorio)?;
    let secreto = secreto.trim();
    #[cfg(feature = "cifrado-secreto-dispositivo-portable")]
    if let Some(cifrado) = cifrado::cifrar(secreto, identificador) {
        return fs::write(directorio.join(FILE_NAME), cifrado);
    }
    fs::write(directorio.join(FILE_NAME), secreto)
}

/// Ver [`guardar_secreto_en_con_identificador`]. Lee tanto un archivo
/// cifrado con este mismo identificador como uno en texto plano legado.
#[must_use]
pub fn cargar_secreto_en_con_identificador(
    directorio: &Path,
    #[cfg_attr(
        not(feature = "cifrado-secreto-dispositivo-portable"),
        allow(unused_variables)
    )]
    identificador: &str,
) -> Option<String> {
    let contenido = fs::read(directorio.join(FILE_NAME)).ok()?;
    #[cfg(feature = "cifrado-secreto-dispositivo-portable")]
    if cifrado::es_cifrado(&contenido) {
        return cifrado::descifrar(&contenido, identificador);
    }
    secreto_de_texto_plano(contenido)
}

/// Sin ningún feature de cifrado activo -- texto plano, como siempre.
fn guardar_secreto_sin_cifrar(directorio: &Path, secreto: &str) -> io::Result<()> {
    fs::create_dir_all(directorio)?;
    fs::write(directorio.join(FILE_NAME), secreto.trim())
}

/// Ver [`guardar_secreto_sin_cifrar`].
fn cargar_secreto_sin_cifrar(directorio: &Path) -> Option<String> {
    secreto_de_texto_plano(fs::read(directorio.join(FILE_NAME)).ok()?)
}

fn secreto_de_texto_plano(contenido: Vec<u8>) -> Option<String> {
    let secreto = String::from_utf8(contenido).ok()?;
    let secreto = secreto.trim();
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

    #[test]
    fn un_archivo_en_texto_plano_legado_se_sigue_leyendo() {
        // Simula un secreto guardado por una versión anterior a este
        // feature (o por un dispositivo que todavía no lo activa) -- no
        // debe quedar huérfano cuando esta misma carpeta la abre una
        // versión con cifrado activo.
        let directorio = directorio_temporal();
        fs::create_dir_all(&directorio).expect("se crea el directorio");
        fs::write(directorio.join(FILE_NAME), "secreto-de-antes\n").expect("se escribe a mano");

        assert_eq!(
            cargar_secreto_en(&directorio).as_deref(),
            Some("secreto-de-antes")
        );
        let _ = fs::remove_dir_all(directorio);
    }

    #[cfg(feature = "cifrado-secreto-dispositivo")]
    #[test]
    fn con_el_feature_activo_el_archivo_en_disco_no_queda_en_texto_plano() {
        let directorio = directorio_temporal();

        guardar_secreto_en(&directorio, "s3cr3t0-de-prueba").expect("se guarda");
        let crudo = fs::read(directorio.join(FILE_NAME)).expect("se lee el archivo crudo");

        assert!(cifrado::es_cifrado(&crudo));
        assert!(
            !crudo
                .windows(b"s3cr3t0-de-prueba".len())
                .any(|v| v == b"s3cr3t0-de-prueba")
        );
        let _ = fs::remove_dir_all(directorio);
    }

    #[cfg(feature = "cifrado-secreto-dispositivo-portable")]
    #[test]
    fn guarda_y_recupera_con_un_identificador_explicito() {
        let directorio = directorio_temporal();

        guardar_secreto_en_con_identificador(&directorio, "s3cr3t0-movil", "android-id-de-prueba")
            .expect("se guarda");
        let recuperado = cargar_secreto_en_con_identificador(&directorio, "android-id-de-prueba");

        assert_eq!(recuperado.as_deref(), Some("s3cr3t0-movil"));
        let _ = fs::remove_dir_all(directorio);
    }

    #[cfg(feature = "cifrado-secreto-dispositivo-portable")]
    #[test]
    fn un_identificador_distinto_no_descifra() {
        // El caso real que motiva todo esto: el archivo se copió (backup,
        // clonado, robo) a un dispositivo con otro ANDROID_ID/Machine GUID.
        let directorio = directorio_temporal();

        guardar_secreto_en_con_identificador(&directorio, "s3cr3t0-movil", "id-original")
            .expect("se guarda");
        let recuperado = cargar_secreto_en_con_identificador(&directorio, "id-de-otro-dispositivo");

        assert_eq!(recuperado, None);
        let _ = fs::remove_dir_all(directorio);
    }
}

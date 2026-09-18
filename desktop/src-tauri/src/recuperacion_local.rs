//! Reconstrucción de un sitio cuya base de datos local no se pudo abrir
//! (corrupción real, o la clave no se pudo recuperar) -- ver
//! `docs/recuperacion-sitio-local.md`. Quien llama (`lib.rs::preparar_nucleo`)
//! decide CUÁNDO ofrecer esto (tras confirmación del usuario); este módulo
//! sólo mueve archivos.
//!
//! El secreto de dispositivo (`dispositivo-nube.secret`) NO se toca acá --
//! desde que vive en `%APPDATA%`, separado de la base (ver
//! `clave_cifrado.rs`), sigue intacto en la mayoría de los casos que
//! motivan esto (disco/perfil de la base dañado, no un reinstall completo
//! del sistema), y dejarlo permite que el dispositivo se reactive con su
//! propio secreto de siempre, sin que un administrador tenga que emitir
//! uno nuevo.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::clave_cifrado::NOMBRE_ARCHIVO_CLAVE;

const CARPETA_CUARENTENA: &str = "respaldos-corruptos";

/// Sufijo de desempate para el nombre de la subcarpeta de cuarentena --
/// en producción esto corre a lo sumo una vez por arranque, pero dos
/// corridas en el mismo milisegundo (visto en los tests) pisarían la
/// carpeta anterior sin esto.
static CONTADOR_CUARENTENA: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, thiserror::Error)]
pub enum ErrorReinicio {
    #[error("no se pudo borrar {ruta}: {origen}", ruta = .ruta.display())]
    NoSePudoBorrar {
        ruta: PathBuf,
        #[source]
        origen: std::io::Error,
    },
}

/// Copia `control_acceso.db` (en `ruta_base_datos`) y `db_key.dat` (en
/// `directorio_credenciales`, si existe) a una carpeta de cuarentena nueva
/// y con marca de tiempo, dentro de la misma carpeta que la base
/// (`<carpeta de la base>/respaldos-corruptos/<timestamp>/`) -- para que
/// alguien pueda intentar rescatarlos después con herramientas de
/// recuperación de `SQLite` (no hay garantía de que sirva, pero copiarlos
/// no cuesta nada y tirarlos sin mirar sí). Ambos van juntos a propósito:
/// sin la clave, la base en cuarentena es sólo un blob cifrado inútil.
///
/// Después borra los originales, para que el próximo intento de abrir la
/// base la trate como un sitio nunca activado (mismo camino que una
/// instalación nueva). El respaldo es "mejor esfuerzo": si copiar falla
/// (permisos, disco lleno), sigue igual con el borrado -- perder la copia
/// de seguridad es peor que no tenerla, pero no debe bloquear que el sitio
/// pueda volver a arrancar. Borrar los originales sí es obligatorio: si no
/// se puede, el próximo intento de abrir volvería a toparse con el mismo
/// archivo dañado.
pub fn poner_en_cuarentena_y_reiniciar(
    ruta_base_datos: &Path,
    directorio_credenciales: &Path,
) -> Result<(), ErrorReinicio> {
    let ruta_clave = directorio_credenciales.join(NOMBRE_ARCHIVO_CLAVE);
    respaldar_mejor_esfuerzo(ruta_base_datos, directorio_credenciales);

    if ruta_base_datos.exists() {
        fs::remove_file(ruta_base_datos).map_err(|origen| ErrorReinicio::NoSePudoBorrar {
            ruta: ruta_base_datos.to_path_buf(),
            origen,
        })?;
    }
    if ruta_clave.exists() {
        fs::remove_file(&ruta_clave).map_err(|origen| ErrorReinicio::NoSePudoBorrar {
            ruta: ruta_clave.clone(),
            origen,
        })?;
    }
    Ok(())
}

/// Separada de `poner_en_cuarentena_y_reiniciar` sólo para que ésta quede
/// corta (tope de líneas de Clippy) -- sin lógica propia más allá de
/// ignorar errores de copia a propósito (ver el doc-comment de arriba).
fn respaldar_mejor_esfuerzo(ruta_base_datos: &Path, directorio_credenciales: &Path) {
    let Some(carpeta_base) = ruta_base_datos.parent() else {
        return;
    };
    let marca_tiempo = chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f");
    let contador = CONTADOR_CUARENTENA.fetch_add(1, Ordering::Relaxed);
    let cuarentena = carpeta_base
        .join(CARPETA_CUARENTENA)
        .join(format!("{marca_tiempo}-{contador}"));
    if fs::create_dir_all(&cuarentena).is_err() {
        return;
    }

    if ruta_base_datos.exists() {
        let _ = fs::copy(
            ruta_base_datos,
            cuarentena.join(
                ruta_base_datos
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("control_acceso.db")),
            ),
        );
    }
    let ruta_clave = directorio_credenciales.join(NOMBRE_ARCHIVO_CLAVE);
    if ruta_clave.exists() {
        let _ = fs::copy(&ruta_clave, cuarentena.join(NOMBRE_ARCHIVO_CLAVE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escribir(ruta: &Path, contenido: &[u8]) {
        if let Some(padre) = ruta.parent() {
            fs::create_dir_all(padre).unwrap();
        }
        fs::write(ruta, contenido).unwrap();
    }

    #[test]
    fn respalda_ambos_archivos_y_borra_los_originales() {
        let raiz = tempfile::tempdir().unwrap();
        let carpeta_base = raiz.path().join("base");
        let carpeta_credenciales = raiz.path().join("credenciales");
        let ruta_base_datos = carpeta_base.join("control_acceso.db");
        escribir(&ruta_base_datos, b"base corrupta");
        escribir(
            &carpeta_credenciales.join(NOMBRE_ARCHIVO_CLAVE),
            b"clave protegida",
        );

        poner_en_cuarentena_y_reiniciar(&ruta_base_datos, &carpeta_credenciales).unwrap();

        assert!(!ruta_base_datos.exists());
        assert!(!carpeta_credenciales.join(NOMBRE_ARCHIVO_CLAVE).exists());

        let cuarentena = carpeta_base.join(CARPETA_CUARENTENA);
        let subcarpetas: Vec<_> = fs::read_dir(&cuarentena)
            .unwrap()
            .map(|entrada| entrada.unwrap().path())
            .collect();
        assert_eq!(subcarpetas.len(), 1);
        assert_eq!(
            fs::read(subcarpetas[0].join("control_acceso.db")).unwrap(),
            b"base corrupta"
        );
        assert_eq!(
            fs::read(subcarpetas[0].join(NOMBRE_ARCHIVO_CLAVE)).unwrap(),
            b"clave protegida"
        );
    }

    #[test]
    fn sin_clave_todavia_igual_respalda_y_borra_la_base() {
        let raiz = tempfile::tempdir().unwrap();
        let carpeta_base = raiz.path().join("base");
        let carpeta_credenciales = raiz.path().join("credenciales");
        let ruta_base_datos = carpeta_base.join("control_acceso.db");
        escribir(&ruta_base_datos, b"base corrupta sin clave recuperable");

        poner_en_cuarentena_y_reiniciar(&ruta_base_datos, &carpeta_credenciales).unwrap();

        assert!(!ruta_base_datos.exists());
        let cuarentena = carpeta_base.join(CARPETA_CUARENTENA);
        let subcarpeta = fs::read_dir(&cuarentena)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert!(subcarpeta.join("control_acceso.db").exists());
        assert!(!subcarpeta.join(NOMBRE_ARCHIVO_CLAVE).exists());
    }

    #[test]
    fn dos_cuarentenas_seguidas_no_se_pisan() {
        let raiz = tempfile::tempdir().unwrap();
        let carpeta_base = raiz.path().join("base");
        let carpeta_credenciales = raiz.path().join("credenciales");
        let ruta_base_datos = carpeta_base.join("control_acceso.db");

        escribir(&ruta_base_datos, b"primer intento");
        poner_en_cuarentena_y_reiniciar(&ruta_base_datos, &carpeta_credenciales).unwrap();

        escribir(&ruta_base_datos, b"segundo intento");
        poner_en_cuarentena_y_reiniciar(&ruta_base_datos, &carpeta_credenciales).unwrap();

        let cuarentena = carpeta_base.join(CARPETA_CUARENTENA);
        let subcarpetas: Vec<_> = fs::read_dir(&cuarentena)
            .unwrap()
            .map(|entrada| entrada.unwrap().path())
            .collect();
        assert_eq!(subcarpetas.len(), 2);
    }
}

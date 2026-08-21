use std::{
    ffi::OsString,
    fs::{File, OpenOptions, TryLockError},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum InstanciaError {
    #[error(
        "Control de Acceso ya está abierto para la base de datos {}",
        .base_datos.display()
    )]
    YaEstaEnEjecucion { base_datos: PathBuf },
    #[error("No se pudo preparar el bloqueo de instancia {}: {origen}", .ruta.display())]
    Bloqueo {
        ruta: PathBuf,
        #[source]
        origen: io::Error,
    },
}

/// Mantiene un bloqueo exclusivo mientras la aplicación usa una base de datos.
///
/// La existencia del archivo no significa que la aplicación esté abierta. El bloqueo
/// pertenece al sistema operativo y se libera automáticamente al cerrar este `File`,
/// incluso cuando el proceso termina de forma inesperada.
#[derive(Debug)]
#[must_use = "el bloqueo se libera al descartar el guard"]
pub struct InstanciaGuard {
    _archivo: File,
}

impl InstanciaGuard {
    pub fn adquirir(base_datos: impl AsRef<Path>) -> Result<Self, InstanciaError> {
        let base_datos = base_datos.as_ref();
        let ruta = ruta_bloqueo(base_datos);
        let archivo = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&ruta)
            .map_err(|origen| InstanciaError::Bloqueo {
                ruta: ruta.clone(),
                origen,
            })?;

        match archivo.try_lock() {
            Ok(()) => Ok(Self { _archivo: archivo }),
            Err(TryLockError::WouldBlock) => Err(InstanciaError::YaEstaEnEjecucion {
                base_datos: base_datos.to_path_buf(),
            }),
            Err(TryLockError::Error(origen)) => Err(InstanciaError::Bloqueo { ruta, origen }),
        }
    }
}

fn ruta_bloqueo(base_datos: &Path) -> PathBuf {
    let mut ruta = OsString::from(base_datos.as_os_str());
    ruta.push(".instance.lock");
    PathBuf::from(ruta)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static SECUENCIA: AtomicU64 = AtomicU64::new(0);

    fn base_temporal(nombre: &str) -> PathBuf {
        let numero = SECUENCIA.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "control_acceso_instancia_{nombre}_{}_{numero}.db",
            std::process::id()
        ))
    }

    fn limpiar(base_datos: &Path) {
        let _ = fs::remove_file(ruta_bloqueo(base_datos));
    }

    #[test]
    fn segunda_instancia_de_la_misma_base_es_rechazada() {
        let base_datos = base_temporal("duplicada");
        let primera = InstanciaGuard::adquirir(&base_datos).unwrap();

        let segunda = InstanciaGuard::adquirir(&base_datos);

        assert!(matches!(
            segunda,
            Err(InstanciaError::YaEstaEnEjecucion { .. })
        ));
        drop(primera);
        limpiar(&base_datos);
    }

    #[test]
    fn bloqueo_se_libera_al_descartar_el_guard() {
        let base_datos = base_temporal("liberada");
        let primera = InstanciaGuard::adquirir(&base_datos).unwrap();
        drop(primera);

        let segunda = InstanciaGuard::adquirir(&base_datos);

        assert!(segunda.is_ok());
        drop(segunda);
        limpiar(&base_datos);
    }

    #[test]
    fn bases_distintas_admiten_instancias_independientes() {
        let primera_base = base_temporal("base_a");
        let segunda_base = base_temporal("base_b");

        let primera = InstanciaGuard::adquirir(&primera_base);
        let segunda = InstanciaGuard::adquirir(&segunda_base);

        assert!(primera.is_ok());
        assert!(segunda.is_ok());
        drop(primera);
        drop(segunda);
        limpiar(&primera_base);
        limpiar(&segunda_base);
    }
}

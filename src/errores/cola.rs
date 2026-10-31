//! Los errores que todavía no llegaron al receptor, en disco: un archivo
//! JSON por error en `errores_pendientes/`. Se escriben en el momento (un
//! pánico puede matar el proceso enseguida) y se borran cuando el receptor
//! los recibió. Sobreviven a un cierre, un apagón o días sin red.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::formato::Reporte;

/// Tope de errores esperando: sin red por mucho tiempo, lo más viejo se
/// borra en vez de llenar el disco.
pub const TOPE: usize = 50;
const CARPETA: &str = "errores_pendientes";
const EXTENSION: &str = "json";

#[derive(Debug, Clone)]
pub struct Cola {
    directorio: PathBuf,
}

impl Cola {
    pub fn en(base: &Path) -> Self {
        Self {
            directorio: base.join(CARPETA),
        }
    }

    /// Guarda el reporte. Escribe primero en un temporal y lo renombra: un
    /// corte a mitad no deja un JSON a medias que parezca un error.
    pub fn guardar(&self, reporte: &Reporte) -> io::Result<()> {
        fs::create_dir_all(&self.directorio)?;
        let pendientes = self.pendientes();
        let sobran = (pendientes.len() + 1).saturating_sub(TOPE);
        for ruta in pendientes.into_iter().take(sobran) {
            let _ = fs::remove_file(ruta);
        }
        let json = serde_json::to_vec(reporte).map_err(io::Error::other)?;
        // El instante va primero: el nombre ordena del más viejo al más nuevo.
        let nombre = format!("{}-{}", reporte.ocurrido_en.replace(':', "-"), reporte.id);
        let temporal = self.directorio.join(format!("{nombre}.tmp"));
        fs::write(&temporal, json)?;
        fs::rename(
            &temporal,
            self.directorio.join(format!("{nombre}.{EXTENSION}")),
        )
    }

    /// Los archivos esperando, del más viejo al más nuevo.
    pub fn pendientes(&self) -> Vec<PathBuf> {
        let Ok(entradas) = fs::read_dir(&self.directorio) else {
            return Vec::new();
        };
        let mut rutas: Vec<PathBuf> = entradas
            .filter_map(Result::ok)
            .map(|entrada| entrada.path())
            .filter(|ruta| ruta.extension().is_some_and(|e| e == EXTENSION))
            .collect();
        rutas.sort();
        rutas
    }

    pub fn borrar(ruta: &Path) {
        let _ = fs::remove_file(ruta);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Cola, TOPE};
    use crate::errores::formato::{ESQUEMA, Reporte};

    fn reporte(i: usize) -> Reporte {
        Reporte {
            esquema: ESQUEMA,
            id: format!("id-{i:03}"),
            ocurrido_en: format!("2026-10-31T12:{:02}:{:02}.000Z", i / 60, i % 60),
            plataforma: "escritorio".into(),
            version_app: "desktop-1.10.6".into(),
            origen: "panico".into(),
            titulo: format!("error {i}"),
            traza: Vec::new(),
            pantalla: None,
            operaciones: Vec::new(),
            contexto: BTreeMap::new(),
        }
    }

    #[test]
    fn guarda_y_lista_del_mas_viejo_al_mas_nuevo() {
        let dir = tempfile::tempdir().unwrap();
        let cola = Cola::en(dir.path());
        assert!(cola.pendientes().is_empty(), "sin carpeta todavía");
        cola.guardar(&reporte(2)).unwrap();
        cola.guardar(&reporte(1)).unwrap();
        let pendientes = cola.pendientes();
        assert_eq!(pendientes.len(), 2);
        let primero: Reporte =
            serde_json::from_slice(&std::fs::read(&pendientes[0]).unwrap()).unwrap();
        assert_eq!(primero, reporte(1));
        Cola::borrar(&pendientes[0]);
        assert_eq!(cola.pendientes().len(), 1);
    }

    #[test]
    fn con_el_tope_lleno_se_va_el_mas_viejo() {
        let dir = tempfile::tempdir().unwrap();
        let cola = Cola::en(dir.path());
        for i in 0..TOPE + 3 {
            cola.guardar(&reporte(i)).unwrap();
        }
        let pendientes = cola.pendientes();
        assert_eq!(pendientes.len(), TOPE);
        assert!(
            pendientes[0].to_string_lossy().contains("id-003"),
            "los tres primeros se borraron"
        );
    }
}

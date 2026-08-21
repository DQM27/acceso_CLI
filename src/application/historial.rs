//! Consulta paginada del historial y exportación a XLSX.

use std::path::{Path, PathBuf};

use crate::database::queries::ingresos::{FiltroHistorial, PaginaHistorial, SqliteIngresosQuery};
use crate::historial::exportacion::{
    ColumnaHistorial, FormatosHistorial, escribir_movimiento, preparar_hoja,
};
use crate::services::error::RegistroIngresoServiceError;
use crate::services::registro_ingreso_service::RegistroIngresoConsultaService;

use super::AppCore;

#[derive(Debug, thiserror::Error)]
pub enum ExportarHistorialError {
    #[error("Seleccione al menos una columna")]
    SinColumnas,
    #[error("El archivo ya existe; elija otro nombre: {}", .0.display())]
    DestinoExiste(PathBuf),
    #[error("La carpeta destino no existe: {}", .0.display())]
    DirectorioNoExiste(PathBuf),
    #[error("La exportación tiene {0} filas y supera el límite de una hoja de Excel")]
    DemasiadasFilas(usize),
    #[error("No se pudo consultar el historial: {0}")]
    Consulta(#[from] RegistroIngresoServiceError),
    #[error("No se pudo crear el archivo XLSX: {0}")]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),
    #[error("No se pudo guardar la exportación: {0}")]
    Io(#[from] std::io::Error),
}

impl AppCore {
    pub fn buscar_historial(
        &self,
        filtro: &FiltroHistorial,
    ) -> Result<PaginaHistorial, RegistroIngresoServiceError> {
        RegistroIngresoConsultaService::new(&SqliteIngresosQuery::new(&self.connection))
            .buscar_historial(filtro)
    }

    /// Exporta todo el conjunto filtrado que representa la pantalla, no sólo
    /// su página actual. Se conserva `corte_id`, por lo que ingresos creados
    /// después de cargar Historial no aparecen inesperadamente en el XLSX.
    pub fn exportar_historial(
        &self,
        filtro: &FiltroHistorial,
        columnas: &[ColumnaHistorial],
        destino: &Path,
    ) -> Result<usize, ExportarHistorialError> {
        const MAX_FILAS_DATOS_XLSX: usize = 1_048_575;

        if columnas.is_empty() {
            return Err(ExportarHistorialError::SinColumnas);
        }
        if destino.exists() {
            return Err(ExportarHistorialError::DestinoExiste(destino.to_owned()));
        }
        let directorio = destino
            .parent()
            .filter(|ruta| !ruta.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if !directorio.is_dir() {
            return Err(ExportarHistorialError::DirectorioNoExiste(
                directorio.to_owned(),
            ));
        }

        let mut libro = rust_xlsxwriter::Workbook::new();
        let mut exportados = 0usize;
        {
            let hoja = libro.add_worksheet_with_constant_memory();
            preparar_hoja(hoja, columnas)?;
            let formatos = FormatosHistorial::default();

            let mut consulta = filtro.clone();
            consulta.offset = 0;
            // La consulta limita internamente cada página a 200 filas. El
            // exportador las consume por lotes para no retener todo en RAM.
            consulta.limite = usize::MAX;
            loop {
                let pagina = self.buscar_historial(&consulta)?;
                if pagina.total > MAX_FILAS_DATOS_XLSX {
                    return Err(ExportarHistorialError::DemasiadasFilas(pagina.total));
                }
                consulta.corte_id = Some(pagina.corte_id);
                if pagina.items.is_empty() {
                    break;
                }
                for movimiento in &pagina.items {
                    let fila = u32::try_from(exportados + 1).unwrap_or(u32::MAX);
                    escribir_movimiento(hoja, fila, columnas, movimiento, &formatos)?;
                    exportados += 1;
                }
                if exportados >= pagina.total {
                    break;
                }
                consulta.offset = exportados;
            }

            let ultima_columna = u16::try_from(columnas.len() - 1).unwrap_or(u16::MAX);
            hoja.autofilter(
                0,
                0,
                u32::try_from(exportados).unwrap_or(u32::MAX),
                ultima_columna,
            )?;
        }

        // Se escribe junto al destino y sólo se publica al finalizar. Así un
        // error no deja un XLSX parcial y nunca se reemplaza otro archivo.
        let temporal = tempfile::Builder::new()
            .prefix(".historial-")
            .suffix(".xlsx")
            .tempfile_in(directorio)?
            .into_temp_path();
        libro.save(&temporal)?;
        temporal.persist_noclobber(destino).map_err(|error| {
            if error.error.kind() == std::io::ErrorKind::AlreadyExists {
                ExportarHistorialError::DestinoExiste(destino.to_owned())
            } else {
                ExportarHistorialError::Io(error.error)
            }
        })?;
        Ok(exportados)
    }
}

//! Consulta paginada del historial y exportación a XLSX.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::database::queries::ingresos::{
    FiltroHistorial, MovimientoIngresoResumen, PaginaHistorial, SqliteIngresosQuery,
};
use crate::historial::exportacion::{
    ColumnaHistorial, FormatosHistorial, escribir_movimiento, preparar_hoja,
};
use crate::services::error::RegistroIngresoServiceError;
use crate::services::registro_ingreso_service::RegistroIngresoConsultaService;

use super::{AppCore, CargaCompleta, LIMITE_CARGA_COMPLETA_MAXIMO};

/// Núcleo de [`AppCore::buscar_historial_completo`] sobre una `Connection`
/// cualquiera — mismo motivo que [`buscar_historial_con_conexion`]: permite
/// a un comando Tauri abrir su propia conexión en vez de retener el
/// `Mutex<AppCore>` compartido durante los ~750ms que puede tardar esta
/// consulta con historiales grandes (medido en la auditoría de las tres
/// capas, `docs/pendientes.md`), bloqueando mientras tanto cualquier otro
/// comando que también necesite el núcleo.
pub fn buscar_historial_completo_con_conexion(
    connection: &Connection,
    filtro: &FiltroHistorial,
) -> Result<CargaCompleta<MovimientoIngresoResumen>, RegistroIngresoServiceError> {
    let mut consulta = filtro.clone();
    consulta.offset = 0;
    consulta.limite = usize::MAX;
    let mut todos = Vec::new();
    let mut total;
    loop {
        let pagina = buscar_historial_con_conexion(connection, &consulta)?;
        consulta.corte_id = Some(pagina.corte_id);
        total = pagina.total;
        if pagina.items.is_empty() {
            break;
        }
        todos.extend(pagina.items);
        if todos.len() >= total || todos.len() >= LIMITE_CARGA_COMPLETA_MAXIMO {
            break;
        }
        consulta.offset = todos.len();
    }
    Ok(CargaCompleta {
        truncado: todos.len() < total,
        items: todos,
    })
}

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

/// Núcleo de [`AppCore::buscar_historial`] sobre una `Connection` cualquiera
/// — separado para que el hilo de exportación (`tui/app/historial_jobs.rs`)
/// pueda abrir su propia conexión de sólo lectura al mismo archivo en vez de
/// compartir la conexión viva de `AppCore` entre hilos, mismo criterio que
/// ya usa el respaldo (`tui/app/backup_jobs.rs`).
pub fn buscar_historial_con_conexion(
    connection: &Connection,
    filtro: &FiltroHistorial,
) -> Result<PaginaHistorial, RegistroIngresoServiceError> {
    RegistroIngresoConsultaService::new(&SqliteIngresosQuery::new(connection))
        .buscar_historial(filtro)
}

/// Núcleo de [`AppCore::movimientos_en_orden`] sobre una `Connection`
/// cualquiera — mismo motivo que [`buscar_historial_con_conexion`].
pub fn movimientos_en_orden_con_conexion(
    connection: &Connection,
    filtro: &FiltroHistorial,
    ids: &[i64],
) -> Result<Vec<MovimientoIngresoResumen>, RegistroIngresoServiceError> {
    let pendientes: std::collections::HashSet<i64> = ids.iter().copied().collect();
    let mut encontrados: std::collections::HashMap<i64, MovimientoIngresoResumen> =
        std::collections::HashMap::with_capacity(ids.len());
    let mut consulta = filtro.clone();
    consulta.offset = 0;
    consulta.limite = usize::MAX;
    loop {
        let pagina = buscar_historial_con_conexion(connection, &consulta)?;
        consulta.corte_id = Some(pagina.corte_id);
        let hay_mas = !pagina.items.is_empty();
        let total_pagina = pagina.total;
        let items_en_pagina = pagina.items.len();
        for movimiento in pagina.items {
            if pendientes.contains(&movimiento.registro_id) {
                encontrados.insert(movimiento.registro_id, movimiento);
            }
        }
        if !hay_mas || encontrados.len() >= pendientes.len() {
            break;
        }
        consulta.offset += items_en_pagina;
        if consulta.offset >= total_pagina {
            break;
        }
    }
    Ok(ids.iter().filter_map(|id| encontrados.remove(id)).collect())
}

/// Núcleo de [`AppCore::exportar_historial_seleccion`] sobre una
/// `Connection` cualquiera — mismo motivo que [`buscar_historial_con_conexion`].
/// Medido (`docs/pendientes.md`): armar el XLSX de 100,000 movimientos tarda
/// ~33 segundos, muy por encima de lo que el respaldo llegó a tardar — este
/// era el punto realmente bloqueante, no el respaldo.
pub fn exportar_historial_seleccion_con_conexion(
    connection: &Connection,
    filtro: &FiltroHistorial,
    ids: Option<&[i64]>,
    columnas: &[ColumnaHistorial],
    destino: &Path,
) -> Result<usize, ExportarHistorialError> {
    const MAX_FILAS_DATOS_XLSX: usize = 1_048_575;

    if columnas.is_empty() {
        return Err(ExportarHistorialError::SinColumnas);
    }
    if let Some(ids) = ids
        && ids.len() > MAX_FILAS_DATOS_XLSX
    {
        return Err(ExportarHistorialError::DemasiadasFilas(ids.len()));
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

    // Con `ids` (recorte + orden de la GUI) hace falta juntar primero
    // los movimientos pedidos antes de poder escribirlos en ESE orden —
    // a diferencia del camino sin `ids`, que puede ir escribiendo
    // página a página según llega de la consulta (orden cronológico) sin
    // retener nada. El tamaño de lo que se retiene está acotado por
    // `ids.len()`, no por el total del historial.
    let ordenados: Option<Vec<MovimientoIngresoResumen>> = match ids {
        Some(ids) => Some(movimientos_en_orden_con_conexion(connection, filtro, ids)?),
        None => None,
    };

    let mut libro = rust_xlsxwriter::Workbook::new();
    let mut exportados = 0usize;
    {
        let hoja = libro.add_worksheet_with_constant_memory();
        preparar_hoja(hoja, columnas)?;
        let formatos = FormatosHistorial::default();

        if let Some(movimientos) = &ordenados {
            for movimiento in movimientos {
                let fila = u32::try_from(exportados + 1).unwrap_or(u32::MAX);
                escribir_movimiento(hoja, fila, columnas, movimiento, &formatos)?;
                exportados += 1;
            }
        } else {
            let mut consulta = filtro.clone();
            consulta.offset = 0;
            // La consulta limita internamente cada página a 200
            // filas. El exportador las consume por lotes para no
            // retener todo en RAM.
            consulta.limite = usize::MAX;
            loop {
                let pagina = buscar_historial_con_conexion(connection, &consulta)?;
                if pagina.total > MAX_FILAS_DATOS_XLSX {
                    return Err(ExportarHistorialError::DemasiadasFilas(pagina.total));
                }
                consulta.corte_id = Some(pagina.corte_id);
                let hay_mas = !pagina.items.is_empty();
                let total_pagina = pagina.total;
                for movimiento in &pagina.items {
                    let fila = u32::try_from(exportados + 1).unwrap_or(u32::MAX);
                    escribir_movimiento(hoja, fila, columnas, movimiento, &formatos)?;
                    exportados += 1;
                }
                if !hay_mas {
                    break;
                }
                consulta.offset += pagina.items.len();
                if consulta.offset >= total_pagina {
                    break;
                }
            }
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

impl AppCore {
    pub fn buscar_historial(
        &self,
        filtro: &FiltroHistorial,
    ) -> Result<PaginaHistorial, RegistroIngresoServiceError> {
        buscar_historial_con_conexion(&self.connection, filtro)
    }

    /// Todo el conjunto filtrado en un solo `Vec`, no sólo una página — para
    /// una interfaz que virtualiza del lado del cliente (AG Grid) en vez de
    /// paginar por su cuenta. Mismo lote/`corte_id` que ya usa
    /// `exportar_historial`, extraído para no repetir el loop en cada lugar
    /// que necesite "todo, no una página". Se corta en
    /// [`LIMITE_CARGA_COMPLETA_MAXIMO`] — con la pantalla acotando por rango
    /// de fechas (`Historial.tsx`) es raro llegar ahí, pero un rango muy
    /// abierto no debe congelar la UI ni el mensaje IPC.
    pub fn buscar_historial_completo(
        &self,
        filtro: &FiltroHistorial,
    ) -> Result<CargaCompleta<MovimientoIngresoResumen>, RegistroIngresoServiceError> {
        buscar_historial_completo_con_conexion(&self.connection, filtro)
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
        self.exportar_historial_seleccion(filtro, None, columnas, destino)
    }

    /// Resuelve, en el orden exacto de `ids`, los movimientos de `filtro`
    /// cuyo `registro_id` esté en esa lista — un id que no matchea nada se
    /// omite en silencio (la GUI pudo haber armado `ids` de una foto de la
    /// grilla ligeramente vieja). Extraído de
    /// [`Self::exportar_historial_seleccion`] para poder probar el orden
    /// resultante sin tener que leer de vuelta un XLSX (`rust_xlsxwriter`
    /// sólo escribe, no lee).
    pub fn movimientos_en_orden(
        &self,
        filtro: &FiltroHistorial,
        ids: &[i64],
    ) -> Result<Vec<MovimientoIngresoResumen>, RegistroIngresoServiceError> {
        movimientos_en_orden_con_conexion(&self.connection, filtro, ids)
    }

    /// Igual que [`Self::exportar_historial`], pero cuando `ids` es `Some`
    /// sólo escribe los movimientos cuyo `registro_id` esté en esa lista, EN
    /// ESE ORDEN — la GUI manda exactamente el orden visible en pantalla
    /// (`AG Grid`, tras su propio filtro y orden de columna, que
    /// `FiltroHistorial`/la consulta SQL no conocen) en vez de siempre el
    /// orden cronológico de la consulta. `None` exporta todo el conjunto de
    /// `filtro` en el orden de la consulta, igual que antes.
    pub fn exportar_historial_seleccion(
        &self,
        filtro: &FiltroHistorial,
        ids: Option<&[i64]>,
        columnas: &[ColumnaHistorial],
        destino: &Path,
    ) -> Result<usize, ExportarHistorialError> {
        exportar_historial_seleccion_con_conexion(&self.connection, filtro, ids, columnas, destino)
    }
}

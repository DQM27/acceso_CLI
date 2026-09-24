//! Consulta paginada del historial y exportación a XLSX.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::database::queries::ingresos::{
    FiltroHistorial, MovimientoIngresoResumen, PaginaHistorial, SqliteIngresosQuery,
};
use crate::historial::exportacion::{
    ColumnaHistorial, ColumnaTabla, FormatosHistorial, MovimientoExportable, escribir_movimiento,
    escribir_tabla_generica, preparar_hoja,
};
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::tipo_ingreso::TipoIngreso;
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
    #[error("No se pudo consultar el historial de otros dispositivos: {0}")]
    HistorialSitio(#[from] rusqlite::Error),
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

/// Movimientos de `historial_sitio` (espejo de la nube, ver `MIGRACION_25`)
/// dentro del rango de `filtro`, salvo los `uuid` de `excluir` -- este
/// mismo dispositivo también se ve reflejado ahí (respaldo ante una
/// reinstalación), y para esos la fila local es la autoritativa. Sólo se
/// aplica `desde`/`hasta`: el resto de `FiltroHistorial` (texto, empresa...)
/// son filtros de la consulta local que la GUI de escritorio no usa -- ella
/// filtra del lado del cliente y manda los `uuid` ya recortados. Una fila
/// con fecha ilegible se omite (mismo criterio que el sync al recibirla).
fn movimientos_del_sitio_con_conexion(
    connection: &Connection,
    filtro: &FiltroHistorial,
    excluir: &HashSet<String>,
) -> Result<Vec<MovimientoExportable>, rusqlite::Error> {
    let mut consulta = connection.prepare(
        "SELECT uuid, contratista_cedula, contratista_nombre, empresa_nombre, tipo_ingreso,
                medio_ingreso, hora_entrada, hora_salida, gafete_numero, placa,
                usuario_entrada_nombre, usuario_salida_nombre
         FROM historial_sitio
         WHERE hora_entrada >= ?1 AND hora_entrada < ?2
         ORDER BY hora_entrada DESC, uuid",
    )?;
    let filas = consulta.query_map(
        rusqlite::params![
            crate::tiempo::serializar_utc(filtro.desde),
            crate::tiempo::serializar_utc(filtro.hasta)
        ],
        |row| {
            let tipo: Option<String> = row.get(4)?;
            let medio: Option<String> = row.get(5)?;
            let entrada: String = row.get(6)?;
            let salida: Option<String> = row.get(7)?;
            let Ok(fecha_hora_ingreso) = crate::tiempo::parsear_utc(&entrada) else {
                return Ok(None);
            };
            let Ok(fecha_hora_salida) = salida
                .as_deref()
                .map(crate::tiempo::parsear_utc)
                .transpose()
            else {
                return Ok(None);
            };
            Ok(Some(MovimientoExportable {
                uuid: row.get(0)?,
                cedula: row.get(1)?,
                contratista_nombre: row.get(2)?,
                empresa_nombre: row.get(3)?,
                tipo_ingreso: tipo.as_deref().and_then(TipoIngreso::from_str_sql),
                medio_ingreso: medio.as_deref().and_then(|medio| match medio {
                    "CAMINANDO" => Some(MedioIngreso::Caminando),
                    "VEHICULO" => Some(MedioIngreso::Vehiculo),
                    _ => None,
                }),
                fecha_hora_ingreso,
                fecha_hora_salida,
                gafete_numero: row.get(8)?,
                placa: row.get(9)?,
                usuario_ingreso_nombre: row.get(10)?,
                usuario_salida_nombre: row.get(11)?,
            }))
        },
    )?;
    let mut movimientos = Vec::new();
    for fila in filas {
        if let Some(movimiento) = fila?
            && !excluir.contains(&movimiento.uuid)
        {
            movimientos.push(movimiento);
        }
    }
    Ok(movimientos)
}

/// Núcleo de [`AppCore::movimientos_en_orden`] sobre una `Connection`
/// cualquiera — mismo motivo que [`buscar_historial_con_conexion`]. Busca
/// cada `uuid` primero entre los movimientos locales y, si no está, en
/// `historial_sitio` (movimientos de otro dispositivo del sitio) — antes
/// recortaba por `registro_id` local y dejaba afuera todo lo remoto.
pub fn movimientos_en_orden_con_conexion(
    connection: &Connection,
    filtro: &FiltroHistorial,
    uuids: &[String],
) -> Result<Vec<MovimientoExportable>, ExportarHistorialError> {
    let pendientes: HashSet<&str> = uuids.iter().map(String::as_str).collect();
    let mut encontrados: HashMap<String, MovimientoExportable> =
        HashMap::with_capacity(uuids.len());
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
            if pendientes.contains(movimiento.uuid.as_str()) {
                encontrados.insert(movimiento.uuid.clone(), movimiento.into());
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
    if encontrados.len() < pendientes.len() {
        let locales: HashSet<String> = encontrados.keys().cloned().collect();
        for movimiento in movimientos_del_sitio_con_conexion(connection, filtro, &locales)? {
            if pendientes.contains(movimiento.uuid.as_str()) {
                encontrados.insert(movimiento.uuid.clone(), movimiento);
            }
        }
    }
    Ok(uuids
        .iter()
        .filter_map(|uuid| encontrados.remove(uuid))
        .collect())
}

/// Todo el rango de `filtro` para exportar sin recorte de la GUI — los
/// movimientos locales (acotados por [`LIMITE_CARGA_COMPLETA_MAXIMO`], ver
/// [`buscar_historial_completo_con_conexion`]) más los de otros
/// dispositivos del sitio, ordenados juntos de más nuevo a más viejo, igual
/// que la grilla. Lo usa el PDF, que necesita todo en memoria de todos
/// modos.
pub fn movimientos_completos_con_conexion(
    connection: &Connection,
    filtro: &FiltroHistorial,
) -> Result<Vec<MovimientoExportable>, ExportarHistorialError> {
    let locales = buscar_historial_completo_con_conexion(connection, filtro)?.items;
    let uuids_locales: HashSet<String> = locales.iter().map(|m| m.uuid.clone()).collect();
    let mut movimientos: Vec<MovimientoExportable> =
        locales.into_iter().map(MovimientoExportable::from).collect();
    movimientos.extend(movimientos_del_sitio_con_conexion(
        connection,
        filtro,
        &uuids_locales,
    )?);
    movimientos.sort_by(|a, b| b.fecha_hora_ingreso.cmp(&a.fecha_hora_ingreso));
    Ok(movimientos)
}

/// Núcleo de [`AppCore::exportar_historial_seleccion`] sobre una
/// `Connection` cualquiera — mismo motivo que [`buscar_historial_con_conexion`].
/// Medido (`docs/pendientes.md`): armar el XLSX de 100,000 movimientos tarda
/// ~33 segundos, muy por encima de lo que el respaldo llegó a tardar — este
/// era el punto realmente bloqueante, no el respaldo.
/// Filas de datos que entran en una hoja de Excel (1.048.576 menos el
/// encabezado).
const MAX_FILAS_DATOS_XLSX: usize = 1_048_575;

/// El destino no debe existir (nunca se pisa otro archivo acá; la GUI
/// aparta el existente antes, ver `RespaldoDestino`) y su carpeta sí.
/// Devuelve esa carpeta, donde después se arma el archivo temporal.
fn validar_destino(destino: &Path) -> Result<PathBuf, ExportarHistorialError> {
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
    Ok(directorio.to_owned())
}

/// Se escribe junto al destino y sólo se publica al finalizar. Así un
/// error no deja un XLSX parcial y nunca se reemplaza otro archivo.
fn guardar_libro(
    libro: &mut rust_xlsxwriter::Workbook,
    destino: &Path,
    directorio: &Path,
) -> Result<(), ExportarHistorialError> {
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
    Ok(())
}

/// Exporta a XLSX una tabla ya armada del lado de la GUI (títulos y
/// valores como texto, tal cual se ven en la grilla) -- ver
/// [`escribir_tabla_generica`]. Para grillas sin exportador propio
/// (historial de proveedores y de KOF). Devuelve la cantidad de filas.
pub fn exportar_tabla_xlsx(
    columnas: &[ColumnaTabla],
    filas: &[Vec<String>],
    destino: &Path,
) -> Result<usize, ExportarHistorialError> {
    if columnas.is_empty() {
        return Err(ExportarHistorialError::SinColumnas);
    }
    if filas.len() > MAX_FILAS_DATOS_XLSX {
        return Err(ExportarHistorialError::DemasiadasFilas(filas.len()));
    }
    let directorio = validar_destino(destino)?;
    let mut libro = rust_xlsxwriter::Workbook::new();
    escribir_tabla_generica(libro.add_worksheet(), columnas, filas)?;
    guardar_libro(&mut libro, destino, &directorio)?;
    Ok(filas.len())
}

pub fn exportar_historial_seleccion_con_conexion(
    connection: &Connection,
    filtro: &FiltroHistorial,
    uuids: Option<&[String]>,
    columnas: &[ColumnaHistorial],
    destino: &Path,
) -> Result<usize, ExportarHistorialError> {
    if columnas.is_empty() {
        return Err(ExportarHistorialError::SinColumnas);
    }
    if let Some(uuids) = uuids
        && uuids.len() > MAX_FILAS_DATOS_XLSX
    {
        return Err(ExportarHistorialError::DemasiadasFilas(uuids.len()));
    }
    let directorio = validar_destino(destino)?;

    // Con `uuids` (recorte + orden de la GUI) hace falta juntar primero
    // los movimientos pedidos antes de poder escribirlos en ESE orden —
    // a diferencia del camino sin `uuids`, que puede ir escribiendo
    // página a página según llega de la consulta (orden cronológico) sin
    // retener nada. El tamaño de lo que se retiene está acotado por
    // `uuids.len()`, no por el total del historial.
    let ordenados: Option<Vec<MovimientoExportable>> = match uuids {
        Some(uuids) => Some(movimientos_en_orden_con_conexion(connection, filtro, uuids)?),
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
            // retener todo en RAM (sólo los `uuid`, para descartar después
            // su reflejo en `historial_sitio`).
            consulta.limite = usize::MAX;
            let mut uuids_locales = HashSet::new();
            loop {
                let pagina = buscar_historial_con_conexion(connection, &consulta)?;
                if pagina.total > MAX_FILAS_DATOS_XLSX {
                    return Err(ExportarHistorialError::DemasiadasFilas(pagina.total));
                }
                consulta.corte_id = Some(pagina.corte_id);
                let hay_mas = !pagina.items.is_empty();
                let total_pagina = pagina.total;
                let items_en_pagina = pagina.items.len();
                for movimiento in pagina.items {
                    let fila = u32::try_from(exportados + 1).unwrap_or(u32::MAX);
                    uuids_locales.insert(movimiento.uuid.clone());
                    escribir_movimiento(hoja, fila, columnas, &movimiento.into(), &formatos)?;
                    exportados += 1;
                }
                if !hay_mas {
                    break;
                }
                consulta.offset += items_en_pagina;
                if consulta.offset >= total_pagina {
                    break;
                }
            }
            // Los de otros dispositivos del sitio van al final (no
            // intercalados por fecha): intercalarlos obligaría a retener todo
            // el historial en memoria, justo lo que este camino evita.
            for movimiento in
                movimientos_del_sitio_con_conexion(connection, filtro, &uuids_locales)?
            {
                if exportados >= MAX_FILAS_DATOS_XLSX {
                    return Err(ExportarHistorialError::DemasiadasFilas(exportados + 1));
                }
                let fila = u32::try_from(exportados + 1).unwrap_or(u32::MAX);
                escribir_movimiento(hoja, fila, columnas, &movimiento, &formatos)?;
                exportados += 1;
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

    guardar_libro(&mut libro, destino, &directorio)?;
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

    /// Resuelve, en el orden exacto de `uuids`, los movimientos de `filtro`
    /// (locales o de otro dispositivo del sitio, ver
    /// [`movimientos_en_orden_con_conexion`]) cuyo `uuid` esté en esa lista —
    /// uno que no matchea nada se omite en silencio (la GUI pudo haber
    /// armado `uuids` de una foto de la grilla ligeramente vieja). Extraído de
    /// [`Self::exportar_historial_seleccion`] para poder probar el orden
    /// resultante sin tener que leer de vuelta un XLSX (`rust_xlsxwriter`
    /// sólo escribe, no lee).
    pub fn movimientos_en_orden(
        &self,
        filtro: &FiltroHistorial,
        uuids: &[String],
    ) -> Result<Vec<MovimientoExportable>, ExportarHistorialError> {
        movimientos_en_orden_con_conexion(&self.connection, filtro, uuids)
    }

    /// Ver [`movimientos_completos_con_conexion`].
    pub fn movimientos_completos(
        &self,
        filtro: &FiltroHistorial,
    ) -> Result<Vec<MovimientoExportable>, ExportarHistorialError> {
        movimientos_completos_con_conexion(&self.connection, filtro)
    }

    /// Igual que [`Self::exportar_historial`], pero cuando `uuids` es `Some`
    /// sólo escribe los movimientos cuyo `uuid` esté en esa lista, EN
    /// ESE ORDEN — la GUI manda exactamente el orden visible en pantalla
    /// (`AG Grid`, tras su propio filtro y orden de columna, que
    /// `FiltroHistorial`/la consulta SQL no conocen) en vez de siempre el
    /// orden cronológico de la consulta. `None` exporta todo el conjunto de
    /// `filtro` en el orden de la consulta, igual que antes.
    pub fn exportar_historial_seleccion(
        &self,
        filtro: &FiltroHistorial,
        uuids: Option<&[String]>,
        columnas: &[ColumnaHistorial],
        destino: &Path,
    ) -> Result<usize, ExportarHistorialError> {
        exportar_historial_seleccion_con_conexion(&self.connection, filtro, uuids, columnas, destino)
    }
}

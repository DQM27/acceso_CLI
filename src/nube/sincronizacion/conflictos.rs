use rusqlite::Connection;

use super::{ContextoSincronizacion, SincronizacionError, SitioEmbebido, llamar_rpc, obtener_json};
use crate::nube::cliente::cliente_http;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ConflictoIngresoActivo {
    pub cedula: String,
    pub contratista_nombre: String,
    /// Sitio donde ESTE mismo dispositivo también lo tiene activo ahora
    /// mismo -- no necesariamente el único conflicto que existe (podría
    /// haber un tercero), sólo el primero que el receptor devolvió.
    pub sitio_conflicto: String,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaConflictoActivo {
    pub(super) contratista_cedula: Option<String>,
    pub(super) sitio_nombre: Option<String>,
}

/// `docs/pendientes.md`, mitad "offline, registrar y alertar luego al
/// sincronizar" de la regla de ingresos activos:
/// `contratista_con_ingreso_activo` chequea UNA cédula puntual al momento
/// de registrar; ésta, en cambio, corre después de un sync
/// exitoso (ya hay red, por definición) y revisa TODOS los ingresos que
/// quedaron activos localmente, para encontrar los que igual se colaron --
/// por ejemplo, registrados mientras este dispositivo estaba offline.
///
/// Deliberadamente simétrica: ambos sitios en conflicto corren esta misma
/// consulta contra el mismo estado remoto, cada uno mirando sus propios
/// ingresos activos -- así cada lado se entera y puede avisar sin
/// necesitar un canal de mensajería aparte entre sitios ni una tabla nueva
/// de "notificaciones pendientes".
///
/// Va por la función `contratistas_activos_en_otras_unidades` por el mismo
/// motivo que `contratista_con_ingreso_activo`: la RLS de `ingresos` no deja
/// ver lo de otras unidades, y la consulta directa siempre daba vacío.
pub fn contratistas_con_conflicto_activo(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<ConflictoIngresoActivo>, SincronizacionError> {
    let mut statement = connection.prepare(
        "SELECT contratista_cedula, contratista_nombre FROM registro_ingresos
         WHERE fecha_hora_salida IS NULL",
    )?;
    let activos_locales: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    drop(statement);
    if activos_locales.is_empty() {
        return Ok(Vec::new());
    }

    let cedulas = activos_locales
        .iter()
        .map(|(cedula, _)| cedula.as_str())
        .collect::<Vec<_>>();
    let cliente = cliente_http();
    let filas: Vec<FilaConflictoActivo> = llamar_rpc(
        &cliente,
        contexto,
        "contratistas_activos_en_otras_unidades",
        &serde_json::json!({ "p_cedulas": cedulas }),
    )?;

    Ok(filas
        .into_iter()
        .filter_map(|fila| {
            let cedula = fila.contratista_cedula?;
            let sitio = fila.sitio_nombre?;
            let nombre = activos_locales
                .iter()
                .find(|(c, _)| *c == cedula)
                .map(|(_, nombre)| nombre.clone())?;
            Some(ConflictoIngresoActivo {
                cedula,
                contratista_nombre: nombre,
                sitio_conflicto: sitio,
            })
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ConflictoMovimientoVisitaActivo {
    pub cedula: String,
    pub visitante_nombre: String,
    /// Sitio donde ESTE mismo dispositivo también lo tiene activo ahora
    /// mismo -- no necesariamente el único conflicto que existe.
    pub sitio_conflicto: String,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaConflictoVisitaActivo {
    pub(super) visitante_cedula: Option<String>,
    pub(super) sitios: Option<SitioEmbebido>,
}

/// Mismo criterio y misma forma que `contratistas_con_conflicto_activo`,
/// pero para `movimientos_visita`: corre después de un sync exitoso y
/// revisa todos los movimientos de visita que quedaron activos localmente
/// para encontrar los que igual se colaron en otro sitio (ej. registrados
/// mientras este dispositivo estaba offline). Deliberadamente simétrica --
/// ambos sitios en conflicto corren esta misma consulta.
pub fn visitantes_con_conflicto_activo(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<ConflictoMovimientoVisitaActivo>, SincronizacionError> {
    let mut statement = connection.prepare(
        "SELECT visitante_cedula, visitante_nombre FROM movimientos_visita
         WHERE fecha_hora_salida IS NULL",
    )?;
    let activos_locales: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    drop(statement);
    if activos_locales.is_empty() {
        return Ok(Vec::new());
    }

    let cedulas = activos_locales
        .iter()
        .map(|(cedula, _)| cedula.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/movimientos_visita?visitante_cedula=in.({cedulas})&sitio_id=neq.{}\
         &hora_salida=is.null&select=visitante_cedula,sitios(nombre)",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaConflictoVisitaActivo> = obtener_json(&cliente, contexto, &url)?;

    Ok(filas
        .into_iter()
        .filter_map(|fila| {
            let cedula = fila.visitante_cedula?;
            let sitio = fila.sitios?.nombre;
            let nombre = activos_locales
                .iter()
                .find(|(c, _)| *c == cedula)
                .map(|(_, nombre)| nombre.clone())?;
            Some(ConflictoMovimientoVisitaActivo {
                cedula,
                visitante_nombre: nombre,
                sitio_conflicto: sitio,
            })
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ConflictoIngresoProveedorActivo {
    pub cedula: String,
    pub nombre: String,
    /// Sitio donde ESTE mismo dispositivo también lo tiene activo ahora
    /// mismo -- no necesariamente el único conflicto que existe.
    pub sitio_conflicto: String,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaConflictoProveedorActivo {
    pub(super) cedula: Option<String>,
    pub(super) sitio_nombre: Option<String>,
}

/// Espejo de [`contratistas_con_conflicto_activo`], pero contra
/// `ingresos_proveedor`: va por la función
/// `proveedores_activos_en_otras_unidades` porque la RLS no deja ver lo de
/// otras unidades (la consulta directa siempre daba vacío).
pub fn proveedores_con_conflicto_activo(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<ConflictoIngresoProveedorActivo>, SincronizacionError> {
    let mut statement = connection.prepare(
        "SELECT cedula, nombre FROM registro_ingresos_proveedor
         WHERE fecha_hora_salida IS NULL",
    )?;
    let activos_locales: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    drop(statement);
    if activos_locales.is_empty() {
        return Ok(Vec::new());
    }

    let cedulas = activos_locales
        .iter()
        .map(|(cedula, _)| cedula.as_str())
        .collect::<Vec<_>>();
    let cliente = cliente_http();
    let filas: Vec<FilaConflictoProveedorActivo> = llamar_rpc(
        &cliente,
        contexto,
        "proveedores_activos_en_otras_unidades",
        &serde_json::json!({ "p_cedulas": cedulas }),
    )?;

    Ok(filas
        .into_iter()
        .filter_map(|fila| {
            let cedula = fila.cedula?;
            let sitio = fila.sitio_nombre?;
            let nombre = activos_locales
                .iter()
                .find(|(c, _)| *c == cedula)
                .map(|(_, nombre)| nombre.clone())?;
            Some(ConflictoIngresoProveedorActivo {
                cedula,
                nombre,
                sitio_conflicto: sitio,
            })
        })
        .collect())
}

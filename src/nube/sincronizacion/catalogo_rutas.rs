use rusqlite::{Connection, params};

use super::{ContextoSincronizacion, SincronizacionError, avanzar_marca, guardar_por_pagina};
use crate::nube::cliente::cliente_http;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResumenCatalogoRutas {
    /// Siempre 0: los vehículos de ruta ya no se descargan (función de rutas
    /// retirada). Se mantiene para no cambiar lo que leen las apps.
    pub vehiculos_recibidos: u32,
    pub encargados_recibidos: u32,
}

/// Sin `cedula` en el `SELECT` a propósito -- el catálogo remoto la trae
/// siempre `NULL` (pedido explícito del usuario, 2026-09-15: "es solo
/// nombre y código de empleado, la cédula ya me indicaron que no va") y el
/// formulario de escritorio tampoco la captura -- no hay ningún dato real
/// que este pull pudiera traer para esa columna.
#[derive(serde::Deserialize)]
pub(super) struct FilaEncargadoRutaRemota {
    pub(super) id: String,
    pub(super) codigo_empleado: String,
    pub(super) nombre: String,
    pub(super) activo: bool,
    pub(super) updated_at: String,
}

pub(super) fn guardar_encargados_ruta(
    transaction: &rusqlite::Transaction<'_>,
    encargados: &[FilaEncargadoRutaRemota],
) -> Result<u32, SincronizacionError> {
    let mut recibidos = 0;
    for encargado in encargados {
        transaction.execute(
            "
            INSERT INTO encargados_ruta (codigo_empleado, nombre, activo, uuid)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(codigo_empleado) DO UPDATE SET
                nombre = excluded.nombre,
                activo = excluded.activo,
                uuid = COALESCE(encargados_ruta.uuid, excluded.uuid)
            ",
            params![
                encargado.codigo_empleado,
                encargado.nombre,
                encargado.activo,
                encargado.id,
            ],
        )?;
        recibidos += 1;
    }
    Ok(recibidos)
}

/// Espejo de `recibir_catalogo_del_sitio`, separado en su propia función
/// (no sumado a esa) porque nace después y con marca de agua propia
/// (`catalogo_rutas_actualizado_hasta`, `MIGRACION_37`) -- mismo criterio que
/// `gafetes_actualizado_hasta`/`historial_visitas_actualizado_hasta`: ritmo
/// de sync independiente, sin depender de que el cursor del resto del
/// catálogo ya existiera cuando este dominio se sumó.
pub fn recibir_catalogo_rutas_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<ResumenCatalogoRutas, SincronizacionError> {
    let marca_anterior: Option<String> = connection.query_row(
        "SELECT catalogo_rutas_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let filtro_incremental = marca_anterior
        .as_deref()
        .map(|marca| format!("&updated_at=gt.{marca}"))
        .unwrap_or_default();
    let mut marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>> = marca_anterior
        .as_deref()
        .and_then(|marca| crate::tiempo::parsear_utc(marca).ok());
    let mut resumen = ResumenCatalogoRutas::default();
    let cliente = cliente_http();

    // Trae el personal KOF (`encargados_ruta`, lo usan los gafetes
    // provisionales) que este dispositivo todavía no tiene localmente. Los
    // vehículos de ruta ya no se piden: la función de rutas está retirada y su
    // tabla se va a borrar. Sin `sitio_id=eq...` a propósito -- la tabla es
    // global (ver `docs/planes-implementados/plan-control-rutas.md`,
    // "Alcance de RLS por tabla"), mismo criterio que empresas/contratistas.
    // Página por página, cada una en su propia transacción corta, y la
    // marca de agua recién al final -- mismo criterio que
    // `recibir_catalogo_del_sitio` (N6).
    guardar_por_pagina(
        connection,
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/encargados_ruta?select=id,codigo_empleado,nombre,activo,updated_at{filtro_incremental}",
            contexto.base_url
        ),
        |transaction, pagina: &[FilaEncargadoRutaRemota]| {
            avanzar_marca(&mut marca_mas_nueva, pagina.iter().map(|f| &f.updated_at))?;
            resumen.encargados_recibidos += guardar_encargados_ruta(transaction, pagina)?;
            Ok(())
        },
    )?;

    if let Some(marca) = marca_mas_nueva {
        connection.execute(
            "UPDATE sincronizacion_estado SET catalogo_rutas_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_marca_utc(marca)],
        )?;
    }
    Ok(resumen)
}

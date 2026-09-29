use super::anticipados::{self, Pedido};
use super::{ContextoSincronizacion, SincronizacionError};
use crate::nube::cliente::NubeError;

/// `GET` autenticado + deserializar la lista de filas -- compartido por
/// todo lo que trae datos de la nube hacia acá (`recibir_ingresos_abiertos`,
/// `recibir_catalogo_del_sitio`).
pub(super) fn obtener_json<T: serde::de::DeserializeOwned>(
    cliente: &reqwest::blocking::Client,
    contexto: &ContextoSincronizacion<'_>,
    url: &str,
) -> Result<Vec<T>, SincronizacionError> {
    // Si no se puede leer, se repite por la red con el manejo de siempre.
    if let Some(filas) = anticipados::tomar(Pedido::simple(url))
        .and_then(|cuerpo| serde_json::from_str(&cuerpo).ok())
    {
        return Ok(filas);
    }
    let respuesta = cliente
        .get(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .send()
        .map_err(NubeError::Red)?;

    if !respuesta.status().is_success() {
        let status = respuesta.status().as_u16();
        let cuerpo = respuesta.text().unwrap_or_default();
        return Err(SincronizacionError::RespuestaInesperada { status, cuerpo });
    }
    let filas = respuesta.json().map_err(NubeError::Red)?;
    Ok(filas)
}

/// Filas por página al paginar con `obtener_json_paginado_con` -- por
/// debajo del tope de filas por respuesta que Supabase/`PostgREST` impone
/// por defecto (`db-max-rows`, 1000) para que ninguna página sola pueda
/// chocar con ese límite y perder el resto en silencio.
pub(super) const TAMANO_PAGINA_REMOTA: usize = 500;

/// Traslape largo de los historiales: la reconciliación al abrir la app
/// (`AlcanceSincronizacion::arranque`).
pub(super) const DIAS_TRASLAPE_HISTORIAL: i64 = 7;

/// Traslape corto de los historiales: el pulso y los avisos en vivo.
/// `updated_at = now()` es la hora de INICIO de la transacción, así que una
/// fila puede hacerse visible con un `updated_at` apenas anterior a la
/// marca ya guardada; unos minutos alcanzan de sobra para eso (ver
/// docs/auditorias/auditoria-integral-2026-09-24, NS-09) sin volver a
/// bajar la semana entera en cada consulta.
pub(super) const MINUTOS_TRASLAPE_CORTO: i64 = 5;

/// Cuánto retrocede la consulta incremental de un historial respecto de su
/// marca de agua.
pub fn traslape_historial(reconciliar: bool) -> chrono::Duration {
    if reconciliar {
        chrono::Duration::days(DIAS_TRASLAPE_HISTORIAL)
    } else {
        chrono::Duration::minutes(MINUTOS_TRASLAPE_CORTO)
    }
}

/// Igual que `obtener_json`, pero para listas que pueden superar el tope de
/// filas por respuesta de `PostgREST` -- sin esto, un catálogo o un
/// historial que ya cruzó ese tope (típico en el primer sync de un sitio
/// grande, sin marca de agua que acote nada) perdía en silencio todo lo
/// que sobraba: no un error, sólo datos que nunca llegaban. Pagina con
/// `Range` (protocolo estándar de `PostgREST`) hasta que una página vuelve
/// con menos filas que `TAMANO_PAGINA_REMOTA`, señal de que no queda nada
/// más. `url_base` no debe traer su propio `order=` -- esta función agrega
/// uno por `id` (columna presente en todo lo que hoy pagina) para que el
/// orden entre páginas sea estable; sin un orden fijo, dos páginas
/// consecutivas de una tabla que sigue cambiando mientras se pagina
/// podrían saltarse o repetir filas.
///
/// Le entrega cada página a `por_pagina` a medida que llega, en vez de
/// juntarlas todas en memoria (hallazgo R-03 de
/// `docs/auditorias/auditoria-rendimiento-core-rust-2026-09-10.md` y N6 de
/// `docs/auditorias/auditoria-nucleo-rust-2026-09-27.md`): el primer sync
/// de un equipo nuevo trae el catálogo y el historial completos.
/// `por_pagina` puede abrir su propia transacción corta y comitearla por
/// página (ver [`guardar_por_pagina`]) -- eso además acorta cuánto tiempo se
/// retiene el candado de escritura comparado con una única transacción
/// gigante al final, y dos ventajas más: si la red se corta a mitad de la
/// descarga, las páginas ya comiteadas no se pierden (a diferencia de una
/// transacción única, que revierte todo); y las lecturas concurrentes
/// (`GuiState::conexion_secundaria`) sólo se bloquean durante cada
/// transacción corta, no durante toda la descarga.
pub(super) fn obtener_json_paginado_con<T, F>(
    cliente: &reqwest::blocking::Client,
    contexto: &ContextoSincronizacion<'_>,
    url_base: &str,
    mut por_pagina: F,
) -> Result<(), SincronizacionError>
where
    T: serde::de::DeserializeOwned,
    F: FnMut(Vec<T>) -> Result<(), SincronizacionError>,
{
    let separador = if url_base.contains('?') { '&' } else { '?' };
    let url = format!("{url_base}{separador}order=id.asc");

    let mut desde = 0_usize;
    loop {
        let anticipada: Option<Vec<T>> = if desde == 0 {
            anticipados::tomar(Pedido::primera_pagina(&url))
                .and_then(|cuerpo| serde_json::from_str(&cuerpo).ok())
        } else {
            None
        };
        let pagina = match anticipada {
            Some(pagina) => pagina,
            None => pedir_pagina(cliente, contexto, &url, desde)?,
        };
        let recibidas_en_esta_pagina = pagina.len();
        let hay_mas = recibidas_en_esta_pagina == TAMANO_PAGINA_REMOTA;
        por_pagina(pagina)?;
        if !hay_mas {
            break;
        }
        desde += TAMANO_PAGINA_REMOTA;
    }
    Ok(())
}

/// Una página de `url` (que ya trae su `order=`) a partir de la fila
/// `desde`.
fn pedir_pagina<T: serde::de::DeserializeOwned>(
    cliente: &reqwest::blocking::Client,
    contexto: &ContextoSincronizacion<'_>,
    url: &str,
    desde: usize,
) -> Result<Vec<T>, SincronizacionError> {
    let respuesta = cliente
        .get(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Range-Unit", "items")
        .header(
            "Range",
            format!("{desde}-{}", desde + TAMANO_PAGINA_REMOTA - 1),
        )
        .send()
        .map_err(NubeError::Red)?;

    // `is_success()` ya cubre el 206 Partial Content que `PostgREST`
    // devuelve cuando la página pedida no alcanza a cubrir todo lo que hay
    // -- no hace falta distinguirlo de un 200 normal, el criterio de "¿hay
    // más?" de `obtener_json_paginado_con` (cuántas filas vinieron) es el
    // mismo.
    if !respuesta.status().is_success() {
        let status = respuesta.status().as_u16();
        let cuerpo = respuesta.text().unwrap_or_default();
        return Err(SincronizacionError::RespuestaInesperada { status, cuerpo });
    }
    Ok(respuesta.json().map_err(NubeError::Red)?)
}

/// Baja `url` página por página y guarda cada página en su propia
/// transacción corta. Para el catálogo (empresas, contratistas, gafetes,
/// rutas...): una tabla termina de guardarse antes de pedir la siguiente,
/// así que el orden entre tablas que dependen una de otra (empresas antes
/// que contratistas, contratistas antes que gafetes) se mantiene igual que
/// cuando todo iba en una sola transacción.
///
/// Las marcas de agua NO se avanzan acá: quien llama las guarda sólo al
/// final, cuando todas las tablas llegaron. Si la red se corta a mitad, lo
/// ya guardado queda (cada fila es un `INSERT ... ON CONFLICT DO UPDATE`,
/// repetirla no duplica nada) y el próximo sync vuelve a pedir desde la
/// marca anterior.
pub(super) fn guardar_por_pagina<T, F>(
    connection: &rusqlite::Connection,
    cliente: &reqwest::blocking::Client,
    contexto: &ContextoSincronizacion<'_>,
    url: &str,
    mut guardar: F,
) -> Result<(), SincronizacionError>
where
    T: serde::de::DeserializeOwned,
    F: FnMut(&rusqlite::Transaction<'_>, &[T]) -> Result<(), SincronizacionError>,
{
    obtener_json_paginado_con(cliente, contexto, url, |pagina: Vec<T>| {
        let transaction = connection.unchecked_transaction()?;
        guardar(&transaction, &pagina)?;
        transaction.commit()?;
        Ok(())
    })
}

/// Sube `marca` al `updated_at` más nuevo entre `fechas` (texto tal cual lo
/// devolvió el servidor). Una fecha ilegible es un error: la marca de agua
/// no puede calcularse a medias.
pub(super) fn avanzar_marca<'a>(
    marca: &mut Option<chrono::DateTime<chrono::Utc>>,
    fechas: impl IntoIterator<Item = &'a String>,
) -> Result<(), SincronizacionError> {
    for texto in fechas {
        let fecha = crate::tiempo::parsear_utc(texto)
            .map_err(|_| SincronizacionError::FechaInvalida(texto.clone()))?;
        if marca.is_none_or(|actual| fecha > actual) {
            *marca = Some(fecha);
        }
    }
    Ok(())
}

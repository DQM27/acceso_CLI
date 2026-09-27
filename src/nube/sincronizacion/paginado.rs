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

/// Filas por página al paginar con `obtener_json_paginado` -- por debajo
/// del tope de filas por respuesta que Supabase/`PostgREST` impone por
/// defecto (`db-max-rows`, 1000) para que ninguna página sola pueda
/// chocar con ese límite y perder el resto en silencio.
pub(super) const TAMANO_PAGINA_REMOTA: usize = 500;

pub(super) const DIAS_TRASLAPE_HISTORIAL: i64 = 7;

/// Igual que `obtener_json`, pero para listas que pueden superar el tope de
/// filas por respuesta de `PostgREST` -- sin esto, un catálogo o un
/// historial que ya cruzó ese tope (típico en el primer sync de un sitio
/// grande, sin `marca_anterior` que acote nada) perdía en silencio todo lo
/// que sobraba: no un error, sólo datos que nunca llegaban. Pagina con
/// `Range` (protocolo estándar de `PostgREST`) hasta que una página vuelve
/// con menos filas que `TAMANO_PAGINA_REMOTA`, señal de que no queda nada
/// más. `url_base` no debe traer su propio `order=` -- esta función agrega
/// uno por `id` (columna presente en todo lo que hoy pagina) para que el
/// orden entre páginas sea estable; sin un orden fijo, dos páginas
/// consecutivas de una tabla que sigue cambiando mientras se pagina
/// podrían saltarse o repetir filas.
pub(super) fn obtener_json_paginado<T: serde::de::DeserializeOwned>(
    cliente: &reqwest::blocking::Client,
    contexto: &ContextoSincronizacion<'_>,
    url_base: &str,
) -> Result<Vec<T>, SincronizacionError> {
    let mut resultado = Vec::new();
    obtener_json_paginado_con(cliente, contexto, url_base, |pagina: Vec<T>| {
        resultado.extend(pagina);
        Ok(())
    })?;
    Ok(resultado)
}

/// Igual que [`obtener_json_paginado`], pero en vez de acumular todas las
/// páginas en un `Vec` antes de volver, le entrega cada página a
/// `por_pagina` a medida que llega -- pensada para `recibir_historial_del_sitio`,
/// el único llamador cuyo resultado puede llegar a ser grande de verdad (el
/// catálogo de un sitio -- contratistas, gafetes -- está acotado por la
/// plantilla física del sitio; el historial, no). Hallazgo R-03 de
/// `docs/auditorias/auditoria-rendimiento-core-rust-2026-09-10.md`: sin
/// esto, el primer sync de un sitio con historial grande junta todas las
/// páginas en memoria antes de persistir ninguna. `por_pagina` puede abrir
/// su propia transacción corta y comitearla por página -- eso además
/// acorta cuánto tiempo se retiene el candado de escritura comparado con
/// una única transacción gigante al final, y dos ventajas más: si la red
/// se corta a mitad de la descarga, las páginas ya comiteadas no se pierden
/// (a diferencia de una transacción única, que revierte todo); y las
/// lecturas concurrentes (`GuiState::conexion_secundaria`) sólo se bloquean
/// durante cada transacción corta, no durante toda la descarga.
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
        let respuesta = cliente
            .get(&url)
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
        // devuelve cuando la página pedida no alcanza a cubrir todo lo que
        // hay -- no hace falta distinguirlo de un 200 normal, el criterio
        // de "¿hay más?" de abajo (cuántas filas vinieron) es el mismo.
        if !respuesta.status().is_success() {
            let status = respuesta.status().as_u16();
            let cuerpo = respuesta.text().unwrap_or_default();
            return Err(SincronizacionError::RespuestaInesperada { status, cuerpo });
        }
        let pagina: Vec<T> = respuesta.json().map_err(NubeError::Red)?;
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

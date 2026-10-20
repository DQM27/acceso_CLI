//! Lo abierto del sitio y las salidas que otro equipo dio sobre lo de este,
//! en un solo llamado (`estado_del_sitio`, migración
//! `20261020150000_estado_del_sitio`).
//!
//! Reemplaza los hasta ocho GET de contratistas, proveedores, correo y
//! préstamos KOF de cada sincronización. Las etapas siguen siendo las mismas
//! y corren en el mismo orden (`orquestacion::recibir_en_orden`): sólo
//! reciben sus filas de acá en vez de pedirlas. Visitas no está en producción
//! y sigue con sus GET.

use rusqlite::Connection;

use super::correo::{ABIERTOS_LOCALES_CORREO, FilaIngresoCorreoRemoto};
use super::gafetes_provisionales::{
    ABIERTOS_LOCALES_PRESTAMOS, FilaDevolucionPropiaRemota, FilaPrestamoGafeteProvisionalRemoto,
};
use super::ingresos::{ABIERTOS_LOCALES_INGRESOS, FilaCierrePropioRemoto, FilaIngresoRemoto};
use super::proveedores::{ABIERTOS_LOCALES_PROVEEDOR, FilaIngresoProveedorRemoto};
use super::{ContextoSincronizacion, SincronizacionError};
use crate::nube::alcance::AlcanceSincronizacion;
use crate::nube::cliente::{NubeError, cliente_http};

/// Respuesta de `estado_del_sitio`, con los mismos tipos de fila que los GET
/// de cada etapa. Un grupo no pedido (o un cierre sin ids) no viene y queda
/// vacío; la etapa correspondiente o no corre o no tiene nada abierto.
#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub(in crate::nube) struct EstadoRemoto {
    pub(in crate::nube) cierres_ingresos: Vec<FilaCierrePropioRemoto>,
    pub(in crate::nube) ingresos_abiertos: Vec<FilaIngresoRemoto>,
    pub(in crate::nube) cierres_proveedor: Vec<FilaCierrePropioRemoto>,
    pub(in crate::nube) proveedor_abiertos: Vec<FilaIngresoProveedorRemoto>,
    pub(in crate::nube) cierres_correo: Vec<FilaCierrePropioRemoto>,
    pub(in crate::nube) correo_abiertos: Vec<FilaIngresoCorreoRemoto>,
    pub(in crate::nube) devoluciones_prestamos: Vec<FilaDevolucionPropiaRemota>,
    pub(in crate::nube) prestamos_abiertos: Vec<FilaPrestamoGafeteProvisionalRemoto>,
}

fn abiertos_localmente(
    connection: &Connection,
    consulta: &str,
) -> Result<Vec<String>, SincronizacionError> {
    let mut statement = connection.prepare(consulta)?;
    Ok(statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?)
}

/// Pide el estado del sitio para las clases del alcance. `None` si no le toca
/// ninguna, o si la nube todavía no tiene la función (404): entonces cada
/// etapa pide lo suyo por GET, como antes.
pub(in crate::nube) fn pedir_estado_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    alcance: &AlcanceSincronizacion,
) -> Result<Option<EstadoRemoto>, SincronizacionError> {
    let mut nombres = Vec::new();
    let mut cuerpo = serde_json::Map::new();
    for (pedido, nombre, parametro, consulta) in [
        (
            alcance.ingresos,
            "ingresos",
            "p_ingresos",
            ABIERTOS_LOCALES_INGRESOS,
        ),
        (
            alcance.ingresos_proveedor,
            "proveedor",
            "p_proveedor",
            ABIERTOS_LOCALES_PROVEEDOR,
        ),
        (
            alcance.ingresos_correo,
            "correo",
            "p_correo",
            ABIERTOS_LOCALES_CORREO,
        ),
        (
            alcance.gafetes_provisionales,
            "prestamos",
            "p_prestamos",
            ABIERTOS_LOCALES_PRESTAMOS,
        ),
    ] {
        if pedido {
            nombres.push(nombre);
            cuerpo.insert(
                parametro.to_owned(),
                serde_json::json!(abiertos_localmente(connection, consulta)?),
            );
        }
    }
    if nombres.is_empty() {
        return Ok(None);
    }
    cuerpo.insert("p_grupos".to_owned(), serde_json::json!(nombres));

    let respuesta = cliente_http()
        .post(format!(
            "{}/rest/v1/rpc/estado_del_sitio",
            contexto.base_url
        ))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;
    let status = respuesta.status().as_u16();
    if status == 404 {
        log::info!("estado_del_sitio no existe en la nube: cada etapa pide lo suyo");
        return Ok(None);
    }
    if !respuesta.status().is_success() {
        let cuerpo = respuesta.text().unwrap_or_default();
        return Err(SincronizacionError::RespuestaInesperada { status, cuerpo });
    }
    Ok(Some(respuesta.json().map_err(NubeError::Red)?))
}

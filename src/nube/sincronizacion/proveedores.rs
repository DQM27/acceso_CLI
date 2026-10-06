use rusqlite::{Connection, params};
use serde_json::json;

use super::{
    ContextoSincronizacion, FilaCierrePropioRemoto, SincronizacionError, exigir_2xx, obtener_json,
};
use crate::nube::cliente::{NubeError, cliente_http};

/// Espejo de [`recibir_cierres_de_ingresos_propios`], pero contra
/// `registro_ingresos_proveedor`/`ingresos_proveedor` -- faltaba (bug
/// reportado en pruebas reales, 2026-09-17): sin esto, un ingreso de
/// proveedor abierto en ESTE dispositivo y cerrado por OTRO nunca se
/// actualizaba acá -- `recibir_ingresos_proveedor_abiertos` sólo refresca la
/// caché de lo ajeno (`ingresos_proveedor_remotos`), no toca
/// `registro_ingresos_proveedor` propio -- así que el dispositivo dueño del
/// ingreso lo seguía mostrando como abierto para siempre, sin importar
/// cuántas veces sincronizara.
pub fn recibir_cierres_de_ingresos_propios_proveedor(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let abiertos_localmente: Vec<String> = {
        let mut statement = connection.prepare(
            "SELECT uuid FROM registro_ingresos_proveedor WHERE fecha_hora_salida IS NULL",
        )?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?
    };
    if abiertos_localmente.is_empty() {
        return Ok(0);
    }

    let cliente = cliente_http();
    let lista_uuids = abiertos_localmente.join(",");
    let url = format!(
        "{}/rest/v1/ingresos_proveedor?id=in.({lista_uuids})&hora_salida=not.is.null\
         &select=id,hora_salida,usuario_salida_nombre",
        contexto.base_url,
    );
    let filas: Vec<FilaCierrePropioRemoto> = obtener_json(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    let mut aplicados = 0_u32;
    for fila in &filas {
        let nombre_salida = fila
            .usuario_salida_nombre
            .as_deref()
            .unwrap_or("Salida registrada en nube");
        let hora_salida = crate::tiempo::parsear_utc(&fila.hora_salida)
            .map(crate::tiempo::serializar_utc)
            .map_err(|_| SincronizacionError::FechaInvalida(fila.hora_salida.clone()))?;
        let filas_afectadas = transaction.execute(
            "
            UPDATE registro_ingresos_proveedor
            SET
                fecha_hora_salida = ?1,
                usuario_salida_id = NULL,
                usuario_salida_nombre = ?2
            WHERE uuid = ?3
              AND fecha_hora_salida IS NULL
            ",
            params![hora_salida, nombre_salida, fila.id],
        )?;
        let filas_afectadas = u32::try_from(filas_afectadas).unwrap_or(u32::MAX);
        aplicados = aplicados.saturating_add(filas_afectadas);
    }
    transaction.commit()?;

    Ok(aplicados)
}

/// Fila cacheada localmente de un ingreso de proveedor todavía abierto,
/// creado por el otro dispositivo de este mismo sitio -- mismo criterio que
/// [`IngresoRemoto`], pero contra `ingresos_proveedor_remotos`/
/// `ingresos_proveedor` (ver `docs/features-futuras/plan-control-proveedores.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoProveedorRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: String,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaIngresoProveedorRemoto {
    pub(super) id: String,
    pub(super) cedula: String,
    pub(super) nombre: String,
    pub(super) empresa_nombre: String,
    pub(super) placa: Option<String>,
    pub(super) gafete_numero: i64,
    pub(super) hora_entrada: String,
    pub(super) usuario_entrada_nombre: String,
    pub(super) dispositivo_entrada_id: String,
}

/// Espejo de [`recibir_ingresos_abiertos`], pero contra `ingresos_proveedor`
/// -- misma lógica de "traer todo lo abierto del sitio y descartar lo que
/// ya vive local" (reinstalación de app incluida), mismo reemplazo completo
/// de la caché en una sola transacción.
pub fn recibir_ingresos_proveedor_abiertos(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<IngresoProveedorRemoto>, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/ingresos_proveedor?sitio_id=eq.{}&hora_salida=is.null\
         &select=id,cedula,nombre,empresa_nombre,placa,gafete_numero,hora_entrada,\
         usuario_entrada_nombre,dispositivo_entrada_id",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaIngresoProveedorRemoto> = obtener_json(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "DELETE FROM ingresos_proveedor_remotos WHERE sitio_id = ?1",
        params![contexto.sitio_id],
    )?;
    super::cierres_remotos::olvidar_cierres_confirmados(
        &transaction,
        "ingresos_proveedor_remotos",
        filas.iter().map(|fila| fila.id.as_str()),
    )?;
    let mut remotos = Vec::with_capacity(filas.len());
    for fila in filas {
        let existe_localmente: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM registro_ingresos_proveedor WHERE uuid = ?1)",
            params![fila.id],
            |row| row.get(0),
        )?;
        // Lo propio no se duplica en la caché; lo que este equipo acaba de
        // cerrar a mano tampoco vuelve (ver `cierres_remotos`).
        if existe_localmente || super::cierres_remotos::cerrado_aca(&transaction, &fila.id)? {
            continue;
        }
        let hora_entrada = crate::tiempo::parsear_utc(&fila.hora_entrada)
            .map(crate::tiempo::serializar_utc)
            .map_err(|_| SincronizacionError::FechaInvalida(fila.hora_entrada.clone()))?;
        transaction.execute(
            "
            INSERT INTO ingresos_proveedor_remotos (
                uuid, sitio_id, cedula, nombre, empresa_nombre, placa, gafete_numero,
                hora_entrada, usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            ",
            params![
                fila.id,
                contexto.sitio_id,
                fila.cedula,
                fila.nombre,
                fila.empresa_nombre,
                fila.placa,
                fila.gafete_numero,
                hora_entrada,
                fila.usuario_entrada_nombre,
                fila.dispositivo_entrada_id,
            ],
        )?;
        remotos.push(IngresoProveedorRemoto {
            uuid: fila.id,
            cedula: fila.cedula,
            nombre: fila.nombre,
            empresa_nombre: fila.empresa_nombre,
            placa: fila.placa,
            gafete_numero: fila.gafete_numero,
            hora_entrada,
            usuario_entrada_nombre: fila.usuario_entrada_nombre,
        });
    }
    transaction.commit()?;

    Ok(remotos)
}

/// Espejo de [`cerrar_ingreso_remoto`] (también en cómo se sella `hora`),
/// pero contra `ingresos_proveedor`.
pub fn cerrar_ingreso_proveedor_remoto(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
    usuario_salida_nombre: &str,
    hora: chrono::DateTime<chrono::Utc>,
) -> Result<(), SincronizacionError> {
    let cliente = cliente_http();
    let cuerpo = json!({
        "hora_salida": crate::tiempo::serializar_utc(hora),
        "dispositivo_salida_id": contexto.dispositivo_id,
        "usuario_salida_nombre": usuario_salida_nombre,
    });

    let url = format!(
        "{}/rest/v1/ingresos_proveedor?id=eq.{uuid}&hora_salida=is.null",
        contexto.base_url
    );
    let respuesta = cliente
        .patch(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)?;

    // Borra de la caché y anota la lápida juntos: una recepción que leyó la
    // nube antes del cierre no la vuelve a meter (ver `cierres_remotos`).
    super::cierres_remotos::anotar_cierre_remoto(connection, "ingresos_proveedor_remotos", uuid)
}

// ---- Gafetes provisionales KOF: sync entre dispositivos ----
//
// Faltaba por completo -- sólo existían el push (`enviar_prestamo_gafete_provisional`/
// `enviar_cierre_prestamo_gafete_provisional`) y el chequeo en vivo
// (`gafete_provisional_ocupado_en_otro_dispositivo`), pero nada traía de
// vuelta lo que OTRO dispositivo entregó/devolvió -- un préstamo hecho en
// el celular nunca aparecía en la PC (ni viceversa), y un préstamo cerrado
// por otro dispositivo se quedaba "abierto" para siempre del lado de quien
// lo entregó. Mismo patrón exacto que `ingresos_proveedor_remotos`/
// `recibir_cierres_de_ingresos_propios_proveedor` -- bug reportado en
// pruebas reales, 2026-09-17 ("yo sabía que no estaba sincronizada").

//! Ingreso "por correo" entre dispositivos del sitio: mismo patrón exacto que
//! `proveedores.rs` (`ingresos_proveedor_remotos`), contra
//! `registro_ingresos_correo`/`ingresos_correo_remotos`/`ingresos_correo`.

use rusqlite::{Connection, params};
use serde_json::json;

use super::{
    ConflictoIngresoProveedorActivo, ContextoSincronizacion, FilaCierrePropioRemoto,
    SincronizacionError, exigir_2xx, llamar_rpc, obtener_json,
};
use crate::nube::cliente::{NubeError, cliente_http};

/// Un ingreso por correo abierto en ESTE dispositivo y cerrado por OTRO: se
/// cierra acá también (mismo motivo que
/// `recibir_cierres_de_ingresos_propios_proveedor`).
pub fn recibir_cierres_de_ingresos_propios_correo(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let abiertos_localmente: Vec<String> = {
        let mut statement = connection
            .prepare("SELECT uuid FROM registro_ingresos_correo WHERE fecha_hora_salida IS NULL")?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?
    };
    if abiertos_localmente.is_empty() {
        return Ok(0);
    }

    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/ingresos_correo?id=in.({})&hora_salida=not.is.null\
         &select=id,hora_salida,usuario_salida_nombre",
        contexto.base_url,
        abiertos_localmente.join(","),
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
            UPDATE registro_ingresos_correo
            SET
                fecha_hora_salida = ?1,
                usuario_salida_id = NULL,
                usuario_salida_nombre = ?2
            WHERE uuid = ?3
              AND fecha_hora_salida IS NULL
            ",
            params![hora_salida, nombre_salida, fila.id],
        )?;
        aplicados = aplicados.saturating_add(u32::try_from(filas_afectadas).unwrap_or(u32::MAX));
    }
    transaction.commit()?;
    Ok(aplicados)
}

/// Ingreso por correo todavía abierto, creado por el otro dispositivo del
/// sitio (caché `ingresos_correo_remotos`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoCorreoRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub motivo: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: String,
}

#[derive(serde::Deserialize)]
struct FilaIngresoCorreoRemoto {
    id: String,
    cedula: String,
    nombre: String,
    motivo: String,
    placa: Option<String>,
    gafete_numero: i64,
    hora_entrada: String,
    usuario_entrada_nombre: String,
    dispositivo_entrada_id: String,
}

/// Trae todo lo abierto del sitio y descarta lo que ya vive local; reemplaza
/// la caché completa en una sola transacción (mismo criterio que
/// `recibir_ingresos_proveedor_abiertos`).
pub fn recibir_ingresos_correo_abiertos(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<IngresoCorreoRemoto>, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/ingresos_correo?sitio_id=eq.{}&hora_salida=is.null\
         &select=id,cedula,nombre,motivo,placa,gafete_numero,hora_entrada,\
         usuario_entrada_nombre,dispositivo_entrada_id",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaIngresoCorreoRemoto> = obtener_json(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "DELETE FROM ingresos_correo_remotos WHERE sitio_id = ?1",
        params![contexto.sitio_id],
    )?;
    super::cierres_remotos::olvidar_cierres_confirmados(
        &transaction,
        "ingresos_correo_remotos",
        filas.iter().map(|fila| fila.id.as_str()),
    )?;
    let mut remotos = Vec::with_capacity(filas.len());
    for fila in filas {
        let existe_localmente: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM registro_ingresos_correo WHERE uuid = ?1)",
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
            INSERT INTO ingresos_correo_remotos (
                uuid, sitio_id, cedula, nombre, motivo, placa, gafete_numero,
                hora_entrada, usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            ",
            params![
                fila.id,
                contexto.sitio_id,
                fila.cedula,
                fila.nombre,
                fila.motivo,
                fila.placa,
                fila.gafete_numero,
                hora_entrada,
                fila.usuario_entrada_nombre,
                fila.dispositivo_entrada_id,
            ],
        )?;
        remotos.push(IngresoCorreoRemoto {
            uuid: fila.id,
            cedula: fila.cedula,
            nombre: fila.nombre,
            motivo: fila.motivo,
            placa: fila.placa,
            gafete_numero: fila.gafete_numero,
            hora_entrada,
            usuario_entrada_nombre: fila.usuario_entrada_nombre,
        });
    }
    transaction.commit()?;
    Ok(remotos)
}

/// Cierra en la nube un ingreso por correo abierto por el otro dispositivo
/// del sitio (mismo criterio que `cerrar_ingreso_proveedor_remoto`).
pub fn cerrar_ingreso_correo_remoto(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
    usuario_salida_nombre: &str,
    hora: chrono::DateTime<chrono::Utc>,
) -> Result<(), SincronizacionError> {
    let cuerpo = json!({
        "hora_salida": crate::tiempo::serializar_utc(hora),
        "dispositivo_salida_id": contexto.dispositivo_id,
        "usuario_salida_nombre": usuario_salida_nombre,
    });
    let respuesta = cliente_http()
        .patch(format!(
            "{}/rest/v1/ingresos_correo?id=eq.{uuid}&hora_salida=is.null",
            contexto.base_url
        ))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;
    exigir_2xx(respuesta)?;

    // Borra de la caché y anota la lápida juntos: una recepción que leyó la
    // nube antes del cierre no la vuelve a meter (ver `cierres_remotos`).
    super::cierres_remotos::anotar_cierre_remoto(connection, "ingresos_correo_remotos", uuid)
}

#[derive(serde::Deserialize)]
struct FilaConflictoCorreoActivo {
    cedula: Option<String>,
    sitio_nombre: Option<String>,
}

/// Ingresos por correo abiertos en ESTE dispositivo cuya cédula también está
/// adentro en otra unidad (función `correos_activos_en_otras_unidades`) --
/// mismo criterio que `proveedores_con_conflicto_activo`, con el mismo tipo
/// de aviso.
pub fn correos_con_conflicto_activo(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<ConflictoIngresoProveedorActivo>, SincronizacionError> {
    let activos_locales: Vec<(String, String)> = {
        let mut statement = connection.prepare(
            "SELECT cedula, nombre FROM registro_ingresos_correo WHERE fecha_hora_salida IS NULL",
        )?;
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    if activos_locales.is_empty() {
        return Ok(Vec::new());
    }

    let cedulas = activos_locales
        .iter()
        .map(|(cedula, _)| cedula.as_str())
        .collect::<Vec<_>>();
    let filas: Vec<FilaConflictoCorreoActivo> = llamar_rpc(
        &cliente_http(),
        contexto,
        "correos_activos_en_otras_unidades",
        &serde_json::json!({ "p_cedulas": cedulas }),
    )?;

    Ok(filas
        .into_iter()
        .filter_map(|fila| {
            let cedula = fila.cedula?;
            let sitio_conflicto = fila.sitio_nombre?;
            let nombre = activos_locales
                .iter()
                .find(|(c, _)| *c == cedula)
                .map(|(_, nombre)| nombre.clone())?;
            Some(ConflictoIngresoProveedorActivo {
                cedula,
                nombre,
                sitio_conflicto,
            })
        })
        .collect())
}

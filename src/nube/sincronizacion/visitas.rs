//! Visitas agendadas entre los equipos de la unidad: mismo patrón que
//! `correo.rs`, contra `movimientos_visita`/`movimientos_visita_remotos`.
//! Así el teléfono y la PC ven quién está adentro en toda la unidad y le dan
//! salida desde cualquiera de los dos (pedido del dueño 2026-10-06).

use rusqlite::{Connection, params};
use serde_json::json;

use super::{
    ContextoSincronizacion, FilaCierrePropioRemoto, SincronizacionError, exigir_2xx, obtener_json,
};
use crate::nube::cliente::{NubeError, cliente_http};

/// Una visita abierta en ESTE equipo y cerrada por el OTRO: se cierra acá
/// también (mismo motivo que `recibir_cierres_de_ingresos_propios_correo`).
/// Sin esto el teléfono la seguía mostrando adentro después de que la PC le
/// dio salida.
pub fn recibir_cierres_de_movimientos_visita_propios(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let abiertos_localmente: Vec<String> = {
        let mut statement = connection
            .prepare("SELECT uuid FROM movimientos_visita WHERE fecha_hora_salida IS NULL")?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?
    };
    if abiertos_localmente.is_empty() {
        return Ok(0);
    }

    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/movimientos_visita?id=in.({})&hora_salida=not.is.null\
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
            UPDATE movimientos_visita
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

/// Visita agendada todavía abierta, registrada por el otro equipo de la
/// unidad (caché `movimientos_visita_remotos`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MovimientoVisitaRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub empresa: Option<String>,
    pub anfitrion_nombre: Option<String>,
    pub motivo: Option<String>,
    pub gafete_numero: Option<i64>,
    pub placa: Option<String>,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: Option<String>,
}

#[derive(serde::Deserialize)]
struct FilaMovimientoVisitaRemoto {
    id: String,
    visitante_cedula: String,
    visitante_nombre: String,
    empresa: Option<String>,
    anfitrion_nombre: Option<String>,
    motivo: Option<String>,
    gafete_numero: Option<i64>,
    placa: Option<String>,
    hora_entrada: String,
    usuario_entrada_nombre: Option<String>,
    dispositivo_entrada_id: String,
}

/// Trae todo lo abierto de la unidad y descarta lo que ya vive local;
/// reemplaza la caché completa en una sola transacción (mismo criterio que
/// `recibir_ingresos_correo_abiertos`).
pub fn recibir_movimientos_visita_abiertos(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<MovimientoVisitaRemoto>, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/movimientos_visita?sitio_id=eq.{}&hora_salida=is.null\
         &select=id,visitante_cedula,visitante_nombre,empresa,anfitrion_nombre,motivo,\
         gafete_numero,placa,hora_entrada,usuario_entrada_nombre,dispositivo_entrada_id",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaMovimientoVisitaRemoto> = obtener_json(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "DELETE FROM movimientos_visita_remotos WHERE sitio_id = ?1",
        params![contexto.sitio_id],
    )?;
    super::cierres_remotos::olvidar_cierres_confirmados(
        &transaction,
        "movimientos_visita_remotos",
        filas.iter().map(|fila| fila.id.as_str()),
    )?;
    let mut remotos = Vec::with_capacity(filas.len());
    for fila in filas {
        let existe_localmente: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM movimientos_visita WHERE uuid = ?1)",
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
            INSERT INTO movimientos_visita_remotos (
                uuid, sitio_id, cedula, nombre, empresa, anfitrion_nombre, motivo,
                gafete_numero, placa, hora_entrada, usuario_entrada_nombre,
                dispositivo_entrada_id, actualizado_en
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                      strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            ",
            params![
                fila.id,
                contexto.sitio_id,
                fila.visitante_cedula,
                fila.visitante_nombre,
                fila.empresa,
                fila.anfitrion_nombre,
                fila.motivo,
                fila.gafete_numero,
                fila.placa,
                hora_entrada,
                fila.usuario_entrada_nombre,
                fila.dispositivo_entrada_id,
            ],
        )?;
        remotos.push(MovimientoVisitaRemoto {
            uuid: fila.id,
            cedula: fila.visitante_cedula,
            nombre: fila.visitante_nombre,
            empresa: fila.empresa,
            anfitrion_nombre: fila.anfitrion_nombre,
            motivo: fila.motivo,
            gafete_numero: fila.gafete_numero,
            placa: fila.placa,
            hora_entrada,
            usuario_entrada_nombre: fila.usuario_entrada_nombre,
        });
    }
    transaction.commit()?;
    Ok(remotos)
}

/// Da salida en la nube a una visita abierta por el otro equipo de la
/// unidad (mismo criterio que `cerrar_ingreso_correo_remoto`).
pub fn cerrar_movimiento_visita_remoto(
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
            "{}/rest/v1/movimientos_visita?id=eq.{uuid}&hora_salida=is.null",
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
    super::cierres_remotos::anotar_cierre_remoto(connection, "movimientos_visita_remotos", uuid)
}

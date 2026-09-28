//! Veto por persona: baja la copia local de `personas_vetadas` (ver
//! `docs/features-futuras/plan-veto-por-persona.md`). Global a todos los
//! sitios, como contratistas y empresas. Los equipos sólo pueden leer lo
//! mínimo para bloquear (id, cédula, si está levantado y `updated_at`); el
//! motivo nunca viaja.
//!
//! Marca de agua propia (`vetos_actualizado_hasta`), incremental por
//! `updated_at`. Bajan también los vetos levantados: son los que dejan de
//! bloquear.

use rusqlite::{Connection, params};

use super::{ContextoSincronizacion, SincronizacionError, avanzar_marca, guardar_por_pagina};
use crate::nube::cliente::cliente_http;

#[derive(serde::Deserialize)]
pub(in crate::nube) struct FilaPersonaVetadaRemota {
    pub(in crate::nube) id: String,
    pub(in crate::nube) cedula: String,
    pub(in crate::nube) levantado_en: Option<String>,
    pub(in crate::nube) updated_at: String,
}

/// Cuántos vetos (nuevos, cambiados o levantados) se aplicaron.
pub fn recibir_personas_vetadas(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let marca_anterior: Option<String> = connection.query_row(
        "SELECT vetos_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
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

    let mut recibidos = 0_u32;
    guardar_por_pagina(
        connection,
        &cliente_http(),
        contexto,
        &format!(
            "{}/rest/v1/personas_vetadas?select=id,cedula,levantado_en,updated_at{filtro_incremental}",
            contexto.base_url
        ),
        |transaction, pagina: &[FilaPersonaVetadaRemota]| {
            avanzar_marca(&mut marca_mas_nueva, pagina.iter().map(|f| &f.updated_at))?;
            for fila in pagina {
                guardar_persona_vetada(transaction, fila)?;
                recibidos += 1;
            }
            Ok(())
        },
    )?;

    // La marca sólo avanza cuando llegaron todas las páginas, igual que el
    // resto del catálogo (N6).
    if let Some(marca) = marca_mas_nueva {
        connection.execute(
            "UPDATE sincronizacion_estado SET vetos_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_utc(marca)],
        )?;
    }
    Ok(recibidos)
}

/// Guarda o actualiza un veto en la copia local. Compartida con el aviso en
/// vivo (`nube::en_vivo`), así que el resultado es idéntico por los dos
/// caminos. La cédula ya viene en forma única desde la nube (CHECK de la
/// tabla), pero se normaliza igual: un veto nunca debe depender del formato.
pub(in crate::nube) fn guardar_persona_vetada(
    connection: &Connection,
    fila: &FilaPersonaVetadaRemota,
) -> Result<(), SincronizacionError> {
    let cedula = crate::domain::cedula::Cedula::normalizar(&fila.cedula).map_or_else(
        |_| fila.cedula.clone(),
        crate::domain::cedula::Cedula::into_string,
    );
    let actualizado_en = crate::tiempo::parsear_utc(&fila.updated_at)
        .map(crate::tiempo::serializar_utc)
        .map_err(|_| SincronizacionError::FechaInvalida(fila.updated_at.clone()))?;
    connection.execute(
        "
        INSERT INTO personas_vetadas (uuid, cedula, vigente, actualizado_en)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(uuid) DO UPDATE SET
            cedula = excluded.cedula,
            vigente = excluded.vigente,
            actualizado_en = excluded.actualizado_en
        ",
        params![fila.id, cedula, fila.levantado_en.is_none(), actualizado_en],
    )?;
    Ok(())
}

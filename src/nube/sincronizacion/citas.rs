use rusqlite::{Connection, params};

use super::{ContextoSincronizacion, SincronizacionError, obtener_json_paginado_con};
use crate::nube::cliente::cliente_http;

/// Nombre del anfitrión, embebido vía `PostgREST`
/// (`anfitrion:anfitriones!citas_anfitrion_correo_fkey(nombre)`) -- la
/// política de `anfitriones` que deja leerlo desde un dispositivo del sitio
/// vive en la migración `autoriza_lectura_anfitrion_por_dispositivo_del_sitio`.
/// `Option` porque un embed que RLS filtra vuelve `null`, no un error --
/// nunca debería pasar dado que esa política ya existe, pero no hay forma
/// de que el tipo lo garantice.
#[derive(serde::Deserialize)]
pub(super) struct AnfitrionEmbebido {
    pub(super) nombre: String,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaCitaVisitanteRemota {
    pub(super) id: String,
    pub(super) cedula: String,
    pub(super) nombre: String,
    pub(super) empresa: Option<String>,
    pub(super) placa_vehiculo: Option<String>,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaCitaRemota {
    pub(super) id: String,
    pub(super) motivo: Option<String>,
    // `date` de Postgres, no `timestamptz` -- PostgREST ya lo manda como
    // "YYYY-MM-DD" sin hora, mismo formato que espera `citas.fecha_desde`/
    // `fecha_hasta` local -- a diferencia de `hora_entrada`/`updated_at` en
    // otras filas remotas, esto no necesita reparsear/reformatear.
    pub(super) fecha_desde: String,
    pub(super) fecha_hasta: String,
    /// Texto libre tipo "HH:MM", puramente informativo -- ver el
    /// doc-comment de `MIGRACION_33` del lado local.
    pub(super) hora_estimada: Option<String>,
    pub(super) anfitrion_correo: String,
    pub(super) anfitrion: Option<AnfitrionEmbebido>,
    pub(super) estado: String,
    pub(super) updated_at: String,
    // Embebido en la misma consulta (`cita_visitantes(...)`) -- un solo
    // viaje de red trae la cita completa con su grupo, en vez de una
    // consulta aparte por cada una.
    pub(super) cita_visitantes: Vec<FilaCitaVisitanteRemota>,
}

/// Guarda una cita y su grupo de visitantes; devuelve el `updated_at`
/// parseado si se aplicó, o `None` si se omitió por una fecha ilegible --
/// mismo criterio de resiliencia que `aplicar_pagina_historial`: una fila mala
/// no puede abortar la transacción entera y dejar sin citas a un
/// dispositivo que necesita traerlas todas (recién reinstalado, sin marca
/// de agua todavía).
pub(super) fn guardar_cita_remota(
    transaction: &rusqlite::Transaction<'_>,
    fila: &FilaCitaRemota,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, SincronizacionError> {
    let Ok(actualizado_en) = crate::tiempo::parsear_utc(&fila.updated_at) else {
        return Ok(None);
    };

    let anfitrion_nombre = fila
        .anfitrion
        .as_ref()
        .map_or("—", |anfitrion| anfitrion.nombre.as_str());

    transaction.execute(
        "
        INSERT INTO citas (
            uuid, motivo, fecha_desde, fecha_hasta, hora_estimada, anfitrion_nombre,
            anfitrion_correo, estado, creado_en
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(uuid) DO UPDATE SET
            motivo = excluded.motivo,
            fecha_desde = excluded.fecha_desde,
            fecha_hasta = excluded.fecha_hasta,
            hora_estimada = excluded.hora_estimada,
            anfitrion_nombre = excluded.anfitrion_nombre,
            anfitrion_correo = excluded.anfitrion_correo,
            estado = excluded.estado
        ",
        params![
            fila.id,
            fila.motivo,
            fila.fecha_desde,
            fila.fecha_hasta,
            fila.hora_estimada,
            anfitrion_nombre,
            fila.anfitrion_correo,
            fila.estado,
            crate::tiempo::serializar_utc(actualizado_en),
        ],
    )?;

    let cita_id_local: i64 = transaction.query_row(
        "SELECT id FROM citas WHERE uuid = ?1",
        params![fila.id],
        |row| row.get(0),
    )?;

    for visitante in &fila.cita_visitantes {
        transaction.execute(
            "
            INSERT INTO cita_visitantes (uuid, cita_id, cedula, nombre, empresa, placa_vehiculo)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(uuid) DO UPDATE SET
                cita_id = excluded.cita_id,
                cedula = excluded.cedula,
                nombre = excluded.nombre,
                empresa = excluded.empresa,
                placa_vehiculo = excluded.placa_vehiculo
            ",
            params![
                visitante.id,
                cita_id_local,
                visitante.cedula,
                visitante.nombre,
                visitante.empresa,
                visitante.placa_vehiculo,
            ],
        )?;
    }

    Ok(Some(actualizado_en))
}

/// Trae a `citas`/`cita_visitantes` las citas que aplican a este sitio --
/// mismo mecanismo incremental que `recibir_historial_del_sitio`
/// (`citas_actualizado_hasta`, columna propia, ritmo de sync independiente),
/// pero sin filtro explícito de `sitio_id` en la URL: a diferencia de
/// `ingresos`/`gafetes`, una cita no tiene una columna de sitio directa
/// (vive en `cita_sitios`, el puente muchos-a-muchos) -- la política RLS
/// "leer citas propias, del sitio, o admin" ya resuelve ese filtro del lado
/// del servidor, agregarlo acá sería repetir la misma pregunta dos veces.
pub fn recibir_citas_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let cliente = cliente_http();

    let marca_anterior: Option<String> = connection.query_row(
        "SELECT citas_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let filtro_incremental = marca_anterior
        .as_deref()
        .map(|marca| format!("&updated_at=gt.{marca}"))
        .unwrap_or_default();

    let url = format!(
        "{}/rest/v1/citas?select=id,motivo,fecha_desde,fecha_hasta,hora_estimada,\
         anfitrion_correo,estado,updated_at,\
         anfitrion:anfitriones!citas_anfitrion_correo_fkey(nombre),\
         cita_visitantes(id,cedula,nombre,empresa,placa_vehiculo){filtro_incremental}",
        contexto.base_url,
    );
    // Página por página en vez de acumular todas las citas remotas en un
    // `Vec` antes de tocar la base -- mismo criterio que
    // `recibir_historial_del_sitio` (hallazgo R-03 de
    // `docs/auditorias/AUDITORIA_RENDIMIENTO_CORE_RUST_2026-09-10.md`): a
    // diferencia del catálogo (contratistas/gafetes, acotado por la
    // plantilla física del sitio), las citas se acumulan con el tiempo sin
    // un tope natural.
    let mut recibidas_total = 0_u32;
    let mut marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>> = marca_anterior
        .as_deref()
        .and_then(|marca| crate::tiempo::parsear_utc(marca).ok());
    obtener_json_paginado_con(&cliente, contexto, &url, |pagina: Vec<FilaCitaRemota>| {
        let (recibidas, marca_actualizada) =
            aplicar_pagina_citas(connection, &pagina, marca_mas_nueva)?;
        recibidas_total += recibidas;
        marca_mas_nueva = marca_actualizada;
        Ok(())
    })?;

    Ok(recibidas_total)
}

/// Persiste una página de citas remotas en su propia transacción corta,
/// incluida la marca de agua -- ver el doc-comment de
/// `aplicar_pagina_historial`, mismo criterio.
pub(super) fn aplicar_pagina_citas(
    connection: &Connection,
    pagina: &[FilaCitaRemota],
    marca_previa: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(u32, Option<chrono::DateTime<chrono::Utc>>), SincronizacionError> {
    let transaction = connection.unchecked_transaction()?;
    let mut recibidas = 0_u32;
    let mut marca_mas_nueva = marca_previa;
    for fila in pagina {
        let Some(actualizado_en) = guardar_cita_remota(&transaction, fila)? else {
            continue;
        };
        recibidas += 1;
        if marca_mas_nueva.is_none_or(|marca| actualizado_en > marca) {
            marca_mas_nueva = Some(actualizado_en);
        }
    }

    if let Some(marca) = marca_mas_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET citas_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_utc(marca)],
        )?;
    }
    transaction.commit()?;
    Ok((recibidas, marca_mas_nueva))
}

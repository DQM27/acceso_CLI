//! Drena la bandeja de salida (`cola_salida`, ver
//! `docs/planes-implementados/plan-persistencia-nube.md`) hacia el receptor: por cada fila
//! pendiente, arma el pedido HTTP correspondiente y la marca `enviado` o
//! `fallido` según la respuesta. Una fila fallida no detiene a las demás --
//! se reintenta en la próxima llamada, no bloquea el resto de la cola.
//!
//! Las filas de un mismo tipo (`empresa`, `contratista`, `gafete`,
//! `usuario`, apertura de `ingreso`) se agrupan y se intentan mandar en un
//! solo `POST` con un array -- `PostgREST` hace `upsert` de un array igual
//! que de un objeto suelto. Si el lote entero falla, se cae a mandar esas
//! mismas filas una por una: un `INSERT`/`upsert` de varias filas es
//! atómico en Postgres, así que una sola fila inválida (por ejemplo, un
//! contratista cuya empresa todavía no llegó) tumbaría a todo el lote junto
//! si no se aislara así -- ver hallazgo R-02 de
//! `docs/auditorias/auditoria-rendimiento-core-rust-2026-09-10.md`. El
//! cierre de un ingreso (`PATCH .../ingresos?...&hora_salida=is.null`)
//! queda afuera del lote a propósito: es un `UPDATE` condicional
//! ("primero en llegar gana"), no un `upsert`, y un `PATCH` con array no
//! aplica una condición distinta por fila.

use super::cliente::NubeError;

mod catalogo;
mod catalogo_rutas;
mod citas;
mod cola;
mod conflictos;
mod consultas_en_vivo;
mod gafetes_provisionales;
mod historial;
mod ingresos;
mod paginado;
mod proveedores;

pub use catalogo::*;
pub use catalogo_rutas::*;
pub use citas::*;
pub use cola::*;
pub use conflictos::*;
pub use consultas_en_vivo::*;
pub use gafetes_provisionales::*;
pub use historial::*;
pub use ingresos::*;
use paginado::{
    DIAS_TRASLAPE_HISTORIAL, avanzar_marca, guardar_por_pagina, obtener_json,
    obtener_json_paginado_con,
};
pub use proveedores::*;

/// Todo lo que hace falta para hablar con el receptor en nombre de este
/// dispositivo. `apikey` es la clave publicable del proyecto (no un
/// secreto -- ver `get_publishable_keys`), separada del `token` (el JWT que
/// ya identifica a este dispositivo y a su sitio).
pub struct ContextoSincronizacion<'a> {
    pub base_url: &'a str,
    pub apikey: &'a str,
    pub token: &'a str,
    pub dispositivo_id: &'a str,
    pub sitio_id: &'a str,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResumenDrenado {
    pub enviados: u32,
    pub fallidos: u32,
    /// Filas que fallaron de forma permanente por un choque real de
    /// gafete entre dos dispositivos del mismo sitio -- ver
    /// [`ConflictoGafeteActivo`] y `procesar_fila_individual`. A
    /// diferencia de `ConflictoIngresoActivo` (que hace falta consultar
    /// aparte, con su propia llamada de red por plataforma), esto se
    /// calcula acá mismo con datos ya locales, así que ambas plataformas
    /// lo reciben gratis con sólo leer este campo -- sin duplicar la
    /// consulta.
    pub conflictos_gafete: Vec<ConflictoGafeteActivo>,
}

/// Un ingreso que este dispositivo registró con gafete, pero cuyo envío a
/// la nube fue rechazado porque otro dispositivo del MISMO sitio ya tiene
/// ese número activo -- el índice único parcial `ingresos_gafete_activo_sitio_idx`
/// (ver la migración) lo detecta en el momento del `POST`, no en una
/// consulta aparte. A diferencia de [`ConflictoIngresoActivo`] (simétrico:
/// ambos lados "tienen razón" hasta que alguien decide), acá Postgres ya
/// decidió -- el ingreso local de ESTE dispositivo es el que no quedó
/// válido, así que el mensaje puede decirlo con esa certeza.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ConflictoGafeteActivo {
    pub contratista_nombre: String,
    pub gafete_numero: i64,
    pub fecha_hora_ingreso: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SincronizacionError {
    #[error("Error de base de datos local: {0}")]
    BaseLocal(#[from] rusqlite::Error),
    #[error(transparent)]
    Red(#[from] NubeError),
    #[error("El receptor respondió {status}: {cuerpo}")]
    RespuestaInesperada { status: u16, cuerpo: String },
    #[error("El receptor mandó una fecha inválida: {0}")]
    FechaInvalida(String),
}

impl SincronizacionError {
    /// El `TokenDispositivo` cacheado (ver `Nucleo::autenticar_con_cache` en
    /// móvil, `GuiState::autenticar_con_cache` en escritorio) parecía
    /// vigente del lado del cliente (no pasó su `expires_in` con margen) pero
    /// el receptor lo rechazó igual -- desfase de reloj, revocación a mitad
    /// de sesión, o el dispositivo estuvo inactivo más de lo que el caché
    /// asumía. Quien orquesta la sincronización usa esto para invalidar el
    /// caché y reintentar UNA vez con un token recién pedido, en vez de
    /// fallar la sincronización entera por un token que el propio cliente
    /// creía bueno.
    pub fn token_dispositivo_vencido(&self) -> bool {
        matches!(self, Self::RespuestaInesperada { status: 401, .. })
    }
}

#[cfg(test)]
mod tests;

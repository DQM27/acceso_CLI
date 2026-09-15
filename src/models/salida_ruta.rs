use chrono::{DateTime, NaiveDate, Utc};

use crate::domain::resultado_salida_ruta::ResultadoSalidaRuta;

/// `vehiculo_id`/`encargado_id` opcionales a propósito (pedido explícito
/// del usuario, 2026-09-15 -- ver `MIGRACION_36` en `schema.rs`): a
/// diferencia de `NuevoRegistroIngreso::contratista_id` (obligatorio,
/// bloquea el ingreso si no existe en el catálogo), acá el snapshot de
/// texto (`vehiculo_placa`, `encargado_nombre`) es la fuente real y el
/// link al catálogo es sólo un plus cuando hay coincidencia -- el
/// catálogo de vehículos/encargados todavía no tiene dueño confirmado
/// (mobile/desktop/ambos, ver plan) y el OCR/entrada manual nunca es
/// obligatorio.
#[derive(Debug, Clone)]
pub struct NuevaSalidaRuta {
    pub vehiculo_id: Option<i64>,
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_id: Option<i64>,
    pub encargado_nombre: String,
    pub numero_ruta: String,
    pub sub_numero: i64,
    pub numero_documento: String,
    pub fecha_documento: NaiveDate,
    pub resultado: ResultadoSalidaRuta,
    pub fecha_hora_salida: DateTime<Utc>,
    pub usuario_salida_id: i64,
}

/// Fecha y usuario van juntos a propósito, en vez de ser 2 `Option`
/// independientes en `SalidaRuta` -- mismo criterio que
/// `SalidaRegistroIngreso`/`SalidaMovimientoVisita`: la base ya exige
/// "ambos o ninguno" con un `CHECK` (`MIGRACION_36`), este tipo hace esa
/// regla imposible de romper del lado de Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetornoSalidaRuta {
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
}

#[derive(Debug, Clone)]
pub struct SalidaRuta {
    pub id: i64,
    pub vehiculo_id: Option<i64>,
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_id: Option<i64>,
    pub encargado_nombre: String,
    pub numero_ruta: String,
    pub sub_numero: i64,
    pub numero_documento: String,
    pub fecha_documento: NaiveDate,
    pub resultado: ResultadoSalidaRuta,
    pub fecha_hora_salida: DateTime<Utc>,
    pub usuario_salida_id: i64,
    /// `None` mientras la ruta sigue activa; `Some` una vez registrado el retorno.
    pub retorno: Option<RetornoSalidaRuta>,
}

/// Fila ya aplanada para la pantalla "Rutas activas" -- análoga a
/// `IngresoActivoResumen`/`MovimientoVisitaActivoResumen`, con los nombres
/// (usuario de salida, y de vehículo/encargado si hubo match de catálogo)
/// que la base `SalidaRuta` no trae.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SalidaRutaActivaResumen {
    pub id: i64,
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_nombre: String,
    pub numero_ruta: String,
    pub sub_numero: i64,
    pub numero_documento: String,
    pub fecha_documento: NaiveDate,
    pub resultado: ResultadoSalidaRuta,
    pub fecha_hora_salida: DateTime<Utc>,
    pub usuario_salida_nombre: String,
}

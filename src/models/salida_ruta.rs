use chrono::{DateTime, Utc};

/// `vehiculo_id`/`encargado_id` opcionales a propósito (pedido explícito
/// del usuario, 2026-09-15 -- ver `MIGRACION_36` en `schema.rs`): a
/// diferencia de `NuevoRegistroIngreso::contratista_id` (obligatorio,
/// bloquea el ingreso si no existe en el catálogo), acá el snapshot de
/// texto (`vehiculo_placa`, `encargado_nombre`) es la fuente real y el
/// link al catálogo es sólo un plus cuando hay coincidencia.
///
/// A partir del rediseño documento/tramo/viaje (2026-09-19/20, ver
/// `docs/planes-implementados/plan-control-rutas.md`), esta fila deja de
/// cargar `numero_documento`/`fecha_documento`/`resultado`/
/// `motivo_resultado`/`numero_ruta`/`sub_numero` -- se movieron a
/// `crate::models::documento_ruta::DocumentoRuta` y al vínculo
/// `salida_ruta_documentos` (ver `crate::models::salida_ruta_documento`).
/// Un tramo es sólo el cruce físico de portón; a qué documento(s)
/// corresponde vive aparte porque un documento puede repartirse en 2+
/// tramos, o un tramo puede llevar varios documentos a la vez.
#[derive(Debug, Clone)]
pub struct NuevaSalidaRuta {
    pub viaje_id: i64,
    pub vehiculo_id: Option<i64>,
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_id: Option<i64>,
    pub encargado_nombre: String,
    pub fecha_hora_salida: DateTime<Utc>,
    pub usuario_salida_id: i64,
}

/// Fecha y usuario van juntos a propósito, en vez de ser 2 `Option`
/// independientes en `SalidaRuta` -- mismo criterio que
/// `SalidaRegistroIngreso`/`SalidaMovimientoVisita`: la base ya exige
/// "ambos o ninguno" con un `CHECK`, este tipo hace esa regla imposible
/// de romper del lado de Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RetornoSalidaRuta {
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SalidaRuta {
    pub id: i64,
    pub viaje_id: i64,
    pub vehiculo_id: Option<i64>,
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_id: Option<i64>,
    pub encargado_nombre: String,
    pub fecha_hora_salida: DateTime<Utc>,
    pub usuario_salida_id: i64,
    /// `None` mientras la ruta sigue activa; `Some` una vez registrado el retorno.
    pub retorno: Option<RetornoSalidaRuta>,
}

/// Fila ya aplanada para la pantalla "Rutas activas" -- análoga a
/// `IngresoActivoResumen`/`MovimientoVisitaActivoResumen`. Ya no trae los
/// campos de un único documento (un tramo puede tener varios) -- la UI
/// que necesite mostrarlos consulta aparte
/// `SalidaRutaDocumentoRepository::listar_por_salida`. `viaje_id`
/// permite a la UI agrupar los tramos activos de un mismo viaje en una
/// sola tarjeta (ver el mockup ya aprobado por el usuario).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SalidaRutaActivaResumen {
    pub id: i64,
    pub viaje_id: i64,
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_nombre: String,
    pub fecha_hora_salida: DateTime<Utc>,
    pub usuario_salida_nombre: String,
}

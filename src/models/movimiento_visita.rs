use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct NuevoMovimientoVisita {
    pub cita_visitante_id: i64,
    /// `Some(numero)` = tiene gafete asignado, `None` = sin gafete (S/G) --
    /// mismo criterio que `NuevoRegistroIngreso::gafete_numero`.
    pub gafete_numero: Option<i64>,
    pub fecha_hora_entrada: DateTime<Utc>,
    pub usuario_entrada_id: i64,
}

/// Fecha y usuario van juntos a propósito, en vez de ser 2 `Option`
/// independientes en `MovimientoVisita` -- mismo criterio que
/// `SalidaRegistroIngreso`: la base ya exige "ambos o ninguno" con un
/// `CHECK` (`MIGRACION_28`), este tipo hace esa regla imposible de romper
/// del lado de Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SalidaMovimientoVisita {
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
}

#[derive(Debug, Clone)]
pub struct MovimientoVisita {
    pub id: i64,
    pub cita_visitante_id: i64,
    pub gafete_numero: Option<i64>,
    pub fecha_hora_entrada: DateTime<Utc>,
    pub usuario_entrada_id: i64,
    /// `None` mientras el movimiento sigue activo; `Some` una vez
    /// registrada la salida.
    pub salida: Option<SalidaMovimientoVisita>,
}

/// Fila ya aplanada (join `movimientos_visita` + `cita_visitantes` + `citas`)
/// para la pantalla "Visitas activas" -- análoga a `IngresoActivoResumen`
/// (contratistas), pero sin ningún campo de PRAIND/empresa: no aplica acá.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MovimientoVisitaActivoResumen {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa: Option<String>,
    pub gafete_numero: Option<i64>,
    pub fecha_hora_entrada: DateTime<Utc>,
    pub anfitrion_nombre: String,
    pub motivo: Option<String>,
}

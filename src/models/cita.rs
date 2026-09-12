use chrono::NaiveDate;

/// Mismo criterio que `EstadoGafete` (`models::gafete`): sólo los estados
/// que alguien decide de verdad. 'VENCIDA' no es un valor persistido -- se
/// calcula comparando `fecha_hasta` contra hoy en el momento de la consulta
/// (ver `domain::cita::verificar_cita`), no hace falta un proceso de fondo
/// que vaya actualizando filas sólo para que caduquen solas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EstadoCita {
    Vigente,
    Cancelada,
}

impl EstadoCita {
    /// Codificación canónica usada para persistir/filtrar en `SQLite`
    /// (columna `citas.estado`).
    pub fn as_str_sql(self) -> &'static str {
        match self {
            Self::Vigente => "VIGENTE",
            Self::Cancelada => "CANCELADA",
        }
    }

    /// Inverso de [`Self::as_str_sql`]. `None` si el texto no es ninguno de
    /// los 2 valores conocidos.
    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "VIGENTE" => Some(Self::Vigente),
            "CANCELADA" => Some(Self::Cancelada),
            _ => None,
        }
    }
}

/// Fila de `citas` -- la autorización con vigencia, no un movimiento (ver
/// `docs/planes-implementados/plan-control-visitas.md`). Puede agendarse para varias personas a
/// la vez: motivo/vigencia/anfitrión son compartidos por todo el grupo y
/// viven acá; cédula/nombre/empresa/placa son por persona y viven en
/// [`CitaVisitante`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Cita {
    pub id: i64,
    pub motivo: Option<String>,
    pub fecha_desde: NaiveDate,
    pub fecha_hasta: NaiveDate,
    /// Texto libre tipo "HH:MM" -- puramente informativo, `domain::cita`
    /// no la usa para decidir nada (ver el doc-comment de `MIGRACION_33`).
    pub hora_estimada: Option<String>,
    pub anfitrion_nombre: String,
    pub anfitrion_correo: String,
    pub estado: EstadoCita,
}

/// Fila de `cita_visitantes` -- una persona dentro de una `Cita` (que puede
/// ser individual o grupal).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CitaVisitante {
    pub id: i64,
    pub cita_id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa: Option<String>,
    pub placa_vehiculo: Option<String>,
}

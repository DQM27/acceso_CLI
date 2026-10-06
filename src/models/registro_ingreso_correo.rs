use chrono::{DateTime, Utc};

/// Ingreso "por correo": visita autorizada por correo (generalmente
/// entrevistas de RH), comodín mientras se termina el módulo de Visitas.
/// Mismo armazón que `NuevoRegistroIngresoProveedor`: `cedula`/`nombre` son
/// snapshot puro (sin catálogo de personas), `placa` nullable (su ausencia
/// ya comunica "llegó a pie") y `gafete_numero` siempre obligatorio -- pero
/// de VISITA, y con `motivo` libre ("a quién visita") en lugar de empresa.
#[derive(Debug, Clone)]
pub struct NuevoRegistroIngresoCorreo {
    pub cedula: String,
    pub nombre: String,
    pub motivo: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub usuario_ingreso_id: i64,
}

/// Fecha y usuario van juntos -- mismo criterio que
/// `SalidaRegistroIngresoProveedor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SalidaRegistroIngresoCorreo {
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
}

#[derive(Debug, Clone)]
pub struct RegistroIngresoCorreo {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub motivo: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub usuario_ingreso_id: i64,
    /// `None` mientras el ingreso sigue activo; `Some` una vez registrada
    /// la salida.
    pub salida: Option<SalidaRegistroIngresoCorreo>,
}

/// Fila ya aplanada para la lista de activos -- análoga a
/// `RegistroIngresoProveedorActivoResumen`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RegistroIngresoCorreoActivoResumen {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub motivo: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub usuario_ingreso_nombre: String,
}

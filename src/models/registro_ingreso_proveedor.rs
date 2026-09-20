use chrono::{DateTime, Utc};

/// `cedula`/`nombre` son snapshot puro (OCR o tecleado), sin FK a ningún
/// catálogo de personas -- pedido explícito del usuario: el colaborador de
/// un proveedor nunca se repite, no hay catálogo posible. `empresa_id` sí
/// es un catálogo real (`empresas_proveedor`); `empresa_nombre` queda
/// igual de denormalizado que `NuevoRegistroIngreso::contratista_nombre`,
/// mismo motivo (trazabilidad si el nombre de la empresa cambia después).
/// `placa` nullable: su ausencia ya comunica "llegó a pie". `gafete_numero`
/// es siempre obligatorio (a diferencia de `NuevoRegistroIngreso`, donde es
/// condicional a `requiere_gafete`) -- pedido explícito del usuario.
#[derive(Debug, Clone)]
pub struct NuevoRegistroIngresoProveedor {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub empresa_nombre: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub usuario_ingreso_id: i64,
}

/// Fecha y usuario van juntos a propósito -- mismo criterio que
/// `SalidaRegistroIngreso`/`SalidaMovimientoVisita`: el `CHECK` del esquema
/// ya exige "ambos o ninguno", este tipo hace esa regla imposible de romper
/// del lado de Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SalidaRegistroIngresoProveedor {
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
}

#[derive(Debug, Clone)]
pub struct RegistroIngresoProveedor {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub empresa_nombre: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub usuario_ingreso_id: i64,
    /// `None` mientras el ingreso sigue activo; `Some` una vez registrada
    /// la salida.
    pub salida: Option<SalidaRegistroIngresoProveedor>,
}

/// Fila ya aplanada para la lista de "Activos" -- análoga a
/// `MovimientoVisitaActivoResumen`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RegistroIngresoProveedorActivoResumen {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub fecha_hora_ingreso: DateTime<Utc>,
    /// Ya vivía en la tabla (`usuario_ingreso_nombre`, ver `crear`) pero
    /// nunca se seleccionaba acá -- pedido explícito del usuario 2026-09-20:
    /// mostrar en la tarjeta de "Proveedores activos" quién dio el ingreso,
    /// igual que ya hace `IngresoActivoResumen` para contratistas.
    pub usuario_ingreso_nombre: String,
}

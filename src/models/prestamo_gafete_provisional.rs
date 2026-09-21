use chrono::{DateTime, Utc};

/// `encargado_nombre`/`encargado_codigo_empleado` son un snapshot al
/// momento de la entrega -- mismo criterio que `NuevoMovimientoVisita`
/// (`visitante_cedula`/`visitante_nombre`): el código de empleado es justo
/// el dato que el guardia corrobora de palabra contra lo que la persona
/// dice, así que tiene que quedar grabado tal como era en ese momento, sin
/// depender de que `encargados_ruta` no cambie después.
#[derive(Debug, Clone)]
pub struct NuevoPrestamoGafeteProvisional {
    pub encargado_id: i64,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: String,
    pub gafete_numero: i64,
    pub fecha_hora_entrega: DateTime<Utc>,
    pub usuario_entrega_id: i64,
}

/// Fecha y usuario van juntos a propósito -- mismo criterio que
/// `SalidaMovimientoVisita`: el `CHECK` del esquema ya exige "ambos o
/// ninguno", este tipo hace esa regla imposible de romper del lado de Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevolucionPrestamoGafeteProvisional {
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
}

#[derive(Debug, Clone)]
pub struct PrestamoGafeteProvisional {
    pub id: i64,
    pub encargado_id: i64,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: String,
    pub gafete_numero: i64,
    pub fecha_hora_entrega: DateTime<Utc>,
    pub usuario_entrega_id: i64,
    /// `None` mientras el préstamo sigue activo; `Some` una vez registrada
    /// la devolución.
    pub devolucion: Option<DevolucionPrestamoGafeteProvisional>,
}

/// Fila ya aplanada para la lista de "Activos" -- análoga a
/// `MovimientoVisitaActivoResumen`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PrestamoGafeteProvisionalActivoResumen {
    pub id: i64,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: String,
    pub gafete_numero: i64,
    pub fecha_hora_entrega: DateTime<Utc>,
    /// Nombre de quien entregó el gafete -- ya se guarda como snapshot en
    /// `prestamos_gafete_provisional.usuario_entrega_nombre` (mismo patrón
    /// que `contratista_nombre` en `registro_ingresos`), pero este resumen
    /// nunca lo leía. Mismo criterio que `IngresoActivoResumen.usuario_ingreso_nombre`
    /// -- pedido explícito del usuario 2026-09-21, para mostrarlo en mobile
    /// igual que ya se muestra "dio ingreso X" en la tarjeta de contratista.
    pub usuario_entrega_nombre: String,
}

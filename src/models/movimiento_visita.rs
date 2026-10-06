use chrono::{DateTime, Utc};

/// `visitante_cedula`/`visitante_nombre`/`empresa`/`anfitrion_nombre`/`motivo`
/// son un snapshot al momento del check-in -- mismo criterio que
/// `NuevoRegistroIngreso` (que ya guarda `contratista_nombre`/`empresa_nombre`
/// propios en vez de un JOIN en cada lectura): la trazabilidad para
/// auditoría necesita mostrar cómo ERA el visitante en ese momento, sin
/// depender de que `cita_visitantes`/`citas` sigan sin cambios más
/// adelante. Quien llama ya tiene esta info a mano (la misma `Cita`/
/// `CitaVisitante` que `verificar_check_in` acaba de confirmar), así que
/// no hace falta otra consulta acá para juntarla.
#[derive(Debug, Clone)]
pub struct NuevoMovimientoVisita {
    pub cita_visitante_id: i64,
    /// `Some(numero)` = tiene gafete asignado, `None` = sin gafete (S/G) --
    /// mismo criterio que `NuevoRegistroIngreso::gafete_numero`.
    pub gafete_numero: Option<i64>,
    pub fecha_hora_entrada: DateTime<Utc>,
    pub usuario_entrada_id: i64,
    pub visitante_cedula: String,
    pub visitante_nombre: String,
    pub empresa: Option<String>,
    pub anfitrion_nombre: String,
    pub motivo: Option<String>,
    /// Medio de ingreso: la placa si entró en vehículo, `None` si caminando.
    pub placa: Option<String>,
}

/// La salida de un movimiento: la fecha siempre; el usuario local sólo si
/// la dio este equipo (la que da el otro equipo de la unidad llega con su
/// nombre, sin usuario de esta base).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SalidaMovimientoVisita {
    pub fecha_hora: DateTime<Utc>,
    /// `None` si la salida la dio el otro equipo de la unidad
    /// (`recibir_cierres_de_movimientos_visita_propios`): no hay usuario
    /// local que anotar, sólo su nombre.
    pub usuario_id: Option<i64>,
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
    /// La placa si entró en vehículo, `None` si caminando (o si entró antes
    /// de que se anotara el medio).
    pub placa: Option<String>,
    /// Quién le dio la entrada en la portería.
    pub usuario_entrada_nombre: String,
}

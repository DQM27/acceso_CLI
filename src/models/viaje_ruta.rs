use chrono::{DateTime, Utc};

/// Ver `docs/planes-implementados/plan-control-rutas.md`, sección
/// "Rediseño del núcleo de rutas -- documento/tramo/viaje". A diferencia
/// de `ResultadoSalidaRuta` (que no tiene variante "Denegado" real),
/// acá sí hay dos estados de verdad -- un viaje se abre y, en algún
/// momento, se cierra explícitamente, sin volver a abrirse jamás.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum EstadoViaje {
    Abierto,
    Cerrado,
}

/// Fecha + usuario de cierre van juntos, igual criterio que
/// `RetornoSalidaRuta`: la base ya exige "ambos o ninguno" con un
/// `CHECK`, este tipo lo hace imposible de romper del lado de Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CierreViajeRuta {
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
}

/// Agrupa 1+ tramos (`salidas_ruta`) de la misma unidad + mismo
/// encargado el mismo día. Vehículo y encargado quedan fijos por viaje
/// (snapshot tomado al abrirlo) -- confirmado explícitamente que el
/// encargado no se vuelve a pedir en tramos siguientes del mismo viaje,
/// ni siquiera ante un cambio de turno; si de verdad cambia, hay que
/// cerrar el viaje y abrir uno nuevo.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ViajeRuta {
    pub id: i64,
    pub vehiculo_id: Option<i64>,
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_id: Option<i64>,
    pub encargado_nombre: String,
    pub estado: EstadoViaje,
    pub fecha_hora_creacion: DateTime<Utc>,
    pub usuario_creacion_id: i64,
    /// `None` mientras el viaje sigue `Abierto`; `Some` una vez cerrado.
    pub cierre: Option<CierreViajeRuta>,
}

#[derive(Debug, Clone)]
pub struct NuevoViajeRuta {
    pub vehiculo_id: Option<i64>,
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_id: Option<i64>,
    pub encargado_nombre: String,
    pub fecha_hora_creacion: DateTime<Utc>,
    pub usuario_creacion_id: i64,
}

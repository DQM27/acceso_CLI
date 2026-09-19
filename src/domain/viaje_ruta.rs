/// La decisión que el guardia declara, en el mismo acto de confirmar el
/// retorno de un tramo (ver
/// `docs/planes-implementados/plan-control-rutas.md`, sección "Rediseño
/// del núcleo de rutas -- documento/tramo/viaje"): confirmado
/// explícitamente que responderla es obligatorio en ese momento, no
/// puede quedar pendiente para después.
///
/// `MismaRuta` y `OtraRutaOTercero` sólo difieren del lado de la UI/del
/// llamador (si la próxima `registrar_salida` manda
/// `continuar_viaje_id` o no) -- `RutaService::registrar_retorno` no
/// necesita distinguir entre esos dos casos, sólo entre "cierra el viaje
/// ahora" y "lo deja abierto". Se modelan igual como variantes propias
/// (en vez de un simple `bool`) porque documentan la intención real y
/// dejan lugar para que, más adelante, cada una dispare una acción
/// distinta sin tener que tocar la firma del método otra vez.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionRetornoViaje {
    NoVuelveASalir,
    MismaRuta,
    OtraRutaOTercero,
}

impl DecisionRetornoViaje {
    /// `true` cuando el viaje debe cerrarse al registrar este retorno.
    pub fn cierra_el_viaje(self) -> bool {
        matches!(self, Self::NoVuelveASalir)
    }
}

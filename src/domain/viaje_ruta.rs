/// La decisión que el guardia declara, en el mismo acto de confirmar el
/// retorno de un tramo (ver
/// `docs/planes-implementados/plan-control-rutas.md`, sección "Rediseño
/// del núcleo de rutas -- documento/tramo/viaje"): confirmado
/// explícitamente que responderla es obligatorio en ese momento, no
/// puede quedar pendiente para después.
///
/// `MismaRuta` es la ÚNICA variante que deja el viaje abierto -- es la
/// única que de verdad continúa la misma asignación (confirmado
/// explícitamente por el usuario: "la condición es si vuelve a salir es
/// mismo documento, con el mismo encargado y la misma unidad"). Tanto
/// `NoVuelveASalir` como `OtraRutaOTercero` cierran el viaje actual --
/// "otra ruta/tercero" es, por definición, una asignación distinta (otro
/// documento, posiblemente sin ruta de catálogo), así que el viaje previo
/// no puede quedar como candidato a continuar: si `registrar_salida`
/// alguna vez consulta `buscar_viaje_abierto_por_placa` para una unidad
/// que ya volvió con "otra ruta", el único viaje que debe encontrar es
/// uno realmente continuable, nunca uno que el guardia ya declaró
/// terminado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionRetornoViaje {
    NoVuelveASalir,
    MismaRuta,
    OtraRutaOTercero,
}

impl DecisionRetornoViaje {
    /// `true` cuando el viaje debe cerrarse al registrar este retorno.
    pub fn cierra_el_viaje(self) -> bool {
        !matches!(self, Self::MismaRuta)
    }
}

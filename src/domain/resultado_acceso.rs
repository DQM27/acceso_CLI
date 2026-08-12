#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotivoDenegacion {
    SinAcceso,
    PraindVencido,
    IngresoActivo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoAcceso {
    Permitido,
    PermitidoConAdvertencia,
    Denegado(MotivoDenegacion),
}

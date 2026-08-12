#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotivoDenegacion {
    SinAcceso,
    PraindVencido,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoAcceso {
    Permitido,
    PermitidoConAdvertencia,
    Denegado(MotivoDenegacion),
}

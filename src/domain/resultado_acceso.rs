#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum MotivoDenegacion {
    SinAcceso,
    /// Tiene fecha de vencimiento registrada y ya pasó. Distinto de
    /// `PraindNoRegistrado` — antes ambos casos colapsaban en esta misma
    /// variante y eran indistinguibles en mensajes/auditoría
    /// (`docs/auditoria-dominio-2026-08-20.md`, hallazgo #9).
    PraindVencido,
    /// Requiere PRAIND pero no tiene ninguna fecha de vencimiento registrada
    /// — nunca se cargó el dato, no es que haya vencido.
    PraindNoRegistrado,
    EmpresaInactiva,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ResultadoAcceso {
    Permitido,
    PermitidoConAdvertencia,
    Denegado(MotivoDenegacion),
}

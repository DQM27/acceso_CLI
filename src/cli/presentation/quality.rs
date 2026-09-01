//! Calidad visual de las mutaciones.
//!
//! Primera versión: sólo `Off`/`Normal` (ver DEC-007 en
//! `docs/lenguaje-visual-mutaciones.md`). `Reduced`, `High` y `Auto`
//! (adaptativos por rendimiento observado) quedan para una fase futura —
//! no se construyen todavía porque nada los necesita aún.

/// `Off`: toda transición resuelve al instante, el mismo estado final que
/// `Normal` sin interpolación. Ninguna funcionalidad depende de `Normal`;
/// es sólo preferencia de presentación (DEC-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualQuality {
    #[default]
    Normal,
    Off,
}

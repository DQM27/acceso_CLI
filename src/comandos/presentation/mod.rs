//! Motor de presentación (Fase 4 del plan — ver
//! `docs/lenguaje-visual-mutaciones.md` §8). Exclusivo de la interfaz de
//! comandos: `src/tui/` no lo usa ni lo conoce.
//!
//! Alcance de esta primera versión: sólo lo que hace falta para que el
//! login tenga una aparición (fade-in) real basada en tiempo. Sin foco, sin
//! breakpoints, sin métricas, sin calidad adaptativa — eso es de fases
//! posteriores y no se construye antes de tener una necesidad concreta.

mod animation;
mod easing;
mod engine;
mod quality;

pub use engine::Engine;
pub use quality::VisualQuality;

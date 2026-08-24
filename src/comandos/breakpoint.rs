//! Breakpoints de ancho (Fase 2, §10 de `docs/lenguaje-visual-mutaciones.md`).
//!
//! Sólo `Compact`/`Normal` por ahora — no `Wide`. El documento original
//! sugería tres variantes, pero hoy no hay ningún componente que necesite
//! distinguir "ancho" de "muy ancho": el único umbral real que existe en
//! todo `--comandos` es "¿entra la columna Empresa de `/activos` o no?"
//! (antes `ANCHO_TABLA_COMPLETA`, un `const` suelto). Fabricar `Wide` sin un
//! consumidor real violaría el mismo principio que ya se aplicó a
//! `VisualQuality` (§11: "`Reduced`/`High`/`Auto`... quedan para fase
//! futura explícita") — se agrega cuando una escena real lo necesite, no
//! antes (§10: "Los límites numéricos se determinan viendo las escenas
//! reales, no se asumen de antemano").
//!
//! `FocusTarget` (la otra mitad de la Fase 2 en el documento) queda fuera
//! de este archivo por el mismo motivo: hoy ningún componente necesita
//! preguntar "¿qué tiene el foco?" de forma genérica — cada Surface ya sabe
//! resolver su propio foco (`Campo` en el formulario, el índice de
//! selección en Historial/columnas). El primer consumidor real llegaría con
//! el motor de presentación animando algo más que el login (Fase 5, no
//! ésta) — se construye entonces.

/// Ancho por debajo del cual las tablas dejan de mostrar columnas
/// opcionales — hoy sólo Empresa en `/activos` (`render.rs`).
const ANCHO_COMPACTO: u16 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    Compact,
    Normal,
}

impl Breakpoint {
    pub fn desde_ancho(ancho: u16) -> Self {
        if ancho < ANCHO_COMPACTO {
            Self::Compact
        } else {
            Self::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angosto_es_compact() {
        assert_eq!(Breakpoint::desde_ancho(40), Breakpoint::Compact);
        assert_eq!(Breakpoint::desde_ancho(63), Breakpoint::Compact);
    }

    #[test]
    fn en_el_umbral_y_mas_ancho_es_normal() {
        assert_eq!(Breakpoint::desde_ancho(64), Breakpoint::Normal);
        assert_eq!(Breakpoint::desde_ancho(200), Breakpoint::Normal);
    }
}

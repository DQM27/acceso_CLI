//! Puente entre `AppCore` y el lenguaje de comandos de `--comandos`
//! (`crate::lenguaje_comandos` — parser + resolver, sin ninguna
//! dependencia de terminal, ver sus propios doc-comments) para reutilizar
//! su lógica desde otra interfaz. Hoy sólo la GUI Tauri
//! (`desktop/src-tauri/src/comandos/consola.rs`); el loop real de
//! `--comandos` (`crate::comandos::run`, con su `TerminalGuard` de
//! `crossterm`, detrás de la feature `terminal-ui`) no se toca ni se
//! ejecuta acá — por eso este puente no necesita esa feature.

use crate::lenguaje_comandos::{
    ContextState, autocompletar, calcular_sugerencias, parsear, resolver,
};
use crate::services::autenticacion_service::UsuarioSesion;

use super::AppCore;

/// Sugerencias en vivo + autocompletado para un texto todavía sin confirmar
/// — mismo par que ya usa `--comandos` en cada tecla (`sugerencias`/Tab).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Autocompletado {
    pub sugerencias: Vec<String>,
    pub completado: Option<String>,
}

impl AppCore {
    pub fn ejecutar_comando(&self, sesion: &UsuarioSesion, texto: &str) -> ContextState {
        resolver(self, &parsear(texto), sesion)
    }

    pub fn autocompletar_comando(&self, texto: &str) -> Autocompletado {
        let entrada = parsear(texto);
        Autocompletado {
            sugerencias: calcular_sugerencias(self, texto, &entrada),
            completado: autocompletar(self, texto),
        }
    }
}

//! El lenguaje de comandos de `--cli` (parser + resolver + resultado),
//! separado de `comandos/` a propósito: ninguno de estos tres archivos
//! depende de `ratatui`/`crossterm`/`tui-input` (ver sus propios
//! doc-comments), así que viven en un módulo siempre disponible, sin la
//! feature `terminal-ui` — permite que otra interfaz (hoy la GUI Tauri, ver
//! `application/comandos.rs`) reuse el mismo lenguaje sin arrastrar
//! dependencias de terminal que nunca va a usar. `comandos/` (el loop real,
//! con su `TerminalGuard` de modo raw) sigue detrás de esa feature y
//! reexporta estos mismos tipos para no romper a quien ya los usaba como
//! `comandos::parsear`/`comandos::resolver`/`comandos::ContextState`.

mod contexto;
// pub (no sólo `pub use` de una lista curada): `comandos/` referencia varias
// funciones/const de acá por ruta completa (`parser::clasificar_token` sólo
// en comentarios, pero `resolver::MIN_CONSULTA`,
// `resolver::nuevo_offset_coincidencias`, `resolver::pagina_*`,
// `resolver::es_comodin_todos`, etc. sí en código real) — más simple
// mantener el módulo entero público que ir persiguiendo cada símbolo nuevo
// que `comandos/` empiece a usar.
pub mod parser;
pub mod resolver;

pub use contexto::ContextState;
pub use parser::{Comando, Entrada, GafeteParse, MedioParse, parsear};
pub use resolver::{
    autocompletar, calcular_sugerencias, ficha_desde_resumen, preparar_resumen_ingreso, resolver,
};

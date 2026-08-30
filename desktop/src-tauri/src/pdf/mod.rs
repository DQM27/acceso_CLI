//! Exportación de Historial a PDF vía WebView2 (`PrintToPdf`) — HTML/CSS
//! renderizado por el mismo WebView2 que ya usa la app, en vez de un motor
//! de tipografía nuevo (Typst) o una API de dibujo de bajo nivel
//! (printpdf/genpdf). Validado con una prueba de concepto aislada antes de
//! construir esto (ver conversación) — el pipeline completo (ventana
//! oculta → cargar HTML → `PrintToPdf`) funciona en Windows con la versión
//! de Tauri/wry en uso.

pub mod generador;
pub mod html;

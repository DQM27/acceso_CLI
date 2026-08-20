//! Re-exporta el plegado de diacríticos compartido (`crate::texto`) para que
//! las pantallas sigan importándolo desde `ui_kit`, igual que el resto de
//! primitivas de esta carpeta — la implementación vive a nivel de crate
//! porque `database::schema` también la necesita (función SQL `PLEGAR`).
pub use crate::texto::plegar_diacriticos;

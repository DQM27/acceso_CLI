//! El formato propio de un reporte de error: lo que se guarda en disco y lo
//! que recibe `receptor-errores` en `POST .../receptor-errores/v1/errores`.
//! Ver `docs/telemetria-contrato.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Versión del formato. Sube si cambia algo que el receptor tenga que leer
/// distinto.
pub const ESQUEMA: u32 = 1;

/// Un marco de la traza, del más nuevo (donde ocurrió) al más viejo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marco {
    pub funcion: Option<String>,
    pub archivo: Option<String>,
    pub linea: Option<u32>,
    /// Código nuestro, no de una librería ni del sistema.
    pub propio: bool,
}

/// Una de las últimas operaciones antes del error: sólo el nombre técnico
/// (comando o llamada al núcleo), si salió bien y cuándo. Nunca datos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operacion {
    pub nombre: String,
    pub ok: Option<bool>,
    pub en: String,
}

/// De dónde salió el error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origen {
    /// Pánico de Rust (escritorio o núcleo del móvil).
    Panico,
    /// Excepción de Kotlin que nadie atrapó.
    Excepcion,
    /// El sistema cerró la app por no responder (móvil).
    Anr,
    /// Fallo nativo del proceso (móvil).
    Nativo,
    /// Error de JavaScript del escritorio.
    Js,
    /// Error inesperado del núcleo (sin variante de negocio).
    Nucleo,
}

impl Origen {
    pub const TODOS: [Self; 6] = [
        Self::Panico,
        Self::Excepcion,
        Self::Anr,
        Self::Nativo,
        Self::Js,
        Self::Nucleo,
    ];

    pub const fn nombre(self) -> &'static str {
        match self {
            Self::Panico => "panico",
            Self::Excepcion => "excepcion",
            Self::Anr => "anr",
            Self::Nativo => "nativo",
            Self::Js => "js",
            Self::Nucleo => "nucleo",
        }
    }

    /// El origen con ese nombre; `None` si no es uno de la lista.
    pub fn desde_nombre(nombre: &str) -> Option<Self> {
        Self::TODOS
            .into_iter()
            .find(|origen| origen.nombre() == nombre)
    }
}

/// Un error listo para guardar y mandar. Todo el texto ya pasó por
/// [`super::limpiar_texto`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reporte {
    pub esquema: u32,
    /// Identificador aleatorio: el receptor descarta un reintento repetido.
    pub id: String,
    /// UTC con milisegundos.
    pub ocurrido_en: String,
    /// `escritorio` o `movil`.
    pub plataforma: String,
    pub version_app: String,
    /// [`Origen::nombre`].
    pub origen: String,
    pub titulo: String,
    pub traza: Vec<Marco>,
    pub pantalla: Option<String>,
    pub operaciones: Vec<Operacion>,
    /// Sistema operativo, modelo, arquitectura… Sólo datos del equipo.
    pub contexto: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::Origen;

    #[test]
    fn cada_origen_se_lee_por_su_nombre() {
        for origen in Origen::TODOS {
            assert_eq!(Origen::desde_nombre(origen.nombre()), Some(origen));
        }
        assert_eq!(Origen::desde_nombre("otro"), None);
    }
}

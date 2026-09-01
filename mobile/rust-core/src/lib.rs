//! Puente `uniffi` sobre `control_acceso`: expone al Kotlin de la app móvil
//! sólo lo puntual que cada pantalla necesita, sin tocar la lógica del
//! crate raíz. Ver docs/plan-app-movil.md.

use std::sync::Mutex;

use control_acceso::application::AppCore;
use control_acceso::database::queries::contratistas::{
    ContratistaResumen as ContratistaResumenNucleo, FiltroContratistas as FiltroContratistasNucleo,
};
use control_acceso::models::tipo_ingreso::TipoIngreso as TipoIngresoNucleo;
use control_acceso::models::usuario::RolUsuario as RolUsuarioNucleo;
use control_acceso::services::autenticacion_service::UsuarioSesion as UsuarioSesionNucleo;
use control_acceso::services::error::AutenticacionError as AutenticacionErrorNucleo;

uniffi::setup_scaffolding!();

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum RolUsuario {
    Root,
    Administrador,
    Operador,
}

impl From<RolUsuarioNucleo> for RolUsuario {
    fn from(rol: RolUsuarioNucleo) -> Self {
        match rol {
            RolUsuarioNucleo::Root => Self::Root,
            RolUsuarioNucleo::Administrador => Self::Administrador,
            RolUsuarioNucleo::Operador => Self::Operador,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct UsuarioSesion {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub rol: RolUsuario,
}

impl From<UsuarioSesionNucleo> for UsuarioSesion {
    fn from(sesion: UsuarioSesionNucleo) -> Self {
        Self {
            id: sesion.id,
            cedula: sesion.cedula,
            nombre: sesion.nombre,
            rol: sesion.rol.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
pub enum TipoIngreso {
    Praind,
    InHouse,
    PorCorreo,
    Swat,
}

impl From<TipoIngresoNucleo> for TipoIngreso {
    fn from(tipo: TipoIngresoNucleo) -> Self {
        match tipo {
            TipoIngresoNucleo::Praind => Self::Praind,
            TipoIngresoNucleo::InHouse => Self::InHouse,
            TipoIngresoNucleo::PorCorreo => Self::PorCorreo,
            TipoIngresoNucleo::Swat => Self::Swat,
        }
    }
}

/// Espejo de `ContratistaResumen` — la fecha viaja como texto ISO
/// (`AAAA-MM-DD`) porque `uniffi` no tiene un tipo fecha nativo; decidir si
/// está vencida sigue siendo trabajo de Rust (`domain::acceso`), no de
/// Kotlin, cuando se implemente la pantalla de confirmar entrada.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ContratistaResumen {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<String>,
    pub tiene_acceso: bool,
    pub tiene_ingreso_activo: bool,
}

impl From<ContratistaResumenNucleo> for ContratistaResumen {
    fn from(resumen: ContratistaResumenNucleo) -> Self {
        Self {
            id: resumen.id,
            cedula: resumen.cedula,
            nombre: resumen.nombre,
            empresa_nombre: resumen.empresa_nombre,
            tipo_ingreso: resumen.tipo_ingreso.into(),
            fecha_vencimiento_praind: resumen.fecha_vencimiento_praind.map(|f| f.to_string()),
            tiene_acceso: resumen.tiene_acceso,
            tiene_ingreso_activo: resumen.tiene_ingreso_activo,
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum NucleoError {
    #[error("no se pudo abrir la base de datos: {mensaje}")]
    Apertura { mensaje: String },
    #[error("credenciales inválidas")]
    CredencialesInvalidas,
    #[error("usuario inactivo")]
    UsuarioInactivo,
    #[error("error interno: {mensaje}")]
    Interno { mensaje: String },
}

impl From<AutenticacionErrorNucleo> for NucleoError {
    fn from(error: AutenticacionErrorNucleo) -> Self {
        match error {
            AutenticacionErrorNucleo::CredencialesInvalidas => Self::CredencialesInvalidas,
            AutenticacionErrorNucleo::UsuarioInactivo => Self::UsuarioInactivo,
            otro => Self::Interno {
                mensaje: otro.to_string(),
            },
        }
    }
}

/// Sesión del núcleo: dueña de la única conexión `SQLite` del teléfono. Se
/// abre una vez al arrancar la app y se reusa en todas las pantallas (login,
/// buscar contratista, registrar entrada/salida) — nunca se reabre por
/// pantalla.
#[derive(uniffi::Object)]
pub struct Nucleo {
    core: Mutex<AppCore>,
}

#[uniffi::export]
impl Nucleo {
    #[uniffi::constructor]
    pub fn abrir(ruta_base_datos: String) -> Result<Self, NucleoError> {
        let core = AppCore::abrir(&ruta_base_datos).map_err(|origen| NucleoError::Apertura {
            mensaje: origen.to_string(),
        })?;
        Ok(Self {
            core: Mutex::new(core),
        })
    }

    pub fn autenticar(
        &self,
        cedula: String,
        password: String,
    ) -> Result<UsuarioSesion, NucleoError> {
        let core = self.core.lock().expect("mutex de AppCore envenenado");
        let sesion = core.autenticar(&cedula, &password)?;
        Ok(sesion.into())
    }

    /// Búsqueda en vivo (la vía primaria del guardia — ver
    /// docs/plan-app-movil.md, "Prioridad de esfuerzo: el buscador"). Un
    /// `texto` vacío trae la primera página completa, no una lista vacía.
    ///
    /// A diferencia del desktop (Tauri/AG Grid), que carga el universo
    /// completo de contratistas al cliente y filtra ahí, el teléfono no
    /// tiene esos recursos de sobra — se pide una página acotada
    /// (`LIMITE_MOVIL`, más chica que la paginación normal de 100 que usa
    /// TUI/CLI) filtrada ya en SQL, nunca la lista entera.
    pub fn buscar_contratistas(
        &self,
        texto: String,
    ) -> Result<Vec<ContratistaResumen>, NucleoError> {
        const LIMITE_MOVIL: usize = 30;

        let core = self.core.lock().expect("mutex de AppCore envenenado");
        let texto_normalizado = texto.trim();
        let filtro = FiltroContratistasNucleo {
            texto: (!texto_normalizado.is_empty()).then(|| texto_normalizado.to_string()),
            limite: LIMITE_MOVIL,
            ..Default::default()
        };
        let pagina = core.buscar_contratistas(&filtro).map_err(|origen| NucleoError::Interno {
            mensaje: origen.to_string(),
        })?;
        Ok(pagina.items.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abre_una_base_de_datos_temporal() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();

        let resultado = Nucleo::abrir(ruta);

        assert!(resultado.is_ok());
    }

    #[test]
    fn autenticar_con_credenciales_invalidas_falla() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();
        let nucleo = Nucleo::abrir(ruta).unwrap();

        let resultado = nucleo.autenticar("000000000".to_string(), "loquesea".to_string());

        assert!(matches!(resultado, Err(NucleoError::CredencialesInvalidas)));
    }

    #[test]
    fn buscar_contratistas_en_base_vacia_no_falla() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();
        let nucleo = Nucleo::abrir(ruta).unwrap();

        let resultado = nucleo.buscar_contratistas(String::new());

        assert_eq!(resultado.unwrap(), Vec::new());
    }
}

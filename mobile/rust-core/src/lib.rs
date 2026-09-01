//! Puente `uniffi` sobre `control_acceso`: expone al Kotlin de la app móvil
//! sólo lo puntual que cada pantalla necesita, sin tocar la lógica del
//! crate raíz. Ver docs/plan-app-movil.md.

use std::sync::Mutex;

use control_acceso::application::AppCore;
use control_acceso::database::queries::contratistas::{
    ContratistaResumen as ContratistaResumenNucleo, FiltroContratistas as FiltroContratistasNucleo,
};
use control_acceso::database::queries::ingresos::FiltroIngresosActivos as FiltroIngresosActivosNucleo;
use control_acceso::domain::resultado_acceso::{
    MotivoDenegacion as MotivoDenegacionNucleo, ResultadoAcceso as ResultadoAccesoNucleo,
};
use control_acceso::models::medio_ingreso::MedioIngreso as MedioIngresoNucleo;
use control_acceso::models::tipo_ingreso::TipoIngreso as TipoIngresoNucleo;
use control_acceso::models::usuario::RolUsuario as RolUsuarioNucleo;
use control_acceso::services::autenticacion_service::UsuarioSesion as UsuarioSesionNucleo;
use control_acceso::services::error::AutenticacionError as AutenticacionErrorNucleo;
use control_acceso::services::error::RegistroIngresoServiceError as RegistroIngresoServiceErrorNucleo;
use control_acceso::services::registro_ingreso_service::{
    IngresoActivoResumen as IngresoActivoResumenNucleo,
    PreparacionIngreso as PreparacionIngresoNucleo,
    ResultadoRegistroEntrada as ResultadoRegistroEntradaNucleo,
};

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

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
pub enum MedioIngreso {
    Caminando,
    Vehiculo,
}

impl From<MedioIngreso> for MedioIngresoNucleo {
    fn from(medio: MedioIngreso) -> Self {
        match medio {
            MedioIngreso::Caminando => Self::Caminando,
            MedioIngreso::Vehiculo => Self::Vehiculo,
        }
    }
}

impl From<MedioIngresoNucleo> for MedioIngreso {
    fn from(medio: MedioIngresoNucleo) -> Self {
        match medio {
            MedioIngresoNucleo::Caminando => Self::Caminando,
            MedioIngresoNucleo::Vehiculo => Self::Vehiculo,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
pub enum MotivoDenegacion {
    SinAcceso,
    PraindVencido,
    PraindNoRegistrado,
    EmpresaInactiva,
}

impl From<MotivoDenegacionNucleo> for MotivoDenegacion {
    fn from(motivo: MotivoDenegacionNucleo) -> Self {
        match motivo {
            MotivoDenegacionNucleo::SinAcceso => Self::SinAcceso,
            MotivoDenegacionNucleo::PraindVencido => Self::PraindVencido,
            MotivoDenegacionNucleo::PraindNoRegistrado => Self::PraindNoRegistrado,
            MotivoDenegacionNucleo::EmpresaInactiva => Self::EmpresaInactiva,
        }
    }
}

/// Espejo de `ResultadoAcceso` — la decisión (PRAIND vencido, empresa
/// inactiva, etc.) ya viene tomada por `domain::acceso::verificar_acceso`;
/// Kotlin sólo la muestra, nunca la recalcula.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
pub enum ResultadoAcceso {
    Permitido,
    PermitidoConAdvertencia,
    Denegado { motivo: MotivoDenegacion },
}

impl From<ResultadoAccesoNucleo> for ResultadoAcceso {
    fn from(resultado: ResultadoAccesoNucleo) -> Self {
        match resultado {
            ResultadoAccesoNucleo::Permitido => Self::Permitido,
            ResultadoAccesoNucleo::PermitidoConAdvertencia => Self::PermitidoConAdvertencia,
            ResultadoAccesoNucleo::Denegado(motivo) => Self::Denegado {
                motivo: motivo.into(),
            },
        }
    }
}

/// Espejo de `PreparacionIngreso` — vista previa antes de confirmar; no es
/// una autorización cacheada, `registrar_ingreso` vuelve a validar todo.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PreparacionIngreso {
    pub contratista_id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub resultado_acceso: ResultadoAcceso,
    pub requiere_gafete: bool,
    pub tiene_ingreso_activo: bool,
    pub gafetes_deuda: Vec<i64>,
}

impl From<PreparacionIngresoNucleo> for PreparacionIngreso {
    fn from(preparacion: PreparacionIngresoNucleo) -> Self {
        Self {
            contratista_id: preparacion.contratista_id,
            cedula: preparacion.cedula,
            nombre: preparacion.nombre,
            empresa_nombre: preparacion.empresa_nombre,
            tipo_ingreso: preparacion.tipo_ingreso.into(),
            resultado_acceso: preparacion.resultado_acceso.into(),
            requiere_gafete: preparacion.requiere_gafete,
            tiene_ingreso_activo: preparacion.tiene_ingreso_activo,
            gafetes_deuda: preparacion.gafetes_deuda,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct ResultadoRegistroEntrada {
    pub registro_id: i64,
    pub resultado_acceso: ResultadoAcceso,
}

impl From<ResultadoRegistroEntradaNucleo> for ResultadoRegistroEntrada {
    fn from(resultado: ResultadoRegistroEntradaNucleo) -> Self {
        Self {
            registro_id: resultado.registro_id,
            resultado_acceso: resultado.resultado_acceso.into(),
        }
    }
}

/// Espejo de `IngresoActivoResumen` — `resultado_acceso` se re-evalúa con la
/// fecha de hoy (no es la decisión congelada del momento del ingreso), igual
/// que en `desktop/src/pantallas/Activos.tsx`.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct IngresoActivoResumen {
    pub registro_id: i64,
    pub contratista_id: i64,
    pub cedula: String,
    pub contratista_nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub medio_ingreso: MedioIngreso,
    pub fecha_hora_ingreso: String,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: String,
    pub resultado_acceso: ResultadoAcceso,
}

impl From<IngresoActivoResumenNucleo> for IngresoActivoResumen {
    fn from(activo: IngresoActivoResumenNucleo) -> Self {
        Self {
            registro_id: activo.registro_id,
            contratista_id: activo.contratista_id,
            cedula: activo.cedula,
            contratista_nombre: activo.contratista_nombre,
            empresa_nombre: activo.empresa_nombre,
            tipo_ingreso: activo.tipo_ingreso.into(),
            medio_ingreso: activo.medio_ingreso.into(),
            fecha_hora_ingreso: activo.fecha_hora_ingreso.to_rfc3339(),
            gafete_numero: activo.gafete_numero,
            usuario_ingreso_nombre: activo.usuario_ingreso_nombre,
            resultado_acceso: activo.resultado_acceso.into(),
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum NucleoError {
    #[error("no se pudo abrir la base de datos: {mensaje}")]
    Apertura { mensaje: String },
    #[error("credenciales inválidas")]
    CredencialesInvalidas,
    #[error("usuario inactivo")]
    UsuarioInactivo,
    #[error("no hay una sesión iniciada")]
    NoAutenticado,
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

impl From<RegistroIngresoServiceErrorNucleo> for NucleoError {
    fn from(error: RegistroIngresoServiceErrorNucleo) -> Self {
        Self::Interno {
            mensaje: error.to_string(),
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
    /// Actor autenticado — lo necesitan `registrar_ingreso`/`registrar_salida`
    /// como `usuario_ingreso_id`/`usuario_salida_id`. Se llena en
    /// `autenticar` y vive mientras dure el proceso (no hay "cerrar sesión"
    /// todavía en el piloto).
    sesion: Mutex<Option<UsuarioSesionNucleo>>,
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
            sesion: Mutex::new(None),
        })
    }

    pub fn autenticar(
        &self,
        cedula: String,
        password: String,
    ) -> Result<UsuarioSesion, NucleoError> {
        let core = self.core.lock().expect("mutex de AppCore envenenado");
        let sesion = core.autenticar(&cedula, &password)?;
        *self.sesion.lock().expect("mutex de sesión envenenado") = Some(sesion.clone());
        Ok(sesion.into())
    }

    /// Vista previa antes de confirmar — misma decisión que ya toma la GUI
    /// de escritorio (`desktop/src/pantallas/NuevoIngresoModal.tsx`): no
    /// rechaza PRAIND vencido/ingreso activo aquí, sólo informa; quien llama
    /// (Kotlin) decide si deja continuar mirando los campos ya calculados.
    pub fn preparar_ingreso(&self, contratista_id: i64) -> Result<PreparacionIngreso, NucleoError> {
        let core = self.core.lock().expect("mutex de AppCore envenenado");
        Ok(core.preparar_ingreso(contratista_id)?.into())
    }

    pub fn registrar_ingreso(
        &self,
        contratista_id: i64,
        medio: MedioIngreso,
        gafete: Option<i64>,
    ) -> Result<ResultadoRegistroEntrada, NucleoError> {
        let actor = self
            .sesion
            .lock()
            .expect("mutex de sesión envenenado")
            .clone()
            .ok_or(NucleoError::NoAutenticado)?;
        let core = self.core.lock().expect("mutex de AppCore envenenado");
        Ok(core
            .registrar_ingreso(&actor, contratista_id, medio.into(), gafete)?
            .into())
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

    /// Mismo criterio tacaño que `buscar_contratistas`: página acotada, no
    /// el listado completo que carga AG Grid en desktop.
    pub fn listar_ingresos_activos(
        &self,
        texto: String,
    ) -> Result<Vec<IngresoActivoResumen>, NucleoError> {
        const LIMITE_MOVIL: usize = 30;

        let core = self.core.lock().expect("mutex de AppCore envenenado");
        let texto_normalizado = texto.trim();
        let filtro = FiltroIngresosActivosNucleo {
            texto: (!texto_normalizado.is_empty()).then(|| texto_normalizado.to_string()),
            limite: LIMITE_MOVIL,
            ..Default::default()
        };
        let lista = core
            .listar_ingresos_activos(&filtro)
            .map_err(|origen| NucleoError::Interno {
                mensaje: origen.to_string(),
            })?;
        Ok(lista.items.into_iter().map(Into::into).collect())
    }

    pub fn registrar_salida(&self, registro_id: i64) -> Result<(), NucleoError> {
        let actor = self
            .sesion
            .lock()
            .expect("mutex de sesión envenenado")
            .clone()
            .ok_or(NucleoError::NoAutenticado)?;
        let core = self.core.lock().expect("mutex de AppCore envenenado");
        Ok(core.registrar_salida(&actor, registro_id)?)
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

    #[test]
    fn preparar_y_registrar_ingreso_sin_gafete() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();

        // Aplica el esquema real y siembra lo mínimo: SWAT no requiere ni
        // PRAIND ni gafete (domain::contratista), así que es el caso feliz
        // más simple para probar el camino completo.
        let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
        conexion
            .execute_batch(
                "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
                 INSERT INTO contratistas (
                     cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
                 ) VALUES ('111111111', 'Contratista Test', 1, 'SWAT', 0, 1);
                 INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                     '999999999', 'Actor Test',
                     '$argon2id$v=19$m=19456,t=2,p=1$FZShq0MtV2bGh9nFBgvrGA$dYNDyh7up/wmAY+t/Vf6V5LTCS9sNkQgaH81G650xfM',
                     'ROOT', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar("999999999".to_string(), "daniel27".to_string())
            .unwrap();

        let preparacion = nucleo.preparar_ingreso(1).unwrap();
        assert!(!preparacion.requiere_gafete);
        assert_eq!(preparacion.resultado_acceso, ResultadoAcceso::Permitido);

        let resultado = nucleo
            .registrar_ingreso(1, MedioIngreso::Caminando, None)
            .unwrap();
        assert_eq!(resultado.resultado_acceso, ResultadoAcceso::Permitido);
    }

    #[test]
    fn listar_activos_y_registrar_salida() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();

        let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
        conexion
            .execute_batch(
                "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
                 INSERT INTO contratistas (
                     cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
                 ) VALUES ('111111111', 'Contratista Test', 1, 'SWAT', 0, 1);
                 INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                     '999999999', 'Actor Test',
                     '$argon2id$v=19$m=19456,t=2,p=1$FZShq0MtV2bGh9nFBgvrGA$dYNDyh7up/wmAY+t/Vf6V5LTCS9sNkQgaH81G650xfM',
                     'ROOT', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar("999999999".to_string(), "daniel27".to_string())
            .unwrap();
        let registro = nucleo
            .registrar_ingreso(1, MedioIngreso::Caminando, None)
            .unwrap();

        let activos = nucleo.listar_ingresos_activos(String::new()).unwrap();
        assert_eq!(activos.len(), 1);
        assert_eq!(activos[0].registro_id, registro.registro_id);
        assert_eq!(activos[0].contratista_nombre, "Contratista Test");

        nucleo.registrar_salida(registro.registro_id).unwrap();

        let activos_tras_salida = nucleo.listar_ingresos_activos(String::new()).unwrap();
        assert_eq!(activos_tras_salida, Vec::new());
    }

    #[test]
    fn registrar_ingreso_sin_sesion_falla() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();
        let nucleo = Nucleo::abrir(ruta).unwrap();

        let resultado = nucleo.registrar_ingreso(1, MedioIngreso::Caminando, None);

        assert!(matches!(resultado, Err(NucleoError::NoAutenticado)));
    }
}

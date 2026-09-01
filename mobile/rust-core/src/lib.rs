//! Puente `uniffi` sobre `control_acceso`: expone al Kotlin de la app móvil
//! sólo lo puntual que cada pantalla necesita, sin tocar la lógica del
//! crate raíz. Ver docs/plan-app-movil.md.

use std::sync::Mutex;

use control_acceso::application::AppCore;
use control_acceso::database::queries::contratistas::{
    ContratistaResumen as ContratistaResumenNucleo, FiltroContratistas as FiltroContratistasNucleo,
};
use control_acceso::database::queries::ingresos::{
    FiltroHistorial as FiltroHistorialNucleo, FiltroIngresosActivos as FiltroIngresosActivosNucleo,
    MovimientoIngresoResumen as MovimientoIngresoResumenNucleo,
};
use control_acceso::database::queries::usuarios::{
    FiltroUsuarios as FiltroUsuariosNucleo, UsuarioResumen as UsuarioResumenNucleo,
};
use control_acceso::database::queries::Igualdad;
use control_acceso::domain::resultado_acceso::{
    MotivoDenegacion as MotivoDenegacionNucleo, ResultadoAcceso as ResultadoAccesoNucleo,
};
use control_acceso::models::empresa::Empresa as EmpresaNucleo;
use control_acceso::models::medio_ingreso::MedioIngreso as MedioIngresoNucleo;
use control_acceso::models::registro_ingreso::{
    MotivoResultadoIngreso as MotivoResultadoIngresoNucleo,
    ResultadoIngresoRegistrado as ResultadoIngresoRegistradoNucleo,
};
use control_acceso::models::tipo_ingreso::TipoIngreso as TipoIngresoNucleo;
use control_acceso::models::usuario::RolUsuario as RolUsuarioNucleo;
use control_acceso::services::autenticacion_service::UsuarioSesion as UsuarioSesionNucleo;
use control_acceso::services::contratista_service::DatosContratista as DatosContratistaNucleo;
use control_acceso::services::error::AutenticacionError as AutenticacionErrorNucleo;
use control_acceso::services::error::ContratistaServiceError as ContratistaServiceErrorNucleo;
use control_acceso::services::error::EmpresaServiceError as EmpresaServiceErrorNucleo;
use control_acceso::services::error::RegistroIngresoServiceError as RegistroIngresoServiceErrorNucleo;
use control_acceso::services::error::UsuarioServiceError as UsuarioServiceErrorNucleo;
use control_acceso::services::registro_ingreso_service::{
    IngresoActivoResumen as IngresoActivoResumenNucleo,
    PreparacionIngreso as PreparacionIngresoNucleo,
    ResultadoRegistroEntrada as ResultadoRegistroEntradaNucleo,
};
use control_acceso::services::usuario_service::CrearUsuarioInput as CrearUsuarioInputNucleo;

uniffi::setup_scaffolding!();

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
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

impl From<RolUsuario> for RolUsuarioNucleo {
    fn from(rol: RolUsuario) -> Self {
        match rol {
            RolUsuario::Root => Self::Root,
            RolUsuario::Administrador => Self::Administrador,
            RolUsuario::Operador => Self::Operador,
        }
    }
}

/// Sin espejo en `control_acceso` — es puramente de la UI móvil: decide
/// cómo `Nucleo::listar_ingresos_activos` interpreta el campo de texto
/// cuando se está buscando a quién dar salida entre muchos activos.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
pub enum ModoBusquedaActivos {
    NombreCedula,
    Gafete,
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

impl From<TipoIngreso> for TipoIngresoNucleo {
    fn from(tipo: TipoIngreso) -> Self {
        match tipo {
            TipoIngreso::Praind => Self::Praind,
            TipoIngreso::InHouse => Self::InHouse,
            TipoIngreso::PorCorreo => Self::PorCorreo,
            TipoIngreso::Swat => Self::Swat,
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

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Empresa {
    pub id: i64,
    pub nombre: String,
    pub activo: bool,
}

impl From<EmpresaNucleo> for Empresa {
    fn from(empresa: EmpresaNucleo) -> Self {
        Self {
            id: empresa.id,
            nombre: empresa.nombre,
            activo: empresa.activo,
        }
    }
}

/// Espejo de `DatosContratista` — sólo alta, no edición (ver
/// docs/plan-app-movil.md). `fecha_vencimiento_praind` viaja como texto
/// ISO (`AAAA-MM-DD`); si no parsea se rechaza como `DatosInvalidos` antes
/// de tocar Rust, sin ida y vuelta.
#[derive(Debug, Clone, uniffi::Record)]
pub struct DatosContratista {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<String>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
pub enum MotivoResultadoIngreso {
    PraindProximoVencer,
    DatosReconstruidos,
}

impl From<MotivoResultadoIngresoNucleo> for MotivoResultadoIngreso {
    fn from(motivo: MotivoResultadoIngresoNucleo) -> Self {
        match motivo {
            MotivoResultadoIngresoNucleo::PraindProximoVencer => Self::PraindProximoVencer,
            MotivoResultadoIngresoNucleo::DatosReconstruidos => Self::DatosReconstruidos,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
pub enum ResultadoIngresoRegistrado {
    Permitido,
    PermitidoConAdvertencia { motivo: MotivoResultadoIngreso },
    Migrado,
}

impl From<ResultadoIngresoRegistradoNucleo> for ResultadoIngresoRegistrado {
    fn from(resultado: ResultadoIngresoRegistradoNucleo) -> Self {
        match resultado {
            ResultadoIngresoRegistradoNucleo::Permitido => Self::Permitido,
            ResultadoIngresoRegistradoNucleo::PermitidoConAdvertencia(motivo) => {
                Self::PermitidoConAdvertencia {
                    motivo: motivo.into(),
                }
            }
            ResultadoIngresoRegistradoNucleo::Migrado => Self::Migrado,
        }
    }
}

/// Espejo de `MovimientoIngresoResumen` — un renglón de Historial (entrada
/// + salida, si ya la tiene).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MovimientoHistorial {
    pub registro_id: i64,
    pub cedula: String,
    pub contratista_nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub medio_ingreso: MedioIngreso,
    pub fecha_hora_ingreso: String,
    pub fecha_hora_salida: Option<String>,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: String,
    pub usuario_salida_nombre: Option<String>,
    pub resultado_acceso: ResultadoIngresoRegistrado,
}

impl From<MovimientoIngresoResumenNucleo> for MovimientoHistorial {
    fn from(movimiento: MovimientoIngresoResumenNucleo) -> Self {
        Self {
            registro_id: movimiento.registro_id,
            cedula: movimiento.cedula,
            contratista_nombre: movimiento.contratista_nombre,
            empresa_nombre: movimiento.empresa_nombre,
            tipo_ingreso: movimiento.tipo_ingreso.into(),
            medio_ingreso: movimiento.medio_ingreso.into(),
            fecha_hora_ingreso: movimiento.fecha_hora_ingreso.to_rfc3339(),
            fecha_hora_salida: movimiento.fecha_hora_salida.map(|f| f.to_rfc3339()),
            gafete_numero: movimiento.gafete_numero,
            usuario_ingreso_nombre: movimiento.usuario_ingreso_nombre,
            usuario_salida_nombre: movimiento.usuario_salida_nombre,
            resultado_acceso: movimiento.resultado_acceso.into(),
        }
    }
}

/// Espejo de `UsuarioResumen` — sólo se expone a Root/Administrador
/// (`Operacion::GestionarUsuarios`, `domain/autorizacion.rs`); Rust ya
/// rechaza a un Operador con `OperacionNoAutorizada` aunque Kotlin
/// oculte el menú, así que no hay doble mantenimiento de la regla real.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct UsuarioResumen {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub rol: RolUsuario,
    pub activo: bool,
}

impl From<UsuarioResumenNucleo> for UsuarioResumen {
    fn from(usuario: UsuarioResumenNucleo) -> Self {
        Self {
            id: usuario.id,
            cedula: usuario.cedula,
            nombre: usuario.nombre,
            rol: usuario.rol.into(),
            activo: usuario.activo,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct DatosUsuario {
    pub cedula: String,
    pub nombre: String,
    pub password: String,
    pub rol: RolUsuario,
    pub activo: bool,
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
    #[error("fecha de PRAIND inválida: {mensaje}")]
    FechaInvalida { mensaje: String },
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

impl From<ContratistaServiceErrorNucleo> for NucleoError {
    fn from(error: ContratistaServiceErrorNucleo) -> Self {
        Self::Interno {
            mensaje: error.to_string(),
        }
    }
}

impl From<EmpresaServiceErrorNucleo> for NucleoError {
    fn from(error: EmpresaServiceErrorNucleo) -> Self {
        Self::Interno {
            mensaje: error.to_string(),
        }
    }
}

impl From<UsuarioServiceErrorNucleo> for NucleoError {
    fn from(error: UsuarioServiceErrorNucleo) -> Self {
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
    ///
    /// `modo` decide cómo se interpreta `texto` — separado a propósito de
    /// `NombreCedula`: la búsqueda de texto libre de Rust ya hace `OR` entre
    /// cédula/nombre (`LIKE`) y gafete exacto en la misma consulta, así que
    /// buscar "7" como gafete también trae cualquier cédula que *contenga*
    /// un 7 — ruidoso con muchos activos a la vez. En modo `Gafete` se
    /// filtra sólo por `gafete_numero` exacto, sin ese ruido.
    pub fn listar_ingresos_activos(
        &self,
        texto: String,
        modo: ModoBusquedaActivos,
    ) -> Result<Vec<IngresoActivoResumen>, NucleoError> {
        const LIMITE_MOVIL: usize = 30;

        let core = self.core.lock().expect("mutex de AppCore envenenado");
        let texto_normalizado = texto.trim();
        let mut filtro = FiltroIngresosActivosNucleo {
            limite: LIMITE_MOVIL,
            ..Default::default()
        };
        match modo {
            ModoBusquedaActivos::NombreCedula => {
                filtro.texto =
                    (!texto_normalizado.is_empty()).then(|| texto_normalizado.to_string());
            }
            ModoBusquedaActivos::Gafete => match texto_normalizado.parse::<i64>() {
                Ok(numero) => filtro.gafete_numero = Some(Igualdad::Incluye(numero)),
                Err(_) if texto_normalizado.is_empty() => {}
                // Texto no numérico en modo gafete: no hay coincidencia
                // posible, no es un error del usuario.
                Err(_) => return Ok(Vec::new()),
            },
        }
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

    pub fn listar_empresas(&self) -> Result<Vec<Empresa>, NucleoError> {
        let core = self.core.lock().expect("mutex de AppCore envenenado");
        Ok(core
            .listar_empresas()?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Alta de contratista — mismo formulario que
    /// `desktop/src/pantallas/FormularioContratista.tsx`, sólo creación
    /// (ver docs/plan-app-movil.md). La validación real y definitiva vuelve
    /// a correr en Rust (`ContratistaService::crear`); esto no duplica esa
    /// lógica, sólo convierte tipos en la frontera uniffi.
    pub fn crear_contratista(&self, datos: DatosContratista) -> Result<i64, NucleoError> {
        let actor = self
            .sesion
            .lock()
            .expect("mutex de sesión envenenado")
            .clone()
            .ok_or(NucleoError::NoAutenticado)?;

        let fecha_vencimiento_praind = datos
            .fecha_vencimiento_praind
            .map(|texto| {
                texto
                    .parse()
                    .map_err(|_| NucleoError::FechaInvalida { mensaje: texto })
            })
            .transpose()?;

        let core = self.core.lock().expect("mutex de AppCore envenenado");
        Ok(core.crear_contratista(
            &actor,
            DatosContratistaNucleo {
                cedula: datos.cedula,
                nombre: datos.nombre,
                empresa_id: datos.empresa_id,
                tipo_ingreso: datos.tipo_ingreso.into(),
                fecha_vencimiento_praind,
                es_personal_ruta: datos.es_personal_ruta,
                tiene_acceso: datos.tiene_acceso,
            },
        )?)
    }

    pub fn crear_empresa(&self, nombre: String) -> Result<i64, NucleoError> {
        let actor = self
            .sesion
            .lock()
            .expect("mutex de sesión envenenado")
            .clone()
            .ok_or(NucleoError::NoAutenticado)?;
        let core = self.core.lock().expect("mutex de AppCore envenenado");
        Ok(core.crear_empresa(&actor, &nombre)?)
    }

    /// Sólo olvida el actor en memoria — el `AppCore`/la conexión `SQLite`
    /// se quedan abiertos (son del teléfono, no de la sesión) para que
    /// `Nucleo::autenticar` pueda loguear al siguiente usuario sin
    /// reabrir la base.
    pub fn cerrar_sesion(&self) {
        *self.sesion.lock().expect("mutex de sesión envenenado") = None;
    }

    /// Últimos 6 meses por defecto — mismo default que
    /// `desktop/src/pantallas/Historial.tsx` (`fechaHaceMeses(6)`).
    /// `registro_ingresos` es append-only y crece sin límite, así que a
    /// diferencia de los demás buscadores Historial siempre acota por
    /// fecha, nunca trae "todo".
    pub fn buscar_historial(&self, texto: String) -> Result<Vec<MovimientoHistorial>, NucleoError> {
        const LIMITE_MOVIL: usize = 30;

        let core = self.core.lock().expect("mutex de AppCore envenenado");
        let ahora = chrono::Utc::now();
        let desde = ahora - chrono::Duration::days(30 * 6);
        // `hasta` es un límite exclusivo — dejarlo exactamente en "ahora"
        // puede excluir un movimiento creado en el mismo instante (choca
        // con la resolución del reloj). Mismo margen que ya usa
        // `Historial.tsx` cuando `hasta` queda abierto ("hoy + 1 día").
        let hasta = ahora + chrono::Duration::days(1);
        let texto_normalizado = texto.trim();
        let filtro = FiltroHistorialNucleo {
            texto_persona: (!texto_normalizado.is_empty()).then(|| texto_normalizado.to_string()),
            limite: LIMITE_MOVIL,
            ..FiltroHistorialNucleo::nuevo(desde, hasta)
        };
        let pagina = core.buscar_historial(&filtro).map_err(|origen| NucleoError::Interno {
            mensaje: origen.to_string(),
        })?;
        Ok(pagina.items.into_iter().map(Into::into).collect())
    }

    /// Sólo Root/Administrador — ver el doc-comment de `UsuarioResumen`.
    pub fn listar_usuarios(&self, texto: String) -> Result<Vec<UsuarioResumen>, NucleoError> {
        let actor = self
            .sesion
            .lock()
            .expect("mutex de sesión envenenado")
            .clone()
            .ok_or(NucleoError::NoAutenticado)?;
        let core = self.core.lock().expect("mutex de AppCore envenenado");
        let texto_normalizado = texto.trim();
        let filtro = FiltroUsuariosNucleo {
            texto: (!texto_normalizado.is_empty()).then(|| texto_normalizado.to_string()),
            ..Default::default()
        };
        Ok(core
            .buscar_usuarios(&actor, &filtro)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Sólo Root/Administrador — Rust ya rechaza a un actor sin
    /// `Operacion::GestionarUsuarios` con `OperacionNoAutorizada`
    /// (`verificar_creacion_usuario`), y sólo Root puede crear otro Root
    /// (`puede_gestionar_usuario`). Kotlin oculta el menú para Operador
    /// como atajo de UX, no como el control real.
    pub fn crear_usuario(&self, datos: DatosUsuario) -> Result<i64, NucleoError> {
        let actor = self
            .sesion
            .lock()
            .expect("mutex de sesión envenenado")
            .clone()
            .ok_or(NucleoError::NoAutenticado)?;
        let core = self.core.lock().expect("mutex de AppCore envenenado");
        Ok(core.crear_usuario(
            &actor,
            CrearUsuarioInputNucleo {
                cedula: datos.cedula,
                nombre: datos.nombre,
                password: datos.password,
                rol: datos.rol.into(),
                activo: datos.activo,
            },
        )?)
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

        let activos = nucleo
            .listar_ingresos_activos(String::new(), ModoBusquedaActivos::NombreCedula)
            .unwrap();
        assert_eq!(activos.len(), 1);
        assert_eq!(activos[0].registro_id, registro.registro_id);
        assert_eq!(activos[0].contratista_nombre, "Contratista Test");

        nucleo.registrar_salida(registro.registro_id).unwrap();

        let activos_tras_salida = nucleo
            .listar_ingresos_activos(String::new(), ModoBusquedaActivos::NombreCedula)
            .unwrap();
        assert_eq!(activos_tras_salida, Vec::new());
    }

    /// Regresión directa del motivo por el que `Gafete` es un modo aparte:
    /// una cédula que "contiene" el número de gafete no debe aparecer.
    #[test]
    fn listar_activos_por_gafete_es_exacto_sin_ruido_de_cedula() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();

        let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
        conexion
            .execute_batch(
                "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
                 INSERT INTO contratistas (
                     cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso,
                     fecha_vencimiento_praind
                 ) VALUES
                     ('111111117', 'Con Gafete Siete', 1, 'PRAIND', 0, 1, '2099-12-31'),
                     ('222222222', 'Sin Gafete', 1, 'SWAT', 0, 1, NULL);
                 INSERT INTO gafetes (numero, estado) VALUES (7, 'DISPONIBLE');
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
        nucleo
            .registrar_ingreso(1, MedioIngreso::Caminando, Some(7))
            .unwrap();
        // Cédula "222222222" no contiene un 7, así que si el modo Gafete
        // filtrara mal (o cayera al modo texto) esto no debería confundirse
        // con el otro contratista de todas formas — el segundo ingreso
        // (sin gafete) es el control negativo de esta prueba.
        nucleo.registrar_ingreso(2, MedioIngreso::Caminando, None).unwrap();

        let por_gafete = nucleo
            .listar_ingresos_activos("7".to_string(), ModoBusquedaActivos::Gafete)
            .unwrap();
        assert_eq!(por_gafete.len(), 1);
        assert_eq!(por_gafete[0].contratista_nombre, "Con Gafete Siete");

        let texto_no_numerico = nucleo
            .listar_ingresos_activos("abc".to_string(), ModoBusquedaActivos::Gafete)
            .unwrap();
        assert_eq!(texto_no_numerico, Vec::new());
    }

    #[test]
    fn listar_empresas_y_crear_contratista() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();

        let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
        conexion
            .execute_batch(
                "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
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

        let empresas = nucleo.listar_empresas().unwrap();
        assert_eq!(empresas.len(), 1);
        assert_eq!(empresas[0].nombre, "Empresa Test");

        let id = nucleo
            .crear_contratista(DatosContratista {
                cedula: "222222222".to_string(),
                nombre: "Nuevo Contratista".to_string(),
                empresa_id: empresas[0].id,
                tipo_ingreso: TipoIngreso::Swat,
                fecha_vencimiento_praind: None,
                es_personal_ruta: false,
                tiene_acceso: true,
            })
            .unwrap();
        assert!(id > 0);

        let resultados = nucleo.buscar_contratistas("Nuevo".to_string()).unwrap();
        assert_eq!(resultados.len(), 1);
        assert_eq!(resultados[0].nombre, "Nuevo Contratista");
    }

    #[test]
    fn crear_contratista_con_fecha_praind_invalida_falla() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();

        let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
        conexion
            .execute_batch(
                "INSERT INTO empresas (nombre) VALUES ('Empresa Test');
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

        let resultado = nucleo.crear_contratista(DatosContratista {
            cedula: "333333333".to_string(),
            nombre: "Otro Contratista".to_string(),
            empresa_id: 1,
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: Some("no-es-una-fecha".to_string()),
            es_personal_ruta: false,
            tiene_acceso: true,
        });

        assert!(matches!(resultado, Err(NucleoError::FechaInvalida { .. })));
    }

    #[test]
    fn crear_empresa_y_cerrar_sesion() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();
        let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
        conexion
            .execute_batch(
                "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
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

        let id = nucleo.crear_empresa("Empresa Nueva".to_string()).unwrap();
        assert!(id > 0);

        nucleo.cerrar_sesion();

        let resultado = nucleo.crear_empresa("Otra Empresa".to_string());
        assert!(matches!(resultado, Err(NucleoError::NoAutenticado)));
    }

    #[test]
    fn buscar_historial_encuentra_movimiento_reciente() {
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
        nucleo
            .registrar_ingreso(1, MedioIngreso::Caminando, None)
            .unwrap();

        let movimientos = nucleo.buscar_historial(String::new()).unwrap();
        assert_eq!(movimientos.len(), 1);
        assert_eq!(movimientos[0].contratista_nombre, "Contratista Test");
        assert!(movimientos[0].fecha_hora_salida.is_none());
    }

    #[test]
    fn listar_usuarios_y_crear_usuario_solo_root_o_administrador() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();
        let conexion = control_acceso::database::connection::open_database(&ruta).unwrap();
        conexion
            .execute_batch(
                "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                     '999999999', 'Root Test',
                     '$argon2id$v=19$m=19456,t=2,p=1$FZShq0MtV2bGh9nFBgvrGA$dYNDyh7up/wmAY+t/Vf6V5LTCS9sNkQgaH81G650xfM',
                     'ROOT', 1
                 );
                 INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                     '888888888', 'Operador Test',
                     '$argon2id$v=19$m=19456,t=2,p=1$FZShq0MtV2bGh9nFBgvrGA$dYNDyh7up/wmAY+t/Vf6V5LTCS9sNkQgaH81G650xfM',
                     'OPERADOR', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar("999999999".to_string(), "daniel27".to_string())
            .unwrap();

        let id = nucleo
            .crear_usuario(DatosUsuario {
                cedula: "777777777".to_string(),
                nombre: "Nuevo Usuario".to_string(),
                password: "unaPassword123".to_string(),
                rol: RolUsuario::Operador,
                activo: true,
            })
            .unwrap();
        assert!(id > 0);

        let usuarios = nucleo.listar_usuarios(String::new()).unwrap();
        assert!(usuarios.iter().any(|u| u.cedula == "777777777"));

        nucleo.cerrar_sesion();
        nucleo
            .autenticar("888888888".to_string(), "daniel27".to_string())
            .unwrap();

        let resultado = nucleo.listar_usuarios(String::new());
        assert!(matches!(resultado, Err(NucleoError::Interno { .. })));

        let resultado_crear = nucleo.crear_usuario(DatosUsuario {
            cedula: "666666666".to_string(),
            nombre: "Otro Usuario".to_string(),
            password: "unaPassword123".to_string(),
            rol: RolUsuario::Operador,
            activo: true,
        });
        assert!(matches!(resultado_crear, Err(NucleoError::Interno { .. })));
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

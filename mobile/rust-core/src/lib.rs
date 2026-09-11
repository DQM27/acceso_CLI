//! Puente `uniffi` sobre `control_acceso`: expone al Kotlin de la app móvil
//! sólo lo puntual que cada pantalla necesita, sin tocar la lógica del
//! crate raíz. Ver docs/plan-app-movil.md.

use std::path::PathBuf;
use std::sync::Mutex;

use control_acceso::application::AppCore;
use control_acceso::application::GestionNubeError as GestionNubeErrorNucleo;
use control_acceso::application::ResumenSincronizacion as ResumenSincronizacionNucleo;
use control_acceso::application::SesionRealtimeNube as SesionRealtimeNubeNucleo;
use control_acceso::database::queries::Igualdad;
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
use control_acceso::nube::IngresoRemoto as IngresoRemotoNucleo;
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
use control_acceso::tiempo::RelojCorregido;

uniffi::setup_scaffolding!();

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PreparacionIngreso {
    pub contratista_id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub resultado_acceso: ResultadoAcceso,
    pub requiere_gafete: bool,
    pub tiene_ingreso_activo: bool,
    /// Siempre `None` al volver de `preparar_ingreso` -- ese método no toca
    /// la red (mismo motivo que en el núcleo). Kotlin lo completa llamando
    /// a `contratista_activo_en_otro_sitio_con_secreto` (mejor esfuerzo,
    /// igual que `gafete_ocupado_en_sitio_con_secreto`) antes de dejar
    /// continuar, ver `docs/pendientes.md`.
    pub activo_en_otro_sitio: Option<String>,
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
            activo_en_otro_sitio: preparacion.activo_en_otro_sitio,
            gafetes_deuda: preparacion.gafetes_deuda,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
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
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
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

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MovimientoHistorial {
    pub registro_id: i64,
    pub uuid: String,
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

#[derive(Debug, Clone, uniffi::Record)]
pub struct MovimientoHistorialSitio {
    pub uuid: String,
    pub cedula: Option<String>,
    pub contratista_nombre: String,
    pub empresa_nombre: Option<String>,
    pub fecha_hora_ingreso: String,
    pub fecha_hora_salida: Option<String>,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: Option<String>,
    pub usuario_salida_nombre: Option<String>,
    pub motivo_resultado: Option<String>,
    /// `"pc"`/`"mobile"`, o `None` para filas sincronizadas antes de que
    /// esto existiera (`database::schema`, migración 26) -- pedido del
    /// usuario para diferenciar de un vistazo de qué dispositivo vino un
    /// movimiento.
    pub dispositivo_entrada_tipo: Option<String>,
}

impl From<control_acceso::application::MovimientoHistorialSitio> for MovimientoHistorialSitio {
    fn from(m: control_acceso::application::MovimientoHistorialSitio) -> Self {
        Self {
            uuid: m.uuid,
            cedula: m.cedula,
            contratista_nombre: m.contratista_nombre,
            empresa_nombre: m.empresa_nombre,
            fecha_hora_ingreso: m.fecha_hora_ingreso,
            fecha_hora_salida: m.fecha_hora_salida,
            gafete_numero: m.gafete_numero,
            usuario_ingreso_nombre: m.usuario_ingreso_nombre,
            usuario_salida_nombre: m.usuario_salida_nombre,
            motivo_resultado: m.motivo_resultado,
            dispositivo_entrada_tipo: m.dispositivo_entrada_tipo,
        }
    }
}

impl From<MovimientoIngresoResumenNucleo> for MovimientoHistorial {
    fn from(movimiento: MovimientoIngresoResumenNucleo) -> Self {
        Self {
            registro_id: movimiento.registro_id,
            uuid: movimiento.uuid,
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
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
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

/// Ver `docs/plan-persistencia-nube.md` y `ResumenSincronizacionNucleo`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ResumenSincronizacion {
    pub enviados: u32,
    pub fallidos: u32,
    pub remotos_abiertos: u32,
    pub cierres_recibidos: u32,
    pub empresas_recibidas: u32,
    pub contratistas_recibidos: u32,
    pub gafetes_recibidos: u32,
    pub movimientos_historial_recibidos: u32,
    /// Citas nuevas/actualizadas recibidas para el punto de acceso (con sus
    /// visitantes) -- ver `application::nube::ResumenSincronizacion::citas_recibidas`.
    pub citas_recibidas: u32,
    /// Ver `application::nube::ResumenSincronizacion::historial_visitas_recibidos`.
    pub historial_visitas_recibidos: u32,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
    /// `true` si esta sincronización trajo la baja/desactivación de quien
    /// la disparó -- ver `application::nube::ResumenSincronizacion::sesion_expulsada`.
    /// Kotlin debe cerrar la sesión local y volver al login apenas vea esto.
    pub sesion_expulsada: bool,
    /// `docs/pendientes.md`, "alertar luego al sincronizar" -- ingresos que
    /// quedaron activos en este teléfono pero que la nube dice que TAMBIÉN
    /// están activos en otro sitio (colados mientras este dispositivo
    /// estaba offline). Mejor esfuerzo, vacío si el chequeo falla. Siempre
    /// vacío en la activación inicial (`From<ResumenSincronizacionNucleo>`,
    /// base recién configurada, sin ingresos locales todavía) -- sólo
    /// `sincronizar_con_secreto` lo completa de verdad.
    pub conflictos_ingreso: Vec<ConflictoIngresoActivo>,
}

/// Espejo de `control_acceso::nube::ConflictoIngresoActivo`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConflictoIngresoActivo {
    pub cedula: String,
    pub contratista_nombre: String,
    pub sitio_conflicto: String,
}

impl From<control_acceso::nube::ConflictoIngresoActivo> for ConflictoIngresoActivo {
    fn from(conflicto: control_acceso::nube::ConflictoIngresoActivo) -> Self {
        Self {
            cedula: conflicto.cedula,
            contratista_nombre: conflicto.contratista_nombre,
            sitio_conflicto: conflicto.sitio_conflicto,
        }
    }
}

impl From<ResumenSincronizacionNucleo> for ResumenSincronizacion {
    fn from(resumen: ResumenSincronizacionNucleo) -> Self {
        Self {
            enviados: resumen.enviados,
            fallidos: resumen.fallidos,
            remotos_abiertos: resumen.remotos_abiertos,
            cierres_recibidos: resumen.cierres_recibidos,
            empresas_recibidas: resumen.empresas_recibidas,
            contratistas_recibidos: resumen.contratistas_recibidos,
            gafetes_recibidos: resumen.gafetes_recibidos,
            movimientos_historial_recibidos: resumen.movimientos_historial_recibidos,
            citas_recibidas: resumen.citas_recibidas,
            historial_visitas_recibidos: resumen.historial_visitas_recibidos,
            sitio_id: resumen.sitio_id,
            dispositivo_id: resumen.dispositivo_id,
            tipo: resumen.tipo,
            sesion_expulsada: resumen.sesion_expulsada,
            conflictos_ingreso: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SesionRealtimeNube {
    pub base_url: String,
    pub apikey: String,
    pub access_token: String,
    pub expires_in: u64,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
    pub topic: String,
}

impl From<SesionRealtimeNubeNucleo> for SesionRealtimeNube {
    fn from(sesion: SesionRealtimeNubeNucleo) -> Self {
        Self {
            base_url: sesion.base_url,
            apikey: sesion.apikey,
            access_token: sesion.access_token,
            expires_in: sesion.expires_in,
            sitio_id: sesion.sitio_id,
            dispositivo_id: sesion.dispositivo_id,
            tipo: sesion.tipo,
            topic: sesion.topic,
        }
    }
}

/// Un ingreso abierto por el otro dispositivo del mismo sitio -- no vive
/// en el historial de este teléfono, ver `IngresoRemotoNucleo`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct IngresoRemoto {
    pub uuid: String,
    pub contratista_nombre: String,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: Option<String>,
}

impl From<IngresoRemotoNucleo> for IngresoRemoto {
    fn from(remoto: IngresoRemotoNucleo) -> Self {
        Self {
            uuid: remoto.uuid,
            contratista_nombre: remoto.contratista_nombre,
            hora_entrada: remoto.hora_entrada,
            usuario_entrada_nombre: remoto.usuario_entrada_nombre,
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
    /// Usuario global (sincronizado) que todavía no fijó contraseña en
    /// este teléfono -- Kotlin la distingue para mostrar la pantalla de
    /// "fijar contraseña" en vez de un error de login (ver
    /// `AppCore::fijar_password_inicial`, `PantallaLogin.kt`).
    #[error("todavía no tenés contraseña en este dispositivo")]
    SinPasswordLocal,
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
            AutenticacionErrorNucleo::SinPasswordLocal => Self::SinPasswordLocal,
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

impl From<GestionNubeErrorNucleo> for NucleoError {
    fn from(error: GestionNubeErrorNucleo) -> Self {
        Self::Interno {
            mensaje: control_acceso::mensajes::mensaje_gestion_nube(error),
        }
    }
}

/// Ver `Nucleo::autenticar_con_cache`. Duplica la idea de
/// `application::nube::TokenCacheado` (interno a `AppCore`) en vez de
/// reutilizarla por el mismo motivo que ya la duplicó escritorio
/// (`desktop/src-tauri/src/estado.rs::TokenCacheado`): autenticar contra la
/// nube acá no debe pasar por `core_lock()` -- retener ese candado durante
/// la llamada de red es justo lo que esto evita.
struct TokenCacheadoNucleo {
    secreto: String,
    token: control_acceso::nube::TokenDispositivo,
    obtenido_en: std::time::Instant,
}

const DIAS_HISTORIAL_MOVIL: i64 = 7;

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
    /// Caché del último `TokenDispositivo`, deliberadamente FUERA del
    /// `Mutex<AppCore>` de arriba -- ver `Nucleo::autenticar_con_cache`.
    /// Antes de esto, `autenticar`/`gafete_ocupado_en_sitio` llamaban a los
    /// métodos de red de `AppCore` a través de `core_lock()`, que quedaba
    /// tomado durante toda la llamada HTTP: cualquier otra pantalla
    /// (buscar, listar activos, otro registro) se quedaba esperando ese
    /// mismo candado mientras tanto -- se sentía como que la app se
    /// congelaba al iniciar sesión o al confirmar un ingreso con gafete,
    /// sobre todo si la sincronización periódica estaba en curso al mismo
    /// tiempo.
    token_nube_cacheado: Mutex<Option<TokenCacheadoNucleo>>,
    /// Serializa las sincronizaciones completas (`sincronizar_con_nube`,
    /// llamada desde el timer periódico, un aviso Realtime Y el botón
    /// manual -- ver `SincronizacionPeriodica.kt`/`NubeViewModel.kt`) para
    /// que nunca corran dos en simultáneo pisándose la cola de salida --
    /// mismo motivo que el `static SINCRONIZACION: Mutex<()>` de
    /// `desktop/src-tauri/src/comandos/nube.rs::ejecutar_sincronizacion`.
    /// Deliberadamente NO es el mismo candado que `core`: mientras una
    /// sincronización espera acá (o corre su red), cualquier búsqueda o
    /// registro sigue andando con total normalidad.
    sincronizacion_en_curso: Mutex<()>,
    /// Capturada una sola vez en `abrir` -- el núcleo ya no expone
    /// `ruta_base_datos()` como método (ver `database::connection::ruta_base_datos`,
    /// que resuelve el path por defecto; acá ya llega como parámetro del
    /// constructor). Misma idea que `GuiState::ruta_base_datos` en escritorio.
    ruta_base_datos: PathBuf,
}

#[uniffi::export]
impl Nucleo {
    #[uniffi::constructor]
    pub fn abrir(ruta_base_datos: String) -> Result<Self, NucleoError> {
        // `RelojCorregido`, no `RelojSistema` -- un teléfono con la hora mal
        // puesta manualmente (o sin datos/GPS para que Android la ajuste
        // solo) tiene el mismo problema que se vio en escritorio: cada
        // autenticación contra la nube mide el desfase real y lo aplica acá
        // (ver `application::nube::AppCore::actualizar_desfase_reloj`).
        let core = AppCore::abrir_con_reloj(
            &ruta_base_datos,
            std::sync::Arc::new(RelojCorregido::nuevo()),
        )
        .map_err(|origen| NucleoError::Apertura {
            mensaje: origen.to_string(),
        })?;
        Ok(Self {
            core: Mutex::new(core),
            sesion: Mutex::new(None),
            token_nube_cacheado: Mutex::new(None),
            sincronizacion_en_curso: Mutex::new(()),
            ruta_base_datos: PathBuf::from(&ruta_base_datos),
        })
    }

    /// `directorio` sólo para los intentos de sincronización (ver abajo) --
    /// el resto del login sigue sin necesitarlo, la base ya está abierta
    /// desde `abrir`.
    ///
    /// Dos chequeos contra la nube, uno para cada dirección de un cambio de
    /// estado remoto -- decisión explícita: "por seguridad, pero nunca
    /// bloqueante" (el teléfono tiene que poder operar sin internet), así
    /// que los dos son best-effort (con el tope de `nube::cliente::TIMEOUT_HTTP`):
    ///
    /// 1. **Alta o reactivación**: si el chequeo local dice "inactivo"
    ///    (`AutenticacionErrorNucleo::UsuarioInactivo`) o "no existe"
    ///    (`CredencialesInvalidas` -- que es la misma variante que una
    ///    contraseña incorrecta, ver `autenticacion_service.rs`), puede ser
    ///    que a este usuario lo hayan reactivado en otro dispositivo, o
    ///    creado en el panel/otro sitio DESPUÉS del primer arranque de este
    ///    teléfono, y esta base todavía no se enteró -- antes de rendirse,
    ///    refresca sólo el catálogo (`refrescar_catalogo_sin_sesion`, sin
    ///    sesión) y reintenta el login local una vez más. Sin esto, un
    ///    usuario nuevo o una reactivación remota nunca se podían reflejar
    ///    acá: la sincronización periódica (`SincronizacionPeriodica.kt`)
    ///    recién arranca DESPUÉS de un primer login exitoso, así que una
    ///    cédula que todavía no existe en este teléfono se quedaba
    ///    "credenciales inválidas" para siempre, sin importar cuánto se
    ///    esperara -- reportado en vivo: un ROOT creado en Supabase después
    ///    del primer arranque del emulador nunca podía entrar. Costo
    ///    aceptado: una contraseña tipeada mal también dispara este
    ///    refresco de más (no hay forma barata de distinguir los dos casos
    ///    antes de sincronizar) -- mismo costo que ya paga escritorio, que
    ///    sincroniza el catálogo en CADA intento de login, acierte o no
    ///    (`desktop/src-tauri/src/comandos/autenticacion.rs`).
    /// 2. **Baja**: tras un login local exitoso, confirma en vivo que la
    ///    cédula sigue activa (`usuario_sigue_activo_remoto` -- una fila,
    ///    una columna, no la sincronización completa que hacía esto antes:
    ///    medida como la causa real del retraso de "un par de segundos"
    ///    que se sentía al entrar). La sincronización completa (cola,
    ///    catálogo, historial...) sigue disparándose, pero Kotlin la lanza
    ///    aparte (ver `LoginViewModel.autenticar`) sin que este método la
    ///    espere -- acá retener el candado durante una sincronización
    ///    entera hubiera vuelto a sentirse lento.
    pub fn autenticar(
        &self,
        cedula: String,
        password: String,
        directorio: String,
        identificador_dispositivo: String,
    ) -> Result<UsuarioSesion, NucleoError> {
        let intento = self.core_lock().autenticar(&cedula, &password);
        let sesion = match intento {
            Ok(sesion) => sesion,
            Err(
                AutenticacionErrorNucleo::UsuarioInactivo
                | AutenticacionErrorNucleo::CredencialesInvalidas,
            ) => {
                let _ = self.refrescar_catalogo_sin_sesion(&directorio, &identificador_dispositivo);
                self.core_lock().autenticar(&cedula, &password)?
            }
            Err(otro) => return Err(otro.into()),
        };

        // Ver el comentario de `token_nube_cacheado`: a diferencia de la
        // línea de arriba (autenticación local, SQLite puro), este chequeo
        // habla con la nube -- por eso ya no pasa por `core_lock()` más que
        // un instante para `autorizar_uso_nube` (verificar que `sesion`
        // todavía puede usar la nube, chequeo local rápido). El resto
        // (caché de token + la llamada HTTP en sí) corre sin el candado de
        // `AppCore` tomado.
        let autorizado_para_nube = self.core_lock().autorizar_uso_nube(&sesion).is_ok();
        let sigue_activo = if autorizado_para_nube {
            control_acceso::nube::credenciales::cargar_secreto_en_con_identificador(
                std::path::Path::new(&directorio),
                &identificador_dispositivo,
            )
            .and_then(|secreto| {
                let token = self.autenticar_con_cache(&secreto).ok()?;
                let contexto = control_acceso::nube::ContextoSincronizacion {
                    base_url: control_acceso::nube::BASE_URL,
                    apikey: control_acceso::nube::APIKEY,
                    token: &token.access_token,
                    dispositivo_id: &token.dispositivo_id,
                    sitio_id: &token.sitio_id,
                };
                control_acceso::nube::usuario_sigue_activo_remoto(&contexto, &sesion.cedula).ok()
            })
            .unwrap_or(true)
        } else {
            true
        };
        if !sigue_activo {
            return Err(NucleoError::UsuarioInactivo);
        }

        *self.sesion_lock() = Some(sesion.clone());
        Ok(sesion.into())
    }

    /// Igual que [`Nucleo::autenticar`], pero con el secreto ya descifrado
    /// por Android Keystore. Si el login local necesita refrescar catálogo
    /// por un usuario recién creado/reactivado, usa este secreto en memoria
    /// sin leer credenciales desde disco.
    pub fn autenticar_con_secreto(
        &self,
        cedula: String,
        password: String,
        secreto: String,
    ) -> Result<UsuarioSesion, NucleoError> {
        let intento = self.core_lock().autenticar(&cedula, &password);
        let sesion = match intento {
            Ok(sesion) => sesion,
            Err(
                AutenticacionErrorNucleo::UsuarioInactivo
                | AutenticacionErrorNucleo::CredencialesInvalidas,
            ) => {
                if !secreto.trim().is_empty() {
                    let _ = self.refrescar_catalogo_sin_sesion_con_secreto(&secreto);
                }
                self.core_lock().autenticar(&cedula, &password)?
            }
            Err(otro) => return Err(otro.into()),
        };

        let autorizado_para_nube = self.core_lock().autorizar_uso_nube(&sesion).is_ok();
        let sigue_activo = if autorizado_para_nube && !secreto.trim().is_empty() {
            let token = self.autenticar_con_cache(&secreto).ok();
            token
                .and_then(|token| {
                    let contexto = control_acceso::nube::ContextoSincronizacion {
                        base_url: control_acceso::nube::BASE_URL,
                        apikey: control_acceso::nube::APIKEY,
                        token: &token.access_token,
                        dispositivo_id: &token.dispositivo_id,
                        sitio_id: &token.sitio_id,
                    };
                    control_acceso::nube::usuario_sigue_activo_remoto(&contexto, &sesion.cedula)
                        .ok()
                })
                .unwrap_or(true)
        } else {
            true
        };
        if !sigue_activo {
            return Err(NucleoError::UsuarioInactivo);
        }

        *self.sesion_lock() = Some(sesion.clone());
        Ok(sesion.into())
    }

    /// Completa el alta de contraseña de un usuario global que `autenticar`
    /// rechazó con `NucleoError::SinPasswordLocal` -- ver
    /// `AppCore::fijar_password_inicial`. Deja la sesión iniciada directo.
    pub fn fijar_password_inicial(
        &self,
        cedula: String,
        nueva_password: String,
    ) -> Result<UsuarioSesion, NucleoError> {
        let sesion = self
            .core_lock()
            .fijar_password_inicial(&cedula, &nueva_password)?;
        *self.sesion_lock() = Some(sesion.clone());
        Ok(sesion.into())
    }

    /// Vista previa antes de confirmar — misma decisión que ya toma la GUI
    /// de escritorio (`desktop/src/pantallas/NuevoIngresoModal.tsx`): no
    /// rechaza PRAIND vencido/ingreso activo aquí, sólo informa; quien llama
    /// (Kotlin) decide si deja continuar mirando los campos ya calculados.
    pub fn preparar_ingreso(&self, contratista_id: i64) -> Result<PreparacionIngreso, NucleoError> {
        Ok(self.core_lock().preparar_ingreso(contratista_id)?.into())
    }

    pub fn registrar_ingreso(
        &self,
        contratista_id: i64,
        medio: MedioIngreso,
        gafete: Option<i64>,
    ) -> Result<ResultadoRegistroEntrada, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
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

        let texto_normalizado = texto.trim();
        let filtro = FiltroContratistasNucleo {
            texto: (!texto_normalizado.is_empty()).then(|| texto_normalizado.to_string()),
            limite: LIMITE_MOVIL,
            ..Default::default()
        };
        let pagina = self
            .core_lock()
            .buscar_contratistas(&filtro)
            .map_err(|origen| NucleoError::Interno {
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
        let lista = self
            .core_lock()
            .listar_ingresos_activos(&filtro)
            .map_err(|origen| NucleoError::Interno {
                mensaje: origen.to_string(),
            })?;
        Ok(lista.items.into_iter().map(Into::into).collect())
    }

    pub fn registrar_salida(&self, registro_id: i64) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().registrar_salida(&actor, registro_id)?)
    }

    pub fn listar_empresas(&self) -> Result<Vec<Empresa>, NucleoError> {
        Ok(self
            .core_lock()
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
        let actor = self.actor_autenticado()?;

        let fecha_vencimiento_praind = datos
            .fecha_vencimiento_praind
            .map(|texto| {
                texto
                    .parse()
                    .map_err(|_| NucleoError::FechaInvalida { mensaje: texto })
            })
            .transpose()?;

        Ok(self.core_lock().crear_contratista(
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
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().crear_empresa(&actor, &nombre)?)
    }

    /// Sólo olvida el actor en memoria — el `AppCore`/la conexión `SQLite`
    /// se quedan abiertos (son del teléfono, no de la sesión) para que
    /// `Nucleo::autenticar` pueda loguear al siguiente usuario sin
    /// reabrir la base.
    pub fn cerrar_sesion(&self) {
        *self.sesion_lock() = None;
    }

    /// Últimos 7 días por defecto: en Android el historial es contexto
    /// operativo reciente, no auditoría exhaustiva. Para rangos amplios,
    /// filtros densos y exportación están web/escritorio.
    pub fn buscar_historial(&self, texto: String) -> Result<Vec<MovimientoHistorial>, NucleoError> {
        const LIMITE_MOVIL: usize = 30;

        let ahora = chrono::Utc::now();
        let desde = ahora - chrono::Duration::days(DIAS_HISTORIAL_MOVIL);
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
        let pagina = self
            .core_lock()
            .buscar_historial(&filtro)
            .map_err(|origen| NucleoError::Interno {
                mensaje: origen.to_string(),
            })?;
        Ok(pagina.items.into_iter().map(Into::into).collect())
    }

    pub fn listar_historial_sitio(
        &self,
        texto: String,
    ) -> Result<Vec<MovimientoHistorialSitio>, NucleoError> {
        let actor = self.actor_autenticado()?;
        let ahora = chrono::Utc::now();
        Ok(self
            .core_lock()
            .listar_historial_sitio(
                &actor,
                ahora - chrono::Duration::days(DIAS_HISTORIAL_MOVIL),
                ahora + chrono::Duration::days(1),
                &texto,
            )?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Sólo Root/Administrador — ver el doc-comment de `UsuarioResumen`.
    pub fn listar_usuarios(&self, texto: String) -> Result<Vec<UsuarioResumen>, NucleoError> {
        let actor = self.actor_autenticado()?;
        let core = self.core_lock();
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
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().crear_usuario(
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

    /// `true` mientras la base no tenga ningún usuario todavía -- Kotlin lo
    /// usa para decidir si mostrar la pantalla de arranque (pegar el
    /// secreto) en vez del login (ver `MainActivity.kt`).
    pub fn requiere_configuracion_inicial(&self) -> Result<bool, NucleoError> {
        Ok(self.core_lock().requiere_configuracion_inicial()?)
    }

    /// Arranque de una base vacía -- sin sesión, porque todavía no existe
    /// ningún usuario con quien autenticar. Guarda el secreto pegado en la
    /// pantalla de arranque y trae el catálogo remoto (usuarios incluidos),
    /// para que el próximo intento de login ya tenga con quién autenticar
    /// (con el centinela `SIN_PASSWORD_LOCAL`, cae solo en "fijar
    /// contraseña"). Método legado: la app Android nueva usa
    /// [`Nucleo::configurar_dispositivo_inicial_con_secreto`] y persiste el
    /// secreto con Android Keystore.
    pub fn configurar_dispositivo_inicial(
        &self,
        directorio: String,
        identificador_dispositivo: String,
        secreto: String,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        Ok(self
            .core_lock()
            .configurar_dispositivo_inicial(
                Some(std::path::Path::new(&directorio)),
                Some(&identificador_dispositivo),
                &secreto,
                None,
            )?
            .into())
    }

    /// Igual que [`Nucleo::configurar_dispositivo_inicial`], pero sin
    /// persistir el secreto desde Rust. Android lo guarda con Android
    /// Keystore y sólo entrega el secreto descifrado en memoria para esta
    /// autenticación inicial.
    ///
    /// Los parámetros de metadata (todos opcionales, `""` = no disponible)
    /// viajan una única vez, en esta primera autenticación -- ver
    /// `control_acceso::nube::MetadatosDispositivo`. No se vuelven a
    /// reenviar en cada renovación de token porque casi nunca cambian, y
    /// esto ya alcanza para que el panel de administración distinga el
    /// teléfono físico detrás de cada secreto (ver
    /// `docs/plan-sesion-unica-dispositivos.md`).
    #[allow(clippy::too_many_arguments)]
    pub fn configurar_dispositivo_inicial_con_secreto(
        &self,
        secreto: String,
        identificador_hardware: String,
        nombre_dispositivo: String,
        plataforma: String,
        version_build: String,
        app_version: String,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        if !self.core_lock().requiere_configuracion_inicial()? {
            return Err(NucleoError::from(GestionNubeErrorNucleo::YaConfigurado));
        }

        let cadena_opcional = |texto: String| (!texto.trim().is_empty()).then_some(texto);
        let metadata = control_acceso::nube::MetadatosDispositivo {
            identificador_hardware: cadena_opcional(identificador_hardware),
            nombre_dispositivo: cadena_opcional(nombre_dispositivo),
            plataforma: cadena_opcional(plataforma),
            version_build: cadena_opcional(version_build),
            app_version: cadena_opcional(app_version),
        };
        let token = self
            .autenticar_y_cachear(&secreto, Some(&metadata))
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })?;
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.core_lock().actualizar_desfase_reloj(desfase_ms);
        }

        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::BASE_URL,
            apikey: control_acceso::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        let catalogo = control_acceso::nube::recibir_catalogo_del_sitio(&conexion, &contexto)
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })?;

        Ok(ResumenSincronizacion {
            enviados: 0,
            fallidos: 0,
            remotos_abiertos: 0,
            cierres_recibidos: 0,
            empresas_recibidas: catalogo.empresas_recibidas,
            contratistas_recibidos: catalogo.contratistas_recibidos,
            gafetes_recibidos: catalogo.gafetes_recibidos,
            movimientos_historial_recibidos: 0,
            citas_recibidas: 0,
            historial_visitas_recibidos: 0,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            sesion_expulsada: false,
            // Activación inicial: base recién configurada, sin ingresos
            // locales todavía -- mismo criterio que
            // `From<ResumenSincronizacionNucleo>` arriba y que el equivalente
            // en desktop/src-tauri/src/comandos/nube.rs.
            conflictos_ingreso: Vec::new(),
        })
    }

    /// Guarda el secreto de este dispositivo en el archivo administrado por
    /// Rust. Método legado: Android nuevo usa Android Keystore desde Kotlin
    /// y sólo mantiene este camino para compatibilidad/migración.
    pub fn guardar_secreto_dispositivo(
        &self,
        directorio: String,
        identificador_dispositivo: String,
        secreto: String,
    ) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().guardar_secreto_dispositivo(
            &actor,
            Some(std::path::Path::new(&directorio)),
            Some(&identificador_dispositivo),
            &secreto,
        )?)
    }

    /// No revela el secreto -- sólo si ya hay uno guardado. Ver
    /// [`Nucleo::guardar_secreto_dispositivo`] sobre `identificador_dispositivo`.
    pub fn secreto_dispositivo_guardado(
        &self,
        directorio: String,
        identificador_dispositivo: String,
    ) -> Result<bool, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().secreto_dispositivo_guardado(
            &actor,
            Some(std::path::Path::new(&directorio)),
            Some(&identificador_dispositivo),
        )?)
    }

    /// Lee el secreto guardado por versiones móviles anteriores a Android
    /// Keystore. Kotlin lo usa sólo para migrarlo al almacén seguro nuevo.
    pub fn cargar_secreto_dispositivo_legado(
        &self,
        directorio: String,
        identificador_dispositivo: String,
    ) -> Option<String> {
        control_acceso::nube::credenciales::cargar_secreto_en_con_identificador(
            std::path::Path::new(&directorio),
            &identificador_dispositivo,
        )
    }

    /// Borra el archivo legado de `cargar_secreto_dispositivo_legado` --
    /// Kotlin lo llama justo después de migrar ese secreto al Keystore, para
    /// no dejar la copia vieja (en texto plano, ver el módulo
    /// `nube::credenciales`) huérfana en el almacenamiento de la app.
    pub fn borrar_secreto_dispositivo_legado(&self, directorio: String) -> Result<(), NucleoError> {
        control_acceso::nube::credenciales::borrar_secreto_en(std::path::Path::new(&directorio))
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })
    }

    /// Autentica este dispositivo, drena la bandeja de salida pendiente y
    /// refresca la caché de lo que el otro dispositivo del mismo sitio
    /// tiene abierto ahora mismo.
    pub fn sincronizar_con_nube(
        &self,
        directorio: String,
        identificador_dispositivo: String,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        // Serializa contra cualquier otra sincronización ya en curso (timer
        // periódico, un aviso Realtime, este mismo método llamado dos veces
        // seguidas) -- nunca dos a la vez pisándose la cola de salida. Ver
        // el comentario de `sincronizacion_en_curso`: mientras se espera
        // acá (o corre la red de abajo), NINGÚN otro método del núcleo se
        // ve afectado, sólo otra sincronización.
        let _sincronizacion = self
            .sincronizacion_en_curso
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;

        let secreto = control_acceso::nube::credenciales::cargar_secreto_en_con_identificador(
            std::path::Path::new(&directorio),
            &identificador_dispositivo,
        )
        .ok_or_else(|| NucleoError::Interno {
            mensaje: "Todavía no se guardó el secreto de este dispositivo".to_string(),
        })?;
        let token = self
            .autenticar_con_cache(&secreto)
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })?;
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.core_lock().actualizar_desfase_reloj(desfase_ms);
        }

        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::BASE_URL,
            apikey: control_acceso::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        // A partir de acá, ninguna llamada más toca `core_lock()` hasta el
        // chequeo de `sesion_sigue_activa` al final -- toda la cadena de
        // red corre sobre `conexion`, propia, sin bloquear ninguna otra
        // pantalla mientras dura.
        let conexion = self.conexion_secundaria()?;
        let mapear = |error: control_acceso::nube::SincronizacionError| NucleoError::Interno {
            mensaje: error.to_string(),
        };
        let resumen_cola =
            control_acceso::nube::drenar_cola(&conexion, &contexto, 200).map_err(mapear)?;
        let cierres_recibidos =
            control_acceso::nube::recibir_cierres_de_ingresos_propios(&conexion, &contexto)
                .map_err(mapear)?;
        let remotos = control_acceso::nube::recibir_ingresos_abiertos(&conexion, &contexto)
            .map_err(mapear)?;
        let catalogo = control_acceso::nube::recibir_catalogo_del_sitio(&conexion, &contexto)
            .map_err(mapear)?;
        let movimientos_historial_recibidos =
            control_acceso::nube::recibir_historial_del_sitio(&conexion, &contexto)
                .map_err(mapear)?;
        let citas_recibidas = control_acceso::nube::recibir_citas_del_sitio(&conexion, &contexto)
            .map_err(mapear)?;
        // Sin `recibir_historial_visitas_del_sitio` a propósito -- decisión
        // explícita del usuario: el celular es para acciones rápidas del
        // guardia (check-in/check-out), auditar el historial de visitas es
        // algo esporádico que le corresponde a la PC. Mismo campo en el
        // struct compartido (con `0` acá) para no bifurcar el tipo entre
        // plataformas, no porque el celular lo necesite.
        let historial_visitas_recibidos = 0;

        // Mejor esfuerzo a propósito, igual que en escritorio -- ya se llegó
        // hasta acá con la nube respondiendo bien, pero si este chequeo
        // puntual falla no tiene sentido tumbar un sync que por lo demás
        // anduvo. Vacío en ese caso, no error (ver
        // `ConflictoIngresoActivo`/`nube::contratistas_con_conflicto_activo`).
        let conflictos_ingreso =
            control_acceso::nube::contratistas_con_conflicto_activo(&conexion, &contexto)
                .unwrap_or_default()
                .into_iter()
                .map(ConflictoIngresoActivo::from)
                .collect();

        // Igual que en escritorio: si esta sincronización trajo la baja de
        // quien la disparó, la sesión de ESTE teléfono se cierra sola acá
        // mismo, no sólo se avisa -- cualquier llamada siguiente que
        // dependa de `actor_autenticado()` debe fallar de inmediato.
        let sesion_expulsada = !self.core_lock().sesion_sigue_activa(&actor);
        if sesion_expulsada {
            *self.sesion_lock() = None;
        }

        Ok(ResumenSincronizacion {
            enviados: resumen_cola.enviados,
            fallidos: resumen_cola.fallidos,
            remotos_abiertos: u32::try_from(remotos.len()).unwrap_or(u32::MAX),
            cierres_recibidos,
            empresas_recibidas: catalogo.empresas_recibidas,
            contratistas_recibidos: catalogo.contratistas_recibidos,
            gafetes_recibidos: catalogo.gafetes_recibidos,
            movimientos_historial_recibidos,
            citas_recibidas,
            historial_visitas_recibidos,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            sesion_expulsada,
            conflictos_ingreso,
        })
    }

    /// Sincroniza usando el secreto ya descifrado por Android Keystore.
    /// Evita que el núcleo móvil lea un secreto persistido en texto plano.
    pub fn sincronizar_con_nube_con_secreto(
        &self,
        secreto: String,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        self.sincronizar_con_secreto(&secreto)
    }

    /// Devuelve lo mínimo para que Kotlin escuche Broadcast privado por
    /// sitio; el socket y sus reconexiones viven fuera del núcleo.
    pub fn sesion_realtime_nube(
        &self,
        directorio: String,
    ) -> Result<SesionRealtimeNube, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
            .sesion_realtime_nube(&actor, Some(std::path::Path::new(&directorio)))?
            .into())
    }

    /// Igual que [`Nucleo::sesion_realtime_nube`], pero tomando el secreto
    /// desde Android Keystore en Kotlin en vez del archivo administrado por
    /// Rust.
    pub fn sesion_realtime_nube_con_secreto(
        &self,
        secreto: String,
    ) -> Result<SesionRealtimeNube, NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache(&secreto)
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })?;
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.core_lock().actualizar_desfase_reloj(desfase_ms);
        }
        let topic = format!("sitio:{}", token.sitio_id);

        Ok(SesionRealtimeNube {
            base_url: control_acceso::nube::BASE_URL.to_string(),
            apikey: control_acceso::nube::APIKEY.to_string(),
            access_token: token.access_token,
            expires_in: token.expires_in,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            topic,
        })
    }

    /// Lectura pura de la caché local `ingresos_remotos` -- no hace falta
    /// red para mostrarla, ya la llenó la última `sincronizar_con_nube`.
    pub fn listar_ingresos_remotos(&self) -> Result<Vec<IngresoRemoto>, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
            .listar_ingresos_remotos(&actor)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Chequeo en vivo (no la caché local) de si `gafete_numero` ya está
    /// activo en este sitio del lado de OTRO dispositivo -- llamar justo
    /// antes de `registrar_ingreso` cuando el ingreso lleva gafete. Cada
    /// dispositivo sólo valida el gafete contra su propia base `SQLite`,
    /// que nunca ve lo que hizo el otro hasta sincronizar, así que dos
    /// dispositivos del mismo sitio podían aceptar el mismo número como
    /// activo a la vez.
    pub fn gafete_ocupado_en_sitio(
        &self,
        directorio: String,
        gafete_numero: i64,
    ) -> Result<bool, NucleoError> {
        let actor = self.actor_autenticado()?;
        // Ver el comentario de `token_nube_cacheado`: este chequeo corre
        // justo antes de confirmar un ingreso con gafete, así que retener
        // `core_lock()` durante la red acá es exactamente el freeze que se
        // sentía al registrar. `autorizar_uso_nube` sigue pasando por el
        // candado -- es SQLite puro, dura microsegundos -- pero se libera
        // antes de tocar la red.
        self.core_lock().autorizar_uso_nube(&actor)?;

        let Some(secreto) = control_acceso::nube::credenciales::cargar_secreto_en(
            std::path::Path::new(&directorio),
        ) else {
            // Sin secreto guardado (sitio de un solo dispositivo, o nube
            // sin configurar): no hay con quién chocar, no hace falta red.
            return Ok(false);
        };
        let token = self
            .autenticar_con_cache(&secreto)
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::BASE_URL,
            apikey: control_acceso::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        // A diferencia de `autenticar`, acá un fallo de red SÍ se propaga
        // (no `.unwrap_or`): con nube configurada, más vale bloquear el
        // ingreso que arriesgar el mismo gafete duplicado entre
        // dispositivos -- decisión ya documentada en
        // `application::nube::AppCore::gafete_ocupado_en_sitio`.
        control_acceso::nube::gafete_ocupado_en_otro_dispositivo(&contexto, gafete_numero).map_err(
            |error| NucleoError::Interno {
                mensaje: error.to_string(),
            },
        )
    }

    /// Chequeo remoto usando el secreto ya descifrado por Android Keystore.
    pub fn gafete_ocupado_en_sitio_con_secreto(
        &self,
        secreto: String,
        gafete_numero: i64,
    ) -> Result<bool, NucleoError> {
        if secreto.trim().is_empty() {
            return Ok(false);
        }
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache(&secreto)
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::BASE_URL,
            apikey: control_acceso::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        control_acceso::nube::gafete_ocupado_en_otro_dispositivo(&contexto, gafete_numero).map_err(
            |error| NucleoError::Interno {
                mensaje: error.to_string(),
            },
        )
    }

    /// Chequeo cruzado entre sitios (`docs/pendientes.md`, "Chequeo cruzado
    /// de ingresos abiertos entre sitios") -- mismo patrón que
    /// `gafete_ocupado_en_sitio_con_secreto`, pero de mejor esfuerzo: sin
    /// secreto, o si la red falla, `Ok(None)` en vez de propagar el error
    /// (acá SÍ hay con qué chocar sin red -- un ingreso registrado offline
    /// queda local igual, y el conflicto se detecta después al sincronizar,
    /// ver `sincronizar_con_secreto`/`ResumenSincronizacion::conflictos_ingreso`).
    /// Kotlin la llama después de `preparar_ingreso`, sólo si los chequeos
    /// locales ya dejaron pasar.
    pub fn contratista_activo_en_otro_sitio_con_secreto(
        &self,
        secreto: String,
        cedula: String,
    ) -> Option<String> {
        if secreto.trim().is_empty() {
            return None;
        }
        let token = self.autenticar_con_cache(&secreto).ok()?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::BASE_URL,
            apikey: control_acceso::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        control_acceso::nube::contratista_activo_en_otro_sitio(&contexto, &cedula)
            .ok()
            .flatten()
    }

    /// Cierra, contra la nube, un ingreso abierto por el otro dispositivo
    /// del mismo sitio -- nunca toca el historial local de este teléfono.
    pub fn cerrar_ingreso_remoto(
        &self,
        directorio: String,
        uuid: String,
    ) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().cerrar_ingreso_remoto(
            &actor,
            Some(std::path::Path::new(&directorio)),
            &uuid,
        )?)
    }

    /// Cierra un ingreso remoto usando el secreto ya descifrado por Android
    /// Keystore.
    pub fn cerrar_ingreso_remoto_con_secreto(
        &self,
        secreto: String,
        uuid: String,
    ) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache(&secreto)
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::BASE_URL,
            apikey: control_acceso::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        control_acceso::nube::cerrar_ingreso_remoto(&conexion, &contexto, &uuid, &actor.nombre)
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })?;
        Ok(())
    }
}

impl Nucleo {
    /// Recupera el guard aunque el mutex haya quedado "envenenado" (un
    /// panic anterior mientras alguien lo sostenía) en vez de propagar ese
    /// panic a cada llamada futura — con `uniffi` cada método público es una
    /// frontera FFI: un solo bug de una llamada no debe dejar inservibles
    /// todas las demás pantallas hasta reiniciar la app. `AppCore` no deja
    /// datos a medio escribir visibles tras un panic a mitad de operación
    /// (`SQLite` ya maneja sus propias transacciones), así que el estado
    /// recuperado sigue siendo válido para seguir operando.
    fn core_lock(&self) -> std::sync::MutexGuard<'_, AppCore> {
        self.core
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Reusa el último `TokenDispositivo` mientras siga vigente en vez de
    /// autenticar de cero -- mismo margen y misma lógica que
    /// `GuiState::autenticar_con_cache` en escritorio (y que
    /// `AppCore::autenticar_con_cache`, que este método reemplaza para
    /// móvil: ver el comentario de `token_nube_cacheado`). Nunca toca
    /// `core_lock()`.
    fn autenticar_con_cache(
        &self,
        secreto: &str,
    ) -> Result<control_acceso::nube::TokenDispositivo, control_acceso::nube::NubeError> {
        self.autenticar_y_cachear(secreto, None)
    }

    /// Igual que [`Nucleo::autenticar_con_cache`], pero permite adjuntar
    /// `metadata` cuando hace falta mandarla (sólo la activación inicial,
    /// ver [`Nucleo::configurar_dispositivo_inicial_con_secreto`]). El resto
    /// de los llamadores pasan `None` a través de `autenticar_con_cache`.
    fn autenticar_y_cachear(
        &self,
        secreto: &str,
        metadata: Option<&control_acceso::nube::MetadatosDispositivo>,
    ) -> Result<control_acceso::nube::TokenDispositivo, control_acceso::nube::NubeError> {
        const MARGEN_EXPIRACION: std::time::Duration = std::time::Duration::from_secs(30);

        {
            let cache = self
                .token_nube_cacheado
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entrada) = cache.as_ref() {
                let vigente_por = std::time::Duration::from_secs(entrada.token.expires_in)
                    .saturating_sub(MARGEN_EXPIRACION);
                if entrada.secreto == secreto && entrada.obtenido_en.elapsed() < vigente_por {
                    let mut token = entrada.token.clone();
                    token.desfase_reloj_ms = None;
                    return Ok(token);
                }
            }
        }

        let token = control_acceso::nube::autenticar_dispositivo(
            control_acceso::nube::BASE_URL,
            secreto,
            metadata,
        )?;
        *self
            .token_nube_cacheado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(TokenCacheadoNucleo {
            secreto: secreto.to_string(),
            token: token.clone(),
            obtenido_en: std::time::Instant::now(),
        });
        Ok(token)
    }

    /// Ver `AppCore::refrescar_catalogo_sin_sesion` -- misma idea (la
    /// identidad ante la nube es del dispositivo, no de un usuario que
    /// todavía no logró entrar), reimplementada acá para no retener
    /// `core_lock()` durante la red ni la escritura del catálogo -- mismo
    /// motivo que el resto de este archivo. Sólo se llama desde el camino
    /// de reintento de `autenticar` (usuario recién reactivado o creado en
    /// otro dispositivo), best-effort a propósito: el `let _ =` de quien
    /// llama ya ignora el resultado.
    fn refrescar_catalogo_sin_sesion(
        &self,
        directorio: &str,
        identificador_dispositivo: &str,
    ) -> Result<(), NucleoError> {
        let mapear_nube = |error: control_acceso::nube::NubeError| NucleoError::Interno {
            mensaje: error.to_string(),
        };
        let secreto = control_acceso::nube::credenciales::cargar_secreto_en_con_identificador(
            std::path::Path::new(directorio),
            identificador_dispositivo,
        )
        .ok_or_else(|| NucleoError::Interno {
            mensaje: "Todavía no se guardó el secreto de este dispositivo".to_string(),
        })?;
        let token = self.autenticar_con_cache(&secreto).map_err(mapear_nube)?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::BASE_URL,
            apikey: control_acceso::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        control_acceso::nube::recibir_catalogo_del_sitio(&conexion, &contexto).map_err(
            |error| NucleoError::Interno {
                mensaje: error.to_string(),
            },
        )?;
        Ok(())
    }

    fn refrescar_catalogo_sin_sesion_con_secreto(&self, secreto: &str) -> Result<(), NucleoError> {
        let mapear_nube = |error: control_acceso::nube::NubeError| NucleoError::Interno {
            mensaje: error.to_string(),
        };
        let token = self.autenticar_con_cache(secreto).map_err(mapear_nube)?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::BASE_URL,
            apikey: control_acceso::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        control_acceso::nube::recibir_catalogo_del_sitio(&conexion, &contexto).map_err(
            |error| NucleoError::Interno {
                mensaje: error.to_string(),
            },
        )?;
        Ok(())
    }

    /// Conexión propia al mismo archivo, independiente de `core` -- mismo
    /// patrón y mismo motivo que `GuiState::conexion_secundaria` en
    /// escritorio: `sincronizar_con_nube` hace varias llamadas HTTP
    /// seguidas (drenar cola, cierres, ingresos abiertos, catálogo,
    /// historial) y cada una escribe lo que trae -- sin esto, esa cadena
    /// entera retendría `core_lock()`, bloqueando cualquier otra pantalla
    /// mientras dura. Sólo funciona sin pisarse con la conexión principal
    /// porque la base está en `journal_mode=WAL` (ver `database::schema`):
    /// con el rollback journal clásico, la primera escritura de cualquiera
    /// de las dos conexiones bloquearía a la otra igual que si compartieran
    /// el mismo candado. Reusa la fábrica central de escritura (mismos
    /// pragmas que `GuiState::conexion_secundaria` en escritorio: antes
    /// esta sólo aplicaba `busy_timeout`/`foreign_keys`, le faltaban
    /// `synchronous`/`trusted_schema`/`secure_delete`). `clave` en `None`
    /// -- Android todavía no aplica ninguna clave de SQLCipher a la base
    /// (pendiente aparte, ver `docs/auditorias/AUDITORIA_INTEGRAL_ANDROID_2026-09-09.md`);
    /// el día que se active acá, pasa a la vez por la conexión principal y
    /// por ésta.
    fn conexion_secundaria(&self) -> Result<rusqlite::Connection, NucleoError> {
        control_acceso::database::connection::abrir_conexion_secundaria_escritura(
            &self.ruta_base_datos,
            None,
        )
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })
    }

    fn sesion_lock(&self) -> std::sync::MutexGuard<'_, Option<UsuarioSesionNucleo>> {
        self.sesion
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn actor_autenticado(&self) -> Result<UsuarioSesionNucleo, NucleoError> {
        self.sesion_lock().clone().ok_or(NucleoError::NoAutenticado)
    }

    fn sincronizar_con_secreto(&self, secreto: &str) -> Result<ResumenSincronizacion, NucleoError> {
        let _sincronizacion = self
            .sincronizacion_en_curso
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;

        let token = self
            .autenticar_con_cache(secreto)
            .map_err(|error| NucleoError::Interno {
                mensaje: error.to_string(),
            })?;
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.core_lock().actualizar_desfase_reloj(desfase_ms);
        }

        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::BASE_URL,
            apikey: control_acceso::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        let mapear = |error: control_acceso::nube::SincronizacionError| NucleoError::Interno {
            mensaje: error.to_string(),
        };
        let resumen_cola =
            control_acceso::nube::drenar_cola(&conexion, &contexto, 200).map_err(mapear)?;
        let cierres_recibidos =
            control_acceso::nube::recibir_cierres_de_ingresos_propios(&conexion, &contexto)
                .map_err(mapear)?;
        let remotos = control_acceso::nube::recibir_ingresos_abiertos(&conexion, &contexto)
            .map_err(mapear)?;
        let catalogo = control_acceso::nube::recibir_catalogo_del_sitio(&conexion, &contexto)
            .map_err(mapear)?;
        let movimientos_historial_recibidos =
            control_acceso::nube::recibir_historial_del_sitio(&conexion, &contexto)
                .map_err(mapear)?;
        let citas_recibidas = control_acceso::nube::recibir_citas_del_sitio(&conexion, &contexto)
            .map_err(mapear)?;
        // Ver el comentario del otro método de sync en este mismo archivo:
        // el celular no trae historial de visitas a propósito.
        let historial_visitas_recibidos = 0;
        // Mejor esfuerzo -- ya se llegó hasta acá con la nube respondiendo
        // bien, pero si este chequeo puntual falla no tiene sentido tumbar
        // un sync que por lo demás anduvo.
        let conflictos_ingreso =
            control_acceso::nube::contratistas_con_conflicto_activo(&conexion, &contexto)
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect();

        let sesion_expulsada = !self.core_lock().sesion_sigue_activa(&actor);
        if sesion_expulsada {
            *self.sesion_lock() = None;
        }

        Ok(ResumenSincronizacion {
            enviados: resumen_cola.enviados,
            fallidos: resumen_cola.fallidos,
            remotos_abiertos: u32::try_from(remotos.len()).unwrap_or(u32::MAX),
            cierres_recibidos,
            empresas_recibidas: catalogo.empresas_recibidas,
            contratistas_recibidos: catalogo.contratistas_recibidos,
            gafetes_recibidos: catalogo.gafetes_recibidos,
            movimientos_historial_recibidos,
            citas_recibidas,
            historial_visitas_recibidos,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            sesion_expulsada,
            conflictos_ingreso,
        })
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

        let resultado = nucleo.autenticar(
            "000000000".to_string(),
            "loquesea".to_string(),
            String::new(),
            String::new(),
        );

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
                     '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                     'ROOT', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar(
                "999999999".to_string(),
                "clave_prueba_123".to_string(),
                String::new(),
                String::new(),
            )
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
                     '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                     'ROOT', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar(
                "999999999".to_string(),
                "clave_prueba_123".to_string(),
                String::new(),
                String::new(),
            )
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
                     '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                     'ROOT', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar(
                "999999999".to_string(),
                "clave_prueba_123".to_string(),
                String::new(),
                String::new(),
            )
            .unwrap();
        nucleo
            .registrar_ingreso(1, MedioIngreso::Caminando, Some(7))
            .unwrap();
        // Cédula "222222222" no contiene un 7, así que si el modo Gafete
        // filtrara mal (o cayera al modo texto) esto no debería confundirse
        // con el otro contratista de todas formas — el segundo ingreso
        // (sin gafete) es el control negativo de esta prueba.
        nucleo
            .registrar_ingreso(2, MedioIngreso::Caminando, None)
            .unwrap();

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
                     '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                     'ROOT', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar(
                "999999999".to_string(),
                "clave_prueba_123".to_string(),
                String::new(),
                String::new(),
            )
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
                     '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                     'ROOT', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar(
                "999999999".to_string(),
                "clave_prueba_123".to_string(),
                String::new(),
                String::new(),
            )
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
                     '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                     'ROOT', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar(
                "999999999".to_string(),
                "clave_prueba_123".to_string(),
                String::new(),
                String::new(),
            )
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
                     '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                     'ROOT', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar(
                "999999999".to_string(),
                "clave_prueba_123".to_string(),
                String::new(),
                String::new(),
            )
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
                     '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                     'ROOT', 1
                 );
                 INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
                     '888888888', 'Operador Test',
                     '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
                     'OPERADOR', 1
                 );",
            )
            .unwrap();
        drop(conexion);

        let nucleo = Nucleo::abrir(ruta).unwrap();
        nucleo
            .autenticar(
                "999999999".to_string(),
                "clave_prueba_123".to_string(),
                String::new(),
                String::new(),
            )
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
            .autenticar(
                "888888888".to_string(),
                "clave_prueba_123".to_string(),
                String::new(),
                String::new(),
            )
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

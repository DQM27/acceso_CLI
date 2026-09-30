//! Tipos que cruzan la frontera FFI hacia Kotlin y su conversión desde el núcleo.

use control_acceso::application::ResumenSincronizacion as ResumenSincronizacionNucleo;
use control_acceso::database::queries::contratistas::ContratistaResumen as ContratistaResumenNucleo;
use control_acceso::database::queries::usuarios::UsuarioResumen as UsuarioResumenNucleo;
use control_acceso::domain::resultado_acceso::{
    MotivoDenegacion as MotivoDenegacionNucleo, ResultadoAcceso as ResultadoAccesoNucleo,
};
use control_acceso::domain::resultado_salida_ruta::ResultadoSalidaRuta as ResultadoSalidaRutaNucleo;
use control_acceso::models::empresa::Empresa as EmpresaNucleo;
use control_acceso::models::empresa_proveedor::EmpresaProveedor as EmpresaProveedorNucleo;
use control_acceso::models::encargado_ruta::EncargadoRuta as EncargadoRutaNucleo;
use control_acceso::models::medio_ingreso::MedioIngreso as MedioIngresoNucleo;
use control_acceso::models::prestamo_gafete_provisional::PrestamoGafeteProvisionalActivoResumen as PrestamoGafeteProvisionalActivoResumenNucleo;
use control_acceso::models::registro_ingreso::{
    MotivoResultadoIngreso as MotivoResultadoIngresoNucleo,
    ResultadoIngresoRegistrado as ResultadoIngresoRegistradoNucleo,
};
use control_acceso::models::registro_ingreso_proveedor::RegistroIngresoProveedorActivoResumen as RegistroIngresoProveedorActivoResumenNucleo;
use control_acceso::models::ruta::Ruta as RutaNucleo;
use control_acceso::models::salida_ruta::SalidaRutaActivaResumen as SalidaRutaActivaResumenNucleo;
use control_acceso::models::tipo_ingreso::TipoIngreso as TipoIngresoNucleo;
use control_acceso::models::usuario::RolUsuario as RolUsuarioNucleo;
use control_acceso::models::vehiculo_ruta::VehiculoRuta as VehiculoRutaNucleo;
use control_acceso::nube::IngresoProveedorRemoto as IngresoProveedorRemotoNucleo;
use control_acceso::nube::IngresoRemoto as IngresoRemotoNucleo;
use control_acceso::nube::PrestamoGafeteProvisionalRemoto as PrestamoGafeteProvisionalRemotoNucleo;
use control_acceso::services::autenticacion_service::UsuarioSesion as UsuarioSesionNucleo;
use control_acceso::services::registro_ingreso_service::{
    IngresoActivoResumen as IngresoActivoResumenNucleo,
    PreparacionIngreso as PreparacionIngresoNucleo,
    ResultadoRegistroEntrada as ResultadoRegistroEntradaNucleo,
};
use control_acceso::services::ruta_service::ResultadoRegistroSalidaRuta as ResultadoRegistroSalidaRutaNucleo;

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

/// Éxito de `Nucleo::autenticar`/`autenticar` -- mismo motivo
/// que `desktop/src-tauri/src/comandos/autenticacion.rs::ResultadoLogin`:
/// Kotlin necesita saber si tiene que forzar el cambio de contraseña antes
/// de dejar operar. `false` siempre en la rama local (ROOT del arranque
/// inicial, o cualquier cuenta que ya tenía password local de antes de esta
/// migración) -- esa contraseña ya es la real, no una temporal de un solo
/// uso. Ver docs/planes-implementados/plan-autenticacion-supabase-auth.md.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ResultadoLogin {
    pub sesion: UsuarioSesion,
    pub debe_cambiar_password: bool,
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
    /// "ACCESO DENEGADO" / "PRAIND VENCIDO", resuelto por el núcleo con su
    /// reloj; Kotlin sólo lo muestra.
    pub aviso_acceso: Option<String>,
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
            aviso_acceso: resumen.aviso_acceso,
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
    /// `None` para SWAT (nunca vence). Viaja como texto ISO
    /// (`AAAA-MM-DD`), mismo criterio que `DatosContratista` en este mismo
    /// archivo.
    pub fecha_vencimiento_praind: Option<String>,
    pub resultado_acceso: ResultadoAcceso,
    pub requiere_gafete: bool,
    pub tiene_ingreso_activo: bool,
    /// Siempre `None`: el ingreso activo en otro sitio llega resuelto en
    /// `mensaje_bloqueo` (ver `preparar_ingreso_verificado`).
    pub activo_en_otro_sitio: Option<String>,
    pub gafetes_deuda: Vec<i64>,
    /// `None` si se puede continuar con este contratista; si no, el texto
    /// ya resuelto del motivo (ingreso activo local, activo en otro sitio,
    /// o acceso denegado, en ese orden de prioridad). Reemplaza
    /// `puedeContinuar`/`mensajeBloqueo`/`mensajeMotivoDenegacion`, que
    /// antes vivían duplicados en Kotlin (con su propio orden y su propio
    /// texto, ya divergido del de escritorio) -- ver
    /// `PreparacionIngreso::bloqueo` y `mensajes::mensaje_bloqueo_ingreso`
    /// en el crate raíz. Kotlin sólo debe mirar este campo: `!= null`
    /// significa bloqueado, y es el texto a mostrar tal cual.
    pub mensaje_bloqueo: Option<String>,
    /// Con `PermitidoConAdvertencia`, el aviso listo para mostrar ("PRAIND
    /// vence en 3 días (15-09-2026)"), con el reloj del núcleo. Reemplaza
    /// `mensajeVencimientoPraind` de Kotlin.
    pub aviso_praind: Option<String>,
}

impl From<PreparacionIngresoNucleo> for PreparacionIngreso {
    fn from(preparacion: PreparacionIngresoNucleo) -> Self {
        // Antes de mover el resto de los campos -- `bloqueo()` sólo lee,
        // no consume.
        let mensaje_bloqueo = preparacion
            .bloqueo()
            .as_ref()
            .map(control_acceso::mensajes::mensaje_bloqueo_ingreso);
        Self {
            contratista_id: preparacion.contratista_id,
            cedula: preparacion.cedula,
            nombre: preparacion.nombre,
            empresa_nombre: preparacion.empresa_nombre,
            tipo_ingreso: preparacion.tipo_ingreso.into(),
            fecha_vencimiento_praind: preparacion.fecha_vencimiento_praind.map(|f| f.to_string()),
            resultado_acceso: preparacion.resultado_acceso.into(),
            requiere_gafete: preparacion.requiere_gafete,
            tiene_ingreso_activo: preparacion.tiene_ingreso_activo,
            activo_en_otro_sitio: preparacion.activo_en_otro_sitio,
            gafetes_deuda: preparacion.gafetes_deuda,
            mensaje_bloqueo,
            aviso_praind: preparacion.aviso_praind,
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
    /// Placa del vehículo -- `Some` sólo cuando `medio_ingreso` es
    /// `Vehiculo`; `None` en datos viejos pre-migración aunque el medio sea
    /// `Vehiculo` (ver el doc-comment del mismo campo en el núcleo,
    /// `services::registro_ingreso_service::IngresoActivoResumen`).
    pub placa: Option<String>,
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
            placa: activo.placa,
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

/// Espejo de `EmpresaProveedor` -- catálogo separado de `Empresa` a
/// propósito (`docs/features-futuras/plan-control-proveedores.md`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EmpresaProveedor {
    pub id: i64,
    pub nombre: String,
    pub activo: bool,
}

impl From<EmpresaProveedorNucleo> for EmpresaProveedor {
    fn from(empresa: EmpresaProveedorNucleo) -> Self {
        Self {
            id: empresa.id,
            nombre: empresa.nombre,
            activo: empresa.activo,
        }
    }
}

/// Espejo de `DatosContratista` — sólo alta en persona, no edición (ver
/// docs/plan-app-movil.md). `fecha_vencimiento_praind` viaja como texto
/// ISO (`AAAA-MM-DD`); si no parsea se rechaza como `FechaInvalida`. Sin
/// `tiene_acceso`: el alta en persona siempre queda con acceso y eso lo
/// decide el núcleo (`ContratistaService::crear`), no Kotlin.
#[derive(Debug, Clone, uniffi::Record)]
pub struct DatosContratista {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<String>,
    pub es_personal_ruta: bool,
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

/// Ver `docs/planes-implementados/plan-persistencia-nube.md` y `ResumenSincronizacionNucleo`.
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
    /// `sincronizar` lo completa de verdad.
    pub conflictos_ingreso: Vec<ConflictoIngresoActivo>,
    /// Mismo criterio que `conflictos_ingreso`, pero para ingresos de
    /// proveedor -- ver `control_acceso::nube::proveedores_con_conflicto_activo`.
    pub conflictos_ingreso_proveedor: Vec<ConflictoIngresoProveedorActivo>,
    /// Ingresos con gafete que ESTE dispositivo registró, pero cuyo envío a
    /// la nube fue rechazado porque otro dispositivo del mismo sitio ya
    /// tiene ese número activo (índice único
    /// `ingresos_gafete_activo_sitio_idx`) -- a diferencia de
    /// `conflictos_ingreso`, se calcula con datos locales dentro del mismo
    /// `drenar_cola`, sin una consulta remota aparte -- ver
    /// `control_acceso::nube::ConflictoGafeteActivo`.
    pub conflictos_gafete: Vec<ConflictoGafeteActivo>,
}

/// Espejo de `control_acceso::nube::ConflictoGafeteActivo`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConflictoGafeteActivo {
    pub tipo: TipoMovimientoGafete,
    /// Contratista, proveedor o encargado de ruta (KOF).
    pub nombre: String,
    pub gafete_numero: i64,
    /// Hora de entrada, o de entrega en KOF.
    pub fecha_hora: String,
}

/// Espejo de `control_acceso::nube::TipoMovimientoGafete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TipoMovimientoGafete {
    Contratista,
    Proveedor,
    ProvisionalKof,
}

impl From<control_acceso::nube::TipoMovimientoGafete> for TipoMovimientoGafete {
    fn from(tipo: control_acceso::nube::TipoMovimientoGafete) -> Self {
        use control_acceso::nube::TipoMovimientoGafete as Nucleo;
        match tipo {
            Nucleo::Contratista => Self::Contratista,
            Nucleo::Proveedor => Self::Proveedor,
            Nucleo::ProvisionalKof => Self::ProvisionalKof,
        }
    }
}

impl From<control_acceso::nube::ConflictoGafeteActivo> for ConflictoGafeteActivo {
    fn from(conflicto: control_acceso::nube::ConflictoGafeteActivo) -> Self {
        Self {
            tipo: conflicto.tipo.into(),
            nombre: conflicto.nombre,
            gafete_numero: conflicto.gafete_numero,
            fecha_hora: conflicto.fecha_hora,
        }
    }
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

/// Espejo de `control_acceso::nube::ConflictoIngresoProveedorActivo`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConflictoIngresoProveedorActivo {
    pub cedula: String,
    pub nombre: String,
    pub sitio_conflicto: String,
}

impl From<control_acceso::nube::ConflictoIngresoProveedorActivo>
    for ConflictoIngresoProveedorActivo
{
    fn from(conflicto: control_acceso::nube::ConflictoIngresoProveedorActivo) -> Self {
        Self {
            cedula: conflicto.cedula,
            nombre: conflicto.nombre,
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
            conflictos_ingreso_proveedor: Vec::new(),
            conflictos_gafete: Vec::new(),
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

/// Espejo de [`IngresoRemoto`], pero para el ciclo de proveedores -- ver
/// `IngresoProveedorRemotoNucleo`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct IngresoProveedorRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: String,
}

impl From<IngresoProveedorRemotoNucleo> for IngresoProveedorRemoto {
    fn from(remoto: IngresoProveedorRemotoNucleo) -> Self {
        Self {
            uuid: remoto.uuid,
            cedula: remoto.cedula,
            nombre: remoto.nombre,
            empresa_nombre: remoto.empresa_nombre,
            placa: remoto.placa,
            gafete_numero: remoto.gafete_numero,
            hora_entrada: remoto.hora_entrada,
            usuario_entrada_nombre: remoto.usuario_entrada_nombre,
        }
    }
}

/// Espejo de [`IngresoProveedorRemoto`], pero para el ciclo de
/// entrega/devolución de gafetes provisionales KOF -- ver
/// `PrestamoGafeteProvisionalRemotoNucleo`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PrestamoGafeteProvisionalRemoto {
    pub uuid: String,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: String,
    pub gafete_numero: i64,
    pub hora_entrega: String,
    pub usuario_entrega_nombre: String,
}

impl From<PrestamoGafeteProvisionalRemotoNucleo> for PrestamoGafeteProvisionalRemoto {
    fn from(remoto: PrestamoGafeteProvisionalRemotoNucleo) -> Self {
        Self {
            uuid: remoto.uuid,
            encargado_nombre: remoto.encargado_nombre,
            encargado_codigo_empleado: remoto.encargado_codigo_empleado,
            gafete_numero: remoto.gafete_numero,
            hora_entrega: remoto.hora_entrega,
            usuario_entrega_nombre: remoto.usuario_entrega_nombre,
        }
    }
}

/// Espejo de `domain::resultado_salida_ruta::ResultadoSalidaRuta` --
/// `Permitido`/`PermitidoConAutorizacion` según si el documento de carga
/// es de hoy (ver `RutaService::registrar_salida`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ResultadoSalidaRuta {
    Permitido,
    PermitidoConAutorizacion,
}

impl From<ResultadoSalidaRutaNucleo> for ResultadoSalidaRuta {
    fn from(resultado: ResultadoSalidaRutaNucleo) -> Self {
        match resultado {
            ResultadoSalidaRutaNucleo::Permitido => Self::Permitido,
            ResultadoSalidaRutaNucleo::PermitidoConAutorizacion => Self::PermitidoConAutorizacion,
        }
    }
}

/// Espejo de `EncargadoRuta` -- sin `cedula` a propósito: el catálogo KOF
/// nunca la trae (pedido explícito del usuario, ver el modelo real) y el
/// checklist mobile no la necesita para nada, sólo confirma nombre +
/// código de empleado.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EncargadoRuta {
    pub id: i64,
    pub codigo_empleado: String,
    pub nombre: String,
    pub activo: bool,
}

impl From<EncargadoRutaNucleo> for EncargadoRuta {
    fn from(encargado: EncargadoRutaNucleo) -> Self {
        Self {
            id: encargado.id,
            codigo_empleado: encargado.codigo_empleado,
            nombre: encargado.nombre,
            activo: encargado.activo,
        }
    }
}

/// Espejo de `VehiculoRuta` -- `numero_unidad` sigue `Option<String>`
/// (`None` para vehículos de apoyo/particulares, sólo tienen placa, ver el
/// modelo real).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VehiculoRuta {
    pub id: i64,
    pub numero_unidad: Option<String>,
    pub placa: String,
    pub activo: bool,
}

impl From<VehiculoRutaNucleo> for VehiculoRuta {
    fn from(vehiculo: VehiculoRutaNucleo) -> Self {
        Self {
            id: vehiculo.id,
            numero_unidad: vehiculo.numero_unidad,
            placa: vehiculo.placa,
            activo: vehiculo.activo,
        }
    }
}

/// Espejo de `Ruta` (catálogo de números válidos) -- el checklist mobile
/// sólo lo consume vía `Nucleo::buscar_rutas` para confirmar el número
/// leído por OCR contra el catálogo, nunca lo administra (alta/baja/rango
/// quedan exclusivas de escritorio, ver `plan-control-rutas.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct Ruta {
    pub id: i64,
    pub numero: i64,
    pub activo: bool,
}

impl From<RutaNucleo> for Ruta {
    fn from(ruta: RutaNucleo) -> Self {
        Self {
            id: ruta.id,
            numero: ruta.numero,
            activo: ruta.activo,
        }
    }
}

/// Espejo de `SolicitudSalidaRuta` -- sin `usuario_salida_id`/
/// `fecha_hora_salida` (el núcleo los pisa siempre con el actor/reloj
/// reales, igual que `AppCore::registrar_salida_ruta`/`desktop/src-tauri/src/dto/rutas.rs`).
/// `fecha_documento` viaja como texto ISO (`AAAA-MM-DD`), mismo criterio
/// que `fecha_vencimiento_praind` en `DatosContratista`.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SolicitudSalidaRuta {
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: Option<String>,
    pub numero_ruta: i64,
    pub sub_numero: i64,
    pub numero_documento: String,
    pub fecha_documento: String,
    pub tiene_correo_autorizacion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ResultadoRegistroSalidaRuta {
    pub salida_id: i64,
    pub resultado: ResultadoSalidaRuta,
}

impl From<ResultadoRegistroSalidaRutaNucleo> for ResultadoRegistroSalidaRuta {
    fn from(resultado: ResultadoRegistroSalidaRutaNucleo) -> Self {
        Self {
            salida_id: resultado.salida_id,
            resultado: resultado.resultado.into(),
        }
    }
}

/// Espejo de `SalidaRutaActivaResumen` -- fila de "rutas activas" (salidas
/// sin retorno todavía), análoga a `IngresoActivoResumen`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SalidaRutaActivaResumen {
    pub id: i64,
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_nombre: String,
    pub numero_ruta: i64,
    pub sub_numero: i64,
    pub numero_documento: String,
    pub fecha_documento: String,
    pub resultado: ResultadoSalidaRuta,
    pub fecha_hora_salida: String,
    pub usuario_salida_nombre: String,
}

impl From<SalidaRutaActivaResumenNucleo> for SalidaRutaActivaResumen {
    fn from(activa: SalidaRutaActivaResumenNucleo) -> Self {
        Self {
            id: activa.id,
            vehiculo_placa: activa.vehiculo_placa,
            vehiculo_numero_unidad: activa.vehiculo_numero_unidad,
            encargado_nombre: activa.encargado_nombre,
            numero_ruta: activa.numero_ruta,
            sub_numero: activa.sub_numero,
            numero_documento: activa.numero_documento,
            fecha_documento: activa.fecha_documento.to_string(),
            resultado: activa.resultado.into(),
            fecha_hora_salida: activa.fecha_hora_salida.to_rfc3339(),
            usuario_salida_nombre: activa.usuario_salida_nombre,
        }
    }
}

/// Espejo de `PrestamoGafeteProvisionalActivoResumen` -- fila de "préstamos
/// activos" (gafetes provisionales KOF entregados sin devolver todavía).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PrestamoGafeteProvisionalActivoResumen {
    pub id: i64,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: String,
    pub gafete_numero: i64,
    pub fecha_hora_entrega: String,
    pub usuario_entrega_nombre: String,
}

impl From<PrestamoGafeteProvisionalActivoResumenNucleo> for PrestamoGafeteProvisionalActivoResumen {
    fn from(activo: PrestamoGafeteProvisionalActivoResumenNucleo) -> Self {
        Self {
            id: activo.id,
            encargado_nombre: activo.encargado_nombre,
            encargado_codigo_empleado: activo.encargado_codigo_empleado,
            gafete_numero: activo.gafete_numero,
            fecha_hora_entrega: activo.fecha_hora_entrega.to_rfc3339(),
            usuario_entrega_nombre: activo.usuario_entrega_nombre,
        }
    }
}

/// Espejo de `RegistroIngresoProveedorActivoResumen` -- fila de "proveedores
/// activos" (ingresos de proveedor sin salida todavía).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RegistroIngresoProveedorActivoResumen {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub fecha_hora_ingreso: String,
    pub usuario_ingreso_nombre: String,
}

impl From<RegistroIngresoProveedorActivoResumenNucleo> for RegistroIngresoProveedorActivoResumen {
    fn from(activo: RegistroIngresoProveedorActivoResumenNucleo) -> Self {
        Self {
            id: activo.id,
            cedula: activo.cedula,
            nombre: activo.nombre,
            empresa_nombre: activo.empresa_nombre,
            placa: activo.placa,
            gafete_numero: activo.gafete_numero,
            fecha_hora_ingreso: activo.fecha_hora_ingreso.to_rfc3339(),
            usuario_ingreso_nombre: activo.usuario_ingreso_nombre,
        }
    }
}

//! Operaciones que, además de sus reglas locales, consultan en vivo a la
//! nube antes de escribir (el otro dispositivo del sitio u otros sitios).
//! El orden de los chequeos y qué falla frena viven sólo acá; escritorio
//! (Tauri) y móvil (`mobile/rust-core`) llaman a estas funciones tal cual.
//!
//! **La portería no se bloquea por falta de red.** Si la nube no se puede
//! alcanzar (sin conexión, sin respuesta a tiempo o caída; ver
//! `GestionNubeError::es_falta_de_conexion`), el registro sigue con las
//! reglas locales y queda en el registro técnico. Lo que la verificación en
//! vivo no pudo ver lo atrapan los índices únicos de Supabase al
//! sincronizar: la fila que choca queda fallida y quien opera recibe el
//! aviso (`ResumenSincronizacion::conflictos_*`). Lo que sí frena, igual
//! que antes, es que la nube **responda** que no: la persona ya está
//! adentro, el gafete ya está en uso, o la nube rechazó la consulta
//! (dispositivo retirado, sesión sin permiso). Ver
//! `docs/decisiones-tecnicas.md`, entrada 2026-10-13.
//!
//! Reciben una función que toma el candado del núcleo en vez de `&AppCore`:
//! el candado se toma sólo para lo local (`SQLite`) y se suelta antes de
//! cada llamada de red, para no colgar al resto de la app mientras tanto
//! (ver el doc-comment de `nube::cache_token`).

use std::ops::Deref;

use super::{AppCore, GestionNubeError};
use crate::models::medio_ingreso::MedioIngreso;
use crate::nube::{
    CacheTokenDispositivo, ContextoSincronizacion, SincronizacionError, TokenDispositivo,
};
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{
    CitaServiceError, GafeteProvisionalServiceError, IngresoCorreoServiceError,
    IngresoProveedorServiceError, RegistroIngresoServiceError,
};
use crate::services::registro_ingreso_service::{
    BloqueoIngreso, PreparacionIngreso, ResultadoRegistroEntrada,
};

/// Token vigente para consultar la nube. Autoriza al actor con el candado
/// tomado (es `SQLite`, microsegundos) y lo suelta antes de autenticar.
///
/// Quien llama ya comprobó `nube.vinculado()`: un equipo sin vincular no
/// tiene con quién chocar y ningún chequeo remoto toca la red.
fn autenticar<G: Deref<Target = AppCore>>(
    nucleo: &impl Fn() -> G,
    nube: &CacheTokenDispositivo,
    actor: &UsuarioSesion,
) -> Result<TokenDispositivo, GestionNubeError> {
    nucleo().autorizar_uso_nube(actor)?;
    let token = nube.autenticar_con_cache()?;
    nucleo().aplicar_token(&token);
    Ok(token)
}

/// Token para verificar en la nube antes de registrar. `Ok(None)` si la
/// nube no se pudo alcanzar: no hay contra qué verificar y se registra con
/// las reglas locales (ver el doc-comment del módulo). Cualquier otra falla
/// (sesión sin permiso, dispositivo retirado) se devuelve tal cual.
fn token_para_verificar<G: Deref<Target = AppCore>>(
    nucleo: &impl Fn() -> G,
    nube: &CacheTokenDispositivo,
    actor: &UsuarioSesion,
    operacion: &str,
) -> Result<Option<TokenDispositivo>, GestionNubeError> {
    match autenticar(nucleo, nube, actor) {
        Ok(token) => Ok(Some(token)),
        Err(error) if error.es_falta_de_conexion() => {
            log::warn!(
                "{operacion}: sin conexión con la nube, se registra con las reglas locales: {error}"
            );
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

/// Resultado de una consulta de verificación con la misma regla: si la nube
/// no se pudo alcanzar, cuenta como `libre` (el valor que no frena) y queda
/// en el registro técnico; cualquier otro error se devuelve, porque es la
/// nube rechazando la consulta y eso no se arregla registrando igual.
fn o_libre_sin_conexion<T>(
    resultado: Result<T, SincronizacionError>,
    libre: T,
    consulta: &str,
) -> Result<T, SincronizacionError> {
    match resultado {
        Err(error) if error.es_falta_de_conexion() => {
            log::warn!(
                "{consulta}: sin conexión con la nube, se registra con las reglas locales: {error}"
            );
            Ok(libre)
        }
        otro => otro,
    }
}

fn contexto(token: &TokenDispositivo) -> ContextoSincronizacion<'_> {
    ContextoSincronizacion {
        base_url: crate::nube::base_url(),
        apikey: crate::nube::apikey(),
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    }
}

/// Espera un hilo de una consulta paralela y relanza su pánico si lo hubo
/// (genérico: cada consulta devuelve un tipo distinto).
fn unir_hilo<T>(hilo: std::thread::ScopedJoinHandle<'_, T>) -> T {
    hilo.join()
        .unwrap_or_else(|panico| std::panic::resume_unwind(panico))
}

// ---- Ingreso de contratista ----

/// Regla: un contratista no puede tener dos ingresos activos, ni en este
/// sitio (este equipo o el otro) ni en otro sitio. Busca en la nube un
/// ingreso abierto con esa cédula en cualquier sitio. `None` = libre; sin
/// nube configurada no hay con quién chocar (sólo cuentan los chequeos
/// locales). Sin conexión con la nube también es `None` (ver el doc-comment
/// del módulo); si la nube rechaza la consulta, `SinVerificarEnLaNube`.
///
/// Además, la persona no puede estar adentro por otra vía (como proveedor o
/// por correo) en ninguna unidad (`persona_adentro_por_otra_via`, a la vez
/// que la consulta principal). Ese caso se muestra con el bloqueo de "otro
/// sitio" y la vía al lado del nombre ("Brisas (como proveedor)"), sin una
/// variante nueva de `BloqueoIngreso`, que viaja a las dos apps.
fn bloqueo_en_la_nube(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
) -> Option<BloqueoIngreso> {
    let (misma_via, otra_via) = std::thread::scope(|hilos| {
        let consulta_otra_via = hilos.spawn(|| {
            crate::nube::persona_adentro_por_otra_via(
                contexto,
                cedula,
                crate::models::via_ingreso::ViaIngreso::Contratista,
            )
        });
        let misma_via = crate::nube::contratista_con_ingreso_activo(contexto, cedula);
        (misma_via, unir_hilo(consulta_otra_via))
    });
    match o_libre_sin_conexion(misma_via, None, "ingreso") {
        Ok(None) => {}
        Ok(Some(activo)) if activo.mismo_sitio => {
            return Some(BloqueoIngreso::IngresoActivoEnOtroDispositivo);
        }
        Ok(Some(activo)) => {
            return Some(BloqueoIngreso::ActivoEnOtroSitio {
                sitio: activo.sitio_nombre,
            });
        }
        Err(error) => {
            log::warn!("ingreso: la nube rechazó la verificación: {error}");
            return Some(BloqueoIngreso::SinVerificarEnLaNube);
        }
    }
    match o_libre_sin_conexion(otra_via, None, "ingreso (otra vía)") {
        Ok(None) => None,
        Ok(Some(adentro)) => Some(BloqueoIngreso::ActivoEnOtroSitio {
            sitio: format!("{} ({})", adentro.sitio_nombre, adentro.via.texto()),
        }),
        Err(error) => {
            log::warn!("ingreso: la nube rechazó la verificación de la otra vía: {error}");
            Some(BloqueoIngreso::SinVerificarEnLaNube)
        }
    }
}

/// Vista previa al elegir a la persona: reglas locales y, si pasan, la
/// verificación en la nube. El bloqueo queda en la preparación (ver
/// `bloqueo_en_la_nube`); las pantallas sólo lo muestran.
pub fn preparar_ingreso_verificado<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: &CacheTokenDispositivo,
    actor: Option<&UsuarioSesion>,
    contratista_id: i64,
) -> Result<(PreparacionIngreso, Option<BloqueoIngreso>), RegistroIngresoServiceError> {
    let preparacion = nucleo().preparar_ingreso(contratista_id)?;
    let local = preparacion.bloqueo();
    if local.is_some() {
        return Ok((preparacion, local));
    }
    // Adentro por otra vía en este equipo o en el otro de la unidad: se
    // avisa ya en la vista previa (la escritura también lo rechaza, con
    // `RegistroIngresoServiceError::AdentroPorOtraVia`).
    if let Some(via) = crate::database::queries::persona_adentro::adentro_por_otra_via(
        &nucleo().connection,
        &preparacion.cedula,
        crate::models::via_ingreso::ViaIngreso::Contratista,
    )? {
        let bloqueo = BloqueoIngreso::ActivoEnOtroSitio {
            sitio: format!("esta unidad ({})", via.texto()),
        };
        return Ok((preparacion, Some(bloqueo)));
    }
    if !nube.vinculado() {
        return Ok((preparacion, None));
    }
    // Sin sesión no hay con qué autorizar la consulta: con nube
    // configurada eso cuenta como "no se pudo verificar".
    let bloqueo = match actor.map(|actor| token_para_verificar(&nucleo, nube, actor, "ingreso")) {
        Some(Ok(Some(token))) => bloqueo_en_la_nube(&contexto(&token), &preparacion.cedula),
        Some(Ok(None)) => None,
        Some(Err(error)) => {
            log::warn!("ingreso: no se pudo autenticar para verificar: {error}");
            Some(BloqueoIngreso::SinVerificarEnLaNube)
        }
        None => Some(BloqueoIngreso::SinVerificarEnLaNube),
    };
    Ok((preparacion, bloqueo))
}

#[derive(Debug, thiserror::Error)]
pub enum IngresoVerificadoError {
    #[error(transparent)]
    Servicio(#[from] RegistroIngresoServiceError),
    #[error("ingreso bloqueado: {0:?}")]
    Bloqueado(BloqueoIngreso),
    #[error("el gafete {numero} ya está en uso en otro dispositivo del sitio")]
    GafeteOcupadoEnSitio { numero: i64 },
}

/// Ingreso de contratista con todas sus reglas, en este orden:
/// 1. reglas locales (este equipo y la caché del otro equipo del sitio);
/// 2. con nube configurada, en vivo: la persona no tiene un ingreso activo
///    en ningún sitio, y el gafete no está en uso en el otro dispositivo.
///    Sin conexión con la nube se sigue con lo local (ver el doc-comment
///    del módulo); si la nube rechaza la consulta, no se registra;
/// 3. escribe (`AppCore::registrar_ingreso`, que repite lo local dentro de
///    su transacción).
///
/// Si dos equipos pasan el paso 2 en el mismo instante, o uno registró sin
/// conexión, el índice único de Supabase decide: gana el primero en
/// escribir y el otro recibe el aviso al sincronizar.
#[allow(clippy::too_many_arguments)]
pub fn registrar_ingreso_verificado<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: &CacheTokenDispositivo,
    actor: &UsuarioSesion,
    contratista_id: i64,
    medio: MedioIngreso,
    gafete: Option<i64>,
    placa: Option<String>,
) -> Result<ResultadoRegistroEntrada, IngresoVerificadoError> {
    let preparacion = nucleo().preparar_ingreso(contratista_id)?;
    if let Some(bloqueo) = preparacion.bloqueo() {
        return Err(IngresoVerificadoError::Bloqueado(bloqueo));
    }

    if nube.vinculado() {
        let token = token_para_verificar(&nucleo, nube, actor, "ingreso").map_err(|error| {
            log::warn!("ingreso: no se pudo autenticar para verificar: {error}");
            IngresoVerificadoError::Bloqueado(BloqueoIngreso::SinVerificarEnLaNube)
        })?;
        if let Some(token) = token {
            verificar_ingreso_en_la_nube(&contexto(&token), &preparacion.cedula, gafete)?;
        }
    }

    Ok(nucleo().registrar_ingreso(actor, contratista_id, medio, gafete, placa)?)
}

/// Paso 2 de [`registrar_ingreso_verificado`].
fn verificar_ingreso_en_la_nube(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
    gafete: Option<i64>,
) -> Result<(), IngresoVerificadoError> {
    // Las dos consultas son independientes: a la vez, un solo viaje de
    // espera en vez de dos (telemetría de staging: ~80-100 ms menos al
    // confirmar). Se evalúan en el mismo orden de siempre, así el
    // mensaje que ve quien opera no cambia.
    let (bloqueo, gafete_ocupado) = std::thread::scope(|hilos| {
        let consulta_gafete = gafete.map(|numero| {
            hilos.spawn(move || crate::nube::gafete_ocupado_en_otro_dispositivo(contexto, numero))
        });
        let bloqueo = bloqueo_en_la_nube(contexto, cedula);
        (bloqueo, consulta_gafete.map(unir_hilo))
    });
    if let Some(bloqueo) = bloqueo {
        return Err(IngresoVerificadoError::Bloqueado(bloqueo));
    }
    if let (Some(numero), Some(ocupado)) = (gafete, gafete_ocupado) {
        let ocupado =
            o_libre_sin_conexion(ocupado, false, "ingreso (gafete)").map_err(|error| {
                log::warn!("ingreso: la nube rechazó la verificación del gafete: {error}");
                IngresoVerificadoError::Bloqueado(BloqueoIngreso::SinVerificarEnLaNube)
            })?;
        if ocupado {
            return Err(IngresoVerificadoError::GafeteOcupadoEnSitio { numero });
        }
    }
    Ok(())
}

// ---- Ingreso de proveedor ----

pub struct NuevoIngresoProveedor {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub placa: Option<String>,
    pub gafete_numero: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum IngresoProveedorVerificadoError {
    #[error(transparent)]
    Servicio(#[from] IngresoProveedorServiceError),
    #[error("la cédula ya tiene un ingreso de proveedor activo en {sitio}")]
    ActivoEnOtroSitio { sitio: String },
    #[error("el gafete {numero} ya está en uso en otro dispositivo del sitio")]
    GafeteOcupadoEnSitio { numero: i64 },
    #[error(transparent)]
    Nube(#[from] GestionNubeError),
}

/// Ingreso de proveedor con todas sus reglas, en este orden:
/// 1. la cédula no tiene otro ingreso abierto en este sitio, ni en este
///    equipo ni en el otro dispositivo
///    (`AppCore::proveedor_con_ingreso_activo_en_sitio`);
/// 2. con nube configurada, en vivo: la cédula no tiene un ingreso abierto
///    en ningún sitio (`ingreso_proveedor_activo`) y el gafete no está en uso
///    en el otro dispositivo del sitio. Sin conexión con la nube se sigue
///    con lo local (ver el doc-comment del módulo); si la nube rechaza la
///    consulta, no se registra;
/// 3. escribe (`AppCore::registrar_ingreso_proveedor`, reglas locales).
///
/// Si dos equipos pasan el paso 2 en el mismo instante, o uno registró sin
/// conexión, decide el índice único `ingresos_proveedor_cedula_activa_idx`:
/// gana el primero en escribir.
pub fn registrar_ingreso_proveedor_verificado<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: &CacheTokenDispositivo,
    actor: &UsuarioSesion,
    mut datos: NuevoIngresoProveedor,
) -> Result<i64, IngresoProveedorVerificadoError> {
    // Una sola forma de la cédula para las tres verificaciones y para lo
    // que se guarda (ver `domain::cedula`). Si no se puede normalizar, se
    // deja como vino: el registro local la rechaza con su propio mensaje.
    if let Ok(cedula) = crate::domain::cedula::Cedula::normalizar(&datos.cedula) {
        datos.cedula = cedula.into_string();
    }
    if nucleo().proveedor_con_ingreso_activo_en_sitio(&datos.cedula)? {
        return Err(IngresoProveedorServiceError::IngresoActivo.into());
    }

    if nube.vinculado()
        && let Some(token) = token_para_verificar(&nucleo, nube, actor, "ingreso de proveedor")?
    {
        verificar_proveedor_en_la_nube(&contexto(&token), &datos)?;
    }

    Ok(nucleo().registrar_ingreso_proveedor(
        actor,
        &datos.cedula,
        &datos.nombre,
        datos.empresa_id,
        datos.placa,
        datos.gafete_numero,
    )?)
}

/// Paso 2 de [`registrar_ingreso_proveedor_verificado`].
fn verificar_proveedor_en_la_nube(
    contexto: &ContextoSincronizacion<'_>,
    datos: &NuevoIngresoProveedor,
) -> Result<(), IngresoProveedorVerificadoError> {
    // A la vez, como en `verificar_ingreso_en_la_nube`; mismo orden al
    // evaluar.
    let (activo_en_otro_sitio, gafete_ocupado, otra_via) = std::thread::scope(|hilos| {
        let consulta_gafete = hilos.spawn(|| {
            crate::nube::gafete_de_proveedor_ocupado_en_otro_dispositivo(
                contexto,
                datos.gafete_numero,
            )
        });
        let consulta_otra_via = hilos.spawn(|| {
            crate::nube::persona_adentro_por_otra_via(
                contexto,
                &datos.cedula,
                crate::models::via_ingreso::ViaIngreso::Proveedor,
            )
        });
        let activo = crate::nube::proveedor_con_ingreso_activo(contexto, &datos.cedula);
        (
            activo,
            unir_hilo(consulta_gafete),
            unir_hilo(consulta_otra_via),
        )
    });
    match o_libre_sin_conexion(activo_en_otro_sitio, None, "ingreso de proveedor")
        .map_err(GestionNubeError::from)?
    {
        Some(activo) if activo.mismo_sitio => {
            return Err(IngresoProveedorServiceError::IngresoActivo.into());
        }
        Some(activo) => {
            return Err(IngresoProveedorVerificadoError::ActivoEnOtroSitio {
                sitio: activo.sitio_nombre,
            });
        }
        None => {}
    }
    // Una persona no puede estar adentro por dos vías (como contratista o
    // por correo, en cualquier unidad).
    if let Some(adentro) = o_libre_sin_conexion(otra_via, None, "ingreso de proveedor (otra vía)")
        .map_err(GestionNubeError::from)?
    {
        return Err(IngresoProveedorServiceError::AdentroPorOtraVia(adentro.via).into());
    }
    if o_libre_sin_conexion(gafete_ocupado, false, "ingreso de proveedor (gafete)")
        .map_err(GestionNubeError::from)?
    {
        return Err(IngresoProveedorVerificadoError::GafeteOcupadoEnSitio {
            numero: datos.gafete_numero,
        });
    }
    Ok(())
}

// ---- Ingreso por correo ----

pub struct NuevoIngresoCorreo {
    pub cedula: String,
    pub nombre: String,
    pub motivo: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum IngresoCorreoVerificadoError {
    #[error(transparent)]
    Servicio(#[from] IngresoCorreoServiceError),
    #[error("la cédula ya tiene un ingreso por correo activo en {sitio}")]
    ActivoEnOtroSitio { sitio: String },
    #[error("el gafete de visita {numero} ya está en uso en otro dispositivo del sitio")]
    GafeteOcupadoEnSitio { numero: i64 },
    #[error(transparent)]
    Nube(#[from] GestionNubeError),
}

/// Ingreso por correo con todas sus reglas -- mismo orden que
/// [`registrar_ingreso_proveedor_verificado`]:
/// 1. la cédula no tiene otro ingreso por correo abierto en este sitio, ni
///    en este equipo ni en el otro dispositivo
///    (`AppCore::correo_con_ingreso_activo_en_sitio`);
/// 2. con nube configurada, en vivo y a la vez: la cédula no está adentro
///    por correo en ninguna unidad (`ingreso_correo_activo`), y el gafete de
///    visita no está en uso en el otro dispositivo del sitio, ni por otro
///    ingreso por correo ni por un movimiento de visita. Sin conexión con la
///    nube se sigue con lo local (ver el doc-comment del módulo); si la
///    nube rechaza la consulta, no se registra;
/// 3. escribe (`AppCore::registrar_ingreso_correo`, reglas locales).
///
/// Si dos equipos pasan el paso 2 en el mismo instante, o uno registró sin
/// conexión, deciden los índices únicos de `ingresos_correo`: gana el
/// primero en escribir.
pub fn registrar_ingreso_correo_verificado<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: &CacheTokenDispositivo,
    actor: &UsuarioSesion,
    mut datos: NuevoIngresoCorreo,
) -> Result<i64, IngresoCorreoVerificadoError> {
    if let Ok(cedula) = crate::domain::cedula::Cedula::normalizar(&datos.cedula) {
        datos.cedula = cedula.into_string();
    }
    if nucleo().correo_con_ingreso_activo_en_sitio(&datos.cedula)? {
        return Err(IngresoCorreoServiceError::IngresoActivo.into());
    }

    if nube.vinculado()
        && let Some(token) = token_para_verificar(&nucleo, nube, actor, "ingreso por correo")?
    {
        verificar_correo_en_la_nube(&contexto(&token), &datos)?;
    }

    Ok(nucleo().registrar_ingreso_correo(
        actor,
        &crate::services::ingreso_correo_service::DatosIngresoCorreo {
            cedula: &datos.cedula,
            nombre: &datos.nombre,
            motivo: &datos.motivo,
            placa: datos.placa,
            gafete_numero: datos.gafete_numero,
        },
    )?)
}

/// Paso 2 de [`registrar_ingreso_correo_verificado`].
fn verificar_correo_en_la_nube(
    contexto: &ContextoSincronizacion<'_>,
    datos: &NuevoIngresoCorreo,
) -> Result<(), IngresoCorreoVerificadoError> {
    let numero = datos.gafete_numero;
    let (activo_en_otro_sitio, gafete_en_correo, gafete_en_visita, otra_via) =
        std::thread::scope(|hilos| {
            let en_correo = hilos.spawn(|| {
                crate::nube::gafete_de_correo_ocupado_en_otro_dispositivo(contexto, numero)
            });
            let en_visita = hilos.spawn(|| {
                crate::nube::gafete_de_visita_ocupado_en_otro_dispositivo(contexto, numero)
            });
            let por_otra_via = hilos.spawn(|| {
                crate::nube::persona_adentro_por_otra_via(
                    contexto,
                    &datos.cedula,
                    crate::models::via_ingreso::ViaIngreso::PorCorreo,
                )
            });
            let activo = crate::nube::correo_con_ingreso_activo(contexto, &datos.cedula);
            (
                activo,
                unir_hilo(en_correo),
                unir_hilo(en_visita),
                unir_hilo(por_otra_via),
            )
        });
    match o_libre_sin_conexion(activo_en_otro_sitio, None, "ingreso por correo")
        .map_err(GestionNubeError::from)?
    {
        Some(activo) if activo.mismo_sitio => {
            return Err(IngresoCorreoServiceError::IngresoActivo.into());
        }
        Some(activo) => {
            return Err(IngresoCorreoVerificadoError::ActivoEnOtroSitio {
                sitio: activo.sitio_nombre,
            });
        }
        None => {}
    }
    // Una persona no puede estar adentro por dos vías (como contratista o
    // como proveedor, en cualquier unidad).
    if let Some(adentro) = o_libre_sin_conexion(otra_via, None, "ingreso por correo (otra vía)")
        .map_err(GestionNubeError::from)?
    {
        return Err(IngresoCorreoServiceError::AdentroPorOtraVia(adentro.via).into());
    }
    if o_libre_sin_conexion(gafete_en_correo, false, "ingreso por correo (gafete)")
        .map_err(GestionNubeError::from)?
        || o_libre_sin_conexion(gafete_en_visita, false, "ingreso por correo (gafete)")
            .map_err(GestionNubeError::from)?
    {
        return Err(IngresoCorreoVerificadoError::GafeteOcupadoEnSitio { numero });
    }
    Ok(())
}

// ---- Entrada de una visita agendada ----

#[derive(Debug, thiserror::Error)]
pub enum EntradaVisitaVerificadaError {
    #[error(transparent)]
    Servicio(#[from] CitaServiceError),
    #[error("el visitante ya tiene una visita activa en {sitio}")]
    ActivoEnOtroSitio { sitio: String },
    #[error("el gafete de visita {numero} ya está en uso en otro dispositivo del sitio")]
    GafeteOcupadoEnSitio { numero: i64 },
    #[error(transparent)]
    Nube(#[from] GestionNubeError),
}

/// Entrada de una visita agendada con todas sus reglas. Antes estos chequeos
/// vivían en el comando de escritorio; ahora los comparten escritorio y
/// teléfono:
/// 1. con nube configurada, en vivo y a la vez:
///    - el gafete de visita (si lleva) no está en uso en el otro dispositivo
///      del sitio, ni por una visita ni por un ingreso por correo (los dos
///      reparten el mismo catálogo). Sin conexión con la nube se sigue con
///      lo local (ver el doc-comment del módulo); si la nube rechaza la
///      consulta, no se registra;
///    - el visitante no está adentro en otra unidad ni con el otro equipo de
///      esta. De **mejor esfuerzo**: si no se pudo verificar, se registra, y
///      si chocó lo frena la nube al sincronizar
///      (`movimientos_visita_cedula_activa_idx`);
/// 2. escribe (`AppCore::registrar_entrada_visita`: vuelve a verificar la
///    cita, el gafete local y que no esté adentro en este equipo).
pub fn registrar_entrada_visita_verificada<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: &CacheTokenDispositivo,
    actor: &UsuarioSesion,
    cedula: &str,
    gafete_numero: Option<i64>,
    placa: Option<String>,
) -> Result<i64, EntradaVisitaVerificadaError> {
    if nube.vinculado() {
        match token_para_verificar(&nucleo, nube, actor, "entrada de visita") {
            Ok(Some(token)) => {
                verificar_visita_en_la_nube(&contexto(&token), cedula, gafete_numero)?;
            }
            Ok(None) => {}
            // Sin gafete no hay nada que deba verificarse sí o sí.
            Err(error) if gafete_numero.is_some() => return Err(error.into()),
            Err(error) => log::info!("no se pudo verificar la visita en la nube: {error}"),
        }
    }

    Ok(nucleo().registrar_entrada_visita(actor, cedula, gafete_numero, placa)?)
}

/// Paso 1 de [`registrar_entrada_visita_verificada`].
fn verificar_visita_en_la_nube(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
    gafete_numero: Option<i64>,
) -> Result<(), EntradaVisitaVerificadaError> {
    let (activo_en_otro_sitio, gafete_ocupado) = std::thread::scope(|hilos| {
        let consulta_gafete = hilos.spawn(|| {
            gafete_numero
                .map(|numero| gafete_de_visita_ocupado(contexto, numero))
                .transpose()
        });
        let activo = crate::nube::visitante_activo_en_otro_sitio(contexto, cedula);
        (activo, unir_hilo(consulta_gafete))
    });
    if let (Some(numero), Some(true)) = (
        gafete_numero,
        o_libre_sin_conexion(gafete_ocupado, None, "entrada de visita (gafete)")
            .map_err(GestionNubeError::from)?,
    ) {
        return Err(EntradaVisitaVerificadaError::GafeteOcupadoEnSitio { numero });
    }
    if let Ok(Some(sitio)) = activo_en_otro_sitio {
        return Err(EntradaVisitaVerificadaError::ActivoEnOtroSitio { sitio });
    }
    Ok(())
}

/// El gafete de visita está en manos de otro dispositivo del sitio, por una
/// visita o por un ingreso por correo.
fn gafete_de_visita_ocupado(
    contexto: &ContextoSincronizacion<'_>,
    numero: i64,
) -> Result<bool, SincronizacionError> {
    Ok(
        crate::nube::gafete_de_visita_ocupado_en_otro_dispositivo(contexto, numero)?
            || crate::nube::gafete_de_correo_ocupado_en_otro_dispositivo(contexto, numero)?,
    )
}

// ---- Entrega de gafete provisional KOF ----

#[derive(Debug, thiserror::Error)]
pub enum EntregaGafeteProvisionalVerificadaError {
    #[error(transparent)]
    Servicio(#[from] GafeteProvisionalServiceError),
    #[error("el gafete {numero} ya está prestado en otro dispositivo del sitio")]
    GafeteOcupadoEnSitio { numero: i64 },
    #[error(transparent)]
    Nube(#[from] GestionNubeError),
}

/// Entrega de gafete provisional KOF con todas sus reglas:
/// 1. con nube configurada, en vivo y a la vez: el gafete no está prestado
///    en el otro dispositivo del sitio, y el encargado (por su código de
///    empleado) no tiene otro gafete provisional sin devolver en ningún sitio
///    (`prestamo_provisional_activo_de_encargado`). Sin conexión con la nube
///    se sigue con lo local (ver el doc-comment del módulo); si la nube
///    rechaza la consulta, no se entrega;
/// 2. escribe (`AppCore::entregar_gafete_provisional`, reglas locales: el
///    encargado existe, está activo y no tiene otro préstamo en este equipo).
///
/// Si dos equipos pasan el paso 1 en el mismo instante, o uno entregó sin
/// conexión, decide el índice único
/// `prestamos_gafete_provisional_encargado_activo_idx` (y el de gafete
/// activo del sitio, que además avisa a quien opera al sincronizar).
pub fn entregar_gafete_provisional_verificado<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: &CacheTokenDispositivo,
    actor: &UsuarioSesion,
    encargado_id: i64,
    gafete_numero: i64,
) -> Result<i64, EntregaGafeteProvisionalVerificadaError> {
    if nube.vinculado() {
        // Sin encargado en el catálogo local no hay a quién buscar: la
        // escritura de abajo lo rechaza con su propio error.
        let codigo_empleado = nucleo().codigo_empleado_de_encargado(encargado_id)?;
        if let Some(token) =
            token_para_verificar(&nucleo, nube, actor, "entrega de gafete provisional")?
        {
            verificar_entrega_provisional_en_la_nube(
                &contexto(&token),
                codigo_empleado.as_deref(),
                gafete_numero,
            )?;
        }
    }

    Ok(nucleo().entregar_gafete_provisional(actor, encargado_id, gafete_numero)?)
}

/// Paso 1 de [`entregar_gafete_provisional_verificado`].
fn verificar_entrega_provisional_en_la_nube(
    contexto: &ContextoSincronizacion<'_>,
    codigo_empleado: Option<&str>,
    gafete_numero: i64,
) -> Result<(), EntregaGafeteProvisionalVerificadaError> {
    let (gafete_ocupado, prestamo_del_encargado) = std::thread::scope(|hilos| {
        let consulta_encargado = hilos.spawn(|| {
            codigo_empleado.map_or(Ok(None), |codigo| {
                crate::nube::encargado_con_prestamo_provisional_activo(contexto, codigo)
            })
        });
        let ocupado =
            crate::nube::gafete_provisional_ocupado_en_otro_dispositivo(contexto, gafete_numero);
        (ocupado, unir_hilo(consulta_encargado))
    });
    if o_libre_sin_conexion(
        gafete_ocupado,
        false,
        "entrega de gafete provisional (gafete)",
    )
    .map_err(GestionNubeError::from)?
    {
        return Err(
            EntregaGafeteProvisionalVerificadaError::GafeteOcupadoEnSitio {
                numero: gafete_numero,
            },
        );
    }
    if let Some(prestamo) = o_libre_sin_conexion(
        prestamo_del_encargado,
        None,
        "entrega de gafete provisional (encargado)",
    )
    .map_err(GestionNubeError::from)?
    {
        log::info!(
            "gafete provisional: el encargado ya tiene un préstamo sin devolver en {}",
            prestamo.sitio_nombre
        );
        return Err(GafeteProvisionalServiceError::EncargadoYaTienePrestamoActivo.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::database::repositories::empresa_proveedor_repository::{
        EmpresaProveedorRepository, SqliteEmpresaProveedorRepository,
    };
    use crate::database::repositories::encargado_ruta_repository::{
        EncargadoRutaRepository, SqliteEncargadoRutaRepository,
    };
    use crate::database::repositories::gafete_repository::{
        GafeteRepository, SqliteGafeteRepository,
    };
    use crate::database::schema::initialize_database;
    use crate::models::empresa_proveedor::EmpresaProveedor;
    use crate::models::encargado_ruta::EncargadoRuta;
    use crate::models::gafete::TipoGafete;
    use crate::models::usuario::RolUsuario;
    use crate::tiempo::RelojFijo;

    fn nucleo_con_empresa_y_gafete() -> (Mutex<AppCore>, UsuarioSesion, i64) {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        let empresa_id = SqliteEmpresaProveedorRepository::new(&connection)
            .crear(&EmpresaProveedor {
                id: 0,
                nombre: "Maika".to_string(),
                activo: true,
            })
            .unwrap();
        SqliteGafeteRepository::new(&connection)
            .crear(5, TipoGafete::Proveedor)
            .unwrap();
        let reloj = Arc::new(RelojFijo::new(
            Utc.with_ymd_and_hms(2026, 9, 16, 12, 0, 0).unwrap(),
        ));
        let actor = UsuarioSesion {
            id: 1,
            cedula: "1001".to_string(),
            nombre: "Operador".to_string(),
            rol: RolUsuario::Operador,
        };
        (
            Mutex::new(AppCore::con_reloj(connection, reloj)),
            actor,
            empresa_id,
        )
    }

    fn datos(cedula: &str, empresa_id: i64) -> NuevoIngresoProveedor {
        NuevoIngresoProveedor {
            cedula: cedula.into(),
            nombre: "JUAN PEREZ".into(),
            empresa_id,
            placa: None,
            gafete_numero: 5,
        }
    }

    #[test]
    fn sin_nube_registra_sin_tocar_la_red() {
        let (core, actor, empresa_id) = nucleo_con_empresa_y_gafete();
        let cache = CacheTokenDispositivo::new();

        let id = registrar_ingreso_proveedor_verificado(
            || core.lock().unwrap(),
            &cache,
            &actor,
            datos("1-1111-1111", empresa_id),
        )
        .unwrap();

        assert!(id > 0);
    }

    #[test]
    fn rechaza_la_cedula_abierta_en_el_otro_dispositivo() {
        let (core, actor, empresa_id) = nucleo_con_empresa_y_gafete();
        core.lock()
            .unwrap()
            .connection
            .execute(
                "INSERT INTO ingresos_proveedor_remotos (uuid, sitio_id, cedula, nombre,
                     empresa_nombre, placa, gafete_numero, hora_entrada,
                     usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en)
                 VALUES ('u1', 's1', '2-2222-2222', 'Ana', 'Maika', NULL, 8,
                     '2026-09-16T11:00:00Z', 'Otro', 'd2', '2026-09-16T11:00:00Z')",
                [],
            )
            .unwrap();
        let cache = CacheTokenDispositivo::new();

        let error = registrar_ingreso_proveedor_verificado(
            || core.lock().unwrap(),
            &cache,
            &actor,
            datos("2-2222-2222", empresa_id),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            IngresoProveedorVerificadoError::Servicio(IngresoProveedorServiceError::IngresoActivo)
        ));
    }

    #[test]
    fn sin_nube_la_entrega_aplica_las_reglas_locales() {
        let (core, actor, _) = nucleo_con_empresa_y_gafete();
        let encargado_id = SqliteEncargadoRutaRepository::new(&core.lock().unwrap().connection)
            .crear(&EncargadoRuta {
                id: 0,
                codigo_empleado: "5040017".to_string(),
                nombre: "ANA MORA".to_string(),
                cedula: None,
                activo: true,
            })
            .unwrap();
        SqliteGafeteRepository::new(&core.lock().unwrap().connection)
            .crear(12, TipoGafete::ProvisionalKof)
            .unwrap();
        let cache = CacheTokenDispositivo::new();
        let entregar = || {
            entregar_gafete_provisional_verificado(
                || core.lock().unwrap(),
                &cache,
                &actor,
                encargado_id,
                12,
            )
        };

        assert!(entregar().unwrap() > 0);
        assert!(matches!(
            entregar().unwrap_err(),
            EntregaGafeteProvisionalVerificadaError::Servicio(
                GafeteProvisionalServiceError::EncargadoYaTienePrestamoActivo
                    | GafeteProvisionalServiceError::GafeteYaPrestado
            )
        ));
    }

    // ---- Ingreso de contratista ----

    fn nucleo_con_contratista() -> (Mutex<AppCore>, UsuarioSesion) {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO empresas(id,nombre) VALUES (1,'Empresa');
                 INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo)
                 VALUES (1,'1001','Operador','hash','OPERADOR',1);
                 INSERT INTO contratistas(
                     id,cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso
                 ) VALUES (1,'101110111','PERSONA',1,'SWAT',0,1);",
            )
            .unwrap();
        let actor = UsuarioSesion {
            id: 1,
            cedula: "1001".to_string(),
            nombre: "Operador".to_string(),
            rol: RolUsuario::Operador,
        };
        (Mutex::new(AppCore::new(connection)), actor)
    }

    #[test]
    fn sin_nube_el_ingreso_se_registra() {
        let (core, actor) = nucleo_con_contratista();
        let cache = CacheTokenDispositivo::new();

        registrar_ingreso_verificado(
            || core.lock().unwrap(),
            &cache,
            &actor,
            1,
            MedioIngreso::Caminando,
            None,
            None,
        )
        .unwrap();
    }

    #[test]
    fn adentro_por_el_otro_equipo_se_bloquea_sin_tocar_la_red() {
        let (core, actor) = nucleo_con_contratista();
        core.lock()
            .unwrap()
            .connection
            .execute(
                "INSERT INTO ingresos_remotos (
                     uuid, sitio_id, contratista_nombre, hora_entrada,
                     dispositivo_entrada_id, actualizado_en, contratista_cedula
                 ) VALUES ('u', 's1', 'PERSONA', '2026-09-27T12:00:00Z', 'otro',
                           '2026-09-27T12:00:00Z', '101110111')",
                [],
            )
            .unwrap();
        let cache = CacheTokenDispositivo::new();

        let error = registrar_ingreso_verificado(
            || core.lock().unwrap(),
            &cache,
            &actor,
            1,
            MedioIngreso::Caminando,
            None,
            None,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            IngresoVerificadoError::Bloqueado(BloqueoIngreso::IngresoActivoEnOtroDispositivo)
        ));
    }

    /// Servidor HTTP falso que contesta una sola vez `cuerpo` como JSON.
    /// Nube de prueba para `bloqueo_en_la_nube`, que hace dos consultas a
    /// la vez: `cuerpo` responde la de la misma vía y `otra_via` la de
    /// `persona_adentro_por_otra_via`.
    fn nube_que_responde_con_otra_via(cuerpo: &'static str, otra_via: &'static str) -> String {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let direccion = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut conexion, _)) = listener.accept() else {
                    return;
                };
                let mut lector = BufReader::new(conexion.try_clone().unwrap());
                let mut primera = String::new();
                let _ = lector.read_line(&mut primera);
                loop {
                    let mut linea = String::new();
                    if lector.read_line(&mut linea).unwrap_or(0) == 0 || linea == "\r\n" {
                        break;
                    }
                }
                let cuerpo = if primera.contains("persona_adentro_por_otra_via") {
                    otra_via
                } else {
                    cuerpo
                };
                let respuesta = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
                    cuerpo.len()
                );
                let _ = conexion.write_all(respuesta.as_bytes());
            }
        });
        format!("http://{direccion}")
    }

    fn nube_que_responde(cuerpo: &'static str) -> String {
        nube_que_responde_con_otra_via(cuerpo, "[]")
    }

    fn bloqueo_contra(base_url: &str) -> Option<BloqueoIngreso> {
        let contexto = ContextoSincronizacion {
            base_url,
            apikey: "apikey",
            token: "token",
            dispositivo_id: "este-equipo",
            sitio_id: "sitio-propio",
        };
        bloqueo_en_la_nube(&contexto, "101110111")
    }

    #[test]
    fn la_nube_sin_ingresos_abiertos_deja_pasar() {
        assert_eq!(bloqueo_contra(&nube_que_responde("[]")), None);
    }

    #[test]
    fn la_nube_con_un_ingreso_en_este_sitio_bloquea_por_el_otro_equipo() {
        let base = nube_que_responde(r#"[{"sitio_id":"sitio-propio","sitio_nombre":"Brisas"}]"#);
        assert_eq!(
            bloqueo_contra(&base),
            Some(BloqueoIngreso::IngresoActivoEnOtroDispositivo)
        );
    }

    #[test]
    fn la_nube_con_un_ingreso_en_otro_sitio_bloquea_con_su_nombre() {
        let base = nube_que_responde(r#"[{"sitio_id":"otro","sitio_nombre":"Cartago"}]"#);
        assert_eq!(
            bloqueo_contra(&base),
            Some(BloqueoIngreso::ActivoEnOtroSitio {
                sitio: "Cartago".into()
            })
        );
    }

    /// Una persona no puede estar adentro por dos vías: adentro como
    /// proveedor en otra unidad, no entra como contratista. Se muestra con el
    /// bloqueo de "otro sitio" y la vía al lado del nombre.
    #[test]
    fn la_nube_con_la_persona_adentro_como_proveedor_bloquea_nombrando_la_via() {
        let base = nube_que_responde_con_otra_via(
            "[]",
            r#"[{"via":"PROVEEDOR","sitio_id":"otro","sitio_nombre":"Cartago"}]"#,
        );
        assert_eq!(
            bloqueo_contra(&base),
            Some(BloqueoIngreso::ActivoEnOtroSitio {
                sitio: "Cartago (como proveedor)".into()
            })
        );
    }

    /// La portería no se bloquea por falta de red: sin conexión con la
    /// nube se registra con las reglas locales (el choque, si lo hubo, lo
    /// atrapa el índice único al sincronizar).
    #[test]
    fn sin_conexion_con_la_nube_se_deja_pasar() {
        let cerrado = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://{}", listener.local_addr().unwrap())
        };
        assert_eq!(bloqueo_contra(&cerrado), None);
    }

    /// Una nube que acepta la conexión pero no contesta cuenta como sin
    /// conexión pasado `TIMEOUT_VERIFICACION`: quien opera no queda
    /// esperando el tope general de la sincronización.
    #[test]
    fn la_nube_que_no_contesta_a_tiempo_deja_pasar() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let mudo = std::thread::spawn(move || {
            let conexiones: Vec<_> = listener.incoming().take(2).collect();
            std::thread::sleep(std::time::Duration::from_secs(5));
            drop(conexiones);
        });

        let inicio = std::time::Instant::now();
        assert_eq!(bloqueo_contra(&base), None);
        assert!(
            inicio.elapsed() < std::time::Duration::from_secs(5),
            "la verificación debía rendirse antes que la nube muda"
        );
        mudo.join().unwrap();
    }

    /// Servidor HTTP falso que contesta `estado` (línea de estado completa,
    /// sin cuerpo útil) a las dos consultas de `bloqueo_en_la_nube`.
    fn nube_que_responde_estado(estado: &'static str) -> String {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let direccion = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut conexion, _)) = listener.accept() else {
                    return;
                };
                let mut lector = BufReader::new(conexion.try_clone().unwrap());
                loop {
                    let mut linea = String::new();
                    if lector.read_line(&mut linea).unwrap_or(0) == 0 || linea == "\r\n" {
                        break;
                    }
                }
                let respuesta = format!(
                    "HTTP/1.1 {estado}\r\nContent-Type: application/json\r\n\
                     Content-Length: 2\r\nConnection: close\r\n\r\n{{}}"
                );
                let _ = conexion.write_all(respuesta.as_bytes());
            }
        });
        format!("http://{direccion}")
    }

    /// Nube caída (`5xx`): cuenta como sin conexión.
    #[test]
    fn la_nube_caida_deja_pasar() {
        let base = nube_que_responde_estado("503 Service Unavailable");
        assert_eq!(bloqueo_contra(&base), None);
    }

    /// La nube contestó que no (`4xx`: dispositivo retirado, token
    /// rechazado): eso no es falta de red y sigue frenando.
    #[test]
    fn la_nube_que_rechaza_la_consulta_no_deja_pasar() {
        let base = nube_que_responde_estado("403 Forbidden");
        assert_eq!(
            bloqueo_contra(&base),
            Some(BloqueoIngreso::SinVerificarEnLaNube)
        );
    }
}

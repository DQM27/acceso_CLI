//! Operaciones que, además de sus reglas locales, consultan en vivo a la
//! nube antes de escribir (el otro dispositivo del sitio u otros sitios).
//! El orden de los chequeos y qué falla frena viven sólo acá; escritorio
//! (Tauri) y móvil (`mobile/rust-core`) llaman a estas funciones tal cual.
//!
//! Reciben una función que toma el candado del núcleo en vez de `&AppCore`:
//! el candado se toma sólo para lo local (`SQLite`) y se suelta antes de
//! cada llamada de red, para no colgar al resto de la app mientras tanto
//! (ver el doc-comment de `nube::cache_token`).

use std::ops::Deref;

use super::{AppCore, GestionNubeError};
use crate::models::medio_ingreso::MedioIngreso;
use crate::nube::{CacheTokenDispositivo, ContextoSincronizacion, TokenDispositivo};
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{
    GafeteProvisionalServiceError, IngresoProveedorServiceError, RegistroIngresoServiceError,
};
use crate::services::registro_ingreso_service::{
    BloqueoIngreso, PreparacionIngreso, ResultadoRegistroEntrada,
};

/// Con qué hablar con la nube desde este dispositivo. `secreto: None`
/// significa nube sin configurar: no hay con quién chocar y ningún
/// chequeo remoto toca la red.
#[derive(Clone, Copy)]
pub struct NubeDelDispositivo<'a> {
    pub cache_token: &'a CacheTokenDispositivo,
    pub secreto: Option<&'a str>,
}

impl NubeDelDispositivo<'_> {
    fn secreto(&self) -> Option<&str> {
        self.secreto.filter(|secreto| !secreto.trim().is_empty())
    }
}

/// Token vigente para consultar la nube. Autoriza al actor con el candado
/// tomado (es `SQLite`, microsegundos) y lo suelta antes de autenticar.
fn autenticar<G: Deref<Target = AppCore>>(
    nucleo: &impl Fn() -> G,
    nube: NubeDelDispositivo<'_>,
    secreto: &str,
    actor: &UsuarioSesion,
) -> Result<TokenDispositivo, GestionNubeError> {
    nucleo().autorizar_uso_nube(actor)?;
    let token = nube.cache_token.autenticar_con_cache(secreto)?;
    if let Some(desfase_ms) = token.desfase_reloj_ms {
        nucleo().actualizar_desfase_reloj(desfase_ms);
    }
    Ok(token)
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

// ---- Ingreso de contratista ----

/// Regla: un contratista no puede tener dos ingresos activos, ni en este
/// sitio (este equipo o el otro) ni en otro sitio. Busca en la nube un
/// ingreso abierto con esa cédula en cualquier sitio. `Ok(None)` = libre;
/// sin nube configurada no hay con quién chocar (sólo cuentan los chequeos
/// locales). Si la consulta falla, `SinVerificarEnLaNube`: con nube
/// configurada no se registra sin verificar (decisión del dueño, igual que
/// el gafete).
fn bloqueo_en_la_nube(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
) -> Option<BloqueoIngreso> {
    match crate::nube::contratista_con_ingreso_activo(contexto, cedula) {
        Ok(None) => None,
        Ok(Some(activo)) if activo.mismo_sitio => {
            Some(BloqueoIngreso::IngresoActivoEnOtroDispositivo)
        }
        Ok(Some(activo)) => Some(BloqueoIngreso::ActivoEnOtroSitio {
            sitio: activo.sitio_nombre,
        }),
        Err(error) => {
            log::warn!("ingreso: no se pudo verificar en la nube: {error}");
            Some(BloqueoIngreso::SinVerificarEnLaNube)
        }
    }
}

/// Vista previa al elegir a la persona: reglas locales y, si pasan, la
/// verificación en la nube. El bloqueo queda en la preparación (ver
/// `bloqueo_en_la_nube`); las pantallas sólo lo muestran.
pub fn preparar_ingreso_verificado<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: NubeDelDispositivo<'_>,
    actor: Option<&UsuarioSesion>,
    contratista_id: i64,
) -> Result<(PreparacionIngreso, Option<BloqueoIngreso>), RegistroIngresoServiceError> {
    let preparacion = nucleo().preparar_ingreso(contratista_id)?;
    let local = preparacion.bloqueo();
    if local.is_some() {
        return Ok((preparacion, local));
    }
    let Some(secreto) = nube.secreto() else {
        return Ok((preparacion, None));
    };
    // Sin sesión no hay con qué autorizar la consulta: con nube
    // configurada eso cuenta como "no se pudo verificar".
    let bloqueo = match actor.map(|actor| autenticar(&nucleo, nube, secreto, actor)) {
        Some(Ok(token)) => bloqueo_en_la_nube(&contexto(&token), &preparacion.cedula),
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
///    Si no se puede verificar, no se registra;
/// 3. escribe (`AppCore::registrar_ingreso`, que repite lo local dentro de
///    su transacción).
///
/// Si dos equipos pasan el paso 2 en el mismo instante, el índice único de
/// Supabase decide: gana el primero en escribir.
#[allow(clippy::too_many_arguments)]
pub fn registrar_ingreso_verificado<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: NubeDelDispositivo<'_>,
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

    if let Some(secreto) = nube.secreto() {
        let token = autenticar(&nucleo, nube, secreto, actor).map_err(|error| {
            log::warn!("ingreso: no se pudo autenticar para verificar: {error}");
            IngresoVerificadoError::Bloqueado(BloqueoIngreso::SinVerificarEnLaNube)
        })?;
        let contexto = contexto(&token);
        if let Some(bloqueo) = bloqueo_en_la_nube(&contexto, &preparacion.cedula) {
            return Err(IngresoVerificadoError::Bloqueado(bloqueo));
        }
        if let Some(numero) = gafete {
            let ocupado = crate::nube::gafete_ocupado_en_otro_dispositivo(&contexto, numero)
                .map_err(|error| {
                    log::warn!("ingreso: no se pudo verificar el gafete: {error}");
                    IngresoVerificadoError::Bloqueado(BloqueoIngreso::SinVerificarEnLaNube)
                })?;
            if ocupado {
                return Err(IngresoVerificadoError::GafeteOcupadoEnSitio { numero });
            }
        }
    }

    Ok(nucleo().registrar_ingreso(actor, contratista_id, medio, gafete, placa)?)
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
/// 2. ni en otro sitio (nube, mejor esfuerzo: si la consulta falla, deja
///    pasar y el conflicto se detecta al sincronizar);
/// 3. el gafete no está en uso en el otro dispositivo del sitio (nube; si
///    no se puede verificar, frena: un gafete físico no puede duplicarse);
/// 4. escribe (`AppCore::registrar_ingreso_proveedor`, reglas locales).
pub fn registrar_ingreso_proveedor_verificado<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: NubeDelDispositivo<'_>,
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

    if let Some(secreto) = nube.secreto() {
        let token = autenticar(&nucleo, nube, secreto, actor)?;
        let contexto = contexto(&token);
        if let Some(sitio) = crate::nube::proveedor_activo_en_otro_sitio(&contexto, &datos.cedula)
            .ok()
            .flatten()
        {
            return Err(IngresoProveedorVerificadoError::ActivoEnOtroSitio { sitio });
        }
        if crate::nube::gafete_de_proveedor_ocupado_en_otro_dispositivo(
            &contexto,
            datos.gafete_numero,
        )
        .map_err(GestionNubeError::from)?
        {
            return Err(IngresoProveedorVerificadoError::GafeteOcupadoEnSitio {
                numero: datos.gafete_numero,
            });
        }
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

/// Entrega de gafete provisional: si el gafete ya está prestado en el otro
/// dispositivo del sitio (nube) no se entrega, y si no se puede verificar
/// tampoco (un gafete físico no puede duplicarse). Recién ahí escribe
/// (`AppCore::entregar_gafete_provisional`, reglas locales).
pub fn entregar_gafete_provisional_verificado<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: NubeDelDispositivo<'_>,
    actor: &UsuarioSesion,
    encargado_id: i64,
    gafete_numero: i64,
) -> Result<i64, EntregaGafeteProvisionalVerificadaError> {
    if let Some(secreto) = nube.secreto() {
        let token = autenticar(&nucleo, nube, secreto, actor)?;
        if crate::nube::gafete_provisional_ocupado_en_otro_dispositivo(
            &contexto(&token),
            gafete_numero,
        )
        .map_err(GestionNubeError::from)?
        {
            return Err(
                EntregaGafeteProvisionalVerificadaError::GafeteOcupadoEnSitio {
                    numero: gafete_numero,
                },
            );
        }
    }

    Ok(nucleo().entregar_gafete_provisional(actor, encargado_id, gafete_numero)?)
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

    fn sin_nube(cache: &CacheTokenDispositivo) -> NubeDelDispositivo<'_> {
        NubeDelDispositivo {
            cache_token: cache,
            secreto: None,
        }
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
            sin_nube(&cache),
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
            sin_nube(&cache),
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
    fn un_secreto_en_blanco_cuenta_como_nube_sin_configurar() {
        let cache = CacheTokenDispositivo::new();
        let nube = NubeDelDispositivo {
            cache_token: &cache,
            secreto: Some("   "),
        };
        assert!(nube.secreto().is_none());
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
        let cache = CacheTokenDispositivo::new();
        let entregar = || {
            entregar_gafete_provisional_verificado(
                || core.lock().unwrap(),
                sin_nube(&cache),
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
            sin_nube(&cache),
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
            sin_nube(&cache),
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
    fn nube_que_responde(cuerpo: &'static str) -> String {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let direccion = listener.local_addr().unwrap();
        std::thread::spawn(move || {
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
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
                cuerpo.len()
            );
            let _ = conexion.write_all(respuesta.as_bytes());
        });
        format!("http://{direccion}")
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
        let base =
            nube_que_responde(r#"[{"sitio_id":"sitio-propio","sitios":{"nombre":"Brisas"}}]"#);
        assert_eq!(
            bloqueo_contra(&base),
            Some(BloqueoIngreso::IngresoActivoEnOtroDispositivo)
        );
    }

    #[test]
    fn la_nube_con_un_ingreso_en_otro_sitio_bloquea_con_su_nombre() {
        let base = nube_que_responde(r#"[{"sitio_id":"otro","sitios":{"nombre":"Cartago"}}]"#);
        assert_eq!(
            bloqueo_contra(&base),
            Some(BloqueoIngreso::ActivoEnOtroSitio {
                sitio: "Cartago".into()
            })
        );
    }

    #[test]
    fn sin_respuesta_de_la_nube_no_se_deja_pasar() {
        let cerrado = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://{}", listener.local_addr().unwrap())
        };
        assert_eq!(
            bloqueo_contra(&cerrado),
            Some(BloqueoIngreso::SinVerificarEnLaNube)
        );
    }
}

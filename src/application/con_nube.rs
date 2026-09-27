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
use crate::nube::{CacheTokenDispositivo, ContextoSincronizacion, TokenDispositivo};
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{GafeteProvisionalServiceError, IngresoProveedorServiceError};

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
    datos: NuevoIngresoProveedor,
) -> Result<i64, IngresoProveedorVerificadoError> {
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
            datos("1-1111", empresa_id),
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
                 VALUES ('u1', 's1', '2-2222', 'Ana', 'Maika', NULL, 8,
                     '2026-09-16T11:00:00Z', 'Otro', 'd2', '2026-09-16T11:00:00Z')",
                [],
            )
            .unwrap();
        let cache = CacheTokenDispositivo::new();

        let error = registrar_ingreso_proveedor_verificado(
            || core.lock().unwrap(),
            sin_nube(&cache),
            &actor,
            datos("2-2222", empresa_id),
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
}

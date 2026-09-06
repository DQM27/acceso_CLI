//! Gestión de la persistencia en la nube (`docs/plan-persistencia-nube.md`)
//! desde la fachada de aplicación. Configurar el secreto del dispositivo
//! (`Operacion::GestionarNube`) es exclusivo de ROOT -- esa credencial es
//! la identidad de todo el equipo ante el receptor, no una preferencia que
//! un Administrador deba poder tocar. Sincronizar, leer y cerrar ingresos
//! remotos (`Operacion::UsarNube`) es de cualquier rol -- ya es uso diario
//! normal (la pantalla Activos los usa), no administración.
//!
//! `directorio` es `None` en escritorio (resuelve `%LOCALAPPDATA%` solo,
//! ver `nube::credenciales::guardar_secreto`) y `Some(...)` en el celular
//! (recibe el mismo directorio que ya usa para abrir la base `SQLite`,
//! ver `mobile/rust-core/src/lib.rs` -- Android no tiene `%LOCALAPPDATA%`).

use std::path::Path;

use crate::database::error::DatabaseError;
use crate::domain::autorizacion::Operacion;
use crate::nube::IngresoRemoto;
use crate::services::autenticacion_service::UsuarioSesion;

use super::{AppCore, verificar_actor_activo};

/// Campo interno de `AppCore` (ver `token_nube_cacheado` en `application::mod`)
/// -- guarda el último `TokenDispositivo` obtenido junto con cuándo, para
/// que `AppCore::autenticar_con_cache` decida si todavía es reutilizable.
pub(super) struct TokenCacheado {
    pub(super) secreto: String,
    pub(super) token: crate::nube::TokenDispositivo,
    pub(super) obtenido_en: std::time::Instant,
}

/// Movimiento del espejo de Supabase, sin inventar valores para datos antiguos ausentes.
#[derive(Debug, Clone)]
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
}

impl AppCore {
    /// Lee el historial recibido del sitio con el mismo rango y búsqueda del móvil.
    pub fn listar_historial_sitio(
        &self,
        actor: &UsuarioSesion,
        desde: chrono::DateTime<chrono::Utc>,
        hasta: chrono::DateTime<chrono::Utc>,
        texto: &str,
    ) -> Result<Vec<MovimientoHistorialSitio>, GestionNubeError> {
        self.autorizar_uso_nube(actor)?;
        let mut consulta = self.connection.prepare(
            "SELECT uuid, contratista_cedula, contratista_nombre, empresa_nombre,
                    hora_entrada, hora_salida, gafete_numero, usuario_entrada_nombre,
                    usuario_salida_nombre, motivo_resultado
             FROM historial_sitio
             WHERE hora_entrada >= ?1 AND hora_entrada < ?2
               AND (?3 = '' OR instr(lower(contratista_nombre), lower(?3)) > 0
                    OR instr(contratista_cedula, ?3) > 0)
             ORDER BY hora_entrada DESC, uuid LIMIT 30",
        )?;
        Ok(consulta
            .query_map(
                rusqlite::params![
                    crate::tiempo::serializar_utc(desde),
                    crate::tiempo::serializar_utc(hasta),
                    texto.trim()
                ],
                |row| {
                    Ok(MovimientoHistorialSitio {
                        uuid: row.get(0)?,
                        cedula: row.get(1)?,
                        contratista_nombre: row.get(2)?,
                        empresa_nombre: row.get(3)?,
                        fecha_hora_ingreso: row.get(4)?,
                        fecha_hora_salida: row.get(5)?,
                        gafete_numero: row.get(6)?,
                        usuario_ingreso_nombre: row.get(7)?,
                        usuario_salida_nombre: row.get(8)?,
                        motivo_resultado: row.get(9)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GestionNubeError {
    #[error("Sólo una sesión ROOT activa puede gestionar la nube")]
    OperacionNoAutorizada,
    #[error("Su sesión no está autorizada para usar la nube")]
    UsoNoAutorizado,
    #[error("Error de SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Todavía no se guardó el secreto de este dispositivo")]
    SinSecreto,
    #[error("No se pudo guardar el secreto localmente: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Autenticacion(#[from] crate::nube::NubeError),
    #[error(transparent)]
    Sincronizacion(#[from] crate::nube::SincronizacionError),
}

/// Resultado de una sincronización manual -- lo suficiente para que la
/// pantalla muestre "sitio X, dispositivo Y: 12 enviados, 0 fallidos, 2
/// abiertos del otro lado".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumenSincronizacion {
    pub enviados: u32,
    pub fallidos: u32,
    pub remotos_abiertos: u32,
    pub cierres_recibidos: u32,
    pub empresas_recibidas: u32,
    pub contratistas_recibidos: u32,
    pub gafetes_recibidos: u32,
    pub movimientos_historial_recibidos: u32,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
    /// `true` si esta misma sincronización trajo la baja/desactivación de
    /// quien la disparó -- ver `AppCore::sesion_sigue_activa`. Decisión
    /// explícita del usuario: el login local (offline-first) no puede
    /// exigir estar en línea para dejar operar, pero una sesión YA abierta
    /// que se entera -- en cuanto vuelve a tener señal -- de que a su
    /// usuario lo desactivaron en otro dispositivo, se cierra sola en vez
    /// de seguir operando con un permiso que ya no existe. Quien recibe
    /// esto debe cerrar la sesión local y volver al login.
    pub sesion_expulsada: bool,
}

/// Datos temporales para que una capa de plataforma abra un canal Realtime.
/// El núcleo autentica y autoriza; el WebSocket queda fuera de esta capa.
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl AppCore {
    pub fn guardar_secreto_dispositivo(
        &self,
        actor: &UsuarioSesion,
        directorio: Option<&Path>,
        secreto: &str,
    ) -> Result<(), GestionNubeError> {
        self.autorizar_gestion_nube(actor)?;
        match directorio {
            Some(directorio) => crate::nube::credenciales::guardar_secreto_en(directorio, secreto)?,
            None => crate::nube::credenciales::guardar_secreto(secreto)?,
        }
        Ok(())
    }

    /// No revela el secreto ya guardado -- sólo si hay uno o no, para que
    /// la pantalla sepa si mostrar "pegá el secreto" o "dispositivo ya
    /// configurado".
    pub fn secreto_dispositivo_guardado(
        &self,
        actor: &UsuarioSesion,
        directorio: Option<&Path>,
    ) -> Result<bool, GestionNubeError> {
        self.autorizar_gestion_nube(actor)?;
        let guardado = directorio.map_or_else(
            crate::nube::credenciales::cargar_secreto,
            crate::nube::credenciales::cargar_secreto_en,
        );
        Ok(guardado.is_some())
    }

    /// Autentica este dispositivo, drena la bandeja de salida y refresca
    /// la caché de lo que el otro dispositivo del mismo sitio tiene
    /// abierto. Pensado para el celular, que no tiene el concepto de
    /// "conexión secundaria" del escritorio (ver comentario de
    /// `autorizar_gestion_nube`) -- en un teléfono de un solo usuario,
    /// retener el candado durante la llamada de red es una simplificación
    /// razonable, no un cuello de botella real.
    pub fn sincronizar_con_nube(
        &self,
        actor: &UsuarioSesion,
        directorio: Option<&Path>,
    ) -> Result<ResumenSincronizacion, GestionNubeError> {
        self.autorizar_uso_nube(actor)?;

        let secreto = directorio
            .map_or_else(
                crate::nube::credenciales::cargar_secreto,
                crate::nube::credenciales::cargar_secreto_en,
            )
            .ok_or(GestionNubeError::SinSecreto)?;
        let token = self.autenticar_con_cache(&secreto)?;

        let contexto = crate::nube::ContextoSincronizacion {
            base_url: crate::nube::BASE_URL,
            apikey: crate::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let resumen = crate::nube::drenar_cola(&self.connection, &contexto, 200)?;
        let cierres_recibidos =
            crate::nube::recibir_cierres_de_ingresos_propios(&self.connection, &contexto)?;
        let remotos = crate::nube::recibir_ingresos_abiertos(&self.connection, &contexto)?;
        let catalogo = crate::nube::recibir_catalogo_del_sitio(&self.connection, &contexto)?;
        let movimientos_historial_recibidos =
            crate::nube::recibir_historial_del_sitio(&self.connection, &contexto)?;

        Ok(ResumenSincronizacion {
            enviados: resumen.enviados,
            fallidos: resumen.fallidos,
            remotos_abiertos: u32::try_from(remotos.len()).unwrap_or(u32::MAX),
            cierres_recibidos,
            empresas_recibidas: catalogo.empresas_recibidas,
            contratistas_recibidos: catalogo.contratistas_recibidos,
            gafetes_recibidos: catalogo.gafetes_recibidos,
            movimientos_historial_recibidos,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            sesion_expulsada: !self.sesion_sigue_activa(actor),
        })
    }

    /// Autentica este dispositivo y devuelve lo mínimo para que la capa de
    /// plataforma escuche Broadcast privado por sitio. No abre sockets ni
    /// interpreta mensajes: cada aviso debe disparar `sincronizar_con_nube`.
    pub fn sesion_realtime_nube(
        &self,
        actor: &UsuarioSesion,
        directorio: Option<&Path>,
    ) -> Result<SesionRealtimeNube, GestionNubeError> {
        self.autorizar_uso_nube(actor)?;
        let secreto = directorio
            .map_or_else(
                crate::nube::credenciales::cargar_secreto,
                crate::nube::credenciales::cargar_secreto_en,
            )
            .ok_or(GestionNubeError::SinSecreto)?;
        let token = self.autenticar_con_cache(&secreto)?;
        let topic = format!("sitio:{}", token.sitio_id);

        Ok(SesionRealtimeNube {
            base_url: crate::nube::BASE_URL.to_string(),
            apikey: crate::nube::APIKEY.to_string(),
            access_token: token.access_token,
            expires_in: token.expires_in,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            topic,
        })
    }

    /// Lectura pura de la caché local `ingresos_remotos` -- ya la llenó la
    /// última `sincronizar_con_nube`, no hace falta red para mostrarla.
    pub fn listar_ingresos_remotos(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<Vec<IngresoRemoto>, GestionNubeError> {
        self.autorizar_uso_nube(actor)?;
        let mut statement = self.connection.prepare(
            "SELECT uuid, contratista_nombre, hora_entrada, usuario_entrada_nombre,
                    contratista_cedula, empresa_nombre, tipo_ingreso, medio_ingreso, gafete_numero
             FROM ingresos_remotos ORDER BY hora_entrada",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok(IngresoRemoto {
                    uuid: row.get(0)?,
                    contratista_nombre: row.get(1)?,
                    hora_entrada: row.get(2)?,
                    usuario_entrada_nombre: row.get(3)?,
                    contratista_cedula: row.get(4)?,
                    empresa_nombre: row.get(5)?,
                    tipo_ingreso: row.get(6)?,
                    medio_ingreso: row.get(7)?,
                    gafete_numero: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(filas)
    }

    /// Chequeo en vivo (no la caché local) de si `gafete_numero` ya está
    /// activo en este sitio del lado de OTRO dispositivo -- ver
    /// `nube::gafete_ocupado_en_otro_dispositivo`. Pensada para llamarse
    /// justo antes de confirmar un ingreso nuevo con gafete: sin secreto
    /// guardado (dispositivo sin nube configurada, o un sitio de un solo
    /// dispositivo) no hay con quién chocar, así que no hace falta red --
    /// se resuelve `Ok(false)` directo. Con nube configurada, en cambio,
    /// esto exige estar en línea: si la consulta falla, el error se
    /// propaga (`Autenticacion`/`Sincronizacion`) en vez de asumir que el
    /// gafete está libre -- decisión explícita del usuario, prefiere
    /// bloquear el ingreso a arriesgar el mismo número duplicado entre
    /// dispositivos otra vez.
    pub fn gafete_ocupado_en_sitio(
        &self,
        actor: &UsuarioSesion,
        directorio: Option<&Path>,
        gafete_numero: i64,
    ) -> Result<bool, GestionNubeError> {
        self.autorizar_uso_nube(actor)?;
        let secreto = directorio.map_or_else(
            crate::nube::credenciales::cargar_secreto,
            crate::nube::credenciales::cargar_secreto_en,
        );
        let Some(secreto) = secreto else {
            return Ok(false);
        };
        let token = self.autenticar_con_cache(&secreto)?;
        let contexto = crate::nube::ContextoSincronizacion {
            base_url: crate::nube::BASE_URL,
            apikey: crate::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        Ok(crate::nube::gafete_ocupado_en_otro_dispositivo(
            &contexto,
            gafete_numero,
        )?)
    }

    /// Cierra, contra la nube, un ingreso abierto por el otro dispositivo
    /// del mismo sitio -- ver `nube::cerrar_ingreso_remoto`.
    pub fn cerrar_ingreso_remoto(
        &self,
        actor: &UsuarioSesion,
        directorio: Option<&Path>,
        uuid: &str,
    ) -> Result<(), GestionNubeError> {
        self.autorizar_uso_nube(actor)?;

        let secreto = directorio
            .map_or_else(
                crate::nube::credenciales::cargar_secreto,
                crate::nube::credenciales::cargar_secreto_en,
            )
            .ok_or(GestionNubeError::SinSecreto)?;
        let token = self.autenticar_con_cache(&secreto)?;

        let contexto = crate::nube::ContextoSincronizacion {
            base_url: crate::nube::BASE_URL,
            apikey: crate::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        crate::nube::cerrar_ingreso_remoto(&self.connection, &contexto, uuid, &actor.nombre)?;
        Ok(())
    }

    /// Sólo autoriza -- no toca la red ni el archivo del secreto. Separado
    /// por el mismo motivo que `autorizar_creacion_respaldo`: la
    /// sincronización hace red (varios cientos de milisegundos, tal vez
    /// más con conexión lenta) y en escritorio no debe retener el
    /// `Mutex<AppCore>` compartido mientras tanto -- ahí quien llama
    /// autoriza acá, con el candado, y ejecuta `crate::nube::drenar_cola`
    /// sobre una conexión propia (ver `GuiState::conexion_secundaria`).
    pub fn autorizar_gestion_nube(&self, actor: &UsuarioSesion) -> Result<(), GestionNubeError> {
        let usuario = verificar_actor_activo(&self.connection, actor)
            .map_err(|error| match error {
                DatabaseError::Sqlite(error) => GestionNubeError::Sqlite(error),
                _ => GestionNubeError::OperacionNoAutorizada,
            })?
            .ok_or(GestionNubeError::OperacionNoAutorizada)?;
        if !usuario.rol.puede(Operacion::GestionarNube) {
            return Err(GestionNubeError::OperacionNoAutorizada);
        }
        Ok(())
    }

    /// Sólo autoriza -- mismo motivo que `autorizar_gestion_nube`, pero
    /// para `Operacion::UsarNube` (sincronizar/leer/cerrar), que cualquier
    /// rol puede.
    pub fn autorizar_uso_nube(&self, actor: &UsuarioSesion) -> Result<(), GestionNubeError> {
        let usuario = verificar_actor_activo(&self.connection, actor)
            .map_err(|error| match error {
                DatabaseError::Sqlite(error) => GestionNubeError::Sqlite(error),
                _ => GestionNubeError::UsoNoAutorizado,
            })?
            .ok_or(GestionNubeError::UsoNoAutorizado)?;
        if !usuario.rol.puede(Operacion::UsarNube) {
            return Err(GestionNubeError::UsoNoAutorizado);
        }
        Ok(())
    }

    fn aplicar_desfase_reloj(&self, token: &crate::nube::TokenDispositivo) {
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.actualizar_desfase_reloj(desfase_ms);
        }
    }

    /// Reusa el último `TokenDispositivo` mientras siga vigente en vez de
    /// autenticar de cero en cada llamada -- reproducido en el celular:
    /// confirmar un ingreso con gafete (`gafete_ocupado_en_sitio`) hacía
    /// una autenticación completa contra la nube aunque el dispositivo ya
    /// se hubiera autenticado segundos antes para sincronizar, sintiéndose
    /// como que la app se colgaba en cada registro. Margen de 30s antes del
    /// vencimiento real para no arrancar una operación con un token que
    /// puede vencer a mitad de camino. Un acierto de caché no vuelve a
    /// medir el desfase de reloj (`desfase_reloj_ms` queda en `None`) --
    /// no hace falta remedirlo en cada llamada, sólo cuando de verdad se
    /// habla con el receptor.
    fn autenticar_con_cache(
        &self,
        secreto: &str,
    ) -> Result<crate::nube::TokenDispositivo, GestionNubeError> {
        const MARGEN_EXPIRACION: std::time::Duration = std::time::Duration::from_secs(30);

        {
            let cache = self
                .token_nube_cacheado
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entrada) = cache.as_ref() {
                let vigente_por =
                    std::time::Duration::from_secs(entrada.token.expires_in)
                        .saturating_sub(MARGEN_EXPIRACION);
                if entrada.secreto == secreto && entrada.obtenido_en.elapsed() < vigente_por {
                    let mut token = entrada.token.clone();
                    token.desfase_reloj_ms = None;
                    return Ok(token);
                }
            }
        }

        let token = crate::nube::autenticar_dispositivo(crate::nube::BASE_URL, secreto)?;
        self.aplicar_desfase_reloj(&token);
        *self
            .token_nube_cacheado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(TokenCacheado {
            secreto: secreto.to_string(),
            token: token.clone(),
            obtenido_en: std::time::Instant::now(),
        });
        Ok(token)
    }

    /// Corrige el reloj de este `AppCore` con un desfase ya medido (ver
    /// `TokenDispositivo::desfase_reloj_ms`) -- no-op si el reloj inyectado
    /// no es [`crate::tiempo::RelojCorregido`] (CLI/TUI siguen con
    /// `RelojSistema`, que ignora esta llamada). Pública porque algunas
    /// autenticaciones pasan por fuera de `AppCore` a propósito (Tauri
    /// evita retener el `Mutex` compartido durante la parte de red, ver
    /// `comandos/nube.rs::autenticar` en el escritorio) y necesitan un
    /// punto para devolver lo medido. Cada autenticación exitosa vuelve a
    /// medir y sobrescribe -- no acumula, así que un desfase que ya se
    /// corrigió (o empeoró) en Windows se refleja solo, sin reiniciar la
    /// app.
    pub fn actualizar_desfase_reloj(&self, desfase_ms: i64) {
        self.reloj.actualizar_desfase_ms(desfase_ms);
    }
}

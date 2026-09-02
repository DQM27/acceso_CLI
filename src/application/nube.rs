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

#[derive(Debug, thiserror::Error)]
pub enum GestionNubeError {
    #[error("Sólo una sesión ROOT activa puede gestionar la nube")]
    OperacionNoAutorizada,
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
    pub empresas_recibidas: u32,
    pub contratistas_recibidos: u32,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
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
        let token = crate::nube::autenticar_dispositivo(crate::nube::BASE_URL, &secreto)?;

        let contexto = crate::nube::ContextoSincronizacion {
            base_url: crate::nube::BASE_URL,
            apikey: crate::nube::APIKEY,
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let resumen = crate::nube::drenar_cola(&self.connection, &contexto, 200)?;
        let remotos = crate::nube::recibir_ingresos_abiertos(&self.connection, &contexto)?;
        let catalogo = crate::nube::recibir_catalogo_del_sitio(&self.connection, &contexto)?;

        Ok(ResumenSincronizacion {
            enviados: resumen.enviados,
            fallidos: resumen.fallidos,
            remotos_abiertos: u32::try_from(remotos.len()).unwrap_or(u32::MAX),
            empresas_recibidas: catalogo.empresas_recibidas,
            contratistas_recibidos: catalogo.contratistas_recibidos,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
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
            "SELECT uuid, contratista_nombre, hora_entrada, usuario_entrada_nombre
             FROM ingresos_remotos ORDER BY hora_entrada",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok(IngresoRemoto {
                    uuid: row.get(0)?,
                    contratista_nombre: row.get(1)?,
                    hora_entrada: row.get(2)?,
                    usuario_entrada_nombre: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(filas)
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
        let token = crate::nube::autenticar_dispositivo(crate::nube::BASE_URL, &secreto)?;

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
                _ => GestionNubeError::OperacionNoAutorizada,
            })?
            .ok_or(GestionNubeError::OperacionNoAutorizada)?;
        if !usuario.rol.puede(Operacion::UsarNube) {
            return Err(GestionNubeError::OperacionNoAutorizada);
        }
        Ok(())
    }
}

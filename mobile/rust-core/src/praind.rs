//! Carnés PRAIND en el teléfono (fase 6 de
//! `docs/features-futuras/plan-fotos-praind.md`). La lógica es la misma del
//! escritorio (`control_acceso::nube::praind`): el carné se baja recién al
//! pedirlo y queda guardado en la base cifrada; después se ve sin internet.

use control_acceso::nube::praind::{self, ErrorPraind};

use crate::{Nucleo, NucleoError};

/// Estado del carné de un contratista en este teléfono.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EstadoCarnePraind {
    /// Tiene carné cargado (si no, no se ofrece "Ver carné").
    pub tiene_carne: bool,
    /// Ya está guardado en el teléfono: se ve al instante y sin internet.
    pub guardado: bool,
}

/// El carné listo para mostrar.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CarnePraind {
    pub tipo_mime: String,
    pub bytes: Vec<u8>,
}

/// Lo que el teléfono puede hacer con los carnés, según la nube.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct PermisosCarnePraind {
    pub puede_cargar: bool,
    pub sin_conexion: bool,
}

/// Resultado de cargar (o previsualizar) un carné por el QR.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CargaCarnePraind {
    /// `None` en la previsualización (no se guardó nada).
    pub clave: Option<String>,
    /// Nombre de la persona según la página del QR.
    pub nombre_en_origen: Option<String>,
    /// Si ese nombre coincide con el del contratista (regla compartida).
    pub nombre_coincide: bool,
}

fn rechazo(mensaje: String) -> NucleoError {
    NucleoError::Rechazado { mensaje }
}

fn nombre_contratista(conexion: &rusqlite::Connection, id: i64) -> Result<String, NucleoError> {
    conexion
        .query_row(
            "SELECT nombre FROM contratistas WHERE id = ?1",
            [id],
            |fila| fila.get(0),
        )
        .map_err(|_| rechazo("El contratista no existe.".to_string()))
}

use praind::nombres_coinciden;

impl Nucleo {
    fn token_praind(&self) -> Result<String, ErrorPraind> {
        self.autenticar_con_cache()
            .map(|token| token.access_token)
            .map_err(|error| {
                if error.es_falta_de_conexion() {
                    ErrorPraind::SinConexion
                } else {
                    ErrorPraind::Rechazo {
                        estado: 401,
                        mensaje: control_acceso::mensajes::mensaje_nube(error),
                    }
                }
            })
    }
}

#[uniffi::export]
impl Nucleo {
    /// Si el contratista tiene carné y si ya está en el teléfono. Sin red.
    pub fn estado_carne_praind(
        &self,
        contratista_id: i64,
    ) -> Result<EstadoCarnePraind, NucleoError> {
        self.actor_autenticado()?;
        let carne =
            praind::estado_carne(&self.conexion_secundaria()?, contratista_id).map_err(rechazo)?;
        Ok(EstadoCarnePraind {
            tiene_carne: carne.clave.is_some(),
            guardado: carne.guardado,
        })
    }

    /// El carné: del teléfono si ya se vio; si no, de la nube (y queda
    /// guardado). Sin internet y sin haberlo visto antes, avisa que hace
    /// falta conexión.
    pub fn ver_carne_praind(&self, contratista_id: i64) -> Result<CarnePraind, NucleoError> {
        self.actor_autenticado()?;
        let carne = praind::ver_carne(&self.conexion_secundaria()?, contratista_id, || {
            self.token_praind()
        })
        .map_err(rechazo)?;
        Ok(CarnePraind {
            tipo_mime: carne.tipo_mime,
            bytes: carne.bytes,
        })
    }

    /// Si la nube deja cargar carnés desde este teléfono (lo decide el panel).
    pub fn permisos_carne_praind(&self) -> PermisosCarnePraind {
        let resultado = self.token_praind().and_then(|token| {
            praind::permisos(
                control_acceso::nube::base_url(),
                control_acceso::nube::apikey(),
                &token,
            )
        });
        match resultado {
            Ok(permisos) => PermisosCarnePraind {
                puede_cargar: permisos.puede_cargar,
                sin_conexion: false,
            },
            Err(error) => PermisosCarnePraind {
                puede_cargar: false,
                sin_conexion: error.es_sin_conexion(),
            },
        }
    }

    /// Carga el carné con el enlace leído del QR. Con `solo_previsualizar`
    /// no guarda nada: devuelve de quién es para confirmarlo antes.
    pub fn cargar_carne_praind_desde_qr(
        &self,
        contratista_id: i64,
        enlace: String,
        solo_previsualizar: bool,
    ) -> Result<CargaCarnePraind, NucleoError> {
        self.actor_autenticado()?;
        let conexion = self.conexion_secundaria()?;
        let nombre = nombre_contratista(&conexion, contratista_id)?;
        let carga = praind::cargar_carne_desde_qr(
            &conexion,
            contratista_id,
            &enlace,
            solo_previsualizar,
            || self.token_praind(),
        )
        .map_err(rechazo)?;
        Ok(CargaCarnePraind {
            nombre_coincide: nombres_coinciden(carga.nombre_en_origen.as_deref(), &nombre),
            clave: carga.clave,
            nombre_en_origen: carga.nombre_en_origen,
        })
    }
}

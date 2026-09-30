//! Puente entre la clave del teléfono (Android Keystore / Secure Enclave) y
//! la identidad del dispositivo del núcleo (`control_acceso::nube::firmante`).
//!
//! Kotlin/Swift implementan [`AlmacenClaveDispositivo`] (callback de
//! `UniFFI`): generar el par, entregar la clave pública y firmar. La clave
//! privada nunca cruza esta frontera. El resto -- armar la aserción, la
//! huella, hablar con el servidor -- lo hace el mismo código del núcleo que
//! usa escritorio.

use std::sync::Arc;

use control_acceso::nube::{ErrorFirmante, FirmanteDispositivo, firmante::firma_der_a_jws};

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum AlmacenClaveError {
    #[error("{mensaje}")]
    Almacen { mensaje: String },
}

impl From<uniffi::UnexpectedUniFFICallbackError> for AlmacenClaveError {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Almacen {
            mensaje: error.reason,
        }
    }
}

/// Lo que cada plataforma implementa sobre su almacén seguro de claves.
/// Ver `FirmanteDispositivo` en el núcleo para el contrato de cada método;
/// la única diferencia es que [`Self::firmar_der`] devuelve la firma en DER,
/// que es lo que entregan las APIs nativas (`Signature` de Android), y el
/// núcleo la convierte a `r || s`.
#[uniffi::export(with_foreign)]
pub trait AlmacenClaveDispositivo: Send + Sync {
    /// Clave pública como JWK P-256 (`{"kty":"EC","crv":"P-256","x":…,"y":…}`).
    /// Genera el par si todavía no existe.
    fn clave_publica_jwk(&self) -> Result<String, AlmacenClaveError>;
    /// Firma `SHA256withECDSA` de `datos`, en DER.
    fn firmar_der(&self, datos: Vec<u8>) -> Result<Vec<u8>, AlmacenClaveError>;
    /// Descarta el par vigente y genera uno nuevo, sin vincular.
    fn regenerar(&self) -> Result<(), AlmacenClaveError>;
    /// `dispositivo_id` al que el servidor ató la clave vigente.
    fn dispositivo_vinculado(&self) -> Option<String>;
    /// Registra que el servidor aceptó la clave vigente.
    fn marcar_vinculada(&self, dispositivo_id: String) -> Result<(), AlmacenClaveError>;
}

/// Adapta el almacén de la plataforma al trait del núcleo.
pub struct FirmanteMovil(pub Arc<dyn AlmacenClaveDispositivo>);

fn error_del_almacen(error: AlmacenClaveError) -> ErrorFirmante {
    ErrorFirmante::Almacen(error.to_string())
}

impl FirmanteDispositivo for FirmanteMovil {
    fn clave_publica_jwk(&self) -> Result<String, ErrorFirmante> {
        self.0.clave_publica_jwk().map_err(error_del_almacen)
    }

    fn firmar(&self, datos: &[u8]) -> Result<Vec<u8>, ErrorFirmante> {
        let der = self
            .0
            .firmar_der(datos.to_vec())
            .map_err(error_del_almacen)?;
        firma_der_a_jws(&der)
    }

    fn regenerar(&self) -> Result<(), ErrorFirmante> {
        self.0.regenerar().map_err(error_del_almacen)
    }

    fn dispositivo_vinculado(&self) -> Option<String> {
        self.0.dispositivo_vinculado()
    }

    fn marcar_vinculada(&self, dispositivo_id: &str) -> Result<(), ErrorFirmante> {
        self.0
            .marcar_vinculada(dispositivo_id.to_string())
            .map_err(error_del_almacen)
    }
}

/// Almacén en memoria para los tests del crate (este módulo y `tests.rs`).
#[cfg(test)]
pub mod pruebas {
    use std::sync::Mutex;

    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use p256::ecdsa::signature::Signer;
    use p256::ecdsa::{Signature, SigningKey};

    use super::{AlmacenClaveDispositivo, AlmacenClaveError};

    /// Almacén de prueba que se comporta como Android Keystore: firma en
    /// DER y guarda el `dispositivo_id` vinculado.
    pub struct AlmacenComoKeystore {
        clave: SigningKey,
        vinculado: Mutex<Option<String>>,
    }

    impl AlmacenClaveDispositivo for AlmacenComoKeystore {
        fn clave_publica_jwk(&self) -> Result<String, AlmacenClaveError> {
            let punto = self.clave.verifying_key().to_encoded_point(false);
            let coordenada = |bytes: Option<&p256::FieldBytes>| {
                URL_SAFE_NO_PAD.encode(bytes.expect("punto sin comprimir"))
            };
            Ok(format!(
                r#"{{"kty":"EC","crv":"P-256","x":"{}","y":"{}"}}"#,
                coordenada(punto.x()),
                coordenada(punto.y())
            ))
        }

        fn firmar_der(&self, datos: Vec<u8>) -> Result<Vec<u8>, AlmacenClaveError> {
            let firma: Signature = self.clave.sign(&datos);
            Ok(firma.to_der().as_bytes().to_vec())
        }

        fn regenerar(&self) -> Result<(), AlmacenClaveError> {
            Ok(())
        }

        fn dispositivo_vinculado(&self) -> Option<String> {
            self.vinculado.lock().unwrap().clone()
        }

        fn marcar_vinculada(&self, dispositivo_id: String) -> Result<(), AlmacenClaveError> {
            *self.vinculado.lock().unwrap() = Some(dispositivo_id);
            Ok(())
        }
    }

    impl AlmacenComoKeystore {
        pub fn con_clave(clave: SigningKey) -> Self {
            Self {
                clave,
                vinculado: Mutex::new(None),
            }
        }

        pub fn sin_vincular() -> Self {
            Self::con_clave(SigningKey::random(&mut rand_core::OsRng))
        }

        /// Como un teléfono que ya canjeó su código.
        pub fn vinculado_a(dispositivo_id: &str) -> Self {
            let almacen = Self::sin_vincular();
            *almacen.vinculado.lock().unwrap() = Some(dispositivo_id.to_string());
            almacen
        }
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use control_acceso::nube::firmante::{ClavePublicaJwk, construir_asercion};
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::{Signature, SigningKey, VerifyingKey};

    use super::pruebas::AlmacenComoKeystore;
    use super::*;

    #[test]
    fn la_firma_der_del_almacen_produce_una_asercion_valida() {
        let clave = SigningKey::random(&mut rand_core::OsRng);
        let verificadora = VerifyingKey::from(&clave);
        let firmante = FirmanteMovil(Arc::new(AlmacenComoKeystore::con_clave(clave)));

        let asercion = construir_asercion(&firmante, "desafio").unwrap();
        let partes: Vec<&str> = asercion.split('.').collect();
        let firma = Signature::from_slice(&URL_SAFE_NO_PAD.decode(partes[2]).unwrap()).unwrap();
        verificadora
            .verify(format!("{}.{}", partes[0], partes[1]).as_bytes(), &firma)
            .expect("la aserción valida contra la clave pública del almacén");

        let jwk = ClavePublicaJwk::desde_json(&firmante.clave_publica_jwk().unwrap()).unwrap();
        let encabezado: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(partes[0]).unwrap()).unwrap();
        assert_eq!(encabezado["kid"], jwk.huella());
    }

    #[test]
    fn marcar_vinculada_llega_al_almacen() {
        let firmante = FirmanteMovil(Arc::new(AlmacenComoKeystore::sin_vincular()));
        assert_eq!(firmante.dispositivo_vinculado(), None);
        firmante.marcar_vinculada("disp-1").unwrap();
        assert_eq!(firmante.dispositivo_vinculado().as_deref(), Some("disp-1"));
    }
}

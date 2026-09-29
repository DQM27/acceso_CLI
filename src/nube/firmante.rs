//! Identidad del dispositivo por par de claves (ver
//! `docs/features-futuras/propuesta-registro-dispositivos.md`, sección 4.2).
//!
//! Cada equipo genera su propio par EC P-256 y sólo entrega la clave
//! pública; la privada nunca sale del equipo. Para autenticarse, firma un
//! desafío del servidor (`device-auth`) y manda esa aserción en vez de un
//! secreto compartido.
//!
//! El núcleo arma la aserción, calcula la huella y habla con el servidor;
//! lo ÚNICO que depende de la plataforma es dónde vive la clave privada y
//! quién firma, y eso queda detrás de [`FirmanteDispositivo`]:
//!
//! - Escritorio: [`FirmanteArchivo`], en este mismo módulo (clave en disco
//!   protegida con DPAPI, igual que el secreto de antes).
//! - Android/iOS: implementado en Kotlin/Swift sobre Android Keystore /
//!   Secure Enclave, expuesto al núcleo como callback de `UniFFI` (ver
//!   `mobile/rust-core/src/firmante.rs`).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// `aud` que `device-auth` exige en la aserción.
pub const AUDIENCIA_ASERCION: &str = "device-auth";

#[derive(Debug, thiserror::Error)]
pub enum ErrorFirmante {
    #[error("No se pudo acceder a la clave de este dispositivo: {0}")]
    Almacen(String),
    #[error("La clave pública de este dispositivo no es válida: {0}")]
    ClaveInvalida(String),
}

/// Dónde vive la clave privada del equipo y quién firma con ella. El resto
/// (aserción, huella, red) es común a todas las plataformas.
pub trait FirmanteDispositivo: Send + Sync {
    /// Clave pública del par vigente como JWK P-256 (`{"kty","crv","x","y"}`).
    /// Si todavía no hay par, lo genera.
    fn clave_publica_jwk(&self) -> Result<String, ErrorFirmante>;

    /// Firma ES256 de `datos` (SHA-256 + ECDSA P-256) en formato JWS:
    /// `r || s`, 64 bytes.
    fn firmar(&self, datos: &[u8]) -> Result<Vec<u8>, ErrorFirmante>;

    /// Descarta el par vigente y genera uno nuevo. Se usa antes de canjear
    /// un código cuando el par anterior ya estaba vinculado (re-vincular):
    /// cada vinculación estrena clave.
    fn regenerar(&self) -> Result<(), ErrorFirmante>;

    /// `dispositivo_id` al que el servidor ató la clave vigente, o `None` si
    /// todavía no la aceptó.
    fn dispositivo_vinculado(&self) -> Option<String>;

    /// Registra que el servidor aceptó la clave vigente para `dispositivo_id`.
    fn marcar_vinculada(&self, dispositivo_id: &str) -> Result<(), ErrorFirmante>;
}

/// Clave pública JWK, en el orden y con los miembros que exige RFC 7638.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClavePublicaJwk {
    pub crv: String,
    pub kty: String,
    pub x: String,
    pub y: String,
}

impl ClavePublicaJwk {
    pub fn desde_json(json: &str) -> Result<Self, ErrorFirmante> {
        let clave: Self = serde_json::from_str(json)
            .map_err(|error| ErrorFirmante::ClaveInvalida(error.to_string()))?;
        if clave.kty != "EC" || clave.crv != "P-256" {
            return Err(ErrorFirmante::ClaveInvalida(format!(
                "se esperaba EC P-256, llegó {} {}",
                clave.kty, clave.crv
            )));
        }
        Ok(clave)
    }

    fn desde_clave(clave: &VerifyingKey) -> Self {
        let punto = clave.to_encoded_point(false);
        // Un punto sin comprimir de P-256 siempre trae ambas coordenadas.
        let x = punto
            .x()
            .map(|x| URL_SAFE_NO_PAD.encode(x))
            .unwrap_or_default();
        let y = punto
            .y()
            .map(|y| URL_SAFE_NO_PAD.encode(y))
            .unwrap_or_default();
        Self {
            crv: "P-256".to_string(),
            kty: "EC".to_string(),
            x,
            y,
        }
    }

    /// JSON canónico: miembros en orden lexicográfico, sin espacios. Es lo
    /// que se hashea para la huella y lo que se manda al servidor.
    pub fn a_json_canonico(&self) -> String {
        format!(
            r#"{{"crv":"{}","kty":"{}","x":"{}","y":"{}"}}"#,
            self.crv, self.kty, self.x, self.y
        )
    }

    /// Huella RFC 7638 (SHA-256, base64url). Identifica al equipo en el `kid`
    /// de la aserción; el servidor la calcula igual al guardar la clave.
    pub fn huella(&self) -> String {
        URL_SAFE_NO_PAD.encode(Sha256::digest(self.a_json_canonico().as_bytes()))
    }
}

/// Aserción que `device-auth` acepta en lugar del secreto: un JWS compacto
/// firmado por el equipo, con su huella como `kid` y el desafío del
/// servidor adentro. No lleva `iat`/`exp`: la frescura la da el desafío, que
/// emite y fecha el servidor, así que el reloj del equipo no importa.
pub fn construir_asercion(
    firmante: &dyn FirmanteDispositivo,
    desafio: &str,
) -> Result<String, ErrorFirmante> {
    let clave = ClavePublicaJwk::desde_json(&firmante.clave_publica_jwk()?)?;
    let encabezado = serde_json::json!({ "alg": "ES256", "typ": "JWT", "kid": clave.huella() });
    let contenido = serde_json::json!({ "aud": AUDIENCIA_ASERCION, "desafio": desafio });
    let a_firmar = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(encabezado.to_string()),
        URL_SAFE_NO_PAD.encode(contenido.to_string())
    );
    let firma = firmante.firmar(a_firmar.as_bytes())?;
    if firma.len() != 64 {
        return Err(ErrorFirmante::Almacen(format!(
            "la firma debe medir 64 bytes (r||s), midió {}",
            firma.len()
        )));
    }
    Ok(format!("{a_firmar}.{}", URL_SAFE_NO_PAD.encode(firma)))
}

/// Convierte una firma ECDSA en DER (lo que devuelven Android Keystore y la
/// mayoría de las APIs nativas) al formato `r || s` que exige JWS.
pub fn firma_der_a_jws(der: &[u8]) -> Result<Vec<u8>, ErrorFirmante> {
    Signature::from_der(der)
        .map(|firma| firma.to_bytes().to_vec())
        .map_err(|error| ErrorFirmante::Almacen(format!("firma DER inválida: {error}")))
}

// ---- Firmante de escritorio ----

const ARCHIVO_CLAVE: &str = "dispositivo-nube.clave";

#[derive(Serialize, Deserialize)]
struct ClaveGuardada {
    /// Escalar privado P-256 (32 bytes), base64url.
    privada: String,
    dispositivo_id: Option<String>,
}

/// Firmante en software para escritorio: la clave privada vive en
/// `<directorio>/dispositivo-nube.clave`, protegida con la misma DPAPI que
/// ya protegía el secreto (ver `credenciales::guardar_protegido_en`). No es
/// una clave no-exportable como la de Android Keystore, pero nunca viaja
/// por la red y deja al equipo listo para pasar a TPM más adelante sin
/// tocar nada fuera de este tipo.
pub struct FirmanteArchivo {
    directorio: PathBuf,
    /// Copia en memoria de lo guardado, para no leer ni desproteger el
    /// archivo en cada firma.
    estado: Mutex<Option<ClaveGuardada>>,
}

impl FirmanteArchivo {
    pub fn en(directorio: &Path) -> Self {
        Self {
            directorio: directorio.to_path_buf(),
            estado: Mutex::new(None),
        }
    }

    /// El de escritorio, en `%APPDATA%\ControlAcceso` junto al secreto
    /// legado. `None` si `%APPDATA%` no está disponible.
    pub fn por_defecto() -> Option<Self> {
        super::credenciales::directorio_credenciales_roaming()
            .map(|directorio| Self::en(&directorio))
    }

    fn con_clave<T>(
        &self,
        uso: impl FnOnce(&mut ClaveGuardada) -> Result<T, ErrorFirmante>,
    ) -> Result<T, ErrorFirmante> {
        let mut estado = self
            .estado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if estado.is_none() {
            *estado = Some(match self.leer() {
                Some(guardada) => guardada,
                None => self.guardar(ClaveGuardada {
                    privada: nueva_clave_privada(),
                    dispositivo_id: None,
                })?,
            });
        }
        let guardada = estado.as_mut().ok_or_else(|| {
            ErrorFirmante::Almacen("la clave no quedó cargada en memoria".to_string())
        })?;
        // El candado se sostiene durante `uso` a propósito: `regenerar` o
        // `marcar_vinculada` no deben poder cruzarse con una firma a medias.
        let resultado = uso(guardada);
        drop(estado);
        resultado
    }

    fn leer(&self) -> Option<ClaveGuardada> {
        let texto = super::credenciales::cargar_protegido_en(&self.directorio, ARCHIVO_CLAVE)?;
        serde_json::from_str(&texto).ok()
    }

    fn guardar(&self, clave: ClaveGuardada) -> Result<ClaveGuardada, ErrorFirmante> {
        let texto = serde_json::to_string(&clave)
            .map_err(|error| ErrorFirmante::Almacen(error.to_string()))?;
        super::credenciales::guardar_protegido_en(&self.directorio, ARCHIVO_CLAVE, &texto)
            .map_err(|error| ErrorFirmante::Almacen(error.to_string()))?;
        Ok(clave)
    }
}

fn nueva_clave_privada() -> String {
    URL_SAFE_NO_PAD.encode(SigningKey::random(&mut rand_core::OsRng).to_bytes())
}

fn clave_de_firma(guardada: &ClaveGuardada) -> Result<SigningKey, ErrorFirmante> {
    let bytes = URL_SAFE_NO_PAD
        .decode(&guardada.privada)
        .map_err(|error| ErrorFirmante::Almacen(format!("clave guardada ilegible: {error}")))?;
    SigningKey::from_slice(&bytes)
        .map_err(|error| ErrorFirmante::Almacen(format!("clave guardada inválida: {error}")))
}

impl FirmanteDispositivo for FirmanteArchivo {
    fn clave_publica_jwk(&self) -> Result<String, ErrorFirmante> {
        self.con_clave(|guardada| {
            let clave = clave_de_firma(guardada)?;
            Ok(ClavePublicaJwk::desde_clave(clave.verifying_key()).a_json_canonico())
        })
    }

    fn firmar(&self, datos: &[u8]) -> Result<Vec<u8>, ErrorFirmante> {
        self.con_clave(|guardada| {
            let firma: Signature = clave_de_firma(guardada)?.sign(datos);
            Ok(firma.to_bytes().to_vec())
        })
    }

    fn regenerar(&self) -> Result<(), ErrorFirmante> {
        let nueva = self.guardar(ClaveGuardada {
            privada: nueva_clave_privada(),
            dispositivo_id: None,
        })?;
        *self
            .estado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(nueva);
        Ok(())
    }

    fn dispositivo_vinculado(&self) -> Option<String> {
        let mut estado = self
            .estado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if estado.is_none() {
            *estado = self.leer();
        }
        estado
            .as_ref()
            .and_then(|guardada| guardada.dispositivo_id.clone())
    }

    fn marcar_vinculada(&self, dispositivo_id: &str) -> Result<(), ErrorFirmante> {
        self.con_clave(|guardada| {
            let actualizada = ClaveGuardada {
                privada: guardada.privada.clone(),
                dispositivo_id: Some(dispositivo_id.to_string()),
            };
            *guardada = self.guardar(actualizada)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use p256::ecdsa::signature::Verifier;

    use super::*;

    fn directorio_temporal() -> tempfile::TempDir {
        tempfile::tempdir().expect("directorio temporal")
    }

    fn clave_verificadora(jwk: &ClavePublicaJwk) -> VerifyingKey {
        let x = URL_SAFE_NO_PAD.decode(&jwk.x).unwrap();
        let y = URL_SAFE_NO_PAD.decode(&jwk.y).unwrap();
        let mut sec1 = vec![0x04];
        sec1.extend_from_slice(&x);
        sec1.extend_from_slice(&y);
        VerifyingKey::from_sec1_bytes(&sec1).expect("punto válido")
    }

    #[test]
    fn la_asercion_se_verifica_con_la_clave_publica_y_lleva_la_huella() {
        let directorio = directorio_temporal();
        let firmante = FirmanteArchivo::en(directorio.path());

        let asercion = construir_asercion(&firmante, "desafio-de-prueba").unwrap();
        let partes: Vec<&str> = asercion.split('.').collect();
        assert_eq!(partes.len(), 3, "JWS compacto: encabezado.contenido.firma");

        let jwk = ClavePublicaJwk::desde_json(&firmante.clave_publica_jwk().unwrap()).unwrap();
        let encabezado: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(partes[0]).unwrap()).unwrap();
        assert_eq!(encabezado["alg"], "ES256");
        assert_eq!(encabezado["kid"], jwk.huella());

        let contenido: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(partes[1]).unwrap()).unwrap();
        assert_eq!(contenido["aud"], AUDIENCIA_ASERCION);
        assert_eq!(contenido["desafio"], "desafio-de-prueba");

        let firma = Signature::from_slice(&URL_SAFE_NO_PAD.decode(partes[2]).unwrap()).unwrap();
        let firmado = format!("{}.{}", partes[0], partes[1]);
        clave_verificadora(&jwk)
            .verify(firmado.as_bytes(), &firma)
            .expect("la firma valida contra la clave pública publicada");
    }

    #[test]
    fn la_clave_persiste_entre_instancias_y_regenerar_la_cambia() {
        let directorio = directorio_temporal();
        let primera = FirmanteArchivo::en(directorio.path())
            .clave_publica_jwk()
            .unwrap();
        let reabierta = FirmanteArchivo::en(directorio.path());
        assert_eq!(reabierta.clave_publica_jwk().unwrap(), primera);

        reabierta.marcar_vinculada("disp-1").unwrap();
        assert_eq!(
            FirmanteArchivo::en(directorio.path())
                .dispositivo_vinculado()
                .as_deref(),
            Some("disp-1")
        );

        reabierta.regenerar().unwrap();
        assert_ne!(reabierta.clave_publica_jwk().unwrap(), primera);
        assert_eq!(
            reabierta.dispositivo_vinculado(),
            None,
            "una clave nueva no está vinculada"
        );
    }

    #[test]
    fn sin_archivo_no_hay_dispositivo_vinculado_ni_se_crea_la_clave() {
        let directorio = directorio_temporal();
        let firmante = FirmanteArchivo::en(directorio.path());
        assert_eq!(firmante.dispositivo_vinculado(), None);
        assert!(!directorio.path().join(ARCHIVO_CLAVE).exists());
    }

    #[test]
    fn la_huella_es_la_de_rfc_7638_sobre_el_json_canonico() {
        // Valor calculado aparte con `jose.calculateJwkThumbprint` (el mismo
        // que usa `device-vincular` en el servidor) para esta clave pública.
        let jwk = ClavePublicaJwk {
            crv: "P-256".to_string(),
            kty: "EC".to_string(),
            x: "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU".to_string(),
            y: "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0".to_string(),
        };
        assert_eq!(
            jwk.a_json_canonico(),
            r#"{"crv":"P-256","kty":"EC","x":"f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU","y":"x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0"}"#
        );
        assert_eq!(jwk.huella(), "oKIywvGUpTVTyxMQ3bwIIeQUudfr_CkLMjCE19ECD-U");
    }

    #[test]
    fn una_firma_der_se_convierte_a_r_s() {
        let clave = SigningKey::random(&mut rand_core::OsRng);
        let firma: Signature = clave.sign(b"datos");
        let convertida = firma_der_a_jws(firma.to_der().as_bytes()).unwrap();
        assert_eq!(convertida, firma.to_bytes().to_vec());
        assert!(firma_der_a_jws(b"no es der").is_err());
    }
}

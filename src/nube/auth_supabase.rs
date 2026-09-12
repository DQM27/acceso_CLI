//! Login de usuarios globales (Administrador/Operador, y un ROOT ya
//! sincronizado a otro sitio) contra Supabase Auth -- ver
//! docs/plan-autenticacion-supabase-auth.md. Reemplaza la verificación
//! local de Argon2 contra el centinela `SIN_PASSWORD_LOCAL`
//! (`services/password.rs`), que dejaba reclamar una cuenta con sólo saber
//! la cédula (pública, va en los gafetes).
//!
//! El ROOT del arranque inicial de un sitio (`crear_root_inicial`, CLI/TUI,
//! exige base vacía) queda completamente AFUERA de este módulo -- sigue
//! con su hash Argon2 local, sin depender de red, tal como estaba decidido
//! ("Root inicial y login offline: sin cambios").

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

use super::cliente::cliente_http;

/// Email sintético interno -- nunca se manda correo real a esto, sólo sirve
/// como identificador de login para Supabase Auth (que exige email/phone).
/// Mismo sufijo que usan los Edge Functions de alta/reset
/// (`supabase/functions/admin-create-usuario/index.ts`).
fn email_sintetico(cedula: &str) -> String {
    format!("{}@brisas.local", cedula.trim())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SesionSupabase {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    /// `auth.users.id` (uuid) -- identidad estable para enlazar con la fila
    /// de `usuarios` (`auth_user_id`).
    pub usuario_id: String,
    /// `user_metadata.debe_cambiar_password` -- ver `admin-create-usuario`.
    /// La app debe forzar el cambio de contraseña antes de dejar operar
    /// cuando esto es `true`.
    pub debe_cambiar_password: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthSupabaseError {
    #[error("Cédula o contraseña incorrecta")]
    CredencialesInvalidas,
    #[error("No se pudo conectar con el servidor: {0}")]
    Red(String),
    #[error("El servidor devolvió una respuesta inesperada")]
    RespuestaInvalida,
    #[error("La sesión no es válida o venció")]
    TokenInvalido,
}

#[derive(Serialize)]
struct CuerpoLoginPassword<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
struct CuerpoRefresh<'a> {
    refresh_token: &'a str,
}

#[derive(Deserialize)]
struct RespuestaToken {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    user: UsuarioSupabase,
}

#[derive(Deserialize)]
struct UsuarioSupabase {
    id: String,
    #[serde(default)]
    user_metadata: MetadataUsuario,
}

#[derive(Deserialize, Default)]
struct MetadataUsuario {
    #[serde(default)]
    debe_cambiar_password: bool,
}

fn mapear_respuesta_error(status: reqwest::StatusCode) -> AuthSupabaseError {
    if status == reqwest::StatusCode::BAD_REQUEST || status == reqwest::StatusCode::UNAUTHORIZED {
        AuthSupabaseError::CredencialesInvalidas
    } else {
        AuthSupabaseError::RespuestaInvalida
    }
}

fn respuesta_a_sesion(respuesta: RespuestaToken) -> SesionSupabase {
    SesionSupabase {
        access_token: respuesta.access_token,
        refresh_token: respuesta.refresh_token,
        expires_in: respuesta.expires_in,
        usuario_id: respuesta.user.id,
        debe_cambiar_password: respuesta.user.user_metadata.debe_cambiar_password,
    }
}

/// `base_url`/`apikey` como parámetros (no `super::BASE_URL`/`APIKEY`
/// directo) para poder apuntar a un servidor de prueba -- mismo criterio
/// que `ContextoSincronizacion` en `sincronizacion.rs`.
pub fn login(
    base_url: &str,
    apikey: &str,
    cedula: &str,
    password: &str,
) -> Result<SesionSupabase, AuthSupabaseError> {
    let cliente = cliente_http();
    let respuesta = cliente
        .post(format!("{base_url}/auth/v1/token?grant_type=password"))
        .header("apikey", apikey)
        .json(&CuerpoLoginPassword {
            email: &email_sintetico(cedula),
            password,
        })
        .send()
        .map_err(|error| AuthSupabaseError::Red(error.to_string()))?;

    if !respuesta.status().is_success() {
        return Err(mapear_respuesta_error(respuesta.status()));
    }
    let cuerpo: RespuestaToken = respuesta
        .json()
        .map_err(|_| AuthSupabaseError::RespuestaInvalida)?;
    Ok(respuesta_a_sesion(cuerpo))
}

/// Renueva sin volver a pedir contraseña -- ver "renovación silenciosa en
/// segundo plano" en docs/plan-autenticacion-supabase-auth.md. Sólo se
/// invoca mientras el proceso sigue vivo y hay red; el `refresh_token`
/// nunca se persiste a disco (vive en memoria, ver `GuiState`).
pub fn refrescar(
    base_url: &str,
    apikey: &str,
    refresh_token: &str,
) -> Result<SesionSupabase, AuthSupabaseError> {
    let cliente = cliente_http();
    let respuesta = cliente
        .post(format!("{base_url}/auth/v1/token?grant_type=refresh_token"))
        .header("apikey", apikey)
        .json(&CuerpoRefresh { refresh_token })
        .send()
        .map_err(|error| AuthSupabaseError::Red(error.to_string()))?;

    if !respuesta.status().is_success() {
        return Err(AuthSupabaseError::TokenInvalido);
    }
    let cuerpo: RespuestaToken = respuesta
        .json()
        .map_err(|_| AuthSupabaseError::RespuestaInvalida)?;
    Ok(respuesta_a_sesion(cuerpo))
}

#[derive(Serialize)]
struct CuerpoActualizarPassword<'a> {
    password: &'a str,
    data: serde_json::Value,
}

/// Revalida `password_actual` con un login real (no confía en que la
/// sesión siga abierta -- ver "Cambiar contraseña (rutina)" en el plan) y
/// recién entonces cambia a `password_nueva`, limpiando
/// `debe_cambiar_password`. `cedula` hace falta para la revalidación.
pub fn cambiar_password(
    base_url: &str,
    apikey: &str,
    access_token: &str,
    cedula: &str,
    password_actual: &str,
    password_nueva: &str,
) -> Result<(), AuthSupabaseError> {
    login(base_url, apikey, cedula, password_actual)?;

    let cliente = cliente_http();
    let respuesta = cliente
        .put(format!("{base_url}/auth/v1/user"))
        .header("apikey", apikey)
        .bearer_auth(access_token)
        .json(&CuerpoActualizarPassword {
            password: password_nueva,
            data: serde_json::json!({ "debe_cambiar_password": false }),
        })
        .send()
        .map_err(|error| AuthSupabaseError::Red(error.to_string()))?;

    if !respuesta.status().is_success() {
        return Err(AuthSupabaseError::RespuestaInvalida);
    }
    Ok(())
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

/// Una clave pública EC (P-256/ES256) del JWKS de Supabase Auth --
/// confirmado que este proyecto ya las usa (`auth/v1/.well-known/jwks.json`
/// devuelve 3 claves `"kty":"EC"`), no hace falta migrar nada del lado de
/// Supabase para verificar offline.
#[derive(Debug, Clone, Deserialize)]
pub struct Jwk {
    kid: String,
    alg: String,
    x: String,
    y: String,
}

/// Pega a la red -- pensada para llamarse UNA vez por proceso y cachear el
/// resultado del lado de quien la llama (`GuiState` en desktop, ver
/// `docs/plan-autenticacion-supabase-auth.md`: las claves públicas de
/// firma de Supabase Auth rotan con poca frecuencia, no hace falta
/// pedirlas de nuevo en cada verificación). Separada de
/// `verificar_token_offline` a propósito: esa queda pura -- sin red, sin
/// caché propio adentro -- así se puede probar con un JWKS de prueba sin
/// levantar un servidor por cada aserción.
pub fn obtener_jwks(base_url: &str) -> Result<Vec<Jwk>, AuthSupabaseError> {
    let cliente = cliente_http();
    let respuesta = cliente
        .get(format!("{base_url}/auth/v1/.well-known/jwks.json"))
        .send()
        .map_err(|error| AuthSupabaseError::Red(error.to_string()))?;
    if !respuesta.status().is_success() {
        return Err(AuthSupabaseError::RespuestaInvalida);
    }
    let jwks: Jwks = respuesta
        .json()
        .map_err(|_| AuthSupabaseError::RespuestaInvalida)?;
    Ok(jwks.keys)
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Serialize))]
struct ClaimsToken {
    sub: String,
    #[allow(dead_code)]
    exp: usize,
}

/// Verifica la firma de `access_token` **sin red**, contra un JWKS que
/// quien llama ya tiene cacheado (ver `obtener_jwks`) -- esto es lo que
/// permite seguir operando offline con una sesión ya obtenida, ver
/// "Verificación offline del token" en el plan. Devuelve el `sub` (mismo
/// `usuario_id` que ya trae `SesionSupabase`) si la firma y la expiración
/// son válidas.
pub fn verificar_token_offline(
    claves: &[Jwk],
    access_token: &str,
) -> Result<String, AuthSupabaseError> {
    let cabecera =
        jsonwebtoken::decode_header(access_token).map_err(|_| AuthSupabaseError::TokenInvalido)?;
    let kid = cabecera.kid.ok_or(AuthSupabaseError::TokenInvalido)?;

    let clave = claves
        .iter()
        .find(|clave| clave.kid == kid && clave.alg == "ES256")
        .ok_or(AuthSupabaseError::TokenInvalido)?;

    let decoding_key = DecodingKey::from_ec_components(&clave.x, &clave.y)
        .map_err(|_| AuthSupabaseError::TokenInvalido)?;
    let mut validacion = Validation::new(Algorithm::ES256);
    validacion.validate_aud = false;

    let datos = decode::<ClaimsToken>(access_token, &decoding_key, &validacion)
        .map_err(|_| AuthSupabaseError::TokenInvalido)?;
    Ok(datos.claims.sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn servidor_con_respuesta(cuerpo: &'static str, status: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buffer = [0; 4096];
            let _ = socket.read(&mut buffer);
            write!(
                socket,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
                cuerpo.len()
            )
            .unwrap();
        });
        base_url
    }

    #[test]
    fn login_exitoso_devuelve_sesion_con_metadata() {
        let base_url = servidor_con_respuesta(
            "{\"access_token\":\"tok\",\"refresh_token\":\"ref\",\"expires_in\":3600,\
             \"user\":{\"id\":\"uuid-1\",\"user_metadata\":{\"debe_cambiar_password\":true}}}",
            "200 OK",
        );

        let sesion = login(&base_url, "apikey-test", "1-0847-0293", "clave").unwrap();

        assert_eq!(sesion.access_token, "tok");
        assert_eq!(sesion.refresh_token, "ref");
        assert_eq!(sesion.usuario_id, "uuid-1");
        assert!(sesion.debe_cambiar_password);
    }

    #[test]
    fn login_sin_metadata_no_exige_cambiar_password() {
        let base_url = servidor_con_respuesta(
            "{\"access_token\":\"tok\",\"refresh_token\":\"ref\",\"expires_in\":3600,\
             \"user\":{\"id\":\"uuid-1\"}}",
            "200 OK",
        );

        let sesion = login(&base_url, "apikey-test", "1-0847-0293", "clave").unwrap();

        assert!(!sesion.debe_cambiar_password);
    }

    #[test]
    fn login_401_es_credenciales_invalidas() {
        let base_url = servidor_con_respuesta("{\"error\":\"invalid_grant\"}", "400 Bad Request");

        let error = login(&base_url, "apikey-test", "1-0847-0293", "clave").unwrap_err();

        assert!(matches!(error, AuthSupabaseError::CredencialesInvalidas));
    }

    #[test]
    fn refrescar_exitoso_devuelve_sesion_nueva() {
        let base_url = servidor_con_respuesta(
            "{\"access_token\":\"tok2\",\"refresh_token\":\"ref2\",\"expires_in\":3600,\
             \"user\":{\"id\":\"uuid-1\"}}",
            "200 OK",
        );

        let sesion = refrescar(&base_url, "apikey-test", "ref-viejo").unwrap();

        assert_eq!(sesion.access_token, "tok2");
    }

    #[test]
    fn refrescar_con_token_vencido_da_token_invalido() {
        let base_url = servidor_con_respuesta("{\"error\":\"invalid_grant\"}", "401 Unauthorized");

        let error = refrescar(&base_url, "apikey-test", "ref-vencido").unwrap_err();

        assert!(matches!(error, AuthSupabaseError::TokenInvalido));
    }

    // Par de claves ES256/P-256 de prueba, generado sólo para este archivo
    // (`openssl ecparam -name prime256v1 -genkey -noout`) -- no es ningún
    // secreto real, sólo sirve para firmar tokens de prueba y verificarlos
    // contra un JWK con los mismos x/y. Hermético: nada de estos tests
    // pega a la red real de Supabase, a diferencia de una primera versión
    // que sí lo hacía -- no encajaba con el resto del repo (todo mockeado
    // con un servidor local).
    const CLAVE_PRIVADA_PRUEBA_PEM: &[u8] = b"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgqf3KacRIXB4YC4i0
EDrCOWYLtx3al+MCyOVIsa1eaNShRANCAAQtVYE1t9ww8fbrntdrrcb7MUs9j+UU
PnUgQ3hxgOU/ss9vt0d+pRtbxc6xPUI9kgyjGL+AOVg/1noujkf1Gxjo
-----END PRIVATE KEY-----";
    const CLAVE_PRUEBA_X: &str = "LVWBNbfcMPH2657Xa63G-zFLPY_lFD51IEN4cYDlP7I";
    const CLAVE_PRUEBA_Y: &str = "z2-3R36lG1vFzrE9Qj2SDKMYv4A5WD_Wei6OR_UbGOg";
    const CLAVE_PRUEBA_KID: &str = "kid-de-prueba";

    fn jwk_de_prueba() -> Jwk {
        Jwk {
            kid: CLAVE_PRUEBA_KID.to_string(),
            alg: "ES256".to_string(),
            x: CLAVE_PRUEBA_X.to_string(),
            y: CLAVE_PRUEBA_Y.to_string(),
        }
    }

    fn firmar_token_de_prueba(sub: &str, exp: usize) -> String {
        let encoding_key =
            jsonwebtoken::EncodingKey::from_ec_pem(CLAVE_PRIVADA_PRUEBA_PEM).unwrap();
        let mut header = jsonwebtoken::Header::new(Algorithm::ES256);
        header.kid = Some(CLAVE_PRUEBA_KID.to_string());
        jsonwebtoken::encode(
            &header,
            &ClaimsToken {
                sub: sub.to_string(),
                exp,
            },
            &encoding_key,
        )
        .unwrap()
    }

    #[test]
    fn verificar_token_offline_acepta_una_firma_valida() {
        let token = firmar_token_de_prueba("uuid-usuario-1", 9_999_999_999);

        let sub = verificar_token_offline(&[jwk_de_prueba()], &token).unwrap();

        assert_eq!(sub, "uuid-usuario-1");
    }

    #[test]
    fn verificar_token_offline_rechaza_un_token_vencido() {
        let token = firmar_token_de_prueba("uuid-usuario-1", 1);

        let error = verificar_token_offline(&[jwk_de_prueba()], &token).unwrap_err();

        assert!(matches!(error, AuthSupabaseError::TokenInvalido));
    }

    #[test]
    fn verificar_token_offline_rechaza_un_kid_que_no_esta_en_el_jwks() {
        let token = firmar_token_de_prueba("uuid-usuario-1", 9_999_999_999);

        // JWKS vacío -- simula una clave rotada/retirada que el JWKS
        // cacheado todavía no conoce (o nunca conoció).
        let error = verificar_token_offline(&[], &token).unwrap_err();

        assert!(matches!(error, AuthSupabaseError::TokenInvalido));
    }

    #[test]
    fn verificar_token_offline_rechaza_una_firma_de_otra_clave() {
        // Mismo `kid` que declara el JWKS, pero con componentes x/y de otra
        // clave -- el token no pudo haber sido firmado por ESA clave
        // privada. Prueba que la verificación de verdad valida la firma
        // criptográfica, no sólo que el `kid` matchee por nombre.
        let token = firmar_token_de_prueba("uuid-usuario-1", 9_999_999_999);
        let jwk_equivocado = Jwk {
            kid: CLAVE_PRUEBA_KID.to_string(),
            alg: "ES256".to_string(),
            x: "AWTDNmJtPkKUIXtozfLh8927frXlGAaPJxgeTZAAAq4".to_string(),
            y: "PftoDIDzC-IMCRNoUcgC8RmVEKObkPYoBwo4W_hodpo".to_string(),
        };

        let error = verificar_token_offline(&[jwk_equivocado], &token).unwrap_err();

        assert!(matches!(error, AuthSupabaseError::TokenInvalido));
    }

    #[test]
    fn obtener_jwks_trae_las_claves_del_servidor_mock() {
        let base_url = servidor_con_respuesta(
            "{\"keys\":[{\"kid\":\"k1\",\"alg\":\"ES256\",\"kty\":\"EC\",\"crv\":\"P-256\",\
             \"x\":\"LVWBNbfcMPH2657Xa63G-zFLPY_lFD51IEN4cYDlP7I\",\
             \"y\":\"z2-3R36lG1vFzrE9Qj2SDKMYv4A5WD_Wei6OR_UbGOg\"}]}",
            "200 OK",
        );

        let claves = obtener_jwks(&base_url).unwrap();

        assert_eq!(claves.len(), 1);
        assert_eq!(claves[0].kid, "k1");
    }
}

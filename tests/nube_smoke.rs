//! Prueba manual contra el receptor real en Supabase (ver
//! `docs/plan-persistencia-nube.md`). No corre en `cargo test` normal —
//! depende de red y de un secreto de dispositivo real. Se ejecuta a mano:
//!
//! ```text
//! CONTROL_ACCESO_NUBE_SECRETO=<secreto> cargo test --features nube \
//!     --test nube_smoke -- --ignored --nocapture
//! ```

#![cfg(feature = "nube")]

use control_acceso::nube::autenticar_dispositivo;

const BASE_URL: &str = "https://xidaepyaljzkpbsxrqsm.supabase.co";

#[test]
#[ignore = "depende de red y de un secreto de dispositivo real, ver doc-comment del módulo"]
fn autentica_un_dispositivo_real_y_recibe_un_token() {
    let secreto = std::env::var("CONTROL_ACCESO_NUBE_SECRETO")
        .expect("definí CONTROL_ACCESO_NUBE_SECRETO con un secreto de dispositivo válido");

    let token = autenticar_dispositivo(BASE_URL, &secreto).expect("la autenticación no falló");

    assert!(!token.access_token.is_empty());
    assert_eq!(token.expires_in, 3600);
    println!("sitio_id={} dispositivo_id={} tipo={}", token.sitio_id, token.dispositivo_id, token.tipo);
}

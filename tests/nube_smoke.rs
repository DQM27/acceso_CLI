//! Prueba manual contra el receptor real en Supabase (ver
//! `docs/planes-implementados/plan-persistencia-nube.md` y
//! `docs/handoff-registro-dispositivos.md`). No corre en `cargo test`
//! normal — depende de red y de un equipo ya vinculado (su clave privada en
//! `dispositivo-nube.clave`). Se ejecuta a mano, idealmente contra staging:
//!
//! ```text
//! CONTROL_ACCESO_SUPABASE_URL=https://pmrytjktlyiuikxuuxpr.supabase.co \
//! CONTROL_ACCESO_SUPABASE_APIKEY=<publishable de staging> \
//! CONTROL_ACCESO_NUBE_DIRECTORIO_CLAVE=<carpeta con dispositivo-nube.clave> \
//!     cargo test --features nube --test nube_smoke -- --ignored --nocapture
//! ```
//!
//! Para obtener esa carpeta sin la app, ver `examples/probar_vinculacion.rs`.

#![cfg(feature = "nube")]

use std::path::PathBuf;
use std::sync::Arc;

use control_acceso::nube::{CacheTokenDispositivo, FirmanteArchivo};

#[test]
#[ignore = "depende de red y de un equipo ya vinculado, ver doc-comment del módulo"]
fn autentica_un_dispositivo_real_con_su_clave_y_recibe_un_token() {
    let directorio = std::env::var_os("CONTROL_ACCESO_NUBE_DIRECTORIO_CLAVE")
        .map(PathBuf::from)
        .expect("definí CONTROL_ACCESO_NUBE_DIRECTORIO_CLAVE con la carpeta de la clave");
    let cache = CacheTokenDispositivo::new();
    cache.establecer_firmante(Arc::new(FirmanteArchivo::en(&directorio)));
    assert!(
        cache.vinculado(),
        "esa carpeta no tiene un equipo vinculado"
    );

    let token = cache
        .autenticar_y_cachear(None)
        .expect("la autenticación no falló");

    assert!(!token.access_token.is_empty());
    assert_eq!(token.expires_in, 3600);
    println!(
        "sitio_id={} dispositivo_id={} tipo={}",
        token.sitio_id, token.dispositivo_id, token.tipo
    );
}

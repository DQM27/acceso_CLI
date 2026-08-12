use control_acceso::services::error::PasswordError;
use control_acceso::services::password::{generar_hash, verificar_password};

#[test]
fn genera_hash_distinto_del_password_y_verifica_el_correcto() {
    let hash = generar_hash("password-seguro").unwrap();
    assert_ne!(hash, "password-seguro");
    assert!(verificar_password("password-seguro", &hash).unwrap());
}

#[test]
fn password_incorrecto_no_verifica() {
    let hash = generar_hash("password-seguro").unwrap();
    assert!(!verificar_password("otro-password", &hash).unwrap());
}

#[test]
fn hashes_de_la_misma_password_usan_salts_distintos() {
    assert_ne!(
        generar_hash("password-seguro").unwrap(),
        generar_hash("password-seguro").unwrap()
    );
}

#[test]
fn hash_corrupto_devuelve_error() {
    assert!(matches!(
        verificar_password("password-seguro", "hash inválido"),
        Err(PasswordError::HashInvalido)
    ));
}

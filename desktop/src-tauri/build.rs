fn main() {
    // Hash corto del commit, para que `version_app` de la telemetría diga de
    // qué código salió cada build (`desktop-1.6.8-diag+abc1234`). Sin git a
    // mano, "sinhash".
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|salida| salida.status.success())
        .and_then(|salida| String::from_utf8(salida.stdout).ok())
        .map(|texto| texto.trim().to_string())
        .filter(|texto| !texto.is_empty())
        .unwrap_or_else(|| "sinhash".to_string());
    println!("cargo:rustc-env=BUILD_COMMIT_HASH={hash}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    tauri_build::build();
}

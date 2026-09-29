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
    // `.git/HEAD` sólo cambia al cambiar de rama: un commit o un pull en la
    // misma rama mueve el archivo de la rama (o `packed-refs`), así que se
    // vigilan los tres o el hash quedaría viejo en el binario. Sólo los que
    // existen: cargo trata una ruta ausente como "siempre cambió".
    let git = "../../.git";
    println!("cargo:rerun-if-changed={git}/HEAD");
    if let Ok(head) = std::fs::read_to_string(format!("{git}/HEAD"))
        && let Some(referencia) = head.strip_prefix("ref: ")
    {
        let archivo = format!("{git}/{}", referencia.trim());
        if std::path::Path::new(&archivo).exists() {
            println!("cargo:rerun-if-changed={archivo}");
        }
    }
    if std::path::Path::new(&format!("{git}/packed-refs")).exists() {
        println!("cargo:rerun-if-changed={git}/packed-refs");
    }
    tauri_build::build();
}

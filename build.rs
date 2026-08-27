fn main() {
    // Sólo para el binario de consola (feature `terminal-ui`, encendida por
    // defecto). Cuando `desktop/src-tauri` depende de este crate como
    // librería con `default-features = false`, este build script igual se
    // ejecuta — sin este chequeo, el recurso .ico/versión que arma
    // `winresource` se enlaza también dentro del binario GUI y choca con el
    // que ya incrusta `tauri-build` ("duplicate leaf" del linker).
    let compilando_consola = std::env::var("CARGO_FEATURE_TERMINAL_UI").is_ok();
    if compilando_consola && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .set("ProductName", "Control de Acceso")
            .set("FileDescription", "Control de Acceso")
            .set(
                "Comments",
                "Administra empresas, contratistas, usuarios e ingresos/salidas de una instalación",
            )
            .set("CompanyName", "DQM27")
            .set("LegalCopyright", "© DQM27")
            .compile()
            .expect("no se pudo incrustar el icono en el ejecutable");
    }
}

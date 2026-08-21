fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
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

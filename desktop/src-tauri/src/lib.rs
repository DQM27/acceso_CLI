use control_acceso::application::AppCore;
use control_acceso::database::connection::ruta_base_datos;
use control_acceso::instancia::InstanciaGuard;

mod comandos;
mod dto;
mod estado;

use estado::GuiState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let ruta_base_datos =
        ruta_base_datos().expect("no se pudo resolver la ruta de la base de datos");
    let instancia = InstanciaGuard::adquirir(&ruta_base_datos).unwrap_or_else(|error| {
        panic!(
            "no se pudo adquirir el candado de instancia (¿ya hay otra ventana abierta con esta \
             misma base de datos?): {error}"
        )
    });
    let core = AppCore::abrir(&ruta_base_datos).expect("no se pudo abrir la base de datos");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(GuiState::new(core, instancia))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::autenticacion::requiere_configuracion_inicial,
            comandos::autenticacion::login,
            comandos::autenticacion::cerrar_sesion,
            comandos::contratistas::buscar_contratistas,
            comandos::contratistas::crear_contratista,
            comandos::contratistas::actualizar_contratista,
            comandos::empresas::listar_empresas,
            comandos::empresas::buscar_empresas,
            comandos::empresas::crear_empresa,
            comandos::empresas::actualizar_empresa,
            comandos::empresas::establecer_empresa_activa,
            comandos::usuarios::buscar_usuarios,
            comandos::usuarios::crear_usuario,
            comandos::usuarios::actualizar_usuario,
            comandos::usuarios::cambiar_password_usuario,
            comandos::usuarios::cambiar_mi_password,
            comandos::ingresos::listar_ingresos_activos,
            comandos::ingresos::preparar_ingreso,
            comandos::ingresos::registrar_ingreso,
            comandos::ingresos::registrar_salida,
            comandos::consola::ejecutar_comando,
            comandos::consola::autocompletar_comando,
            comandos::historial::listar_historial,
            comandos::historial::exportar_historial,
            comandos::auditoria::listar_auditoria,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

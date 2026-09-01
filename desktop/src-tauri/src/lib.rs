use control_acceso::application::AppCore;
use control_acceso::database::connection::ruta_base_datos;
use control_acceso::instancia::InstanciaGuard;

mod comandos;
mod dto;
mod estado;
mod pdf;

use estado::GuiState;

/// Muestra un diálogo nativo con el error y termina el proceso — para los
/// fallos de arranque previos a `tauri::Builder` (base dañada/bloqueada,
/// doble instancia), donde antes había un `.expect()`/`panic!` crudo sin
/// ventana ni mensaje legible para quien no lee consola.
fn mostrar_error_fatal_y_salir(mensaje: &str) -> ! {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        use windows::core::HSTRING;
        let texto = HSTRING::from(mensaje);
        let titulo = HSTRING::from("Control de Acceso — Error al iniciar");
        unsafe {
            MessageBoxW(None, &texto, &titulo, MB_OK | MB_ICONERROR);
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("{mensaje}");
    }
    std::process::exit(1)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Inicia la aplicación de escritorio y registra todos los comandos Tauri.
///
/// # Panics
///
/// Tauri finaliza el arranque si no puede construir o ejecutar su runtime.
pub fn run() {
    let ruta_base_datos = ruta_base_datos().unwrap_or_else(|error| {
        mostrar_error_fatal_y_salir(&format!(
            "No se pudo resolver la ruta de la base de datos: {error}"
        ))
    });
    let instancia = InstanciaGuard::adquirir(&ruta_base_datos).unwrap_or_else(|error| {
        mostrar_error_fatal_y_salir(&format!(
            "No se pudo adquirir el candado de instancia (¿ya hay otra ventana abierta con esta \
             misma base de datos?): {error}"
        ))
    });
    let core = AppCore::abrir(&ruta_base_datos).unwrap_or_else(|error| {
        mostrar_error_fatal_y_salir(&format!("No se pudo abrir la base de datos: {error}"))
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .manage(GuiState::new(core, instancia))
        .setup(|app| {
            // El updater no existe en móvil — esta app es 100% escritorio (ver
            // el comentario de crate-type arriba), pero se guarda el gate
            // igual, mismo criterio que el ejemplo oficial de Tauri.
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

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
            comandos::historial::listar_historial,
            comandos::historial::exportar_historial,
            comandos::historial::exportar_historial_pdf,
            comandos::auditoria::listar_auditoria,
            comandos::auditoria::listar_auditoria_gafetes,
            comandos::gafetes::buscar_gafetes,
            comandos::gafetes::historial_gafete,
            comandos::gafetes::crear_gafete,
            comandos::gafetes::crear_gafetes_rango,
            comandos::gafetes::dar_de_baja_gafete,
            comandos::gafetes::marcar_gafete_perdido,
            comandos::gafetes::resolver_gafete,
            comandos::respaldos::crear_respaldo,
            comandos::respaldos::listar_respaldos,
            comandos::respaldos::validar_respaldo,
            comandos::respaldos::exportar_respaldo,
            comandos::respaldos::restaurar_respaldo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

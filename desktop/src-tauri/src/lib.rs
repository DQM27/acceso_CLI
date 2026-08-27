use std::sync::Mutex;

use control_acceso::application::AppCore;
use control_acceso::database::connection::ruta_base_datos;
use control_acceso::database::queries::contratistas::{FiltroContratistas, PaginaContratistas};
use control_acceso::instancia::InstanciaGuard;
use control_acceso::services::autenticacion_service::{UsuarioSesion, verificar_candidato};

/// Estado administrado por Tauri. Dos mutexes separados porque ningún flujo
/// necesita actualizar sesión y base de datos como una sola operación atómica
/// (ver docs/plan-tauri.md, sección "Estado y sesión").
struct GuiState {
    core: Mutex<AppCore>,
    sesion: Mutex<Option<UsuarioSesion>>,
    /// Mantiene el candado de instancia vivo mientras dure la app — nunca se
    /// lee, sólo existe para que no se libere antes de tiempo (mismo patrón
    /// que `main.rs` con `_instancia`).
    _instancia: InstanciaGuard,
}

#[tauri::command]
fn requiere_configuracion_inicial(state: tauri::State<GuiState>) -> Result<bool, String> {
    state
        .core
        .lock()
        .unwrap()
        .requiere_configuracion_inicial()
        .map_err(|error| error.to_string())
}

/// Login en dos pasos, igual que la TUI (ver `AutenticacionService`), pero sin
/// el canal `mpsc` que ella necesita: Tauri ya despacha este comando en su
/// propio pool de hilos, así que el cálculo de Argon2 no congela la ventana.
/// Lo que sí se preserva es soltar el lock de `core` ANTES de verificar la
/// contraseña, para no bloquear otros comandos durante el hash.
#[tauri::command]
fn login(
    cedula: String,
    password: String,
    state: tauri::State<GuiState>,
) -> Result<UsuarioSesion, String> {
    let candidato = {
        let core = state.core.lock().unwrap();
        core.buscar_candidato_autenticacion(&cedula)
            .map_err(|error| error.to_string())?
    };
    let sesion = verificar_candidato(candidato, &password).map_err(|error| error.to_string())?;
    *state.sesion.lock().unwrap() = Some(sesion.clone());
    Ok(sesion)
}

#[tauri::command]
fn cerrar_sesion(state: tauri::State<GuiState>) {
    *state.sesion.lock().unwrap() = None;
}

/// Búsqueda mínima por texto libre — el filtro estructurado completo
/// (`clave:valor`, PRAIND, empresa) se suma cuando la pantalla lo necesite de
/// verdad, no por anticipación.
#[tauri::command]
fn buscar_contratistas(
    texto: String,
    state: tauri::State<GuiState>,
) -> Result<PaginaContratistas, String> {
    let texto = texto.trim();
    let filtro = FiltroContratistas {
        texto: (!texto.is_empty()).then(|| texto.to_owned()),
        ..Default::default()
    };
    state
        .core
        .lock()
        .unwrap()
        .buscar_contratistas(&filtro)
        .map_err(|error| error.to_string())
}

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
        .manage(GuiState {
            core: Mutex::new(core),
            sesion: Mutex::new(None),
            _instancia: instancia,
        })
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
            requiere_configuracion_inicial,
            login,
            cerrar_sesion,
            buscar_contratistas
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

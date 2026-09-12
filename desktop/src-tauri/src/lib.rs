use std::sync::Arc;
use std::time::Duration;

use control_acceso::application::AppCore;
use control_acceso::database::connection::ruta_base_datos;
use control_acceso::instancia::InstanciaGuard;
use control_acceso::tiempo::RelojCorregido;
use tauri::{Emitter, Manager};

#[cfg(windows)]
mod clave_cifrado;
mod comandos;
mod dto;
mod estado;
mod pdf;

use estado::GuiState;

/// Cada cuánto reintenta la sincronización automática mientras la app sigue
/// abierta. Realtime dispara sincronizaciones bajo demanda, pero este pulso
/// queda como respaldo cuando el socket no está conectado o se pierde un
/// evento.
const INTERVALO_SINCRONIZACION_AUTOMATICA: Duration = Duration::from_secs(2 * 60);
/// Antes del primer intento, para no competir con el arranque de la ventana.
const ESPERA_INICIAL_SINCRONIZACION: Duration = Duration::from_secs(10);

/// Sincronización con la nube en segundo plano, sin que nadie tenga que
/// apretar "Sincronizar ahora". Silencioso en todo lo que no sea un envío
/// exitoso: sin sesión activa, sesión sin permiso (`Operacion::GestionarNube`
/// es exclusivo de Root), o sin secreto de dispositivo todavía configurado
/// no son errores acá, son estados normales antes/entre sesiones -- no hay
/// consola donde mostrar nada, y no tiene sentido interrumpir a un
/// Administrador u Operador con un fallo de una función que ni les
/// corresponde. Un error de red real tampoco se anuncia: se reintenta solo
/// en la próxima vuelta.
fn iniciar_sincronizacion_automatica(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(ESPERA_INICIAL_SINCRONIZACION).await;
        loop {
            let manejador = app.clone();
            let resultado = tauri::async_runtime::spawn_blocking(move || {
                let estado = manejador.state::<GuiState>();
                comandos::nube::ejecutar_sincronizacion(&estado)
            })
            .await;

            if let Ok(Ok(resumen)) = resultado {
                let _ = app.emit("nube://sincronizado", resumen);
            }

            tokio::time::sleep(INTERVALO_SINCRONIZACION_AUTOMATICA).await;
        }
    });
}

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
    // Clave de SQLCipher: nunca hardcodeada ni derivada de un identificador
    // adivinable (Machine GUID, usuario, MAC...) -- 32 bytes aleatorios,
    // generados una sola vez y protegidos en disco con DPAPI (scope del
    // usuario actual de Windows). Ver `clave_cifrado.rs` para el porqué.
    let directorio_base_datos = ruta_base_datos.parent().unwrap_or_else(|| {
        mostrar_error_fatal_y_salir("No se pudo resolver el directorio de la base de datos")
    });
    let clave_base_datos =
        clave_cifrado::resolver_clave(directorio_base_datos, &ruta_base_datos).unwrap_or_else(
            |error| mostrar_error_fatal_y_salir(&format!("No se pudo resolver la clave de cifrado de la base de datos: {error}")),
        );
    // `RelojCorregido`, no `RelojSistema`: en equipos cuyo reloj de Windows
    // no se puede corregir (visto en producción, ~11 min adelantado y sin
    // sincronizar), cada autenticación contra la nube mide el desfase real
    // contra el receptor y lo aplica acá -- ver
    // `application::nube::AppCore::actualizar_desfase_reloj`. Sin nube
    // configurada nunca se mide nada y este reloj se comporta igual que
    // `RelojSistema`.
    let core = AppCore::abrir_con_reloj_cifrado(
        &ruta_base_datos,
        &clave_base_datos,
        Arc::new(RelojCorregido::nuevo()),
    )
    .unwrap_or_else(|error| {
        mostrar_error_fatal_y_salir(&format!("No se pudo abrir la base de datos: {error}"))
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .manage(GuiState::new(
            core,
            instancia,
            ruta_base_datos,
            clave_base_datos,
        ))
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

            iniciar_sincronizacion_automatica(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::autenticacion::requiere_configuracion_inicial,
            comandos::autenticacion::login,
            comandos::autenticacion::cambiar_password_supabase,
            comandos::autenticacion::cerrar_sesion,
            comandos::contratistas::buscar_contratistas,
            comandos::contratistas::crear_contratista,
            comandos::contratistas::actualizar_contratista,
            comandos::empresas::listar_empresas,
            comandos::empresas::buscar_empresas,
            comandos::empresas::crear_empresa,
            comandos::empresas::actualizar_empresa,
            comandos::empresas::establecer_empresa_activa,
            comandos::usuarios::cambiar_mi_password,
            comandos::ingresos::listar_ingresos_activos,
            comandos::ingresos::preparar_ingreso,
            comandos::ingresos::registrar_ingreso,
            comandos::ingresos::registrar_salida,
            comandos::citas::verificar_check_in_visita,
            comandos::citas::registrar_entrada_visita,
            comandos::citas::registrar_salida_visita,
            comandos::citas::listar_visitas_activas,
            comandos::citas::listar_historial_visitas_sitio,
            comandos::citas::listar_agenda_visitas,
            comandos::historial::listar_historial,
            comandos::historial::listar_historial_sitio,
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
            comandos::nube::configurar_dispositivo_inicial,
            comandos::nube::sincronizar_con_nube,
            comandos::nube::sesion_realtime_nube,
            comandos::nube::listar_ingresos_remotos,
            comandos::nube::cerrar_ingreso_remoto,
            comandos::nube::fallos_permanentes_nube,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use control_acceso::application::AppCore;
use control_acceso::database::connection::ruta_base_datos;
use control_acceso::instancia::InstanciaGuard;
use control_acceso::tiempo::RelojCorregido;
use tauri::{Emitter, Manager};
use zeroize::Zeroizing;

#[cfg(windows)]
mod clave_cifrado;
mod comandos;
mod dto;
mod estado;
mod pdf;
#[cfg(windows)]
mod recuperacion_local;

use estado::GuiState;

/// DSN del proyecto `control-acceso-desktop` en Sentry -- no es un secreto
/// (está pensado para ir embebido en el cliente, misma lógica que la
/// publishable key de Supabase en `web/`), pero se deja overrideable en
/// tiempo de compilación por si algún día se separa un proyecto de Sentry
/// para QA/staging.
const SENTRY_DSN: &str = match option_env!("SENTRY_DSN") {
    Some(dsn) => dsn,
    None => {
        "https://644be3689422e0fa65551a7f3f8a28ff@o4512103124041728.ingest.us.sentry.io/4512104588115968"
    }
};

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

            // Antes los dos casos de error acá quedaban en silencio total --
            // ni el usuario los veía (es automático, sin botón que falle a
            // la vista) ni quedaba ningún rastro (ver
            // docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md,
            // "Observabilidad"/"nada de fallos silenciosos"). Corre después
            // de `configurar_plugins_condicionales`, así que el archivo de
            // log ya está activo acá.
            match resultado {
                Ok(Ok(resumen)) => {
                    let _ = app.emit("nube://sincronizado", resumen);
                }
                Ok(Err(error)) => {
                    log::warn!("sincronización automática falló: {error}");
                    sentry::capture_message(
                        &format!("sincronización automática falló: {error}"),
                        sentry::Level::Warning,
                    );
                }
                Err(error) => {
                    log::error!("tarea de sincronización automática no pudo ejecutarse: {error}");
                    sentry::capture_message(
                        &format!("tarea de sincronización automática no pudo ejecutarse: {error}"),
                        sentry::Level::Error,
                    );
                }
            }

            tokio::time::sleep(INTERVALO_SINCRONIZACION_AUTOMATICA).await;
        }
    });
}

/// Deja rastro de un fallo fatal de arranque en un archivo propio, sin
/// depender de `tauri_plugin_log` -- a esta altura todavía no existe
/// (`configurar_plugins_condicionales` corre recién dentro de `.setup()`,
/// después de tener una `AppHandle`; ver
/// docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md, punto 5.3).
/// Mismo directorio (`%LOCALAPPDATA%\<identifier>\logs`) donde el plugin
/// deja los suyos una vez que arranca, para que quien busque logs de este
/// sitio los encuentre todos juntos. Cualquier fallo acá adentro (no se
/// pudo leer `LOCALAPPDATA`, no se pudo crear el directorio/archivo) se
/// descarta en silencio a propósito: esta función corre en el peor
/// momento posible del arranque, no puede volverse ella misma un segundo
/// punto de fallo.
#[cfg(windows)]
fn registrar_fallo_fatal_en_archivo(mensaje: &str) {
    use std::io::Write;

    let Ok(local_app_data) = std::env::var("LOCALAPPDATA") else {
        return;
    };
    let directorio = std::path::Path::new(&local_app_data)
        .join("com.dqm27.lattis.desktop")
        .join("logs");
    if std::fs::create_dir_all(&directorio).is_err() {
        return;
    }
    let Ok(mut archivo) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directorio.join("fallo-fatal-arranque.log"))
    else {
        return;
    };
    let _ = writeln!(archivo, "[{}] {mensaje}", chrono::Utc::now().to_rfc3339());
}

/// Muestra un diálogo nativo con el error y termina el proceso — para los
/// fallos de arranque previos a `tauri::Builder` (base dañada/bloqueada,
/// doble instancia), donde antes había un `.expect()`/`panic!` crudo sin
/// ventana ni mensaje legible para quien no lee consola.
fn mostrar_error_fatal_y_salir(mensaje: &str) -> ! {
    #[cfg(windows)]
    registrar_fallo_fatal_en_archivo(mensaje);
    sentry::capture_message(mensaje, sentry::Level::Fatal);

    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        use windows::core::HSTRING;
        let texto = HSTRING::from(mensaje);
        let titulo = HSTRING::from("Control de Acceso — Error al iniciar");
        // SAFETY: `texto`/`titulo` son `HSTRING` válidas y viven hasta el
        // final de este bloque; `None` como `hwnd` es válido según la
        // documentación de `MessageBoxW` (sin ventana padre).
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

/// Resuelve la clave y abre `AppCore` una vez -- sin recuperación, eso vive
/// en `abrir_nucleo_con_recuperacion`. Unifica el error de `resolver_clave`
/// (`ErrorClaveBaseDatos`) y el de `abrir_con_reloj_cifrado`
/// (`BootstrapError`) a `String` porque a quien llama sólo le interesa
/// mostrarlo/reintentar, no distinguir de cuál de los dos vino.
#[cfg(windows)]
fn intentar_abrir_nucleo(
    directorio_credenciales: &Path,
    ruta_base_datos: &Path,
) -> Result<(Zeroizing<[u8; 32]>, AppCore), String> {
    let clave = clave_cifrado::resolver_clave(directorio_credenciales, ruta_base_datos)
        .map_err(|error| error.to_string())?;
    // `RelojCorregido`, no `RelojSistema`: en equipos cuyo reloj de Windows
    // no se puede corregir (visto en producción, ~11 min adelantado y sin
    // sincronizar), cada autenticación contra la nube mide el desfase real
    // contra el receptor y lo aplica acá -- ver
    // `application::nube::AppCore::actualizar_desfase_reloj`. Sin nube
    // configurada nunca se mide nada y este reloj se comporta igual que
    // `RelojSistema`.
    let core = AppCore::abrir_con_reloj_cifrado(
        ruta_base_datos,
        &clave,
        Arc::new(RelojCorregido::nuevo()),
    )
    .map_err(|error| error.to_string())?;
    Ok((clave, core))
}

/// Diálogo nativo Sí/No: ofrece reconstruir el sitio desde la nube tras un
/// fallo real de apertura (ver `intentar_abrir_nucleo`). No expone el error
/// técnico crudo -- mismo criterio que `mensajes::mensaje_*`, eso ya quedó
/// en el log/Sentry (`abrir_nucleo_con_recuperacion`).
#[cfg(windows)]
fn confirmar_reconstruccion_desde_nube() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONWARNING, MB_YESNO, MessageBoxW};
    use windows::core::HSTRING;

    let texto = HSTRING::from(
        "No se pudo abrir la base de datos local de este sitio (posible corrupción).\n\n\
         ¿Reconstruirla desde la nube?\n\n\
         Se va a perder, de forma permanente, el historial de auditoría y de \
         incidentes de gafetes de ESTE dispositivo, y cualquier operación \
         reciente que todavía no se hubiera subido. Todo lo demás \
         (contratistas, ingresos, gafetes, rutas, préstamos, etc.) se vuelve \
         a bajar solo, sin pasos adicionales.\n\n\
         Se guarda una copia del archivo dañado por si se puede rescatar más \
         adelante.",
    );
    let titulo = HSTRING::from("Control de Acceso — Base de datos dañada");
    // SAFETY: mismo criterio que `mostrar_error_fatal_y_salir` -- `HSTRING`
    // válidas y vivas hasta el final del bloque; `None` como `hwnd` es
    // válido sin ventana padre.
    let resultado = unsafe { MessageBoxW(None, &texto, &titulo, MB_YESNO | MB_ICONWARNING) };
    resultado == IDYES
}

/// En cualquier plataforma sin este diálogo (no debería alcanzarse -- la
/// app es Windows-only, ver el resto de este archivo), la respuesta segura
/// por defecto es no destruir nada.
#[cfg(not(windows))]
fn confirmar_reconstruccion_desde_nube() -> bool {
    false
}

/// Envoltorio de `intentar_abrir_nucleo` con un único reintento tras
/// ofrecer reconstruir desde la nube -- ver
/// `docs/recuperacion-sitio-local.md`. Separada de `preparar_nucleo` por el
/// mismo motivo que el resto de las funciones de esta sección (tope de
/// líneas de Clippy).
fn abrir_nucleo_con_recuperacion(
    directorio_credenciales: &Path,
    ruta_base_datos: &Path,
) -> (Zeroizing<[u8; 32]>, AppCore) {
    #[cfg(not(windows))]
    {
        intentar_abrir_nucleo(directorio_credenciales, ruta_base_datos)
            .unwrap_or_else(|error| mostrar_error_fatal_y_salir(&error))
    }
    #[cfg(windows)]
    {
        let error_original = match intentar_abrir_nucleo(directorio_credenciales, ruta_base_datos) {
            Ok(resultado) => return resultado,
            Err(error) => error,
        };
        log::error!("no se pudo abrir la base de datos local: {error_original}");
        sentry::capture_message(
            &format!("no se pudo abrir la base de datos local: {error_original}"),
            sentry::Level::Error,
        );
        if !confirmar_reconstruccion_desde_nube() {
            mostrar_error_fatal_y_salir(&error_original);
        }
        if let Err(error) = recuperacion_local::poner_en_cuarentena_y_reiniciar(
            ruta_base_datos,
            directorio_credenciales,
        ) {
            mostrar_error_fatal_y_salir(&format!(
                "No se pudo preparar el sitio para reconstruirlo: {error}"
            ));
        }
        intentar_abrir_nucleo(directorio_credenciales, ruta_base_datos).unwrap_or_else(|error| {
            mostrar_error_fatal_y_salir(&format!(
                "Tampoco se pudo crear una base nueva tras reconstruir: {error}"
            ))
        })
    }
}

/// Resuelve ruta/candado de instancia/clave de cifrado y abre `AppCore` --
/// separado de `run()` únicamente para mantenerla bajo el tope de líneas de
/// Clippy (`too_many_lines`); sin lógica propia, es el mismo arranque que
/// antes vivía inline.
fn preparar_nucleo() -> (PathBuf, InstanciaGuard, Zeroizing<[u8; 32]>, AppCore) {
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
    //
    // Deliberadamente NO vive en la misma carpeta que `control_acceso.db`
    // (`directorio_base_datos`, `%LOCALAPPDATA%`) desde 2026-09-18: un
    // evento que corrompe/borra esa carpeta se llevaba puesto tanto la
    // clave como el secreto de dispositivo (ver
    // `control_acceso::nube::credenciales::directorio_credenciales_roaming`
    // y `docs/recuperacion-sitio-local.md`). `%APPDATA%` es un árbol
    // separado -- mismo motivo, misma carpeta que ese secreto.
    let directorio_credenciales =
        control_acceso::nube::credenciales::directorio_credenciales_roaming().unwrap_or_else(
            || mostrar_error_fatal_y_salir("No se pudo resolver el directorio %APPDATA%"),
        );
    let (clave_base_datos, mut core) =
        abrir_nucleo_con_recuperacion(&directorio_credenciales, &ruta_base_datos);
    // Para que el receptor pueda aplicar `VERSION_MINIMA_ACEPTADA` en
    // cualquier renovación de token, no sólo al activar el dispositivo --
    // ver `AppCore::establecer_version_app` y
    // docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md, punto 9.
    core.establecer_version_app(env!("CARGO_PKG_VERSION"));

    (ruta_base_datos, instancia, clave_base_datos, core)
}

/// Registra los plugins que no se cargan siempre (updater fuera de móvil,
/// logging solo en debug) -- separado de `run()` únicamente para mantenerla
/// bajo el tope de líneas de Clippy (mismo motivo que `preparar_nucleo`).
fn configurar_plugins_condicionales(app: &tauri::AppHandle) -> tauri::Result<()> {
    // El updater no existe en móvil — esta app es 100% escritorio (ver
    // el comentario de crate-type arriba), pero se guarda el gate
    // igual, mismo criterio que el ejemplo oficial de Tauri.
    #[cfg(desktop)]
    app.plugin(tauri_plugin_updater::Builder::new().build())?;

    // Antes solo corría en debug -- en producción no quedaba ningún rastro
    // de qué pasó cuando algo falla en un sitio real (ver
    // docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md, "Logs
    // útiles"/"Observabilidad"). Los targets por defecto del plugin ya
    // escriben a un archivo rotado (`LogDir`, tope 40 KB, `KeepOne`) además
    // de stdout -- no hace falta configurar nada de eso a mano. Solo se baja
    // el nivel en release para no llenar el archivo con ruido de uso normal:
    // `Warn` deja fallos y advertencias reales, no cada operación exitosa.
    let nivel = if cfg!(debug_assertions) {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };
    app.plugin(tauri_plugin_log::Builder::default().level(nivel).build())?;
    Ok(())
}

/// Guarda viva hasta que `run()` retorna (recién ahí termina el proceso) --
/// en el `Drop` hace flush de eventos pendientes antes de cerrar. Captura
/// panics automáticamente (feature "panic" de `sentry`, activa por default)
/// desde que se llama esto en adelante -- separada de `run()` únicamente
/// para mantenerla bajo el tope de líneas de Clippy (mismo motivo que
/// `preparar_nucleo`/`configurar_plugins_condicionales`).
fn inicializar_sentry() -> sentry::ClientInitGuard {
    sentry::init((
        SENTRY_DSN,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            environment: Some(
                if cfg!(debug_assertions) {
                    "development"
                } else {
                    "production"
                }
                .into(),
            ),
            traces_sample_rate: 0.0,
            ..Default::default()
        },
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Inicia la aplicación de escritorio y registra todos los comandos Tauri.
///
/// # Panics
///
/// Tauri finaliza el arranque si no puede construir o ejecutar su runtime.
pub fn run() {
    let _guardia_sentry = inicializar_sentry();
    let (ruta_base_datos, instancia, clave_base_datos, core) = preparar_nucleo();

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
            configurar_plugins_condicionales(app.handle())?;
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
            comandos::rutas::listar_vehiculos_ruta,
            comandos::rutas::crear_vehiculo_ruta,
            comandos::rutas::actualizar_vehiculo_ruta,
            comandos::rutas::listar_encargados_ruta,
            comandos::rutas::crear_encargado_ruta,
            comandos::rutas::actualizar_encargado_ruta,
            comandos::rutas::listar_rutas,
            comandos::rutas::crear_ruta,
            comandos::rutas::crear_rutas_rango,
            comandos::rutas::dar_de_baja_ruta,
            comandos::rutas::reactivar_ruta,
            comandos::rutas::registrar_salida_ruta,
            comandos::rutas::registrar_retorno_ruta,
            comandos::rutas::listar_rutas_activas,
            comandos::rutas::buscar_salida_ruta,
            comandos::proveedores::listar_empresas_proveedor,
            comandos::proveedores::buscar_empresas_proveedor,
            comandos::proveedores::crear_empresa_proveedor,
            comandos::proveedores::establecer_empresa_proveedor_activa,
            comandos::proveedores::registrar_ingreso_proveedor,
            comandos::proveedores::registrar_salida_proveedor,
            comandos::proveedores::listar_proveedores_activos,
            comandos::proveedores::listar_historial_ingresos_proveedor_sitio,
            comandos::gafetes_provisionales::buscar_encargados_ruta_provisional,
            comandos::gafetes_provisionales::entregar_gafete_provisional,
            comandos::gafetes_provisionales::registrar_devolucion_gafete_provisional,
            comandos::gafetes_provisionales::listar_gafetes_provisionales_activos,
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
            comandos::gafetes::marcar_gafete_perdido_contratista,
            comandos::gafetes::marcar_gafete_perdido_visita,
            comandos::gafetes::resolver_gafete,
            comandos::nube::configurar_dispositivo_inicial,
            comandos::nube::sincronizar_con_nube,
            comandos::nube::sesion_realtime_nube,
            comandos::nube::listar_ingresos_remotos,
            comandos::nube::cerrar_ingreso_remoto,
            comandos::nube::listar_ingresos_proveedor_remotos,
            comandos::nube::cerrar_ingreso_proveedor_remoto,
            comandos::nube::listar_prestamos_gafete_provisional_remotos,
            comandos::nube::cerrar_prestamo_gafete_provisional_remoto,
            comandos::nube::fallos_permanentes_nube,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

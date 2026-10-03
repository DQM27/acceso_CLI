use control_acceso::mensajes::{mensaje_gestion_nube, mensaje_nube, mensaje_sincronizacion};
use control_acceso::nube;
use std::sync::Mutex;
use tauri::Manager;

use crate::estado::GuiState;

/// Metadata de esta PC, enviada al vincular y en cada autenticación --
/// misma idea que `MetadatosDispositivoLocal.kt` en el lado móvil (ver
/// `docs/features-futuras/plan-sesion-unica-dispositivos.md`), reusando los mismos nombres
/// de campo aunque el significado en escritorio es distinto: `android_id`
/// pasa a ser el Machine GUID de Windows (el mismo identificador estable
/// de siempre, no uno nuevo), `modelo` el nombre de esta PC en la red,
/// `fabricante`/`fingerprint` el sistema operativo y su arquitectura.
/// Nada de esto es secreto en sí mismo -- viaja igual que el resto de esta
/// metadata, en texto plano en el body de `device-vincular`/`device-auth`.
fn metadata_de_esta_maquina() -> nube::MetadatosDispositivo {
    nube::MetadatosDispositivo {
        identificador_hardware: control_acceso::nube::credenciales::identificador_de_esta_maquina(),
        nombre_dispositivo: std::env::var("COMPUTERNAME").ok(),
        plataforma: Some("Windows".to_string()),
        version_build: Some(format!(
            "{}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )),
        app_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }
}

/// Resultado de una sincronización, para la pantalla y para el evento que
/// emite la sincronización automática en segundo plano (ver `crate::run`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResumenSincronizacion {
    pub enviados: u32,
    pub fallidos: u32,
    pub remotos_abiertos: u32,
    pub cierres_recibidos: u32,
    /// Mismo criterio que `cierres_recibidos`, pero para ingresos de
    /// proveedor (`nube::recibir_cierres_de_ingresos_propios_proveedor`).
    pub cierres_recibidos_proveedor: u32,
    /// Lo mismo para ingresos por correo.
    pub cierres_recibidos_correo: u32,
    pub empresas_recibidas: u32,
    pub contratistas_recibidos: u32,
    pub gafetes_recibidos: u32,
    /// Ver `application::nube::ResumenSincronizacion::vehiculos_ruta_recibidos`.
    pub vehiculos_ruta_recibidos: u32,
    pub encargados_ruta_recibidos: u32,
    pub movimientos_historial_recibidos: u32,
    /// Ver `application::nube::ResumenSincronizacion::citas_recibidas`.
    pub citas_recibidas: u32,
    /// Ver `application::nube::ResumenSincronizacion::historial_visitas_recibidos`.
    pub historial_visitas_recibidos: u32,
    /// Mismo criterio que `historial_visitas_recibidos`, pero para ingresos
    /// de proveedor (`nube::recibir_historial_ingresos_proveedor_del_sitio`).
    pub historial_ingresos_proveedor_recibidos: u32,
    /// Lo mismo para ingresos por correo.
    pub historial_ingresos_correo_recibidos: u32,
    /// Mismo criterio que `historial_visitas_recibidos`, pero para préstamos
    /// de gafete provisional KOF
    /// (`nube::recibir_historial_gafetes_provisionales_del_sitio`).
    pub historial_gafetes_provisionales_recibidos: u32,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
    /// Ver `application::nube::ResumenSincronizacion::sesion_expulsada` --
    /// el frontend debe cerrar la sesión local y volver al login apenas
    /// vea esto en `true`.
    pub sesion_expulsada: bool,
    /// `true` cuando la sesión se cerró porque el usuario inició sesión en
    /// otra unidad (sesión única por unidad, `nube::sesion_en_unidad`); en
    /// ese caso `sesion_expulsada` también es `true`. Sólo cambia el aviso.
    pub sesion_en_otra_unidad: bool,
    /// `docs/pendientes.md`, "alertar luego al sincronizar": ingresos que
    /// quedaron activos en este dispositivo pero que la nube dice que
    /// TAMBIÉN están activos en otro sitio (colado mientras este
    /// dispositivo estaba offline, ver `nube::contratistas_con_conflicto_activo`).
    /// Mejor esfuerzo -- vacío si el chequeo falla, nunca tumba el resto de
    /// la sincronización por esto.
    pub conflictos_ingreso: Vec<nube::ConflictoIngresoActivo>,
    /// Mismo criterio que `conflictos_ingreso`, pero para movimientos de
    /// visita (`nube::visitantes_con_conflicto_activo`) -- un visitante que
    /// quedó activo en este dispositivo mientras estaba offline y que
    /// también terminó activo en otro sitio.
    pub conflictos_movimiento_visita: Vec<nube::ConflictoMovimientoVisitaActivo>,
    /// Mismo criterio que `conflictos_ingreso`, pero para ingresos de
    /// proveedor (`nube::proveedores_con_conflicto_activo`).
    pub conflictos_ingreso_proveedor: Vec<nube::ConflictoIngresoProveedorActivo>,
    /// Lo mismo para ingresos por correo (`nube::correos_con_conflicto_activo`).
    pub conflictos_ingreso_correo: Vec<nube::ConflictoIngresoProveedorActivo>,
    /// Ingresos con gafete que ESTE dispositivo registró, pero cuyo envío a
    /// la nube fue rechazado porque otro dispositivo del mismo sitio ya
    /// tiene ese número activo -- a diferencia de `conflictos_ingreso`, se
    /// calcula con datos locales dentro del mismo `drenar_cola` (ver
    /// `nube::ConflictoGafeteActivo`), sin una consulta remota aparte.
    pub conflictos_gafete: Vec<nube::ConflictoGafeteActivo>,
}

/// Datos temporales para que el frontend abra un canal Realtime privado.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SesionRealtimeNube {
    pub base_url: String,
    pub apikey: String,
    pub access_token: String,
    pub expires_in: u64,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
    pub topic: String,
}

/// Espejo de `nube::IngresoRemoto` -- un ingreso abierto por el otro
/// dispositivo del mismo sitio, listo para mostrarse y, si hace falta,
/// cerrarse desde acá.
#[derive(serde::Serialize)]
pub struct IngresoRemoto {
    pub uuid: String,
    pub contratista_nombre: String,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: Option<String>,
    pub contratista_cedula: Option<String>,
    pub empresa_nombre: Option<String>,
    pub tipo_ingreso: Option<String>,
    pub medio_ingreso: Option<String>,
    pub gafete_numero: Option<i64>,
}

/// Espejo de `nube::IngresoProveedorRemoto` -- mismo criterio que
/// `IngresoRemoto` arriba, pero para el ciclo de proveedores.
#[derive(serde::Serialize)]
pub struct IngresoProveedorRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: String,
}

/// Espejo de `nube::IngresoCorreoRemoto` -- mismo criterio que
/// `IngresoProveedorRemoto`, con el motivo en vez de la empresa.
#[derive(serde::Serialize)]
pub struct IngresoCorreoRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub motivo: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: String,
}

/// Espejo de `nube::PrestamoGafeteProvisionalRemoto` -- mismo criterio que
/// `IngresoRemoto`/`IngresoProveedorRemoto`, pero para el ciclo de
/// entrega/devolución de gafetes provisionales KOF.
#[derive(serde::Serialize)]
pub struct PrestamoGafeteProvisionalRemoto {
    pub uuid: String,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: String,
    pub gafete_numero: i64,
    pub hora_entrega: String,
    pub usuario_entrega_nombre: String,
}

/// Autentica este dispositivo contra el receptor -- un solo lugar para no
/// repetir "autorizar + pedir token" en cada función de este archivo.
/// Autoriza con `Operacion::UsarNube` (cualquier rol), no
/// `GestionarNube` (exclusivo ROOT) -- sincronizar es uso diario normal, y
/// el disparador automático de fondo (`crate::iniciar_sincronizacion_automatica`)
/// no debe depender de que justo haya una sesión ROOT abierta.
fn autenticar(state: &GuiState) -> Result<nube::TokenDispositivo, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .autorizar_uso_nube(&actor)
        .map_err(mensaje_gestion_nube)?;

    let token = state.autenticar_con_cache().map_err(mensaje_nube)?;
    // El candado ya se soltó (`autorizar_uso_nube` arriba fue la única
    // sección crítica) -- volver a pedirlo acá es una lectura/escritura
    // atómica sobre un `AtomicI64` (ver `RelojCorregido`), no compite con
    // nada lento.
    state.core().aplicar_token(&token);
    Ok(token)
}

/// Distingue "el token de dispositivo cacheado quedó vencido a mitad de
/// camino" (ver `SincronizacionError::token_dispositivo_vencido`) de
/// cualquier otro fallo -- sólo el primero amerita invalidar el caché y
/// reintentar, el resto se propaga tal cual con su mensaje ya traducido.
enum FalloSincronizacion {
    TokenVencido,
    Mensaje(String),
}

impl From<nube::SincronizacionError> for FalloSincronizacion {
    fn from(error: nube::SincronizacionError) -> Self {
        if error.token_dispositivo_vencido() {
            Self::TokenVencido
        } else {
            Self::Mensaje(mensaje_sincronizacion(error))
        }
    }
}

/// Autentica, drena la bandeja de salida pendiente y refresca la caché de
/// lo que el otro dispositivo del mismo sitio tiene abierto ahora mismo.
/// Compartida por el comando manual (`sincronizar_con_nube`) y el
/// disparador automático (`crate::run`) -- misma lógica, dos formas de
/// dispararla. Autoriza rápido con el candado compartido (dentro de
/// `autenticar`), lo suelta, y hace la parte lenta (red) sobre una conexión
/// propia -- ver `GuiState::conexion_secundaria`.
///
/// Reintenta UNA vez si `intentar_sincronizacion` falla porque el token
/// cacheado, que `autenticar_con_cache` creía vigente, resultó rechazado
/// por el receptor a mitad de camino (desfase de reloj, o el dispositivo
/// estuvo inactivo más tiempo del que el margen de 30s contemplaba --
/// visto en vivo: más de una hora sin sincronizar, con el token ya vencido
/// hacía rato). Sin este reintento, la sincronización entera fallaba y
/// quien usa la app tenía que notarlo y volver a apretar "Sincronizar" a
/// mano -- ahora se recupera sola.
pub fn ejecutar_sincronizacion(state: &GuiState) -> Result<ResumenSincronizacion, String> {
    ejecutar_sincronizacion_con_alcance(state, nube::AlcanceSincronizacion::completo())
}

/// Igual que [`ejecutar_sincronizacion`], pero corriendo sólo las etapas de
/// recepción de `alcance` -- ver `nube::AlcanceSincronizacion`. Lo usa el
/// aviso en vivo (`sincronizar_cambios_nube`): antes cada aviso corría la
/// sincronización COMPLETA (~13 consultas a la nube) aunque sólo hubiera
/// cambiado una tabla. La bandeja de salida se drena siempre.
pub fn ejecutar_sincronizacion_con_alcance(
    state: &GuiState,
    alcance: nube::AlcanceSincronizacion,
) -> Result<ResumenSincronizacion, String> {
    // El timer, los avisos remotos y el botón manual comparten la misma cola.
    // Sólo una ejecución puede drenarla a la vez; el núcleo queda libre.
    static SINCRONIZACION: Mutex<()> = Mutex::new(());
    let _sincronizacion = SINCRONIZACION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    match intentar_sincronizacion(state, alcance) {
        Err(FalloSincronizacion::TokenVencido) => {
            state.invalidar_token_cacheado();
            intentar_sincronizacion(state, alcance).map_err(|fallo| match fallo {
                FalloSincronizacion::TokenVencido => {
                    "El token de este dispositivo venció y no se pudo renovar -- revisá la conexión"
                        .to_string()
                }
                FalloSincronizacion::Mensaje(mensaje) => mensaje,
            })
        }
        Err(FalloSincronizacion::Mensaje(mensaje)) => Err(mensaje),
        Ok(resumen) => Ok(resumen),
    }
}

fn intentar_sincronizacion(
    state: &GuiState,
    alcance: nube::AlcanceSincronizacion,
) -> Result<ResumenSincronizacion, FalloSincronizacion> {
    let token = autenticar(state).map_err(FalloSincronizacion::Mensaje)?;
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::base_url(),
        apikey: nube::apikey(),
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };

    let conexion = state
        .conexion_secundaria()
        .map_err(FalloSincronizacion::Mensaje)?;
    let resumen = nube::sincronizar(
        &conexion,
        &contexto,
        alcance,
        nube::PerfilDispositivo::Escritorio,
    )?;

    // Si a quien disparó esto lo desactivaron en otro dispositivo, el
    // catálogo recién recibido ya lo refleja -- lo saca de la sesión acá
    // mismo (no sólo avisa al frontend) para que el próximo comando que
    // dependa de `sesion_activa()` falle de inmediato, sin esperar a que la
    // pantalla reaccione al resumen.
    let mut sesion_expulsada = match state.sesion_activa() {
        Ok(actor) if !state.core().sesion_sigue_activa(&actor) => {
            state.cerrar_sesion();
            true
        }
        _ => false,
    };

    // Sesión única por unidad: cada sincronización registra la sesión en la
    // nube y pregunta si sigue vigente (la primera, justo después del login,
    // es la que la registra). Si el usuario entró después en otra unidad,
    // esta sesión se cierra. Falla "abierto": un error de red no expulsa.
    let mut sesion_en_otra_unidad = false;
    if !sesion_expulsada && let Some((actor, inicio)) = state.sesion_con_inicio() {
        let estado = nube::sesion_en_unidad(
            nube::base_url(),
            nube::apikey(),
            &token,
            &actor.cedula,
            &inicio,
        );
        // Build de diagnóstico: qué decidió la nube y con qué duración de
        // sesión. Sin cédula ni nombre.
        if crate::telemetria::activa() {
            crate::telemetria::evento(
                "sesion_unidad",
                serde_json::json!({
                    "resultado": estado.as_ref().map_or("error", |estado| estado.como_texto()),
                    "transcurrido_ms": inicio.transcurrido_ms(),
                }),
            );
        }
        match estado {
            Ok(nube::EstadoSesionUnidad::Desplazada) => {
                state.cerrar_sesion();
                sesion_expulsada = true;
                sesion_en_otra_unidad = true;
            }
            Ok(_) => {}
            Err(error) => log::info!("no se pudo verificar la sesión en la nube: {error}"),
        }
    }

    Ok(ResumenSincronizacion {
        enviados: resumen.enviados,
        fallidos: resumen.fallidos,
        remotos_abiertos: resumen.remotos_abiertos,
        cierres_recibidos: resumen.cierres_recibidos,
        cierres_recibidos_proveedor: resumen.cierres_recibidos_proveedor,
        cierres_recibidos_correo: resumen.cierres_recibidos_correo,
        movimientos_historial_recibidos: resumen.movimientos_historial_recibidos,
        citas_recibidas: resumen.citas_recibidas,
        historial_visitas_recibidos: resumen.historial_visitas_recibidos,
        historial_ingresos_proveedor_recibidos: resumen.historial_ingresos_proveedor_recibidos,
        historial_ingresos_correo_recibidos: resumen.historial_ingresos_correo_recibidos,
        historial_gafetes_provisionales_recibidos: resumen
            .historial_gafetes_provisionales_recibidos,
        empresas_recibidas: resumen.catalogo.empresas_recibidas,
        contratistas_recibidos: resumen.catalogo.contratistas_recibidos,
        gafetes_recibidos: resumen.catalogo.gafetes_recibidos,
        vehiculos_ruta_recibidos: resumen.catalogo_rutas.vehiculos_recibidos,
        encargados_ruta_recibidos: resumen.catalogo_rutas.encargados_recibidos,
        sitio_id: token.sitio_id,
        dispositivo_id: token.dispositivo_id,
        tipo: token.tipo,
        sesion_expulsada,
        sesion_en_otra_unidad,
        conflictos_ingreso: resumen.conflictos_ingreso,
        conflictos_movimiento_visita: resumen.conflictos_movimiento_visita,
        conflictos_ingreso_proveedor: resumen.conflictos_ingreso_proveedor,
        conflictos_ingreso_correo: resumen.conflictos_ingreso_correo,
        conflictos_gafete: resumen.conflictos_gafete,
    })
}

/// Arranque de una base vacía (`requiere_configuracion_inicial` en
/// `comandos::autenticacion`) -- sin `sesion_activa()` a propósito, porque
/// todavía no existe ningún usuario con quien loguearse. Canjea el código de
/// vinculación que el panel generó para este equipo (ver
/// `PrimerArranque.tsx`) y trae el catálogo remoto completo, usuarios
/// incluidos, para que el próximo intento de login ya tenga con quién
/// autenticar (con el centinela `SIN_PASSWORD_LOCAL`, así que cae solo en
/// "fijar contraseña").
#[tauri::command]
pub async fn vincular_dispositivo_inicial(
    app: tauri::AppHandle,
    codigo: String,
) -> Result<ResumenSincronizacion, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<GuiState>();
        let metadata = metadata_de_esta_maquina();
        let resumen = state
            .core()
            .vincular_dispositivo_inicial(
                &codigo,
                Some(&metadata),
                nube::PerfilDispositivo::Escritorio,
            )
            .map_err(mensaje_gestion_nube)?;
        Ok(ResumenSincronizacion {
            enviados: resumen.enviados,
            fallidos: resumen.fallidos,
            remotos_abiertos: resumen.remotos_abiertos,
            cierres_recibidos: resumen.cierres_recibidos,
            // Núcleo (`AppCore::vincular_dispositivo_inicial`) no trae este
            // campo -- mismo criterio que `conflictos_ingreso_proveedor` de
            // abajo: base recién configurada, nada que traer todavía.
            cierres_recibidos_proveedor: 0,
            cierres_recibidos_correo: 0,
            movimientos_historial_recibidos: resumen.movimientos_historial_recibidos,
            citas_recibidas: resumen.citas_recibidas,
            historial_visitas_recibidos: resumen.historial_visitas_recibidos,
            historial_ingresos_proveedor_recibidos: 0,
            historial_ingresos_correo_recibidos: 0,
            historial_gafetes_provisionales_recibidos: 0,
            empresas_recibidas: resumen.empresas_recibidas,
            contratistas_recibidos: resumen.contratistas_recibidos,
            gafetes_recibidos: resumen.gafetes_recibidos,
            vehiculos_ruta_recibidos: resumen.vehiculos_ruta_recibidos,
            encargados_ruta_recibidos: resumen.encargados_ruta_recibidos,
            sitio_id: resumen.sitio_id,
            dispositivo_id: resumen.dispositivo_id,
            tipo: resumen.tipo,
            sesion_expulsada: resumen.sesion_expulsada,
            sesion_en_otra_unidad: false,
            // Base recién configurada, sin ningún ingreso ni movimiento de
            // visita local todavía -- no hay nada que pudiera chocar con
            // otro sitio en este momento.
            conflictos_ingreso: Vec::new(),
            conflictos_movimiento_visita: Vec::new(),
            conflictos_ingreso_proveedor: Vec::new(),
            conflictos_ingreso_correo: Vec::new(),
            conflictos_gafete: Vec::new(),
        })
    })
    .await
    .map_err(|error| format!("No se pudo completar el arranque inicial: {error}"))?
}

/// Aviso en vivo con los datos (`cambio_nube` con `registro`): guarda la
/// fila directo en la base local, sin consultar a la nube -- ver
/// `nube::en_vivo`. `true` si la aplicó; `false` si el aviso no trae datos
/// o la tabla todavía no los manda (el frontend pide entonces
/// `sincronizar_cambios_nube` sólo para esa tabla). Sobre la conexión
/// secundaria: nunca toma el candado del núcleo.
#[tauri::command]
pub async fn aplicar_cambio_nube(
    app: tauri::AppHandle,
    cambio: serde_json::Value,
) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<GuiState>();
        state.sesion_activa()?;
        let conexion = state.conexion_secundaria()?;
        nube::aplicar_cambio_en_vivo(&conexion, &cambio, nube::PerfilDispositivo::Escritorio)
            .map_err(mensaje_sincronizacion)
    })
    .await
    .map_err(|error| format!("No se pudo aplicar el cambio en vivo: {error}"))?
}

/// Sincronización disparada por avisos en vivo (`cambio_nube`) -- corre
/// sólo las etapas de las tablas que cambiaron (`tablas`, tal cual las
/// manda el aviso en `payload.table`), ver `nube::AlcanceSincronizacion`.
/// Una tabla desconocida o una lista vacía caen en la sincronización
/// completa.
#[tauri::command]
pub async fn sincronizar_cambios_nube(
    app: tauri::AppHandle,
    tablas: Vec<String>,
) -> Result<ResumenSincronizacion, String> {
    tauri::async_runtime::spawn_blocking(move || {
        ejecutar_sincronizacion_con_alcance(
            &app.state::<GuiState>(),
            nube::AlcanceSincronizacion::desde_tablas(&tablas),
        )
    })
    .await
    .map_err(|error| format!("No se pudo completar la sincronización: {error}"))?
}

/// Un ingreso/salida registrado en este equipo: sólo se vacía la bandeja de
/// salida (`nube::AlcanceSincronizacion::solo_envio`), sin bajar catálogo
/// ni historial -- no hay nada que traer de la nube por un cambio propio.
#[tauri::command]
pub async fn enviar_cambios_nube(app: tauri::AppHandle) -> Result<ResumenSincronizacion, String> {
    tauri::async_runtime::spawn_blocking(move || {
        ejecutar_sincronizacion_con_alcance(
            &app.state::<GuiState>(),
            nube::AlcanceSincronizacion::solo_envio(),
        )
    })
    .await
    .map_err(|error| format!("No se pudo enviar a la nube: {error}"))?
}

#[tauri::command]
pub async fn sincronizar_con_nube(app: tauri::AppHandle) -> Result<ResumenSincronizacion, String> {
    tauri::async_runtime::spawn_blocking(move || ejecutar_sincronizacion(&app.state::<GuiState>()))
        .await
        .map_err(|error| format!("No se pudo completar la sincronización: {error}"))?
}

#[tauri::command]
pub async fn sesion_realtime_nube(app: tauri::AppHandle) -> Result<SesionRealtimeNube, String> {
    tauri::async_runtime::spawn_blocking(move || preparar_sesion_realtime(&app.state::<GuiState>()))
        .await
        .map_err(|error| format!("No se pudo preparar la sesión de nube: {error}"))?
}

fn preparar_sesion_realtime(state: &GuiState) -> Result<SesionRealtimeNube, String> {
    // Autoriza con el candado y lo suelta antes de la petición de red.
    let sesion = autenticar(state)?;

    Ok(SesionRealtimeNube {
        base_url: nube::base_url().to_string(),
        apikey: nube::apikey().to_string(),
        access_token: sesion.access_token,
        expires_in: sesion.expires_in,
        topic: format!("sitio:{}", sesion.sitio_id),
        sitio_id: sesion.sitio_id,
        dispositivo_id: sesion.dispositivo_id,
        tipo: sesion.tipo,
    })
}

/// El aviso en vivo `dispositivo_expulsado` llegó para este equipo: el
/// token cacheado ya no sirve (el servidor lo rechaza), así que se descarta
/// para que la próxima operación de nube pida uno y muestre el motivo real.
#[tauri::command]
pub fn descartar_token_nube(state: tauri::State<'_, GuiState>) {
    state.invalidar_token_cacheado();
}

/// Cuántas filas de `cola_salida` ya agotaron los reintentos automáticos
/// (ver `INTENTOS_ANTES_DE_FALLO_PERMANENTE`) -- para avisar que algo
/// necesita que alguien lo mire, en vez de fallar en silencio para siempre.
#[tauri::command]
pub fn fallos_permanentes_nube(state: tauri::State<GuiState>) -> Result<i64, String> {
    state.sesion_activa()?;
    let conexion = state.conexion_secundaria()?;
    nube::contar_fallos_permanentes(&conexion).map_err(mensaje_sincronizacion)
}

/// Lectura pura de la caché local `ingresos_remotos` -- ya la llenó la
/// última sincronización (manual o automática), no hace falta red para
/// mostrarla. Errores de `SQLite` crudos acá abajo: no hay un `mensaje_*`
/// para `rusqlite::Error` suelto (siempre viene envuelto en un tipo propio
/// en el resto del núcleo), y este `SELECT` no debería fallar en la
/// práctica.
#[tauri::command]
pub fn listar_ingresos_remotos(
    state: tauri::State<GuiState>,
) -> Result<Vec<IngresoRemoto>, String> {
    state.sesion_activa()?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, contratista_nombre, hora_entrada, usuario_entrada_nombre,
                    contratista_cedula, empresa_nombre, tipo_ingreso, medio_ingreso, gafete_numero
             FROM ingresos_remotos ORDER BY hora_entrada",
        )
        .map_err(super::mensaje_generico)?;
    let filas = statement
        .query_map([], |row| {
            Ok(IngresoRemoto {
                uuid: row.get(0)?,
                contratista_nombre: row.get(1)?,
                hora_entrada: row.get(2)?,
                usuario_entrada_nombre: row.get(3)?,
                contratista_cedula: row.get(4)?,
                empresa_nombre: row.get(5)?,
                tipo_ingreso: row.get(6)?,
                medio_ingreso: row.get(7)?,
                gafete_numero: row.get(8)?,
            })
        })
        .map_err(super::mensaje_generico)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(super::mensaje_generico)?;
    Ok(filas)
}

/// Cierra, contra la nube, un ingreso abierto por el otro dispositivo del
/// mismo sitio -- ver `nube::cerrar_ingreso_remoto`: nunca toca el
/// historial local, sólo la caché.
#[tauri::command]
pub fn cerrar_ingreso_remoto(uuid: String, state: tauri::State<GuiState>) -> Result<(), String> {
    let actor = state.sesion_activa()?;
    let token = autenticar(&state)?;
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::base_url(),
        apikey: nube::apikey(),
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let conexion = state.conexion_secundaria()?;
    let hora = state.core().ahora_utc();
    nube::cerrar_ingreso_remoto(&conexion, &contexto, &uuid, &actor.nombre, hora)
        .map_err(mensaje_sincronizacion)
}

/// Espejo de `listar_ingresos_remotos`, pero contra la caché
/// `ingresos_proveedor_remotos`.
#[tauri::command]
pub fn listar_ingresos_proveedor_remotos(
    state: tauri::State<GuiState>,
) -> Result<Vec<IngresoProveedorRemoto>, String> {
    state.sesion_activa()?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, cedula, nombre, empresa_nombre, placa, gafete_numero,
                    hora_entrada, usuario_entrada_nombre
             FROM ingresos_proveedor_remotos ORDER BY hora_entrada",
        )
        .map_err(super::mensaje_generico)?;
    let filas = statement
        .query_map([], |row| {
            Ok(IngresoProveedorRemoto {
                uuid: row.get(0)?,
                cedula: row.get(1)?,
                nombre: row.get(2)?,
                empresa_nombre: row.get(3)?,
                placa: row.get(4)?,
                gafete_numero: row.get(5)?,
                hora_entrada: row.get(6)?,
                usuario_entrada_nombre: row.get(7)?,
            })
        })
        .map_err(super::mensaje_generico)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(super::mensaje_generico)?;
    Ok(filas)
}

/// Espejo de `cerrar_ingreso_remoto`, pero contra `ingresos_proveedor`.
#[tauri::command]
pub fn cerrar_ingreso_proveedor_remoto(
    uuid: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let actor = state.sesion_activa()?;
    let token = autenticar(&state)?;
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::base_url(),
        apikey: nube::apikey(),
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let conexion = state.conexion_secundaria()?;
    let hora = state.core().ahora_utc();
    nube::cerrar_ingreso_proveedor_remoto(&conexion, &contexto, &uuid, &actor.nombre, hora)
        .map_err(mensaje_sincronizacion)
}

/// Espejo de `listar_ingresos_proveedor_remotos`, contra la caché
/// `ingresos_correo_remotos`.
#[tauri::command]
pub fn listar_ingresos_correo_remotos(
    state: tauri::State<GuiState>,
) -> Result<Vec<IngresoCorreoRemoto>, String> {
    state.sesion_activa()?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, cedula, nombre, motivo, placa, gafete_numero,
                    hora_entrada, usuario_entrada_nombre
             FROM ingresos_correo_remotos ORDER BY hora_entrada",
        )
        .map_err(super::mensaje_generico)?;
    let filas = statement
        .query_map([], |row| {
            Ok(IngresoCorreoRemoto {
                uuid: row.get(0)?,
                cedula: row.get(1)?,
                nombre: row.get(2)?,
                motivo: row.get(3)?,
                placa: row.get(4)?,
                gafete_numero: row.get(5)?,
                hora_entrada: row.get(6)?,
                usuario_entrada_nombre: row.get(7)?,
            })
        })
        .map_err(super::mensaje_generico)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(super::mensaje_generico)?;
    Ok(filas)
}

/// Espejo de `cerrar_ingreso_proveedor_remoto`, contra `ingresos_correo`.
#[tauri::command]
pub fn cerrar_ingreso_correo_remoto(
    uuid: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let actor = state.sesion_activa()?;
    let token = autenticar(&state)?;
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::base_url(),
        apikey: nube::apikey(),
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let conexion = state.conexion_secundaria()?;
    let hora = state.core().ahora_utc();
    nube::cerrar_ingreso_correo_remoto(&conexion, &contexto, &uuid, &actor.nombre, hora)
        .map_err(mensaje_sincronizacion)
}

/// Espejo de `listar_ingresos_proveedor_remotos`, pero contra la caché
/// `prestamos_gafete_provisional_remotos`.
#[tauri::command]
pub fn listar_prestamos_gafete_provisional_remotos(
    state: tauri::State<GuiState>,
) -> Result<Vec<PrestamoGafeteProvisionalRemoto>, String> {
    state.sesion_activa()?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, encargado_nombre, encargado_codigo_empleado, gafete_numero,
                    hora_entrega, usuario_entrega_nombre
             FROM prestamos_gafete_provisional_remotos ORDER BY hora_entrega",
        )
        .map_err(super::mensaje_generico)?;
    let filas = statement
        .query_map([], |row| {
            Ok(PrestamoGafeteProvisionalRemoto {
                uuid: row.get(0)?,
                encargado_nombre: row.get(1)?,
                encargado_codigo_empleado: row.get(2)?,
                gafete_numero: row.get(3)?,
                hora_entrega: row.get(4)?,
                usuario_entrega_nombre: row.get(5)?,
            })
        })
        .map_err(super::mensaje_generico)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(super::mensaje_generico)?;
    Ok(filas)
}

/// Espejo de `cerrar_ingreso_proveedor_remoto`, pero contra
/// `prestamos_gafete_provisional`.
#[tauri::command]
pub fn cerrar_prestamo_gafete_provisional_remoto(
    uuid: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let actor = state.sesion_activa()?;
    let token = autenticar(&state)?;
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::base_url(),
        apikey: nube::apikey(),
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let conexion = state.conexion_secundaria()?;
    let hora = state.core().ahora_utc();
    nube::cerrar_prestamo_gafete_provisional_remoto(
        &conexion,
        &contexto,
        &uuid,
        &actor.nombre,
        hora,
    )
    .map_err(mensaje_sincronizacion)
}

/// Último desfase medido entre el reloj de esta PC y el del servidor (ms,
/// positivo si la PC va adelantada), o `None` si nunca se midió. Lo usa la
/// telemetría de diagnóstico para corregir la latencia de los avisos en
/// vivo (ver `desktop/src/telemetria.ts`).
#[tauri::command]
pub fn desfase_reloj_ms(state: tauri::State<GuiState>) -> Option<i64> {
    state.core().desfase_reloj_ms()
}

/// Unidad y etiqueta con que el panel registró esta PC, para el login y la
/// barra de estado. Sólo informativas: si alguien registró el equipo en la
/// unidad equivocada, se nota acá antes de que la sesión única empiece a
/// cerrar sesiones. Campos `null` mientras no hayan llegado de la nube.
#[derive(serde::Serialize)]
pub struct IdentidadEquipoGui {
    unidad: Option<String>,
    etiqueta: Option<String>,
}

#[tauri::command]
pub fn identidad_equipo(state: tauri::State<GuiState>) -> IdentidadEquipoGui {
    let identidad = state.core().identidad_equipo();
    IdentidadEquipoGui {
        unidad: identidad.unidad,
        etiqueta: identidad.etiqueta,
    }
}

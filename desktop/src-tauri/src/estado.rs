use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use control_acceso::application::AppCore;
use control_acceso::database::connection::{
    abrir_conexion_secundaria, abrir_conexion_secundaria_escritura,
};
use control_acceso::instancia::InstanciaGuard;
use control_acceso::nube::{
    self as nube, CacheTokenDispositivo, FirmanteDispositivo, NubeError, TokenDispositivo,
};
use control_acceso::services::autenticacion_service::UsuarioSesion;
use rusqlite::Connection;
use zeroize::Zeroizing;

/// Estado administrado por Tauri. Dos mutexes separados porque ningún flujo
/// necesita actualizar sesión y base de datos como una sola operación atómica
/// (ver docs/plan-tauri.md, sección "Estado y sesión").
///
/// Campos privados a propósito: todo acceso pasa por los métodos de abajo,
/// que recuperan el mutex si quedó envenenado por un panic en otro comando
/// en vez de dejar ese panic tumbar en cadena cualquier comando futuro que
/// intente tomar el mismo lock — con `std::sync::Mutex`, un solo panic
/// aislado no debería dejar la app entera inutilizable hasta reiniciarla.
pub struct GuiState {
    core: Mutex<AppCore>,
    /// Usuario con sesión abierta y la hora (reloj de este equipo) en que la
    /// abrió: la sesión única por unidad compara esa hora con la de los
    /// ingresos en otras unidades (`nube::sesion_en_unidad`).
    sesion: Mutex<Option<(UsuarioSesion, nube::InicioSesion)>>,
    /// Mantiene el candado de instancia vivo mientras dure la app — nunca se
    /// lee, sólo existe para que no se libere antes de tiempo (mismo patrón
    /// que `main.rs` con `_instancia`).
    _instancia: InstanciaGuard,
    /// Campo HERMANO de `core`, nunca protegido por el mismo candado --
    /// ver el doc-comment de `control_acceso::nube::CacheTokenDispositivo`
    /// sobre por qué (varios comandos autentican sin pasar por
    /// `state.core()` a propósito, para no retener ese candado compartido
    /// durante la llamada de red).
    cache_token: CacheTokenDispositivo,
    /// Ruta del archivo de base de datos, resuelta una sola vez al arrancar
    /// (ver `lib.rs::run`) — el núcleo ya no expone `ruta_base_datos()`
    /// (rama `SQLCipher` sin respaldo local), así que `conexion_secundaria`
    /// la necesita guardada acá.
    ruta_base_datos: PathBuf,
    /// Clave de `SQLCipher` ya resuelta al arrancar (ver
    /// `lib.rs::run`/`clave_cifrado.rs`) — `conexion_secundaria` la necesita
    /// para poder leer el mismo archivo cifrado. `Zeroizing` la borra de
    /// memoria cuando la app cierra.
    clave_base_datos: Zeroizing<[u8; 32]>,
    /// Conexión de sólo lectura que se abre la primera vez y se reutiliza,
    /// para las lecturas chicas y frecuentes (ver `con_lectura`). Abrir una
    /// conexión nueva en cada llamada cuesta más que la consulta misma: lee
    /// el esquema y arranca con la caché de páginas vacía, descifrándolas de
    /// nuevo. La telemetría lo mostró (auditoría 2026-11-14, sección 12):
    /// `aviso_bandeja_salida`, que sólo cuenta pendientes, tardaba 25 ms, y
    /// los cuatro listados remotos de Activos ~80 ms por recarga.
    lectura: Mutex<Option<Connection>>,
}

impl GuiState {
    pub fn new(
        core: AppCore,
        instancia: InstanciaGuard,
        ruta_base_datos: PathBuf,
        clave_base_datos: Zeroizing<[u8; 32]>,
        firmante: Arc<dyn FirmanteDispositivo>,
    ) -> Self {
        let cache_token = CacheTokenDispositivo::new();
        cache_token.establecer_firmante(firmante);
        Self {
            core: Mutex::new(core),
            sesion: Mutex::new(None),
            _instancia: instancia,
            cache_token,
            ruta_base_datos,
            clave_base_datos,
            lectura: Mutex::new(None),
        }
    }

    /// `true` si este equipo ya canjeó un código y tiene su clave. Sin
    /// vincular, la nube está sin configurar: no hay con quién chocar y
    /// ningún chequeo remoto toca la red.
    pub fn nube_vinculada(&self) -> bool {
        self.cache_token.vinculado()
    }

    /// Reusa el último `TokenDispositivo` mientras siga vigente en vez de
    /// autenticar de cero -- ver `CacheTokenDispositivo`. Reproducido en
    /// producción: "Registrar" con gafete pagaba una autenticación
    /// completa contra la nube en cada registro
    /// (`gafete_libre_en_otro_dispositivo`), aunque el dispositivo ya se
    /// hubiera autenticado segundos antes para sincronizar o loguearse --
    /// se sentía como que la app se colgaba en cada registro.
    ///
    /// Sin vincular falla con `NubeError::SinCredencial`.
    ///
    /// El resultado, bueno o malo, lo anota el núcleo: el aviso de "equipo
    /// retirado" refleja la última respuesta con autoridad de la nube (ver
    /// `AppCore::anotar_autenticacion`). Toma el candado del núcleo para
    /// eso; quien llama no debe tenerlo tomado (ninguno lo tiene: todos lo
    /// piden recién después, para aplicar el token).
    pub fn autenticar_con_cache(&self) -> Result<TokenDispositivo, NubeError> {
        let resultado = self.cache_token.autenticar_con_cache();
        self.core().anotar_autenticacion(&resultado);
        resultado
    }

    /// Descarta el `TokenDispositivo` cacheado -- ver
    /// `SincronizacionError::token_dispositivo_vencido`: el receptor lo
    /// rechazó a mitad de una sincronización aunque `autenticar_con_cache`
    /// lo creía vigente (desfase de reloj, o el dispositivo estuvo inactivo
    /// más de lo que el margen de 30s contemplaba). La próxima llamada a
    /// `autenticar_con_cache` pide uno nuevo sin esperar a que este
    /// "`vigente_por`" calculado localmente se cumpla solo.
    pub fn invalidar_token_cacheado(&self) {
        self.cache_token.invalidar();
    }

    /// Para las operaciones del núcleo que consultan la nube antes de
    /// escribir (`control_acceso::application::con_nube`).
    pub fn nube_del_dispositivo(&self) -> &CacheTokenDispositivo {
        &self.cache_token
    }

    /// Acceso al núcleo compartido por todos los comandos.
    pub fn core(&self) -> MutexGuard<'_, AppCore> {
        self.core
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Conexión propia al mismo archivo, independiente de la que vive
    /// dentro de `core` — para comandos cuya consulta puede tardar cientos
    /// de milisegundos o más con datos grandes (exportar/cargar Historial y
    /// Auditoría completos, ver `comandos/historial.rs`/`comandos/auditoria.rs`)
    /// y no deben retener el mutex compartido mientras tanto: aunque el
    /// comando ya corre en el pool de hilos bloqueantes de Tauri (no
    /// congela la ventana), retener `core` sí bloquearía a cualquier OTRO
    /// comando que también lo necesite (mismo hallazgo que motivó el hilo
    /// propio en TUI/CLI para exportar, ver `docs/pendientes.md`). El
    /// candado sólo se toma para leer la ruta del archivo, no durante la
    /// consulta. Escribe (sincronización con la nube, `cerrar_ingreso_remoto`)
    /// -- por eso usa la fábrica central de *escritura*
    /// (`abrir_conexion_secundaria_escritura`), no la de sólo lectura que usan
    /// los hilos de exportación de TUI/CLI.
    pub fn conexion_secundaria(&self) -> Result<Connection, String> {
        abrir_conexion_secundaria_escritura(&self.ruta_base_datos, Some(&self.clave_base_datos))
            .map_err(|error| error.to_string())
    }

    /// Corre `consulta` sobre la conexión de lectura compartida (ver el campo
    /// `lectura`), abriéndola si todavía no existe. Sólo para lecturas
    /// cortas: mientras corre, cualquier otra llamada a `con_lectura` espera.
    /// Las consultas largas (historiales, auditoría, exportaciones) siguen
    /// con `conexion_secundaria`.
    ///
    /// Es de sólo lectura (`PRAGMA query_only`): una escritura por error
    /// falla en vez de pasar. Con WAL, cada consulta ve lo último confirmado
    /// aunque la conexión lleve horas abierta. Si la consulta falla, la
    /// conexión se descarta y la próxima llamada abre una nueva.
    ///
    /// Orden de candados: este primero y después, si hace falta, `core()`
    /// (ver `listar_ingresos_remotos`). Nunca al revés.
    pub fn con_lectura<T>(
        &self,
        consulta: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        con_conexion_reutilizada(
            &self.lectura,
            || {
                abrir_conexion_secundaria(&self.ruta_base_datos, Some(&self.clave_base_datos))
                    .map_err(|error| error.to_string())
            },
            consulta,
        )
    }

    /// Sesión actual o el error que ya usan todos los comandos que la
    /// necesitan — un solo lugar para ese chequeo repetido.
    pub fn sesion_activa(&self) -> Result<UsuarioSesion, String> {
        self.lock_sesion()
            .as_ref()
            .map(|(sesion, _)| sesion.clone())
            .ok_or_else(|| "No hay una sesión activa".to_string())
    }

    /// Sesión actual con su inicio (id y contador de arranque, ver
    /// `nube::InicioSesion`), o `None` sin sesión.
    pub fn sesion_con_inicio(&self) -> Option<(UsuarioSesion, nube::InicioSesion)> {
        self.lock_sesion().clone()
    }

    pub fn iniciar_sesion(&self, sesion: UsuarioSesion) {
        *self.lock_sesion() = Some((sesion, nube::InicioSesion::ahora()));
    }

    pub fn cerrar_sesion(&self) {
        *self.lock_sesion() = None;
    }

    fn lock_sesion(&self) -> MutexGuard<'_, Option<(UsuarioSesion, nube::InicioSesion)>> {
        self.sesion
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Lo que hace `GuiState::con_lectura`, aparte para poder probarlo sin todo
/// el estado de la app: usa la conexión guardada en `ranura` o abre una con
/// `abrir`, corre `consulta` y la vuelve a guardar sólo si salió bien. Ante un
/// error se descarta, así una conexión que quedó en mal estado no se reusa.
fn con_conexion_reutilizada<T>(
    ranura: &Mutex<Option<Connection>>,
    abrir: impl FnOnce() -> Result<Connection, String>,
    consulta: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let mut guarda = ranura
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let conexion = match guarda.take() {
        Some(conexion) => conexion,
        None => abrir()?,
    };
    let resultado = consulta(&conexion);
    if resultado.is_ok() {
        *guarda = Some(conexion);
    }
    resultado
}

#[cfg(test)]
mod tests {
    use super::con_conexion_reutilizada;
    use rusqlite::Connection;
    use std::cell::Cell;
    use std::sync::Mutex;

    fn abrir_contando(aperturas: &Cell<u32>) -> Result<Connection, String> {
        aperturas.set(aperturas.get() + 1);
        Connection::open_in_memory().map_err(|error| error.to_string())
    }

    fn uno(conexion: &Connection) -> Result<i64, String> {
        conexion
            .query_row("SELECT 1", [], |fila| fila.get(0))
            .map_err(|error| error.to_string())
    }

    #[test]
    fn abre_una_sola_vez_y_reutiliza() {
        let ranura = Mutex::new(None);
        let aperturas = Cell::new(0);
        for _ in 0..3 {
            let valor = con_conexion_reutilizada(&ranura, || abrir_contando(&aperturas), uno);
            assert_eq!(valor, Ok(1));
        }
        assert_eq!(aperturas.get(), 1);
    }

    #[test]
    fn tras_un_error_descarta_la_conexion_y_abre_otra() {
        let ranura = Mutex::new(None);
        let aperturas = Cell::new(0);
        let fallo: Result<(), String> = con_conexion_reutilizada(
            &ranura,
            || abrir_contando(&aperturas),
            |_| Err("falló".to_string()),
        );
        assert!(fallo.is_err());
        assert!(ranura.lock().unwrap().is_none());
        let valor = con_conexion_reutilizada(&ranura, || abrir_contando(&aperturas), uno);
        assert_eq!(valor, Ok(1));
        assert_eq!(aperturas.get(), 2);
    }

    #[test]
    fn si_no_se_puede_abrir_devuelve_el_error_sin_guardar_nada() {
        let ranura = Mutex::new(None);
        let valor = con_conexion_reutilizada(&ranura, || Err("sin base".to_string()), uno);
        assert_eq!(valor, Err("sin base".to_string()));
        assert!(ranura.lock().unwrap().is_none());
    }
}

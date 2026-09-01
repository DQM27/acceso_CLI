use std::path::PathBuf;

use control_acceso::database::backup::{RespaldoResumen, ResultadoValidacion, TipoRespaldo};

use crate::estado::GuiState;

/// Respaldo manual (botón "Crear" — la GUI no expone los demás tipos:
/// automático/pre-migración/pre-restauración los dispara el propio sistema,
/// por flag es exclusivo de la CLI). Autoriza rápido con el núcleo
/// (`autorizar_creacion_respaldo`, una fila) y suelta el candado ANTES de
/// copiar+validar la base completa (~200ms con unos pocos miles de
/// movimientos, ~2s con ~100,000 — medido, `docs/pendientes.md`) usando una
/// conexión propia (`GuiState::conexion_secundaria`): mismo criterio que
/// `comandos/historial.rs`/`comandos/auditoria.rs`, para que ese tiempo no
/// deje sin núcleo a cualquier otro comando en vuelo.
#[tauri::command]
pub fn crear_respaldo(state: tauri::State<GuiState>) -> Result<RespaldoResumen, String> {
    let actor = state.sesion_activa()?;
    let directorio_respaldos = {
        let core = state.core();
        core.autorizar_creacion_respaldo(&actor)
            .map_err(|error| error.to_string())?;
        core.directorio_respaldos()
    };
    let conexion = state.conexion_secundaria()?;
    control_acceso::database::backup::crear_respaldo(
        &conexion,
        &directorio_respaldos,
        TipoRespaldo::Manual,
    )
    .map_err(|error| error.to_string())
}

/// Sólo lee el sistema de archivos y el nombre de cada respaldo — barato,
/// no hace falta evitar el candado como en `crear_respaldo`/`validar_respaldo`.
#[tauri::command]
pub fn listar_respaldos(state: tauri::State<GuiState>) -> Result<Vec<RespaldoResumen>, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .listar_respaldos(&actor)
        .map_err(|error| error.to_string())
}

/// Abre el archivo candidato (no la base activa) y corre
/// `integrity_check`/`foreign_key_check` — con un respaldo grande puede
/// tardar tanto como crearlo, así que autoriza con el núcleo y libera el
/// candado antes de la verificación real (que no toca `AppCore` en
/// absoluto, ver `database::backup::validar_respaldo`).
#[tauri::command]
pub fn validar_respaldo(
    ruta: String,
    state: tauri::State<GuiState>,
) -> Result<ResultadoValidacion, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .autorizar_creacion_respaldo(&actor)
        .map_err(|error| error.to_string())?;
    control_acceso::database::backup::validar_respaldo(&PathBuf::from(ruta))
        .map_err(|error| error.to_string())
}

/// Copia un respaldo ya publicado a donde el operador elija (diálogo nativo
/// del lado del frontend) — mismo criterio que `validar_respaldo`: autoriza
/// y suelta el candado antes de copiar el archivo (el respaldo completo
/// puede pesar tanto como la base misma).
#[tauri::command]
pub fn exportar_respaldo(
    ruta: String,
    destino: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .autorizar_creacion_respaldo(&actor)
        .map_err(|error| error.to_string())?;
    std::fs::copy(PathBuf::from(ruta), PathBuf::from(destino))
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Reemplaza la base activa por `ruta` — ver `GuiState::restaurar_respaldo`
/// para el intercambio completo (respaldo de seguridad, cierre de la
/// conexión activa, reemplazo del archivo, apertura de una nueva). A
/// diferencia de los demás comandos de este archivo, retiene el candado
/// compartido durante TODO el proceso a propósito: es una operación rara,
/// exclusiva y destructiva — nada más debería estar tocando la base
/// mientras el archivo se reemplaza. Cierra la sesión al terminar (éxito o
/// error): la base cambió de identidad, el frontend debe volver a Login.
#[tauri::command]
pub fn restaurar_respaldo(ruta: String, state: tauri::State<GuiState>) -> Result<(), String> {
    let actor = state.sesion_activa()?;
    state
        .restaurar_respaldo(&actor, &PathBuf::from(ruta))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialización estable de `TipoRespaldo` — el frontend (`api/respaldos.ts`)
    /// mantiene un tipo de unión con estos mismos literales a mano; si el
    /// nombre de una variante cambia sin querer, este test lo detecta antes
    /// de que sea un mismatch silencioso del lado de TypeScript.
    #[test]
    fn tipo_respaldo_serializa_como_el_nombre_de_la_variante() {
        assert_eq!(
            serde_json::to_string(&TipoRespaldo::Manual).unwrap(),
            "\"Manual\""
        );
        assert_eq!(
            serde_json::to_string(&TipoRespaldo::PreRestauracion).unwrap(),
            "\"PreRestauracion\""
        );
    }

    #[test]
    fn resultado_validacion_serializa_como_objeto_de_una_clave() {
        assert_eq!(
            serde_json::to_string(&ResultadoValidacion::Valido {
                version_esquema: 15
            })
            .unwrap(),
            r#"{"Valido":{"version_esquema":15}}"#
        );
        assert_eq!(
            serde_json::to_string(&ResultadoValidacion::Invalido("boom".to_owned())).unwrap(),
            r#"{"Invalido":"boom"}"#
        );
    }
}

//! Comandos Tauri agrupados por dominio — un archivo por área de negocio
//! (misma idea que `src/application/*.rs` en el núcleo). Cada uno es una
//! función fina que llama a `AppCore`; la lógica real nunca vive acá.
//!
//! Convención para comandos nuevos (accesos/ingresos, historial, respaldos,
//! etc. — ver `docs/plan-tauri.md`):
//!
//! 1. Sesión: si el método de `AppCore` que se llama recibe `actor: &UsuarioSesion`,
//!    el comando arranca con `let sesion = state.sesion_activa()?;`. Si es una
//!    lectura que el núcleo no gatea por actor, igual pedila con
//!    `state.sesion_activa()?;` — la GUI no debe exponer ninguna lectura sin
//!    sesión activa, aunque el núcleo lo permita (a diferencia de la TUI,
//!    donde la navegación misma es la barrera, acá cualquier pantalla puede
//!    invocar el comando directo vía `invoke()`).
//! 2. Acceso al núcleo: siempre `state.core()`, nunca `state.core.lock()` a
//!    mano — `core()` recupera el mutex si otro comando lo dejó envenenado
//!    por un panic (ver `estado.rs`).
//! 3. Errores: mapealos con el `mensaje_*` de `control_acceso::mensajes` que
//!    corresponda al dominio. Sólo usar `.map_err(|e| e.to_string())` cuando
//!    no exista un `mensaje_*` para ese tipo de error (p. ej. `SchemaError`).
//! 4. Sin lógica propia: si un comando empieza a necesitar algo más que
//!    "armar el DTO de entrada y llamar a `AppCore`", esa lógica va al
//!    núcleo (`application`/`services`), no acá.

pub mod auditoria;
pub mod autenticacion;
pub mod consola;
pub mod contratistas;
pub mod empresas;
pub mod gafetes;
pub mod historial;
pub mod ingresos;
pub mod usuarios;

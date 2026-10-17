//! Comandos Tauri agrupados por dominio — un archivo por área de negocio
//! (misma idea que `src/application/*.rs` en el núcleo). Cada uno es una
//! función fina que llama a `AppCore`; la lógica real nunca vive acá.
//!
//! Convención para comandos nuevos (accesos/ingresos, historial, etc. — ver
//! `docs/plan-tauri.md`):
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
//!    corresponda al dominio. Cuando no exista un `mensaje_*` para ese tipo
//!    de error (p. ej. `SchemaError`, `rusqlite::Error` suelto), usar
//!    `mensaje_generico` de este módulo en vez de `.map_err(|e| e.to_string())`
//!    a mano -- deja el mismo rastro en el log que `mensaje_*` (ver
//!    `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`, punto 5.1):
//!    estos son, por definición, errores sin variante de negocio detrás, así
//!    que categóricamente son técnicos/inesperados.
//! 4. Sin lógica propia: si un comando empieza a necesitar algo más que
//!    "armar el DTO de entrada y llamar a `AppCore`", esa lógica va al
//!    núcleo (`application`/`services`), no acá.
//! 5. Hilo: todo comando se declara `#[tauri::command(async)]` (o `async fn`).
//!    Un comando síncrono normal corre en el hilo principal, el mismo que
//!    dibuja la ventana: cualquier espera (el candado del núcleo, `SQLite`, una
//!    consulta a la nube) congela la interfaz. Con `(async)` corre fuera de
//!    ese hilo. Tauri exige que un comando asíncrono con `State<'_, _>` devuelva
//!    `Result`; si no hay error posible, `Result<_, String>` con `Ok(..)`
//!    (la interfaz recibe el mismo valor).

/// Ver el punto 3 de la convención de arriba. Los tres dejan el detalle en
/// el registro técnico y devuelven un texto en claro (definidos en
/// `control_acceso::mensajes`, compartidos con móvil): la interfaz nunca ve
/// el texto crudo de un error técnico.
pub use control_acceso::mensajes::{mensaje_archivo, mensaje_generico, mensaje_tarea_interrumpida};

pub mod auditoria;
pub mod autenticacion;
pub mod citas;
pub mod contratistas;
pub mod correo;
pub mod empresas;
pub mod exportacion;
pub mod gafetes;
pub mod gafetes_provisionales;
pub mod historial;
pub mod ingresos;
pub mod nube;
pub mod proveedores;
pub mod rutas;

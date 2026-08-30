//! Despachadores `procesar_accion_*` agrupados por área funcional, en vez de
//! vivir todos en `app.rs`: accesos (ingresos/historial/salida rápida),
//! catálogos (contratistas/empresas) y administración (usuarios/auditoría/
//! respaldos). Cada archivo es un `impl App` más — el `match` exhaustivo por
//! `Vista` y la navegación global se quedan en `app.rs`.

mod accesos;
mod admin;
mod catalogos;
mod gafetes;

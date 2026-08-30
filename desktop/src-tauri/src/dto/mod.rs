//! Tipos de frontera agrupados por dominio (mismo criterio que `comandos/`):
//! traducen el shape que le conviene a un formulario web hacia los tipos
//! reales del núcleo.
//!
//! Todo `construir()`/`From`/`.input()` que haga algo más que copiar campos
//! uno a uno (trim, mapear un enum, decidir un valor por defecto, depender
//! de la fecha de hoy) lleva sus propios tests acá — ver
//! `contratistas::tests` para el caso con más lógica.

pub mod contratistas;
pub mod empresas;
pub mod gafetes;
pub mod usuarios;

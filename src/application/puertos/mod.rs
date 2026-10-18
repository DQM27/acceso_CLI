//! Los puertos: lo que la aplicación necesita del mundo de afuera, escrito
//! como traits que **define** esta capa y que **implementan** los adaptadores
//! (hoy `SQLite`, en `database/repositories`).
//!
//! La dependencia apunta hacia adentro: el adaptador conoce el puerto, el
//! puerto no conoce al adaptador. Por eso los archivos de esta carpeta no
//! pueden mencionar `rusqlite` ni `crate::database::repositories`
//! (`tests/arquitectura.rs` lo vigila).
//!
//! Una salvedad que se acepta por ahora: los métodos devuelven
//! `DatabaseError`, el error del adaptador `SQLite`. Darle a cada puerto su
//! propio error es un corte aparte, y sólo vale la pena el día que haya un
//! segundo adaptador.
//!
//! Se mudan de a poco, empezando por los de contratistas, ingresos y
//! gafetes. Los demás traits de repositorio siguen en `database/repositories`.

pub mod contratistas;
pub mod gafetes;
pub mod registros_ingreso;

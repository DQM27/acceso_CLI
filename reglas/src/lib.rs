//! Reglas de negocio puras de Control Acceso.
//!
//! Una sola fuente para las reglas de **criterio** (¿el dato es válido?):
//! las usa el núcleo (`control_acceso`, escritorio y Android) y, compiladas
//! a WebAssembly (`reglas/wasm`), el panel web y las Edge Functions de
//! Supabase. Así una regla se cambia en un solo lugar y vale igual en todas
//! partes (ver `docs/arquitectura/reglas-compartidas.md`).
//!
//! Lo que NO va acá: las reglas de **autoridad** (quién puede hacer qué:
//! RLS, administradores, equipos vigentes) y las que necesitan ver datos
//! (cédula repetida, "¿ya está adentro?"). Ésas se aplican donde están los
//! datos: el núcleo con su base local y la nube con Postgres.

pub mod cedula;
pub mod cita;
pub mod contratista;
pub mod tipo_ingreso;

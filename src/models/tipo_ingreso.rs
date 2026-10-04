//! `TipoIngreso` vive en el crate de reglas compartidas
//! (`control_acceso_reglas::tipo_ingreso`): las reglas del contratista lo
//! necesitan y tienen que compilar también para WebAssembly. Se reexporta
//! acá para que el resto del núcleo siga usando `crate::models::tipo_ingreso`.

pub use control_acceso_reglas::tipo_ingreso::*;

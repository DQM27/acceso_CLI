//! La cédula en su forma única vive en el crate de reglas compartidas
//! (`control_acceso_reglas::cedula`), para que el panel web y las Edge
//! Functions la normalicen con el mismo código. Se reexporta acá para que el
//! resto del núcleo siga usando `crate::domain::cedula`.

pub use control_acceso_reglas::cedula::*;

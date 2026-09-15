/// Catálogo del personal KOF (`docs/planes-implementados/plan-control-rutas.md`)
/// -- personal interno de Coca-Cola FEMSA, con su propio carnet permanente
/// (no un gafete del catálogo compartido). `codigo_empleado` es el dato que
/// trae el carnet (ver `LectorCarnetKof.kt`, largo variable 5-7 dígitos
/// confirmado contra `empleados_costa_rica.sql`), no una cédula -- por eso
/// `cedula` es un campo aparte y `None` por defecto (no viene en la fuente
/// que precarga este catálogo).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct EncargadoRuta {
    pub id: i64,
    pub codigo_empleado: String,
    pub nombre: String,
    pub cedula: Option<String>,
    pub activo: bool,
}

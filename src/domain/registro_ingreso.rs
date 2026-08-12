use super::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};

pub fn verificar_registro_entrada(
    resultado_acceso: &ResultadoAcceso,
    tiene_ingreso_activo: bool,
) -> ResultadoAcceso {
    if tiene_ingreso_activo {
        return ResultadoAcceso::Denegado(
            MotivoDenegacion::IngresoActivo,
        );
    }

    resultado_acceso.clone()
}
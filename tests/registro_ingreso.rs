use control_acceso::domain::registro_ingreso::verificar_registro_entrada;
use control_acceso::domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};

#[test]
fn debe_denegar_si_ya_tiene_un_ingreso_activo() {
    let resultado_acceso = ResultadoAcceso::Permitido;

    let resultado = verificar_registro_entrada(&resultado_acceso, true);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(MotivoDenegacion::IngresoActivo)
    );
}

#[test]
fn debe_permitir_si_no_tiene_ingreso_activo() {
    let resultado_acceso = ResultadoAcceso::Permitido;

    let resultado = verificar_registro_entrada(&resultado_acceso, false);

    assert_eq!(resultado, ResultadoAcceso::Permitido);
}

#[test]
fn debe_conservar_advertencia_si_no_tiene_ingreso_activo() {
    let resultado_acceso = ResultadoAcceso::PermitidoConAdvertencia;

    let resultado = verificar_registro_entrada(&resultado_acceso, false);

    assert_eq!(resultado, ResultadoAcceso::PermitidoConAdvertencia);
}

#[test]
fn debe_conservar_denegacion_por_sin_acceso() {
    let resultado_acceso = ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso);

    let resultado = verificar_registro_entrada(&resultado_acceso, false);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso)
    );
}

#[test]
fn debe_conservar_denegacion_por_praind_vencido() {
    let resultado_acceso = ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido);

    let resultado = verificar_registro_entrada(&resultado_acceso, false);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido)
    );
}

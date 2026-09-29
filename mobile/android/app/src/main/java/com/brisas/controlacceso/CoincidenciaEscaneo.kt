package com.brisas.controlacceso

import uniffi.control_acceso_mobile.ContratistaResumen

/// Decide si el resultado de buscar lo que trajo un escaneo identifica a UN
/// contratista sin ambigüedad, para abrirle el formulario de ingreso sin que
/// quien opera tenga que elegirlo de la lista. `null` = dejar los resultados
/// para que se elija a mano.
///
/// Con un número (cédula, DIMEX, número de licencia...) se exige que la
/// cédula del resultado sea EXACTAMENTE ese número (auditoría OCR
/// 2026-09-28). Antes bastaba con que la búsqueda devolviera un único
/// resultado: `buscarContratistas` busca por coincidencia parcial, así que
/// un número con un dígito mal leído (sin dígito verificador en el frente
/// de la cédula) podía devolver una sola persona -- la equivocada -- y
/// abrirse su formulario como si el escaneo la hubiera identificado.
///
/// Sin dígitos (gafete In House leído por nombre) no hay número que
/// comparar: se mantiene el criterio de resultado único, y el formulario
/// muestra el nombre para que quien opera lo confirme.
internal fun contratistaEscaneadoClaro(valor: String, resultados: List<ContratistaResumen>): ContratistaResumen? {
    val digitos = valor.filter(Char::isDigit)
    if (digitos.isEmpty()) return resultados.singleOrNull()
    return resultados.singleOrNull { it.cedula.filter(Char::isDigit) == digitos }
}

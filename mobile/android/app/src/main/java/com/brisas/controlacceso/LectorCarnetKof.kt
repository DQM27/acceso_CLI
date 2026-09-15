package com.brisas.controlacceso

/// Perfil de OCR aislado para el paso "Gafete KOF" del checklist de rutas
/// (ver `docs/planes-implementados/plan-control-rutas.md` y
/// `PantallaRutas.kt`) -- el encargado de ruta presenta su carnet KOF
/// (personal interno de Coca-Cola FEMSA, no un gafete del catálogo
/// compartido). Activa el perfil que ya estaba analizado pero aislado a
/// propósito en `docs/arquitectura/muestras-ocr-aisladas.md` ("Carnet KOF
/// rojo" / "Carnet Coca-Cola FEMSA frontal", muestras del 2026-09-08): en
/// aquel momento no había proyecto que lo necesitara y su código interno
/// de 7 dígitos no debía usarse como si fuera cédula -- acá SÍ es el dato
/// correcto (el encargado se identifica por su código de empleado KOF, no
/// por cédula).
///
/// A diferencia de [LectorComprobanteRuta.kt], esto NO viene de fotos
/// frescas transcritas línea por línea -- viene de las notas descriptivas
/// ya guardadas de esas 2 fotos (frente y reverso son fotos separadas, un
/// solo carnet). Es un primer corte heurístico: falta probarlo contra el
/// carnet físico real con la cámara (mismo camino de overlay debug que ya
/// se usó para `LectorComprobanteRuta.kt` y probablemente necesite ronda
/// de ajuste igual que ése).
data class CarnetKofDetectado(
    val nombre: String?,
    val codigoEmpleado: String?,
)

// Marca compartida con el comprobante de carga de ruta -- por eso NO
// alcanza sola para clasificar "esto es un carnet KOF", hay que excluir
// explícitamente el comprobante (que también imprime "Coca Cola FEMSA" en
// su encabezado).
private val REGEX_MARCA_FEMSA = Regex("""COCA[\s-]*COLA\s+FEMSA""", RegexOption.IGNORE_CASE)
private val REGEX_MARCADOR_COMPROBANTE = Regex("""Ruta\s*/\s*No\.?\s*de\s*Carga""", RegexOption.IGNORE_CASE)

// Texto decorativo de antigüedad visto en la muestra real ("5 AÑOS") --
// señal fuerte de que es el carnet (nada de esto aparece en el
// comprobante ni en una cédula).
private val REGEX_ANTIGUEDAD = Regex("""\b\d{1,2}\s*A[ÑN]OS\b""", RegexOption.IGNORE_CASE)

// Texto de respaldo/emergencia visto en la muestra real del reverso --
// "compartido con otros carnets" según la nota original, es decir: no
// cambia por persona, sirve como ancla de clasificación del reverso.
private val REGEX_RESPALDO_KOF = Regex("""ALERTA\s+Y\s+RESPUESTA""", RegexOption.IGNORE_CASE)

// Código de empleado: la muestra aislada mostraba `5040017` (7 dígitos) y
// en su momento se asumió que el largo era fijo -- **corregido 2026-09-15**
// contra la base real de empleados que trajo el usuario
// (`empleados_costa_rica.sql`, 1438 filas): el código NO es fijo en 7,
// varía 5-7 dígitos (23 casos de 5, 61 de 6, 1354 de 7 -- ej. `77851`,
// `330002`, `5040017` son todos códigos reales). Se admite ese rango en
// vez de un largo fijo.
//
// Exige la línea COMPLETA (no `\b...\b` suelto en cualquier parte del
// texto): el reverso también trae un teléfono de emergencia
// (`800-2256327`, visto en la muestra real) que esconde una corrida de 7
// dígitos (`2256327`) -- con un `\b\d{7}\b` simple ese teléfono se leía
// como si fuera el código de empleado (bug real atrapado por el test de
// este mismo archivo). El código real aparece solo en su propia línea, el
// teléfono no.
private val REGEX_CODIGO_EMPLEADO_LINEA = Regex("""^\d{5,7}$""")

/// Clasificación: ¿este texto viene de un carnet KOF (frente o reverso)?
/// Excluye explícitamente el comprobante de carga (comparte la marca
/// "Coca Cola FEMSA" en su encabezado) para que las dos pantallas de
/// escaneo no se confundan si alguien apunta la cámara al documento
/// equivocado.
fun esCarnetKof(texto: String): Boolean {
    if (REGEX_MARCADOR_COMPROBANTE.containsMatchIn(texto)) return false
    return REGEX_ANTIGUEDAD.containsMatchIn(texto) ||
        REGEX_RESPALDO_KOF.containsMatchIn(texto) ||
        REGEX_MARCA_FEMSA.containsMatchIn(texto)
}

/// Punto de entrada del perfil. Nombre y código de empleado son
/// independientes a propósito -- frente y reverso son escaneos separados
/// del mismo carnet físico, así que un solo frame casi nunca trae los dos
/// a la vez. Devuelve lo que haya (al menos uno de los dos); quien llama
/// decide si con sólo el nombre alcanza para completar el paso o si
/// además quiere pedir el reverso para el código de empleado.
fun extraerCarnetKof(texto: String): CarnetKofDetectado? {
    if (!esCarnetKof(texto)) return null
    val nombre = extraerNombreCarnetKof(texto)
    val codigo = texto.lines().map { it.trim() }.firstOrNull { REGEX_CODIGO_EMPLEADO_LINEA.matches(it) }
    if (nombre == null && codigo == null) return null
    return CarnetKofDetectado(nombre = nombre, codigoEmpleado = codigo)
}

/// Heurística del nombre (frente): la muestra real trae el nombre en 2
/// líneas seguidas ("Brasly Daniel" / "Chaves Bonilla"), sin dígitos, sin
/// la marca ni el texto de antigüedad. Se descartan líneas cortas (menos
/// de 4 caracteres -- iniciales, decorativos sueltos) y se toman las
/// primeras 2 líneas candidatas que sobreviven el filtro.
private fun extraerNombreCarnetKof(texto: String): String? {
    val lineasCandidatas = texto
        .lines()
        .map { it.trim() }
        .filter { it.length >= 4 }
        .filterNot { linea ->
            REGEX_MARCA_FEMSA.containsMatchIn(linea) ||
                REGEX_ANTIGUEDAD.containsMatchIn(linea) ||
                REGEX_RESPALDO_KOF.containsMatchIn(linea) ||
                linea.any(Char::isDigit)
        }
        .filter { linea -> linea.all { it.isLetter() || it == ' ' } }

    if (lineasCandidatas.isEmpty()) return null
    return lineasCandidatas.take(2).joinToString(" ").trim().takeIf { it.isNotBlank() }
}

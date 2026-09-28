package com.brisas.controlacceso

import uniffi.control_acceso_mobile.DocumentoLeido
import uniffi.control_acceso_mobile.FechaOcr
import uniffi.control_acceso_mobile.RegistroMrz
import uniffi.control_acceso_mobile.documentoDesdeMrz
import uniffi.control_acceso_mobile.nombreLegibleTipoDocumento

// La clasificación y la extracción de documentos viven en el núcleo Rust
// (`mobile/rust-core/src/lectura_documentos/`, auditoría OCR 2026-09-28,
// punto A-1): acá quedan el modelo que usan las pantallas y la traducción
// desde y hacia el núcleo. Las funciones conservan sus nombres de antes para
// que pantallas y tests no cambien.

/// Tipos de documento que el lector sabe clasificar (ver el enum en
/// `identidad.rs`). `DESCONOCIDO` no es un error: la pantalla sigue buscando.
typealias TipoDocumento = uniffi.control_acceso_mobile.TipoDocumento

/// Origen de los datos de un documento leído: frente, MRZ o PDF417.
typealias FuenteDatos = uniffi.control_acceso_mobile.FuenteDatos

/// Resultado normalizado de leer un documento, como lo usan las pantallas y
/// los ViewModel. Lo que un tipo de documento no trae queda en `null`.
data class DocumentoDetectado(
    val tipo: TipoDocumento,
    val numeroDocumento: String,
    val textoBusqueda: String? = null,
    val nombre: String? = null,
    val apellidos: String? = null,
    val nacionalidad: String? = null,
    // Sólo el carnet PRAIND: texto tal cual lo imprime el carnet, no un
    // `empresa_id` (lo empareja `PantallaNuevoContratista`).
    val empresa: String? = null,
    val esExtranjero: Boolean = false,
    val vencimiento: FechaDocumento? = null,
    val fechaNacimiento: FechaDocumento? = null,
    val sexo: Char? = null,
    val fuenteDatos: FuenteDatos = FuenteDatos.OCR_FRENTE,
    val checksumValido: Boolean? = null,
)

/// Nombre para mostrar en el mensaje de la cámara.
fun TipoDocumento.nombreLegible(): String = nombreLegibleTipoDocumento(this)

data class FechaDocumento(val dia: Int, val mes: Int, val anio: Int) {
    fun estaVencida(hoy: FechaDocumento): Boolean {
        val propia = anio * 10000 + mes * 100 + dia
        val actual = hoy.anio * 10000 + hoy.mes * 100 + hoy.dia
        return propia < actual
    }

    /// Edad en años cumplidos a la fecha `hoy`.
    fun edadEnAnios(hoy: FechaDocumento): Int {
        val cumpleañosYaPaso = (hoy.mes > mes) || (hoy.mes == mes && hoy.dia >= dia)
        return hoy.anio - anio - if (cumpleañosYaPaso) 0 else 1
    }

    companion object {
        fun crearValida(dia: Int, mes: Int, anio: Int): FechaDocumento? =
            runCatching { java.time.LocalDate.of(anio, mes, dia) }
                .getOrNull()
                ?.let { FechaDocumento(it.dayOfMonth, it.monthValue, it.year) }
    }
}

/// La fecha real del dispositivo, inyectable en tests para no depender del
/// reloj del sistema al probar vigencia y vencimiento.
fun fechaDeHoy(): FechaDocumento {
    val hoy = java.time.LocalDate.now()
    return FechaDocumento(hoy.dayOfMonth, hoy.monthValue, hoy.year)
}

// --- Traducción desde y hacia el núcleo -----------------------------------

internal fun FechaOcr.aFechaDocumento(): FechaDocumento = FechaDocumento(dia.toInt(), mes.toInt(), anio)

internal fun FechaDocumento.aFechaOcr(): FechaOcr = FechaOcr(dia.toUByte(), mes.toUByte(), anio)

internal fun DocumentoLeido.aDocumentoDetectado(): DocumentoDetectado = DocumentoDetectado(
    tipo = tipo,
    numeroDocumento = numeroDocumento,
    textoBusqueda = textoBusqueda,
    nombre = nombre,
    apellidos = apellidos,
    nacionalidad = nacionalidad,
    empresa = empresa,
    esExtranjero = esExtranjero,
    vencimiento = vencimiento?.aFechaDocumento(),
    fechaNacimiento = fechaNacimiento?.aFechaDocumento(),
    sexo = sexo?.firstOrNull(),
    fuenteDatos = fuenteDatos,
    checksumValido = checksumValido,
)

internal fun DocumentoDetectado.aDocumentoLeido(): DocumentoLeido = DocumentoLeido(
    tipo = tipo,
    numeroDocumento = numeroDocumento,
    textoBusqueda = textoBusqueda,
    nombre = nombre,
    apellidos = apellidos,
    nacionalidad = nacionalidad,
    empresa = empresa,
    esExtranjero = esExtranjero,
    vencimiento = vencimiento?.aFechaOcr(),
    fechaNacimiento = fechaNacimiento?.aFechaOcr(),
    sexo = sexo?.toString(),
    fuenteDatos = fuenteDatos,
    checksumValido = checksumValido,
)

// --- Lectores (delegan en el núcleo) ---------------------------------------

/// Un MRZ ya leído al modelo normalizado (cédula `ID`, DIMEX `C<`,
/// pasaporte TD3; el resto `DESCONOCIDO`).
fun RegistroMrz.aDocumentoDetectado(): DocumentoDetectado = documentoDesdeMrz(this).aDocumentoDetectado()

/// La TIM usa el mismo código MRZ que la cédula de adulto: se distingue por
/// la edad.
fun DocumentoDetectado.reclasificarPorEdad(hoy: FechaDocumento): DocumentoDetectado =
    uniffi.control_acceso_mobile.reclasificarPorEdad(aDocumentoLeido(), hoy.aFechaOcr()).aDocumentoDetectado()

/// Clasifica por palabras clave, antes de extraer ningún campo.
fun clasificarTipoDocumento(texto: String): TipoDocumento =
    uniffi.control_acceso_mobile.clasificarTipoDocumento(texto)

/// Clasifica (si `tipo` no viene ya calculado) y extrae. `null` cuando
/// todavía no hay información suficiente: la pantalla sigue esperando.
fun leerDocumentoDeTexto(texto: String, tipo: TipoDocumento? = null): DocumentoDetectado? =
    uniffi.control_acceso_mobile.leerDocumentoDeTexto(texto, tipo)?.aDocumentoDetectado()

/// Pista para el lector de códigos: el texto parece el reverso de la cédula
/// anterior, la cara que trae el PDF417.
fun pareceReversoCedulaAnterior(texto: String): Boolean =
    uniffi.control_acceso_mobile.pareceReversoCedulaAnterior(texto)

/// Número de cédula nacional (9 dígitos, sin separadores) en el texto.
fun extraerCedulaDeTexto(texto: String): String? = uniffi.control_acceso_mobile.extraerCedulaDeTexto(texto)

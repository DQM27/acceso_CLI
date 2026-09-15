package com.brisas.controlacceso

import androidx.compose.runtime.mutableStateListOf
import androidx.lifecycle.ViewModel
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.util.UUID

/// Un documento de carga por salida -- el comprobante real ("Comprobante de
/// Carga de Ruta") trae uno por cada sub-número tras la barra
/// (`CRR079/ 0001` = principal, `0002`/`0003`/`0004` = H2/H3/H4), ver
/// `docs/planes-implementados/plan-control-rutas.md`. Una salida abierta
/// puede acumular varios (una unidad que vuelve a cargar el mismo día).
data class DocumentoRuta(
    val subNumero: Int,
    val numeroDocumento: String,
    val fechaDocumento: String,
) {
    val etiquetaTipo: String get() = if (subNumero <= 1) "Principal" else "H$subNumero"
}

/// Mock en memoria -- todavía no hay núcleo Rust para este módulo (orden
/// invertido a pedido explícito del usuario: mobile primero para poder
/// mostrar un MVP navegable, núcleo real al final). Nada de esto persiste
/// entre reinicios de la app todavía.
data class SalidaRutaActiva(
    val id: String,
    val numeroRuta: String,
    val encargadoNombre: String,
    val vehiculo: String,
    val documentos: List<DocumentoRuta>,
    val horaSalidaTexto: String,
)

fun fechaHoyTexto(): String =
    LocalDateTime.now().format(DateTimeFormatter.ofPattern("dd.MM.yyyy"))

private fun horaAhoraTexto(): String =
    LocalDateTime.now().format(DateTimeFormatter.ofPattern("HH:mm"))

class RutasViewModel : ViewModel() {
    val activas = mutableStateListOf<SalidaRutaActiva>()

    fun registrarSalida(numeroRuta: String, encargado: String, vehiculo: String, documento: DocumentoRuta) {
        activas.add(
            SalidaRutaActiva(
                id = UUID.randomUUID().toString(),
                numeroRuta = numeroRuta,
                encargadoNombre = encargado,
                vehiculo = vehiculo,
                documentos = listOf(documento),
                horaSalidaTexto = horaAhoraTexto(),
            ),
        )
    }

    fun agregarDocumento(id: String, documento: DocumentoRuta) {
        val indice = activas.indexOfFirst { it.id == id }
        if (indice == -1) return
        activas[indice] = activas[indice].let { it.copy(documentos = it.documentos + documento) }
    }

    fun confirmarRetorno(id: String) {
        activas.removeAll { it.id == id }
    }
}

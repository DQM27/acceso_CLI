package com.brisas.controlacceso

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.Nucleo

/// Pestaña "Externos" (pedido del usuario 2026-10-03): quien entra sin estar
/// en el catálogo de contratistas. En vez de sumar una pestaña más a una
/// fila ya apretada, la que era "Proveedores" se renombra y adentro un
/// selector elige el tipo, igual que el selector de vista de la pantalla
/// de ingreso: "Visita" ([PantallaVisitas], las agendadas en la web),
/// "Proveedor" ([PantallaProveedores]) o "Correo" ([PantallaPorCorreo],
/// visitas sin agendar autorizadas por correo).
///
/// "Visita" está oculta del selector (pedido del usuario 2026-10-17):
/// `PantallaVisitas` sigue intacta; para volver a mostrarla basta con sacarla
/// de `enDesarrollo` en [TipoExterno].
@Composable
fun PantallaExternos(
    nucleo: Nucleo,
    refrescarNube: Int = 0,
) {
    var tipo by rememberSaveable { mutableIntStateOf(0) }
    Column(modifier = Modifier.fillMaxSize()) {
        FilaPildoras(
            opciones = TIPOS_VISIBLES.map { it.etiqueta },
            seleccionado = tipo,
            onSeleccionar = { tipo = it },
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp),
        )
        Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
            when (TIPOS_VISIBLES.getOrElse(tipo) { TipoExterno.PROVEEDOR }) {
                TipoExterno.VISITA -> PantallaVisitas(nucleo, refrescarNube)
                TipoExterno.PROVEEDOR -> PantallaProveedores(nucleo, refrescarNube)
                TipoExterno.POR_CORREO -> PantallaPorCorreo(nucleo, refrescarNube)
            }
        }
    }
}

enum class TipoExterno(val etiqueta: String, val enDesarrollo: Boolean = false) {
    VISITA("Visita", enDesarrollo = true),
    PROVEEDOR("Proveedor"),
    POR_CORREO("Correo"),
}

private val TIPOS_VISIBLES = TipoExterno.entries.filterNot { it.enDesarrollo }

package com.brisas.controlacceso

import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
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
///
/// `tipo` (el índice del selector) lo guarda [PantallaPrincipal]: el tipo
/// elegido cambia el color de toda la sección, también el de la pestaña de
/// arriba (ver `AcentoSeccion.kt`).
@Composable
fun PantallaExternos(
    nucleo: Nucleo,
    refrescarNube: Int = 0,
    tipo: Int = 0,
    onCambiarTipo: (Int) -> Unit = {},
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    Column(modifier = Modifier.fillMaxSize()) {
        FilaPildoras(
            opciones = TIPOS_VISIBLES.map { it.etiqueta },
            seleccionado = tipo,
            onSeleccionar = onCambiarTipo,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp),
        )
        // Mismo deslizamiento con fundido que las pestañas principales (ver
        // `TransicionesPantalla.kt`); `AnimatedContent` y no un `Column` por el
        // mismo motivo que en [PantallaPrincipal] (las listas necesitan un
        // alto acotado).
        AnimatedContent(
            targetState = tipoExternoVisible(tipo),
            modifier = Modifier.weight(1f).fillMaxWidth(),
            transitionSpec = {
                transicionDePestana(TIPOS_VISIBLES.indexOf(initialState), TIPOS_VISIBLES.indexOf(targetState))
            },
            label = "tipoExterno",
        ) { tipoMostrado ->
            // Con su propio acento mientras sale: Proveedor y Correo tienen
            // colores distintos (ver `AcentoSeccion.kt`).
            TemaSeccion(SeccionPrincipal.EXTERNOS, tipoMostrado) {
                when (tipoMostrado) {
                    TipoExterno.VISITA -> PantallaVisitas(nucleo, refrescarNube)
                    TipoExterno.PROVEEDOR -> PantallaProveedores(nucleo, refrescarNube, sincronizando, onSincronizar)
                    TipoExterno.POR_CORREO -> PantallaPorCorreo(nucleo, refrescarNube, sincronizando, onSincronizar)
                }
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

/// El tipo que corresponde al índice del selector (ver `FilaPildoras`).
internal fun tipoExternoVisible(indice: Int): TipoExterno =
    TIPOS_VISIBLES.getOrElse(indice) { TipoExterno.PROVEEDOR }

package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/// Tarjeta flotante, semitransparente, que va abajo (sobre el contenido, sin
/// empujarlo) cuando hay una versión nueva. Sólo UI: el estado lo resuelve
/// [ActualizacionViewModel]. No bloquea la app: aunque la actualización sea
/// obligatoria, el oficial puede seguir registrando (el núcleo ya rechaza la
/// sincronización de una versión no compatible).
///
/// La X la cierra ("Más tarde"): la versión queda pendiente, con el puntito en
/// la inicial del menú de la cuenta (ver [MenuCuenta]). Una obligatoria no
/// tiene X. Textos cortos a propósito.
@Composable
fun AvisoActualizacion(
    estado: EstadoActualizacion,
    onActualizar: () -> Unit,
    onPosponer: () -> Unit,
    onAbrirAjustes: () -> Unit,
    modifier: Modifier = Modifier,
) {
    if (estado !is EstadoActualizacion.ConNovedad) return
    if (estado is EstadoActualizacion.Disponible && estado.pospuesta) return
    val urgente = estado.novedad.obligatoria || estado is EstadoActualizacion.Fallo
    val fondo = if (urgente) MaterialTheme.colorScheme.errorContainer else MaterialTheme.colorScheme.tertiaryContainer
    Surface(
        modifier = modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.large,
        color = fondo.copy(alpha = OPACIDAD_TARJETA),
        contentColor = if (urgente) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onTertiaryContainer,
        shadowElevation = 6.dp,
    ) {
        Column(
            modifier = Modifier.padding(horizontal = EspacioControlBrisas, vertical = 10.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    texto(estado),
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.weight(1f),
                )
                when (estado) {
                    is EstadoActualizacion.Descargando, is EstadoActualizacion.Instalando -> Unit
                    else -> {
                        if (estado is EstadoActualizacion.FaltaPermiso) {
                            BotonBrisas(onClick = onAbrirAjustes) { Text("Ajustes") }
                        }
                        BotonBrisas(onClick = onActualizar) {
                            Text(if (estado is EstadoActualizacion.Fallo) "Reintentar" else "Actualizar")
                        }
                        if (estado is EstadoActualizacion.Disponible && !estado.novedad.obligatoria) {
                            BotonIconoCuadradoBrisas(onClick = onPosponer) {
                                Icon(Icons.Default.Close, contentDescription = "Cerrar")
                            }
                        }
                    }
                }
            }
            when (estado) {
                is EstadoActualizacion.Descargando ->
                    LinearProgressIndicator(progress = { estado.porcentaje / 100f }, modifier = Modifier.fillMaxWidth())
                is EstadoActualizacion.Instalando -> LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                else -> Unit
            }
        }
    }
}

/// Lo bastante opaca para leerse sobre cualquier pantalla, y lo bastante
/// transparente para que se vea que flota sobre el contenido.
private const val OPACIDAD_TARJETA = 0.94f

private fun texto(estado: EstadoActualizacion.ConNovedad): String {
    val version = estado.novedad.manifiesto.versionName
    return when (estado) {
        is EstadoActualizacion.Disponible ->
            if (estado.novedad.obligatoria) "Actualización obligatoria: $version" else "Nueva versión: $version"
        is EstadoActualizacion.FaltaPermiso -> "Permita instalar apps (sólo una vez)"
        is EstadoActualizacion.Descargando -> "Descargando $version… ${estado.porcentaje} %"
        is EstadoActualizacion.Instalando -> "Instalando $version. Vuelva a abrir Lattis."
        is EstadoActualizacion.Fallo -> estado.mensaje
    }
}

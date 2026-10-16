package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp

/// Franja sobre todas las pantallas cuando hay una versión nueva (ver
/// [Actualizador]). No bloquea la app: aunque la actualización sea
/// obligatoria, el oficial puede seguir registrando mientras tanto (el núcleo
/// ya rechaza la sincronización de una versión no compatible).
@Composable
fun AvisoActualizacion(modifier: Modifier = Modifier) {
    val estado by Actualizador.estado.collectAsState()
    val context = LocalContext.current
    val novedad = when (val actual = estado) {
        EstadoActualizacion.SinAviso -> return
        is EstadoActualizacion.Disponible -> actual.novedad
        is EstadoActualizacion.FaltaPermiso -> actual.novedad
        is EstadoActualizacion.Descargando -> actual.novedad
        is EstadoActualizacion.Instalando -> actual.novedad
        is EstadoActualizacion.Fallo -> actual.novedad
    }
    val urgente = novedad.obligatoria || estado is EstadoActualizacion.Fallo
    Surface(
        modifier = modifier.fillMaxWidth(),
        color = if (urgente) MaterialTheme.colorScheme.errorContainer else MaterialTheme.colorScheme.tertiaryContainer,
        contentColor = if (urgente) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onTertiaryContainer,
    ) {
        Column(
            modifier = Modifier.padding(horizontal = EspacioControlBrisas, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text(texto(estado, novedad), style = MaterialTheme.typography.bodyMedium)
            when (val actual = estado) {
                is EstadoActualizacion.Descargando -> {
                    val porcentaje = actual.porcentaje
                    if (porcentaje == null) {
                        LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                    } else {
                        LinearProgressIndicator(progress = { porcentaje / 100f }, modifier = Modifier.fillMaxWidth())
                    }
                }
                is EstadoActualizacion.Instalando -> LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                else -> Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    if (actual is EstadoActualizacion.Disponible && !novedad.obligatoria) {
                        BotonDiscretoBrisas(onClick = Actualizador::posponer) { Text("Más tarde") }
                    }
                    if (actual is EstadoActualizacion.FaltaPermiso) {
                        BotonBrisas(onClick = { context.startActivity(Actualizador.intentPermiso(context)) }) {
                            Text("Abrir ajustes")
                        }
                    }
                    BotonBrisas(onClick = { Actualizador.actualizar(context) }) {
                        Text(if (actual is EstadoActualizacion.Fallo) "Reintentar" else "Actualizar")
                    }
                }
            }
        }
    }
}

private fun texto(estado: EstadoActualizacion, novedad: NovedadActualizacion.Disponible): String {
    val version = novedad.manifiesto.versionName
    return when (estado) {
        is EstadoActualizacion.Disponible ->
            if (novedad.obligatoria) {
                "Esta versión ya no es compatible. Actualice a la $version para seguir sincronizando."
            } else {
                "Hay una versión nueva de Lattis: $version."
            }
        is EstadoActualizacion.FaltaPermiso ->
            "Para actualizar, permita que Lattis instale aplicaciones (sólo se hace una vez) y vuelva a tocar Actualizar."
        is EstadoActualizacion.Descargando ->
            estado.porcentaje?.let { "Descargando la versión $version… $it %" } ?: "Descargando la versión $version…"
        is EstadoActualizacion.Instalando ->
            "Instalando la versión $version. Al terminar, Lattis se cierra: vuelva a abrirla."
        is EstadoActualizacion.Fallo -> estado.mensaje
        EstadoActualizacion.SinAviso -> ""
    }
}

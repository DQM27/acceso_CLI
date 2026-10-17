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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/// Franja sobre todas las pantallas cuando hay una versión nueva. Sólo UI:
/// el estado lo resuelve [ActualizacionViewModel]. No bloquea la app: aunque
/// la actualización sea obligatoria, el oficial puede seguir registrando (el
/// núcleo ya rechaza la sincronización de una versión no compatible).
@Composable
fun AvisoActualizacion(
    estado: EstadoActualizacion,
    onActualizar: () -> Unit,
    onPosponer: () -> Unit,
    onAbrirAjustes: () -> Unit,
    modifier: Modifier = Modifier,
) {
    if (estado !is EstadoActualizacion.ConNovedad) return
    val urgente = estado.novedad.obligatoria || estado is EstadoActualizacion.Fallo
    Surface(
        modifier = modifier.fillMaxWidth(),
        color = if (urgente) MaterialTheme.colorScheme.errorContainer else MaterialTheme.colorScheme.tertiaryContainer,
        contentColor = if (urgente) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onTertiaryContainer,
    ) {
        Column(
            modifier = Modifier.padding(horizontal = EspacioControlBrisas, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text(texto(estado), style = MaterialTheme.typography.bodyMedium)
            when (estado) {
                is EstadoActualizacion.Descargando ->
                    LinearProgressIndicator(progress = { estado.porcentaje / 100f }, modifier = Modifier.fillMaxWidth())
                is EstadoActualizacion.Instalando -> LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                else -> Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    if (estado is EstadoActualizacion.Disponible && !estado.novedad.obligatoria) {
                        BotonDiscretoBrisas(onClick = onPosponer) { Text("Más tarde") }
                    }
                    if (estado is EstadoActualizacion.FaltaPermiso) {
                        BotonBrisas(onClick = onAbrirAjustes) { Text("Abrir ajustes") }
                    }
                    BotonBrisas(onClick = onActualizar) {
                        Text(if (estado is EstadoActualizacion.Fallo) "Reintentar" else "Actualizar")
                    }
                }
            }
        }
    }
}

private fun texto(estado: EstadoActualizacion.ConNovedad): String {
    val version = estado.novedad.manifiesto.versionName
    return when (estado) {
        is EstadoActualizacion.Disponible ->
            if (estado.novedad.obligatoria) {
                "Esta versión ya no es compatible. Actualice a la $version para seguir sincronizando."
            } else {
                "Hay una versión nueva de Lattis: $version."
            }
        is EstadoActualizacion.FaltaPermiso ->
            "Para actualizar, permita que Lattis instale aplicaciones (sólo se hace una vez) y vuelva a tocar Actualizar."
        is EstadoActualizacion.Descargando -> "Descargando la versión $version… ${estado.porcentaje} %"
        is EstadoActualizacion.Instalando ->
            "Instalando la versión $version. Al terminar, Lattis se cierra: vuelva a abrirla."
        is EstadoActualizacion.Fallo -> estado.mensaje
    }
}

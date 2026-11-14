package com.brisas.controlacceso

import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import uniffi.control_acceso_mobile.EstadoBandeja

// Sacada de `PantallaPrincipal.kt` (fase 4 de
// `docs/handoff-rediseno-movil-ux.md`: un archivo, una responsabilidad).

/// Campanita de la barra superior: el globito (qué número, si se ve) y si
/// hay una emergencia pendiente lo decide el núcleo (`EstadoBandeja`). El
/// globito va sobre la esquina del botón, no adentro: el cuadradito recorta
/// su contenido.
@Composable
internal fun CampanitaAvisos(bandeja: EstadoBandeja?, onClick: () -> Unit) {
    val contador = bandeja?.contador
    val hayEmergencia = bandeja?.emergenciaSinLeer != null
    BadgedBox(badge = { if (contador != null) Badge { Text(contador) } }) {
        BotonIconoCuadradoBrisas(onClick = onClick) {
            Icon(
                if (hayEmergencia) IconosLucide.CampanaUrgente else IconosLucide.Campana,
                contentDescription = if (contador == null) "Avisos" else "Avisos, ${bandeja?.noLeidos} sin leer",
                tint = if (hayEmergencia) MaterialTheme.colorScheme.error else LocalContentColor.current,
            )
        }
    }
}

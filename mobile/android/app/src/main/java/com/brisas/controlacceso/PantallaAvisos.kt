package com.brisas.controlacceso

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material.icons.filled.NotificationsNone
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.AvisoBandeja
import uniffi.control_acceso_mobile.EstadoBandeja
import uniffi.control_acceso_mobile.GrupoBandeja

/// Bandeja de avisos (la campanita de PantallaPrincipal): lo que mandó la
/// administración desde el panel web. Sólo dibuja [EstadoBandeja], que arma
/// el núcleo (mobile/rust-core/src/avisos.rs): grupos, orden, "hace 5 min",
/// leídos. Pantalla completa, no un panel desplegable: en un teléfono no hay
/// lugar para otra cosa.
///
/// Tocar un aviso lo despliega entero y lo marca como leído. No se borran a
/// mano: el núcleo los vence solo a los 30 días.
@Composable
fun PantallaAvisos(
    estado: EstadoBandeja?,
    onLeer: (String) -> Unit,
    onLeerTodos: () -> Unit,
    onVolver: () -> Unit,
) {
    RegistrarPantalla("avisos")
    BackHandler(onBack = onVolver)
    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("Avisos", style = MaterialTheme.typography.titleMedium)
            BotonDiscretoBrisas(onClick = onVolver) {
                Text("← Volver")
            }
        }
        if (estado == null) return@Column
        if (estado.noLeidos > 0u) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    "${estado.noLeidos} sin leer",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                BotonDiscretoBrisas(onClick = onLeerTodos) {
                    Text("Marcar todo como leído")
                }
            }
        }

        if (estado.grupos.isEmpty()) {
            BandejaVacia(Modifier.weight(1f).fillMaxWidth())
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f).fillMaxWidth(),
                contentPadding = PaddingValues(vertical = 8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                for (grupo in estado.grupos) {
                    item(key = "grupo-${grupo.grupo.name}") {
                        Text(
                            tituloGrupo(grupo.grupo),
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(top = 8.dp),
                        )
                    }
                    items(grupo.avisos, key = { it.id }) { aviso -> FilaAviso(aviso, onLeer) }
                }
            }
        }
    }
}

private fun tituloGrupo(grupo: GrupoBandeja): String = when (grupo) {
    GrupoBandeja.HOY -> "HOY"
    GrupoBandeja.AYER -> "AYER"
    GrupoBandeja.ANTERIORES -> "ANTERIORES"
}

@Composable
private fun FilaAviso(aviso: AvisoBandeja, onLeer: (String) -> Unit) {
    var desplegado by rememberSaveable(aviso.id) { mutableStateOf(false) }
    Surface(
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.surface,
        border = if (aviso.emergente) BorderStroke(1.dp, MaterialTheme.colorScheme.error) else null,
        modifier = Modifier.fillMaxWidth().clickable {
            desplegado = !desplegado
            onLeer(aviso.id)
        },
    ) {
        Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.Top) {
            // Punto de "sin leer": ocupa lugar igual cuando no está, para que
            // los textos queden alineados.
            Box(
                modifier = Modifier.padding(top = 6.dp).size(8.dp).background(
                    if (aviso.leido) Color.Transparent else MaterialTheme.colorScheme.primary,
                    CircleShape,
                ),
            )
            Spacer(Modifier.width(8.dp))
            Icon(
                if (aviso.emergente) Icons.Default.Warning else Icons.Default.Notifications,
                contentDescription = if (aviso.emergente) "Emergencia" else null,
                tint = if (aviso.emergente) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(20.dp),
            )
            Spacer(Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        aviso.titulo,
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = if (aviso.leido) FontWeight.Normal else FontWeight.SemiBold,
                        maxLines = if (desplegado) Int.MAX_VALUE else 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f),
                    )
                    Spacer(Modifier.width(8.dp))
                    Text(
                        aviso.momento,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                if (aviso.cuerpo.isNotBlank()) {
                    Text(
                        aviso.cuerpo,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = if (desplegado) Int.MAX_VALUE else 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
    }
}

@Composable
private fun BandejaVacia(modifier: Modifier) {
    Column(
        modifier = modifier.padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.NotificationsNone,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.size(48.dp),
        )
        Spacer(Modifier.size(12.dp))
        Text("Sin avisos", style = MaterialTheme.typography.titleSmall)
        Text(
            "Acá vas a ver los avisos que mande la administración.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
}

/// Franja roja fija arriba de la pantalla principal mientras haya una
/// emergencia sin leer (lo decide el núcleo): no se descarta, se va al
/// leerla en la bandeja.
@Composable
fun FranjaEmergencia(aviso: AvisoBandeja, onAbrir: () -> Unit, modifier: Modifier = Modifier) {
    Surface(
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.errorContainer,
        contentColor = MaterialTheme.colorScheme.onErrorContainer,
        modifier = modifier.fillMaxWidth().clickable(onClick = onAbrir),
    ) {
        Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(Icons.Default.Warning, contentDescription = null, modifier = Modifier.size(20.dp))
            Spacer(Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    "Aviso urgente: ${aviso.titulo}",
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                Text("Tocá para leerlo", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}

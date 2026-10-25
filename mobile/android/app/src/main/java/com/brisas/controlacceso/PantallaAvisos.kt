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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.foundation.layout.offset
import androidx.compose.material.icons.automirrored.filled.ArrowBackIos
import androidx.compose.material.icons.filled.Archive
import androidx.compose.material.icons.filled.DeleteOutline
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.vector.ImageVector
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
/// leídos, archivados y qué se puede quitar. Pantalla completa, no un panel
/// desplegable: en un teléfono no hay lugar para otra cosa. Diseño tomado del
/// centro de notificaciones de iOS (2026-10-21): título grande, tarjetas de
/// esquinas grandes con el ícono en un cuadradito de color y grupos "Hoy",
/// "Ayer", "Anteriores".
///
/// Tocar un aviso lo despliega entero (con animación) y lo marca como leído;
/// ahí aparecen "Archivar" y "Borrar" (una emergencia sin leer todavía no los ofrece:
/// el núcleo tampoco la deja quitar). Borrar pide confirmación porque queda
/// anotado en el servidor quién lo hizo. Los avisos se van solos a los 30
/// días.
@Composable
fun PantallaAvisos(
    estado: EstadoBandeja?,
    onLeer: (String) -> Unit,
    onLeerTodos: () -> Unit,
    onArchivar: (String) -> Unit,
    onBorrar: (String) -> Unit,
    onVolver: () -> Unit,
) {
    RegistrarPantalla("avisos")
    BackHandler(onBack = onVolver)
    var pestana by rememberSaveable { mutableIntStateOf(0) }
    var porBorrar by rememberSaveable { mutableStateOf<String?>(null) }

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp)) {
        // Como iOS: "‹ Volver" chico arriba y el título grande debajo, con la
        // acción de la pantalla a su derecha.
        BotonDiscretoBrisas(onClick = onVolver, modifier = Modifier.offset(x = (-12).dp)) {
            Icon(Icons.AutoMirrored.Filled.ArrowBackIos, contentDescription = null, modifier = Modifier.size(16.dp))
            Text("Volver")
        }
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                "Avisos",
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.weight(1f),
            )
            if (estado != null && pestana == 0 && estado.noLeidos > 0u) {
                BotonDiscretoBrisas(onClick = onLeerTodos) {
                    Text("Marcar leídos")
                }
            }
        }
        if (estado == null) return@Column

        FilaPildoras(
            opciones = listOf(
                if (estado.noLeidos > 0u) "Avisos (${estado.noLeidos})" else "Avisos",
                "Archivados (${estado.archivados.size})",
            ),
            seleccionado = pestana,
            onSeleccionar = { pestana = it },
            modifier = Modifier.padding(top = 8.dp),
        )

        // Avisos y archivados se deslizan entre sí, como las pestañas.
        AnimatedContent(
            targetState = pestana,
            modifier = Modifier.weight(1f).fillMaxWidth(),
            transitionSpec = { transicionDePestana(initialState, targetState) },
            label = "pestanaAvisos",
        ) { mostrada ->
            if (mostrada == 0) {
                if (estado.grupos.isEmpty()) {
                    MensajeVacio("Todo al día", "Aquí verá los avisos que mande la administración.")
                } else {
                    LazyColumn(
                        contentPadding = PaddingValues(top = 8.dp, bottom = 24.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        for (grupo in estado.grupos) {
                            item(key = "grupo-${grupo.grupo.name}") {
                                Text(
                                    tituloGrupo(grupo.grupo),
                                    style = MaterialTheme.typography.titleSmall,
                                    fontWeight = FontWeight.Bold,
                                    modifier = Modifier.padding(top = 12.dp, start = 4.dp).animateItem(),
                                )
                            }
                            items(grupo.avisos, key = { it.id }) { aviso ->
                                FilaAviso(
                                    aviso = aviso,
                                    onLeer = onLeer,
                                    onArchivar = { onArchivar(aviso.id) },
                                    onBorrar = { porBorrar = aviso.id },
                                    modifier = Modifier.animateItem(),
                                )
                            }
                        }
                    }
                }
            } else if (estado.archivados.isEmpty()) {
                MensajeVacio("Sin archivados", "Los avisos que archive se guardan aquí hasta que se borren solos.")
            } else {
                LazyColumn(
                    contentPadding = PaddingValues(top = 12.dp, bottom = 24.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    items(estado.archivados, key = { it.id }) { aviso ->
                        FilaAviso(
                            aviso = aviso,
                            onLeer = onLeer,
                            onArchivar = null,
                            onBorrar = { porBorrar = aviso.id },
                            modifier = Modifier.animateItem(),
                        )
                    }
                }
            }
        }
    }

    val idPorBorrar = porBorrar
    if (idPorBorrar != null) {
        AlertDialog(
            onDismissRequest = { porBorrar = null },
            title = { Text("¿Borrar este aviso?") },
            text = { Text("Se quita de este teléfono y queda anotado quién lo borró.") },
            confirmButton = {
                BotonDiscretoBrisas(onClick = {
                    onBorrar(idPorBorrar)
                    porBorrar = null
                }) {
                    Text("Borrar", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                BotonDiscretoBrisas(onClick = { porBorrar = null }) {
                    Text("Cancelar")
                }
            },
        )
    }
}

private fun tituloGrupo(grupo: GrupoBandeja): String = when (grupo) {
    GrupoBandeja.HOY -> "Hoy"
    GrupoBandeja.AYER -> "Ayer"
    GrupoBandeja.ANTERIORES -> "Anteriores"
}

/// Una tarjeta como las notificaciones de iOS: esquinas grandes, el ícono en un
/// cuadradito de color a la izquierda (rojo con la advertencia si es una
/// emergencia, que además tiñe la tarjeta), el título con la hora a la derecha
/// y el texto debajo. Tocarla la despliega entera (con animación) y la marca
/// como leída; ahí aparecen "Archivar" y "Borrar" como píldoras.
///
/// `onArchivar` es `null` en la pestaña de archivados (ya lo está).
@Composable
private fun FilaAviso(
    aviso: AvisoBandeja,
    onLeer: (String) -> Unit,
    onArchivar: (() -> Unit)?,
    onBorrar: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var desplegado by rememberSaveable(aviso.id) { mutableStateOf(false) }
    val error = MaterialTheme.colorScheme.error
    Surface(
        shape = RoundedCornerShape(20.dp),
        color = if (aviso.emergente) lerp(MaterialTheme.colorScheme.surface, error, 0.10f) else MaterialTheme.colorScheme.surface,
        border = if (aviso.emergente) BorderStroke(1.dp, error.copy(alpha = 0.5f)) else null,
        modifier = modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier
                .clickable {
                    desplegado = !desplegado
                    onLeer(aviso.id)
                }
                .animateContentSize(spring(dampingRatio = 0.8f, stiffness = Spring.StiffnessMediumLow))
                .padding(12.dp),
        ) {
            Row(verticalAlignment = Alignment.Top) {
                Box(
                    modifier = Modifier
                        .size(38.dp)
                        .background(
                            if (aviso.emergente) error else MaterialTheme.colorScheme.primary.copy(alpha = 0.14f),
                            RoundedCornerShape(10.dp),
                        ),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        if (aviso.emergente) Icons.Default.Warning else Icons.Default.Notifications,
                        contentDescription = if (aviso.emergente) "Emergencia" else null,
                        tint = if (aviso.emergente) MaterialTheme.colorScheme.onError else MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(20.dp),
                    )
                }
                Spacer(Modifier.width(12.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            aviso.titulo,
                            style = MaterialTheme.typography.bodyLarge,
                            fontWeight = if (aviso.leido) FontWeight.Medium else FontWeight.Bold,
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
                        // Sin leer: un punto del color de la app junto a la hora.
                        if (!aviso.leido) {
                            Spacer(Modifier.width(6.dp))
                            Box(Modifier.size(8.dp).background(MaterialTheme.colorScheme.primary, CircleShape))
                        }
                    }
                    if (aviso.cuerpo.isNotBlank()) {
                        Text(
                            aviso.cuerpo,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = if (desplegado) Int.MAX_VALUE else 2,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.padding(top = 2.dp),
                        )
                    }
                }
            }
            // Una emergencia sin leer no se quita: el núcleo tampoco lo
            // permite. Al desplegarla se da por leída y los botones aparecen.
            if (desplegado && (!aviso.emergente || aviso.leido)) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(top = 12.dp, start = 50.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    if (onArchivar != null) {
                        PildoraAccion("Archivar", Icons.Default.Archive, MaterialTheme.colorScheme.onSurface, onArchivar)
                    }
                    PildoraAccion("Borrar", Icons.Default.DeleteOutline, error, onBorrar)
                }
            }
        }
    }
}

/// Botón de acción de una tarjeta de aviso: píldora de fondo suave del mismo
/// color que el texto.
@Composable
private fun PildoraAccion(texto: String, icono: ImageVector, color: Color, onClick: () -> Unit) {
    Row(
        modifier = Modifier
            .clip(CircleShape)
            .background(color.copy(alpha = 0.10f))
            .clickable(onClick = onClick)
            .padding(horizontal = 14.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icono, contentDescription = null, tint = color, modifier = Modifier.size(18.dp))
        Spacer(Modifier.width(6.dp))
        Text(texto, color = color, style = MaterialTheme.typography.labelLarge)
    }
}

@Composable
private fun MensajeVacio(titulo: String, detalle: String) {
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Box(
            modifier = Modifier
                .size(72.dp)
                .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.12f), CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                Icons.Default.NotificationsNone,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(36.dp),
            )
        }
        Spacer(Modifier.size(16.dp))
        Text(titulo, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        Text(
            detalle,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 4.dp),
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
                Text("Toque para leerlo", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}

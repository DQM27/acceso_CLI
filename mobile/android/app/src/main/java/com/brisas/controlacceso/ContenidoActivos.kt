package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import uniffi.control_acceso_mobile.ContratistaResumen

// Bloques de la pantalla de Activos: selector de modo, buscador, mensajes,
// contenido por modo y diálogo de salida (sacado de `PantallaActivos.kt`,
// punto M6 de la auditoría móvil: sólo se movió código).

/// Píldoras "Ingreso"/"Salida"/"Gafete" (`FilaPildoras`, ver ControlesBrisas.kt)
/// en vez del `SegmentedButton` gris de Material3 -- mismos tres modos de
/// siempre, mismo doc-comment de [PantallaActivos] para el porqué de cada uno.
@Composable
internal fun SelectorModoBusqueda(modo: ModoBusqueda, onCambiar: (ModoBusqueda) -> Unit) {
    val opciones = listOf(ModoBusqueda.ENTRADA, ModoBusqueda.SALIDA_NOMBRE, ModoBusqueda.SALIDA_GAFETE)
    FilaPildoras(
        opciones = listOf("Ingreso", "Salida", "Gafete"),
        seleccionado = opciones.indexOf(modo),
        onSeleccionar = { onCambiar(opciones[it]) },
    )
}

@Composable
internal fun CampoBusquedaActivos(
    modo: ModoBusqueda,
    texto: String,
    onCambiarTexto: (String) -> Unit,
    onEscanearCedula: () -> Unit,
    onEscanearGafete: () -> Unit,
) {
    // El botón de cámara va en el color de la sección en los dos modos. Antes
    // salida usaba `secondary` (azul fijo, de cuando toda la app era azul) y
    // con los colores por sección quedaba un botón azul al lado de la
    // píldora "Salida" verde azulado; el modo ya lo dicen la píldora y el
    // texto de ayuda del buscador.
    Row(
        modifier = Modifier.fillMaxWidth().padding(top = 6.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // El mismo campo cambia de sentido según el modo: el texto de ayuda dice
        // qué se busca en cada uno.
        CampoBusquedaBrisas(
            valor = texto,
            onCambiar = onCambiarTexto,
            ayuda = when (modo) {
                ModoBusqueda.ENTRADA -> "Nombre o cédula para ingresar"
                ModoBusqueda.SALIDA_NOMBRE -> "Nombre o cédula de quien sale"
                ModoBusqueda.SALIDA_GAFETE -> "Gafetes: 2, 25, 85"
            },
            keyboardOptions = if (modo == ModoBusqueda.SALIDA_GAFETE) {
                KeyboardOptions(keyboardType = KeyboardType.Number)
            } else {
                KeyboardOptions.Default
            },
            modifier = Modifier.weight(1f),
        )
        if (modo == ModoBusqueda.ENTRADA || modo == ModoBusqueda.SALIDA_GAFETE) {
            BotonCuadradoBordeBrisas(
                icono = Icons.Default.PhotoCamera,
                descripcion = if (modo == ModoBusqueda.ENTRADA) "Escanear documento" else "Escanear gafete",
                onClick = if (modo == ModoBusqueda.ENTRADA) onEscanearCedula else onEscanearGafete,
            )
        }
    }
}

@Composable
internal fun MensajesEstado(error: String?, mensaje: String?, mensajeEsError: Boolean, verificando: Boolean) {
    if (error != null) {
        Text(error, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(top = 12.dp))
    }
    if (mensaje != null) {
        Text(
            mensaje,
            color = if (mensajeEsError) MaterialTheme.colorScheme.error else ColorExitoBrisas,
            modifier = Modifier.padding(top = 12.dp),
        )
    }
    if (verificando) {
        Text(
            "Verificando…",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 12.dp),
        )
    }
}

/// Vacío: lista de quién está adentro (tocar = salida). Con texto: busca en
/// el catálogo completo (tocar = arrancar el flujo de entrada). Ver el
/// doc-comment de [PantallaActivos].
@Composable
internal fun ContenidoModoEntrada(
    texto: String,
    activos: List<FilaActiva>,
    resultadosBusqueda: List<ContratistaResumen>,
    verificando: Boolean,
    cargando: Boolean,
    onElegirActivo: (FilaActiva) -> Unit,
    onElegirContratista: (ContratistaResumen) -> Unit,
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    if (texto.isBlank()) {
        ListaActivos(
            activos = activos,
            cargando = cargando,
            tituloVacio = "Nadie adentro por ahora",
            detalleVacio = "Cuando alguien ingrese, aparece acá.",
            onClick = onElegirActivo,
            sincronizando = sincronizando,
            onSincronizar = onSincronizar,
        )
        return
    }
    // Misma lista que la de activos (animación, estados vacíos, tirar para
    // sincronizar): mientras busca la primera vez muestra marcadores, y si no
    // encuentra a nadie lo dice con el mismo mensaje que las demás listas.
    ListaConEstados(
        filas = resultadosBusqueda,
        clave = { it.id },
        cargando = cargando || verificando,
        tituloVacio = "Sin resultados",
        detalleVacio = "Pruebe con otro nombre o cédula.",
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
    ) { contratista ->
        FilaContratista(contratista, onClick = { onElegirContratista(contratista) })
    }
}

/// Filtra la lista de activos por cédula/nombre — vacío no trae nada, es un
/// buscador para acotar, no una lista para recorrer (esa es el modo
/// Entrada). Ver el doc-comment de [PantallaActivos].
@Composable
internal fun ContenidoModoSalidaNombre(
    texto: String,
    activos: List<FilaActiva>,
    cargando: Boolean,
    onElegirActivo: (FilaActiva) -> Unit,
    sincronizando: Boolean = false,
    onSincronizar: () -> Unit = {},
) {
    ListaActivos(
        activos = activos,
        cargando = cargando,
        tituloVacio = if (texto.isBlank()) "Busque a quien sale" else "Sin resultados",
        detalleVacio = if (texto.isBlank()) "Escriba un nombre o una cédula." else "Pruebe con otro nombre o cédula.",
        onClick = onElegirActivo,
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
    )
}

/// Sólo para el tipeo manual -- uno o más números de gafete separados por
/// coma, con vista previa de a quién le corresponde cada uno antes de un
/// único botón que confirma todos de una vez. El escaneo por cámara
/// (`escanerGafeteSalidaAbierto` en [PantallaActivos]) no pasa por acá:
/// escanear ya es la acción deliberada de sacar un gafete, así que registra
/// la salida al toque, sin botón de confirmar de por medio (pedido
/// explícito del usuario 2026-09-20). Ver el doc-comment de
/// [PantallaActivos].
@Composable
internal fun ContenidoModoSalidaGafete(
    texto: String,
    coincidencias: List<CoincidenciaGafete>,
    enviando: Boolean,
    onRegistrarSalidaGafetes: () -> Unit,
) {
    Column(modifier = Modifier.padding(top = 8.dp)) {
        if (texto.isBlank()) {
            Text(
                "Escriba uno o más números de gafete, separados por coma",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            return
        }

        coincidencias.forEach { coincidencia ->
            val filaCoincidente = coincidencia.fila
            Text(
                "Gafete ${coincidencia.numero} · " +
                    (
                        filaCoincidente?.let { fila ->
                            listOfNotNull(fila.contratistaNombre, fila.empresaNombre).joinToString(" · ")
                        } ?: "Sin ingreso activo"
                    ),
                style = MaterialTheme.typography.bodyMedium,
                color = if (filaCoincidente != null) {
                    MaterialTheme.colorScheme.onSurface
                } else {
                    MaterialTheme.colorScheme.error
                },
                modifier = Modifier.padding(vertical = 4.dp),
            )
        }

        val encontrados = coincidencias.filter { it.fila != null }
        if (encontrados.isNotEmpty()) {
            BotonBrisas(
                onClick = onRegistrarSalidaGafetes,
                enabled = !enviando,
                modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
            ) {
                Text(if (enviando) "Registrando…" else "Registrar salida (${encontrados.size})")
            }
        }
    }
}

/// Lista de activos compartida entre Entrada (campo vacío) y Salida:
/// nombre — mismas filas, mismo comportamiento, sólo cambia qué hace
/// `onClick` según quién la use. El comportamiento de la lista (animación,
/// estados vacíos, tirar para sincronizar) vive en [ListaConEstados].
@Composable
private fun ListaActivos(
    activos: List<FilaActiva>,
    cargando: Boolean,
    tituloVacio: String,
    detalleVacio: String,
    onClick: (FilaActiva) -> Unit,
    sincronizando: Boolean,
    onSincronizar: () -> Unit,
) {
    ListaConEstados(
        filas = activos,
        clave = { fila -> fila.clave },
        cargando = cargando,
        tituloVacio = tituloVacio,
        detalleVacio = detalleVacio,
        sincronizando = sincronizando,
        onSincronizar = onSincronizar,
    ) { fila ->
        FilaActivo(fila, onClick = { onClick(fila) })
    }
}

@Composable
internal fun DialogoConfirmarSalida(
    fila: FilaActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaActiva) -> Unit,
) {
    if (fila == null) return
    DialogoRegistrarSalida(
        detalle = when (fila) {
            is FilaActiva.Local -> "${fila.activo.contratistaNombre} · ${fila.activo.cedula} · ${fila.activo.empresaNombre}"
            is FilaActiva.Remota -> "${fila.remoto.contratistaNombre} · $TEXTO_OTRO_DISPOSITIVO"
        },
        onDismiss = onDismiss,
        onConfirmar = { onConfirmar(fila) },
    )
}

internal const val TEXTO_OTRO_DISPOSITIVO = "registrado en otro dispositivo de la unidad operativa"

/// Modal "Registrar salida" a mano en vez de `AlertDialog` -- el mockup pide
/// un layout que `AlertDialog` no ofrece (icono circular arriba, botón
/// principal de ancho completo, "Cancelar" como link chico debajo, todo
/// centrado) en vez de los dos botones lado a lado de siempre. Lo comparten
/// contratistas y visitas.
@Composable
internal fun DialogoRegistrarSalida(
    detalle: String,
    onDismiss: () -> Unit,
    onConfirmar: () -> Unit,
) {
    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface, MaterialTheme.shapes.medium)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(
                modifier = Modifier
                    .size(56.dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.primaryContainer),
                contentAlignment = Alignment.Center,
            ) {
                Icon(Icons.AutoMirrored.Filled.Logout, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
            }
            Text(
                "Registrar salida",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(top = 16.dp),
            )
            Text(
                detalle,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
            BotonBrisas(
                onClick = onConfirmar,
                modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
            ) {
                Text("Confirmar")
            }
            BotonDiscretoBrisas(onClick = onDismiss, modifier = Modifier.padding(top = 4.dp)) {
                Text("Cancelar")
            }
        }
    }
}

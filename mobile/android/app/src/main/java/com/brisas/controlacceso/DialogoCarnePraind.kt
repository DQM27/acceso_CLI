package com.brisas.controlacceso

import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.semantics.Role
import androidx.compose.material3.Checkbox
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import uniffi.control_acceso_mobile.NivelRevisionGafete

/// El carné PRAIND a pantalla completa, para comparar la foto con la persona
/// que está en la portería. "Cerrar" o el gesto de volver lo cierran. Con
/// [onReemplazar] (la nube deja cargar desde este teléfono) aparece
/// "Reemplazar", que abre la cámara para leer el QR del gafete nuevo.
@Composable
internal fun DialogoCarnePraind(vista: VistaCarne, onCerrar: () -> Unit, onReemplazar: (() -> Unit)? = null) {
    val nombre = when (vista) {
        VistaCarne.Cerrada -> return
        is VistaCarne.Cargando -> vista.nombre
        is VistaCarne.Imagen -> vista.nombre
        is VistaCarne.Error -> vista.nombre
    }
    RegistrarPantalla("carne_praind")
    Dialog(onDismissRequest = onCerrar, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background)
                .systemBarsPadding()
                .padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(
                "Gafete PRAIND · $nombre",
                style = MaterialTheme.typography.titleMedium,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth(),
            )
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                when (vista) {
                    is VistaCarne.Cargando -> CircularProgressIndicator()
                    is VistaCarne.Error -> FranjaEstado(vista.mensaje, TipoEstado.AVISO, modifier = Modifier.fillMaxWidth())
                    is VistaCarne.Imagen -> {
                        val imagen = remember(vista) {
                            BitmapFactory.decodeByteArray(vista.bytes, 0, vista.bytes.size)?.asImageBitmap()
                        }
                        if (imagen != null) {
                            Image(
                                bitmap = imagen,
                                contentDescription = "Gafete PRAIND de $nombre",
                                contentScale = ContentScale.Fit,
                                modifier = Modifier.fillMaxSize(),
                            )
                        } else {
                            FranjaEstado(
                                "No se pudo mostrar la imagen del gafete PRAIND.",
                                TipoEstado.AVISO,
                                modifier = Modifier.fillMaxWidth(),
                            )
                        }
                    }
                    VistaCarne.Cerrada -> Unit
                }
            }
            if (onReemplazar != null) {
                BotonDiscretoBrisas(onClick = onReemplazar, modifier = Modifier.fillMaxWidth()) {
                    Text("Reemplazar gafete PRAIND")
                }
            }
            BotonBrisas(onClick = onCerrar, modifier = Modifier.fillMaxWidth()) {
                Text("Cerrar")
            }
        }
    }
}

/// Cargar el gafete PRAIND con el QR ya leído: primero dice de quién es el
/// gafete que encontró la nube (en verde si coincide con el contratista, en
/// ámbar si no) y recién al confirmar lo guarda. Mismo criterio que el
/// escritorio y el panel.
@Composable
internal fun DialogoCargaCarnePraind(
    carga: CargaCarne,
    onGuardar: () -> Unit,
    onEscanearOtraVez: () -> Unit,
    onCancelar: () -> Unit,
    onCorregirNombre: () -> Unit = {},
    onCorregirEmpresa: () -> Unit = {},
) {
    val nombre = when (carga) {
        CargaCarne.Inactiva -> return
        is CargaCarne.Buscando -> carga.nombre
        is CargaCarne.Previa -> carga.nombre
        is CargaCarne.Error -> carga.nombre
    }
    val guardando = carga is CargaCarne.Previa && carga.guardando
    RegistrarPantalla("cargar_carne_praind")
    Dialog(onDismissRequest = { if (!guardando) onCancelar() }) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(24.dp))
                .background(MaterialTheme.colorScheme.surface)
                .padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Cargar gafete PRAIND", style = MaterialTheme.typography.titleMedium)
            Text(
                nombre,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            when (carga) {
                is CargaCarne.Buscando -> Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                    Text("Buscando y leyendo el gafete PRAIND…")
                }
                is CargaCarne.Previa -> {
                    val revision = carga.revision
                    if (revision != null) {
                        // Cédula, nombre, empresa y vencimiento, con el texto
                        // del núcleo (`lineas_revision_gafete_praind`).
                        revision.lineas.forEach { linea ->
                            FranjaEstado(linea.texto, tipoDeLinea(linea.nivel), modifier = Modifier.fillMaxWidth())
                        }
                        // Corregir la ficha con lo que dice el gafete: sólo si
                        // el portero lo marca (pedido del usuario 2026-11-21).
                        revision.nombreParaFicha?.let { nuevo ->
                            OpcionCorregir(
                                texto = "Corregir el nombre de la ficha: $nuevo",
                                marcada = carga.corregirNombre,
                                habilitada = !guardando,
                                onCambiar = onCorregirNombre,
                            )
                        }
                        revision.empresaParaFicha?.let { empresa ->
                            OpcionCorregir(
                                texto = "Corregir la empresa de la ficha: ${empresa.nombre}",
                                marcada = carga.corregirEmpresa,
                                habilitada = !guardando,
                                onCambiar = onCorregirEmpresa,
                            )
                        }
                    } else {
                        FranjaEstado(
                            textoPrevia(carga),
                            if (carga.coincide) TipoEstado.EXITO else TipoEstado.AVISO,
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                }
                is CargaCarne.Error -> FranjaEstado(carga.mensaje, TipoEstado.ERROR, modifier = Modifier.fillMaxWidth())
                CargaCarne.Inactiva -> Unit
            }
            if (carga is CargaCarne.Previa) {
                BotonBrisas(
                    onClick = onGuardar,
                    enabled = !guardando && carga.sePuedeGuardar,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(textoBotonGuardar(carga))
                }
            }
            if (carga !is CargaCarne.Buscando) {
                BotonDiscretoBrisas(onClick = onEscanearOtraVez, enabled = !guardando, modifier = Modifier.fillMaxWidth()) {
                    Text("Escanear otra vez")
                }
            }
            BotonDiscretoBrisas(onClick = onCancelar, enabled = !guardando, modifier = Modifier.fillMaxWidth()) {
                Text("Cancelar")
            }
        }
    }
}

/// Una casilla con su texto; toda la fila se puede tocar.
@Composable
private fun OpcionCorregir(texto: String, marcada: Boolean, habilitada: Boolean, onCambiar: () -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .toggleable(value = marcada, enabled = habilitada, role = Role.Checkbox, onValueChange = { onCambiar() }),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Checkbox(checked = marcada, onCheckedChange = null, enabled = habilitada)
        Text(texto, style = MaterialTheme.typography.bodyMedium)
    }
}

internal fun tipoDeLinea(nivel: NivelRevisionGafete): TipoEstado = when (nivel) {
    NivelRevisionGafete.BIEN -> TipoEstado.EXITO
    NivelRevisionGafete.AVISO -> TipoEstado.AVISO
    NivelRevisionGafete.ERROR -> TipoEstado.ERROR
}

/// El botón dice qué pasa al tocarlo: con la cédula distinta no se puede;
/// un gafete vencido se guarda sólo como imagen; con algún dato distinto,
/// "Guardar igual".
internal fun textoBotonGuardar(previa: CargaCarne.Previa): String {
    val revision = previa.revision
    return when {
        previa.guardando -> "Guardando…"
        revision == null -> if (previa.coincide) "Guardar" else "Guardar igual"
        revision.bloqueaGuardar -> "No se puede guardar"
        revision.lineas.any { it.nivel == NivelRevisionGafete.ERROR } -> "Guardar como imagen"
        revision.lineas.any { it.nivel == NivelRevisionGafete.AVISO } -> "Guardar igual"
        else -> "Guardar"
    }
}

/// Lo que dice la previsualización: de quién es el gafete según su QR.
internal fun textoPrevia(previa: CargaCarne.Previa): String {
    val enOrigen = previa.nombreEnOrigen
    return when {
        previa.coincide && enOrigen != null -> "El gafete PRAIND es de $enOrigen."
        previa.coincide -> "El gafete PRAIND coincide con ${previa.nombre}."
        else ->
            "El gafete PRAIND es de «${enOrigen ?: "nombre desconocido"}», que no coincide con " +
                "${previa.nombre}. Revise que sea el gafete correcto."
    }
}

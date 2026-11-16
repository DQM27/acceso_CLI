package com.brisas.controlacceso

import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties

/// El carné PRAIND a pantalla completa, para comparar la foto con la persona
/// que está en la portería. "Cerrar" o el gesto de volver lo cierran.
@Composable
internal fun DialogoCarnePraind(vista: VistaCarne, onCerrar: () -> Unit) {
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
            BotonBrisas(onClick = onCerrar, modifier = Modifier.fillMaxWidth()) {
                Text("Cerrar")
            }
        }
    }
}

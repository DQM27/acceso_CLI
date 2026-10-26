package com.brisas.controlacceso

import android.content.Context
import android.content.Intent
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBackIos
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.Nucleo

/// Reconectar este teléfono retirado en el panel, con el código de
/// reconexión que dio un administrador (ver
/// `docs/features-futuras/plan-reconexion-equipo-retirado.md`). Sólo se
/// ofrece cuando el núcleo dice que el teléfono está retirado. Todo lo decide
/// el núcleo: esta pantalla pide el código, muestra el aviso de registros que
/// se pierden si el núcleo lo pide, y reinicia la app si cambió de unidad.
@Composable
fun PantallaReconectarEquipo(
    nucleo: Nucleo,
    metadata: MetadatosDispositivoLocal,
    onReconectado: (String) -> Unit,
    onVolver: () -> Unit,
) {
    RegistrarPantalla("reconectar_equipo")
    val viewModel: ReconexionViewModel = viewModel(factory = ReconexionViewModel.factory(nucleo, metadata))
    var codigo by rememberSaveable { mutableStateOf("") }
    var escaneando by rememberSaveable { mutableStateOf(false) }
    val contexto = LocalContext.current
    val paso = viewModel.paso

    // El mismo lector que la vinculación: el panel muestra el código de
    // reconexión con el mismo QR (`contenidoQr`).
    if (escaneando) {
        PantallaEscanearCodigoVinculacion(
            onCodigoLeido = { leido ->
                escaneando = false
                codigo = CodigoVinculacion.formatear(leido)
                viewModel.reconectar(leido, onReconectado = onReconectado)
            },
            onEscribirCodigo = { escaneando = false },
            onCerrar = { escaneando = false },
        )
        return
    }

    // Ya reconectado en otra unidad: la base de la anterior no sirve, sólo
    // queda reiniciar.
    BackHandler {
        when (paso) {
            is PasoReconexion.OtraUnidad -> reiniciarApp(contexto)
            is PasoReconexion.Confirmar -> viewModel.volverAlCodigo()
            PasoReconexion.Codigo -> onVolver()
        }
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (paso !is PasoReconexion.OtraUnidad) {
            BotonDiscretoBrisas(
                onClick = { if (paso is PasoReconexion.Confirmar) viewModel.volverAlCodigo() else onVolver() },
                modifier = Modifier.offset(x = (-12).dp),
            ) {
                Icon(Icons.AutoMirrored.Filled.ArrowBackIos, contentDescription = null, modifier = Modifier.size(16.dp))
                Text("Volver")
            }
        }
        when (paso) {
            PasoReconexion.Codigo -> {
                Text("Reconectar este teléfono", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
                Text(
                    "Escanee el código QR de reconexión que le muestra un administrador en el panel, " +
                        "o escríbalo. Sirve una sola vez y vence en pocos minutos.",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                AvisoPorteria(viewModel.error)
                BotonBrisas(
                    onClick = { escaneando = true },
                    enabled = !viewModel.enviando,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text("Escanear código QR")
                }
                OutlinedTextField(
                    value = codigo,
                    onValueChange = { codigo = CodigoVinculacion.formatear(it) },
                    label = { Text("Código de reconexión") },
                    placeholder = { Text("XXXX-XXXX-XX") },
                    singleLine = true,
                    enabled = !viewModel.enviando,
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Ascii,
                        capitalization = KeyboardCapitalization.Characters,
                        autoCorrectEnabled = false,
                        imeAction = ImeAction.Done,
                    ),
                    keyboardActions = KeyboardActions(onDone = { viewModel.reconectar(codigo, onReconectado = onReconectado) }),
                    textStyle = MaterialTheme.typography.titleMedium.copy(fontFamily = FontFamily.Monospace),
                    shape = FormaCampoBrisas,
                    colors = ColoresCampoBrisas(),
                    modifier = Modifier.fillMaxWidth(),
                )
                BotonDiscretoBrisas(
                    onClick = { viewModel.reconectar(codigo, onReconectado = onReconectado) },
                    enabled = !viewModel.enviando && CodigoVinculacion.completo(codigo),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(if (viewModel.enviando) "Reconectando…" else "Reconectar con el código escrito")
                }
            }
            is PasoReconexion.Confirmar -> {
                Text("Cambiar de unidad", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
                Text(
                    "El código pasa este teléfono a ${paso.unidad}. Hay ${textoRegistros(paso.pendientes)} " +
                        "sin subir a la nube que se van a perder: los datos de la unidad anterior se borran " +
                        "de este teléfono.",
                )
                AvisoPorteria(viewModel.error)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                    BotonDiscretoBrisas(onClick = { viewModel.volverAlCodigo() }, enabled = !viewModel.enviando, modifier = Modifier.weight(1f)) {
                        Text("Cancelar")
                    }
                    BotonBrisas(
                        onClick = { viewModel.reconectar(codigo, paso.pendientes, onReconectado) },
                        enabled = !viewModel.enviando,
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.error,
                            contentColor = MaterialTheme.colorScheme.onError,
                        ),
                        modifier = Modifier.weight(1f),
                    ) {
                        Text(if (viewModel.enviando) "Reconectando…" else "Reconectar y borrar")
                    }
                }
            }
            is PasoReconexion.OtraUnidad -> {
                Text("Teléfono reconectado", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
                Text(
                    "Este teléfono quedó en ${paso.unidad}. La app se va a reiniciar para borrar los datos " +
                        "de la unidad anterior y bajar los de la nueva.",
                )
                BotonBrisas(onClick = { reiniciarApp(contexto) }, modifier = Modifier.fillMaxWidth()) {
                    Text("Reiniciar ahora")
                }
            }
        }
    }
}

/// Franja fija de la pantalla principal mientras el núcleo diga que el
/// teléfono está retirado: el aviso y el botón para reconectarlo.
@Composable
fun FranjaEquipoRetirado(onReconectar: () -> Unit, modifier: Modifier = Modifier) {
    Surface(
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.errorContainer,
        contentColor = MaterialTheme.colorScheme.onErrorContainer,
        modifier = modifier.fillMaxWidth(),
    ) {
        Row(modifier = Modifier.padding(start = 12.dp, end = 4.dp, top = 4.dp, bottom = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(Icons.Default.Warning, contentDescription = null, modifier = Modifier.size(20.dp))
            Spacer(Modifier.width(12.dp))
            Text(
                "Este teléfono fue retirado en el panel.",
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.weight(1f),
            )
            TextButton(onClick = onReconectar) { Text("Reconectar") }
        }
    }
}

/// Cierra el proceso y vuelve a abrir la app desde cero. Lo usa la
/// reconexión en otra unidad: al arrancar, el núcleo borra la base de la
/// unidad anterior antes de usarla (ver `Nucleo.abrirCifrado`).
fun reiniciarApp(contexto: Context) {
    val intento = contexto.packageManager.getLaunchIntentForPackage(contexto.packageName) ?: return
    contexto.startActivity(Intent.makeRestartActivityTask(intento.component))
    Runtime.getRuntime().exit(0)
}

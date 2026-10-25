package com.brisas.controlacceso

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.CloudSync
import androidx.compose.material.icons.outlined.QrCodeScanner
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.delay
import uniffi.control_acceso_mobile.Nucleo

/// Única pantalla cuando la base está vacía
/// (`nucleo.requiereConfiguracionInicial()` en [MainActivity]) -- sin login
/// todavía, porque no hay ningún usuario con quien autenticar. Canjear el
/// código de vinculación del panel (escaneando su QR o escribiéndolo) trae
/// el catálogo remoto completo (contratistas/empresas/gafetes/usuarios) en
/// el mismo paso; los usuarios llegan con el centinela `SIN_PASSWORD_LOCAL`
/// (ver `src/nube/sincronizacion.rs`), así que el primer login de cualquiera
/// de ellos cae solo en el login contra Supabase Auth.
///
/// El código vence en minutos y sirve una sola vez: no es una credencial.
/// La credencial es la clave que el teléfono genera en Android Keystore al
/// canjearlo (ver [AlmacenClaveKeystore]).
///
/// Tres pasos a la vista (código, sincronizando, listo) sobre la misma
/// [HojaPorteria] del login: la descarga del catálogo puede tardar y antes
/// sólo se leía "Vinculando…" en el botón.
@Composable
fun PantallaPrimerArranque(
    nucleo: Nucleo,
    metadata: MetadatosDispositivoLocal,
    onListo: () -> Unit,
) {
    RegistrarPantalla("primer_arranque")
    val viewModel: PrimerArranqueViewModel =
        viewModel(factory = PrimerArranqueViewModel.factory(nucleo, metadata))
    var codigo by rememberSaveable { mutableStateOf("") }
    var escaneando by rememberSaveable { mutableStateOf(false) }
    var escribiendo by rememberSaveable { mutableStateOf(false) }
    var vinculado by rememberSaveable { mutableStateOf(false) }
    val vincular = { valor: String -> viewModel.vincular(valor) { vinculado = true } }
    // Teléfono ya vinculado (cambió de unidad): baja los datos sin código.
    LaunchedEffect(Unit) { viewModel.prepararSiYaEstaVinculado { vinculado = true } }

    if (escaneando) {
        PantallaEscanearCodigoVinculacion(
            onCodigoLeido = { leido ->
                escaneando = false
                codigo = CodigoVinculacion.formatear(leido)
                vincular(leido)
            },
            onEscribirCodigo = {
                escaneando = false
                escribiendo = true
            },
            onCerrar = { escaneando = false },
        )
        return
    }

    // Un instante con el check a la vista antes de pasar al login: confirma
    // que salió bien sin que haya que tocar nada.
    if (vinculado) {
        LaunchedEffect(Unit) {
            delay(MS_CONFIRMACION_VINCULADO)
            onListo()
        }
    }
    val paso = when {
        vinculado -> PasoVinculacion.LISTO
        viewModel.vinculando || viewModel.preparando -> PasoVinculacion.SINCRONIZANDO
        else -> PasoVinculacion.CODIGO
    }

    HojaPorteria(
        encabezado = { compacto ->
            AnimatedContent(
                targetState = paso,
                transitionSpec = { fadeIn() togetherWith fadeOut() },
                label = "iconoVinculacion",
            ) { actual ->
                when (actual) {
                    PasoVinculacion.CODIGO -> IconoPorteria(Icons.Outlined.QrCodeScanner, compacto)
                    PasoVinculacion.SINCRONIZANDO -> IconoPorteria(Icons.Outlined.CloudSync, compacto)
                    PasoVinculacion.LISTO -> IconoPorteria(
                        Icons.Outlined.CheckCircle,
                        compacto,
                        fondo = ColorExitoBrisas.copy(alpha = 0.16f),
                        tinte = ColorExitoBrisas,
                    )
                }
            }
            IndicadorPasos(paso.ordinal, modifier = Modifier.padding(top = 16.dp))
        },
    ) {
        AnimatedContent(
            targetState = paso,
            transitionSpec = { fadeIn() togetherWith fadeOut() },
            label = "pasoVinculacion",
        ) { actual ->
            Column {
                when (actual) {
                    PasoVinculacion.CODIGO -> PasoCodigo(
                        codigo = codigo,
                        onCodigo = { codigo = CodigoVinculacion.formatear(it) },
                        escribiendo = escribiendo,
                        onEscribir = { escribiendo = true },
                        onEscanear = { escaneando = true },
                        onVincular = { vincular(codigo) },
                        error = viewModel.error,
                    )
                    PasoVinculacion.SINCRONIZANDO -> PasoSincronizando()
                    PasoVinculacion.LISTO -> PasoListo(nucleo)
                }
            }
        }
    }
}

private enum class PasoVinculacion { CODIGO, SINCRONIZANDO, LISTO }

private const val MS_CONFIRMACION_VINCULADO = 1400L

/// Tres rayitas: la del paso actual más larga y en color.
@Composable
private fun IndicadorPasos(actual: Int, modifier: Modifier = Modifier) {
    Row(modifier = modifier, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        repeat(PasoVinculacion.entries.size) { indice ->
            val color by animateColorAsState(
                if (indice <= actual) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant,
                label = "colorPaso",
            )
            Box(
                modifier = Modifier
                    .height(6.dp)
                    .width(if (indice == actual) 28.dp else 12.dp)
                    .clip(RoundedCornerShape(50))
                    .background(color),
            )
        }
    }
}

@Composable
private fun ColumnScope.PasoCodigo(
    codigo: String,
    onCodigo: (String) -> Unit,
    escribiendo: Boolean,
    onEscribir: () -> Unit,
    onEscanear: () -> Unit,
    onVincular: () -> Unit,
    error: String?,
) {
    AvisoPorteria(error)
    TituloHoja(
        "Vincular este teléfono",
        "Pídale al administrador el código QR del panel para este dispositivo. Vence en pocos " +
            "minutos y sirve una sola vez.",
    )
    BotonPorteria(
        texto = "Escanear código QR",
        onClick = onEscanear,
        modifier = Modifier.padding(top = 20.dp),
    )

    // El código escrito es la alternativa: queda plegado para no competir
    // con el QR, que es lo normal.
    if (!escribiendo) {
        TextButton(onClick = onEscribir, modifier = Modifier.align(Alignment.CenterHorizontally).padding(top = 4.dp)) {
            Text("Escribir el código")
        }
    }
    AnimatedVisibility(visible = escribiendo, enter = fadeIn() + expandVertically(), exit = fadeOut() + shrinkVertically()) {
        Column {
            OutlinedTextField(
                value = codigo,
                onValueChange = onCodigo,
                label = { Text("Código de vinculación") },
                placeholder = { Text("XXXX-XXXX-XX") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(
                    keyboardType = KeyboardType.Ascii,
                    capitalization = KeyboardCapitalization.Characters,
                    autoCorrectEnabled = false,
                    imeAction = ImeAction.Done,
                ),
                keyboardActions = KeyboardActions(onDone = { if (CodigoVinculacion.completo(codigo)) onVincular() }),
                textStyle = MaterialTheme.typography.titleMedium.copy(fontFamily = FontFamily.Monospace),
                shape = FormaCampoBrisas,
                colors = ColoresCampoBrisas(),
                modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
            )
            BotonDiscretoBrisas(
                onClick = onVincular,
                enabled = CodigoVinculacion.completo(codigo),
                modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
            ) {
                Text("Vincular con este código")
            }
        }
    }
}

@Composable
private fun PasoSincronizando() {
    TituloHoja(
        "Sincronizando",
        "Descargando contratistas, empresas, gafetes y usuarios de la unidad. No cierre la app.",
    )
    LinearProgressIndicator(
        modifier = Modifier.fillMaxWidth().padding(top = 20.dp, bottom = 8.dp).clip(RoundedCornerShape(50)),
    )
}

@Composable
private fun PasoListo(nucleo: Nucleo) {
    val unidad = rememberTextoUnidadOperativa(nucleo)
    TituloHoja(
        "Teléfono vinculado",
        if (unidad != null) "Quedó registrado en $unidad. Ya puede iniciar sesión." else "Ya puede iniciar sesión.",
    )
}

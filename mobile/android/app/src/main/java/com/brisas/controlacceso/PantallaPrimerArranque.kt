package com.brisas.controlacceso

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
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
@Composable
fun PantallaPrimerArranque(
    nucleo: Nucleo,
    metadata: MetadatosDispositivoLocal,
    onListo: () -> Unit,
) {
    RegistrarPantalla("primer_arranque")
    val viewModel: PrimerArranqueViewModel =
        viewModel(factory = PrimerArranqueViewModel.factory(nucleo, metadata))
    var codigo by remember { mutableStateOf("") }
    var escaneando by remember { mutableStateOf(false) }

    if (escaneando) {
        PantallaEscanearCodigoVinculacion(
            onCodigoLeido = { leido ->
                escaneando = false
                codigo = CodigoVinculacion.formatear(leido)
                viewModel.vincular(leido, onListo)
            },
            onCerrar = { escaneando = false },
        )
        return
    }

    Column(
        modifier = Modifier.fillMaxSize().imePadding().verticalScroll(rememberScrollState()).padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Image(
            painter = painterResource(id = R.drawable.marca),
            contentDescription = null,
            modifier = Modifier.size(96.dp).clip(RoundedCornerShape(20.dp)),
        )

        Text(
            "Vincular este teléfono",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 16.dp),
        )
        Text(
            "Escaneá el código QR que muestra el panel de administración para este " +
                "dispositivo, o escribí el código. Vence en pocos minutos y sirve una sola vez.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        BotonBrisas(
            onClick = { escaneando = true },
            enabled = !viewModel.vinculando,
            modifier = Modifier.fillMaxWidth().padding(top = 32.dp),
        ) {
            Text("Escanear código QR")
        }

        OutlinedTextField(
            value = codigo,
            onValueChange = { codigo = CodigoVinculacion.formatear(it) },
            label = { Text("Código de vinculación") },
            placeholder = { Text("XXXX-XXXX-XX") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(
                keyboardType = KeyboardType.Ascii,
                capitalization = KeyboardCapitalization.Characters,
                autoCorrectEnabled = false,
            ),
            textStyle = MaterialTheme.typography.titleMedium.copy(fontFamily = FontFamily.Monospace),
            enabled = !viewModel.vinculando,
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
        )

        BotonBrisas(
            onClick = { viewModel.vincular(codigo, onListo) },
            enabled = !viewModel.vinculando && CodigoVinculacion.completo(codigo),
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text(if (viewModel.vinculando) "Vinculando…" else "Vincular y sincronizar")
        }

        val mensajeError = viewModel.error
        if (mensajeError != null) {
            Text(
                mensajeError,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(top = 16.dp),
            )
        }
    }
}

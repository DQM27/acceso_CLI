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
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.Nucleo

/// Única pantalla cuando la base está vacía
/// (`nucleo.requiereConfiguracionInicial()` en [MainActivity]) -- sin login
/// todavía, porque no hay ningún usuario con quien autenticar. Pegar el
/// secreto trae el catálogo remoto completo (contratistas/empresas/gafetes/
/// usuarios) en el mismo paso; los usuarios llegan con el centinela
/// `SIN_PASSWORD_LOCAL` (ver `src/nube/sincronizacion.rs`), así que el
/// primer login de cualquiera de ellos cae solo en
/// [PantallaFijarPasswordInicial] -- ya existente, no hay nada nuevo que
/// construir ahí.
///
/// Reemplaza al seed hardcodeado (`assets/semilla.db`, un ROOT de prueba de
/// campo fijo) que vivía en [MainActivity] -- ya no hace falta un usuario
/// de mentira instalado de fábrica.
@Composable
fun PantallaPrimerArranque(
    nucleo: Nucleo,
    secretoStore: SecretoDispositivoStore,
    metadata: MetadatosDispositivoLocal,
    onListo: () -> Unit,
) {
    val viewModel: PrimerArranqueViewModel =
        viewModel(factory = PrimerArranqueViewModel.factory(nucleo, secretoStore, metadata))
    var secreto by remember { mutableStateOf("") }

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
            "Conectar este dispositivo",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 16.dp),
        )
        Text(
            "Todavía no hay usuarios en este teléfono -- pegá el secreto que el panel de " +
                "administración generó para este dispositivo",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        OutlinedTextField(
            value = secreto,
            onValueChange = { secreto = it },
            label = { Text("Secreto del dispositivo") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            enabled = !viewModel.conectando,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 32.dp),
        )

        BotonBrisas(
            onClick = { viewModel.conectar(secreto, onListo) },
            enabled = !viewModel.conectando && secreto.isNotBlank(),
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text(if (viewModel.conectando) "Conectando…" else "Conectar y sincronizar")
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

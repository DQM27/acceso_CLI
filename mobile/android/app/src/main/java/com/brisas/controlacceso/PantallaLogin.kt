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
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import uniffi.control_acceso_mobile.Nucleo

/// Login real contra `Nucleo.autenticar` (Rust) — todo el estado y la
/// llamada viven en [LoginViewModel] (ver mobile/android/ARQUITECTURA.md), este
/// Composable sólo dibuja el formulario. Una vez hay sesión, delega a
/// [PantallaPrincipal] en vez de dibujar nada propio — mismo `Nucleo` para
/// toda la app, no se reabre la base al loguear.
@Composable
fun PantallaLogin(nucleo: Nucleo, directorio: String, secretoStore: SecretoDispositivoStore) {
    val viewModel: LoginViewModel =
        viewModel(factory = LoginViewModel.factory(nucleo, secretoStore))

    val sesionActual = viewModel.sesion
    val propietarioSesion = viewModel.propietarioSesion
    if (sesionActual != null && propietarioSesion != null) {
        CompositionLocalProvider(LocalViewModelStoreOwner provides propietarioSesion) {
            PantallaPrincipal(
                nucleo = nucleo,
                sesion = sesionActual,
                directorio = directorio,
                secretoStore = secretoStore,
                onCerrarSesion = { viewModel.cerrarSesion() },
            )
        }
        return
    }

    val cambioObligatorio = viewModel.cambioObligatorio
    if (cambioObligatorio != null) {
        PantallaCambioObligatorio(
            nombre = cambioObligatorio.first.nombre,
            error = viewModel.error,
            enviando = viewModel.autenticando,
            onCambiar = { nueva -> viewModel.completarCambioObligatorio(nueva) },
            onCancelar = { viewModel.cancelarCambioObligatorio() },
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
            "Control de acceso",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 16.dp),
        )
        Text(
            "Brisas",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        OutlinedTextField(
            value = viewModel.cedula,
            onValueChange = { viewModel.cambiarCedula(it) },
            label = { Text("Cédula") },
            singleLine = true,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 32.dp),
        )
        OutlinedTextField(
            value = viewModel.password,
            onValueChange = { viewModel.cambiarPassword(it) },
            label = { Text("Contraseña") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        )
        BotonBrisas(
            onClick = { viewModel.autenticar() },
            enabled = !viewModel.autenticando && viewModel.cedula.isNotBlank() && viewModel.password.isNotBlank(),
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text(if (viewModel.autenticando) "Verificando…" else "Ingresar")
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

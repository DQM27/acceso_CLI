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
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import uniffi.control_acceso_mobile.Nucleo

/// Login real contra `Nucleo.autenticar` (Rust) — todo el estado y la
/// llamada viven en [LoginViewModel] (ver mobile/android/arquitectura.md), este
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
        // Logo más grande y sin el texto "Lattis" debajo -- redundante, el
        // logo ya dice la marca (pedido explícito del usuario, 2026-09-19).
        // En su lugar, "Control de acceso móvil" con "móvil" en negrita
        // para resaltarlo -- distingue esta app de las otras plataformas
        // (desktop/web) que comparten la misma marca.
        Image(
            painter = painterResource(id = R.drawable.marca),
            contentDescription = null,
            modifier = Modifier.size(128.dp).clip(RoundedCornerShape(26.dp)),
        )

        Text(
            buildAnnotatedString {
                // `titleLarge` ya es bold (peso 700, `TipografiaBrisas` en
                // DisenoMovil.kt) -- ponerle Bold también a "móvil" no
                // resaltaba nada porque ya estaba igual de grueso que el
                // resto. Se aligera "Control de acceso" a Normal para que
                // "móvil" contraste y sea lo que el ojo agarra primero.
                withStyle(SpanStyle(fontWeight = FontWeight.Normal)) {
                    append("Control de acceso ")
                }
                withStyle(SpanStyle(fontWeight = FontWeight.Bold)) {
                    append("MOVIL")
                }
            },
            style = MaterialTheme.typography.titleLarge,
            modifier = Modifier.padding(top = 16.dp),
        )

        OutlinedTextField(
            value = viewModel.cedula,
            onValueChange = { viewModel.cambiarCedula(it) },
            label = { Text("Cédula") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth().padding(top = 32.dp),
        )
        OutlinedTextField(
            value = viewModel.password,
            onValueChange = { viewModel.cambiarPassword(it) },
            label = { Text("Contraseña") },
            singleLine = true,
            // `visualTransformation` sólo oculta el texto EN PANTALLA (los
            // puntos) -- sin `KeyboardType.Password` el teclado del sistema
            // no sabe que es un campo sensible y sigue armando su predictivo
            // con lo tipeado (hallazgo real del usuario 2026-09-21: la barra
            // de sugerencias mostraba "daniel"/"daniel27" mientras escribía
            // la contraseña). Este flag es lo que de verdad apaga
            // predicción/autocorrección/diccionario personal para el campo.
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
            visualTransformation = PasswordVisualTransformation(),
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
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

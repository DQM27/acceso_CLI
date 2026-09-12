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
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp

/// Reemplaza al formulario de login cuando `Nucleo.autenticarConSecreto`
/// devuelve `debe_cambiar_password = true` -- usuario global (Administrador/
/// Operador, o un ROOT ya sincronizado a otro sitio) que todavía tiene la
/// contraseña temporal de un solo uso generada por el panel (ver
/// docs/plan-autenticacion-supabase-auth.md). La sesión YA está abierta del
/// lado de Rust (`autenticar_supabase` la dejó iniciada); esto sólo cambia
/// la contraseña, no vuelve a autenticar. Reemplaza a
/// `PantallaFijarPasswordInicial` (el "reclamo por cédula" viejo, cerrado
/// junto con `SIN_PASSWORD_LOCAL` -- ver `docs/decisiones-tecnicas.md`).
@Composable
fun PantallaCambioObligatorio(
    nombre: String,
    error: String?,
    enviando: Boolean,
    onCambiar: (String) -> Unit,
    onCancelar: () -> Unit,
) {
    var password by remember { mutableStateOf("") }
    var confirmar by remember { mutableStateOf("") }
    var errorLocal by remember { mutableStateOf<String?>(null) }

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
            "Fijar contraseña",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 16.dp),
        )
        Text(
            "$nombre · primera vez con esta contraseña temporal",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("Contraseña nueva") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 32.dp),
        )
        OutlinedTextField(
            value = confirmar,
            onValueChange = { confirmar = it },
            label = { Text("Confirmar contraseña") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
        )

        BotonBrisas(
            onClick = {
                errorLocal = null
                when {
                    password.length < 8 -> errorLocal = "Al menos 8 caracteres"
                    password != confirmar -> errorLocal = "Las contraseñas no coinciden"
                    else -> onCambiar(password)
                }
            },
            enabled = !enviando,
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text(if (enviando) "Guardando…" else "Cambiar y entrar")
        }

        OutlinedButton(
            onClick = onCancelar,
            enabled = !enviando,
            modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
        ) {
            Text("Volver")
        }

        val mensajeError = errorLocal ?: error
        if (mensajeError != null) {
            Text(
                mensajeError,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(top = 16.dp),
            )
        }
    }
}

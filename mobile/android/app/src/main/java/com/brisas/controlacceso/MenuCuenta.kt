package com.brisas.controlacceso

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.filled.AutoAwesome
import androidx.compose.material.icons.filled.DarkMode
import androidx.compose.material.icons.filled.LightMode
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.RolUsuario
import uniffi.control_acceso_mobile.UsuarioSesion

/// Menú de la cuenta de quien tiene la sesión, mismo lugar que el menú de
/// usuario del escritorio (`desktop/src/componentes/MenuUsuario.tsx`): un
/// botón con la inicial del nombre que abre quién es, el tema, la versión
/// instalada y cerrar sesión. Todo lo que es de la persona y no del trabajo
/// del puesto vive acá, para que la barra superior quede con lo operativo
/// (nuevo contratista y sincronizar).
///
/// Sólo UI: el tema lo guarda [GestorTema] y cerrar sesión lo decide quien
/// llama.
@Composable
fun MenuCuenta(
    sesion: UsuarioSesion,
    tema: TemaApp,
    onAlternarTema: () -> Unit,
    onCerrarSesion: () -> Unit,
) {
    var abierto by remember { mutableStateOf(false) }
    Box {
        BotonIconoCuadradoBrisas(onClick = { abierto = true }) {
            Text(
                inicialDe(sesion.nombre),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
            )
        }
        DropdownMenu(expanded = abierto, onDismissRequest = { abierto = false }) {
            Column(modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
                Text(
                    sesion.nombre,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    etiquetaRol(sesion.rol),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            HorizontalDivider()
            // Recorre claro → oscuro → Tokyo Night; el ítem dice a cuál pasa
            // con el próximo toque.
            val siguiente = tema.siguiente()
            DropdownMenuItem(
                text = { Text(textoCambioTema(siguiente)) },
                leadingIcon = { Icon(iconoTema(siguiente), contentDescription = null) },
                onClick = {
                    abierto = false
                    onAlternarTema()
                },
            )
            DropdownMenuItem(
                text = {
                    Text(
                        "Versión ${BuildConfig.VERSION_NAME}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                },
                enabled = false,
                onClick = {},
            )
            HorizontalDivider()
            DropdownMenuItem(
                text = { Text("Cerrar sesión") },
                leadingIcon = { Icon(Icons.AutoMirrored.Filled.ExitToApp, contentDescription = null) },
                onClick = {
                    abierto = false
                    onCerrarSesion()
                },
            )
        }
    }
}

/// La inicial del nombre para el botón (vacío no se espera: toda cuenta tiene
/// nombre; "?" evita un botón sin nada si alguna vez llegara vacío).
internal fun inicialDe(nombre: String): String =
    nombre.trim().firstOrNull()?.uppercase() ?: "?"

internal fun etiquetaRol(rol: RolUsuario): String = when (rol) {
    RolUsuario.ROOT -> "Root"
    RolUsuario.ADMINISTRADOR -> "Administrador"
    RolUsuario.OPERADOR -> "Operador"
}

private fun textoCambioTema(tema: TemaApp): String = when (tema) {
    TemaApp.CLARO -> "Cambiar a modo claro"
    TemaApp.OSCURO -> "Cambiar a modo oscuro"
    TemaApp.TOKYO_NIGHT -> "Cambiar a Tokyo Night"
}

private fun iconoTema(tema: TemaApp) = when (tema) {
    TemaApp.CLARO -> Icons.Default.LightMode
    TemaApp.OSCURO -> Icons.Default.DarkMode
    TemaApp.TOKYO_NIGHT -> Icons.Default.AutoAwesome
}

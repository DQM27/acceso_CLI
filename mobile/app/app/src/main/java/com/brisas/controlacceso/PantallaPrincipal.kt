package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.RolUsuario
import uniffi.control_acceso_mobile.UsuarioSesion

/// Sólo las pantallas de uso frecuente son pestañas (Activos, Historial) —
/// las de creación (uso esporádico: dar de alta un contratista, empresa o
/// usuario nuevos) viven detrás del botón "+", no compitiendo por espacio en
/// la barra de pestañas. `Principal` es la única bandera "no es pantalla de
/// creación" — qué pestaña se ve la decide el `pestana` local más abajo.
///
/// Vive como estado local del Composable (no en un ViewModel) a propósito:
/// es puramente de navegación — qué se ve en pantalla — sin ninguna llamada
/// a [Nucleo] ni regla de negocio detrás; no hay nada que un ViewModel
/// protegería acá (ver mobile/app/ARQUITECTURA.md sobre cuándo sí hace
/// falta uno).
private sealed class Pantalla {
    data object Principal : Pantalla()

    data object NuevoContratista : Pantalla()

    data object NuevaEmpresa : Pantalla()

    data object NuevoUsuario : Pantalla()
}

@Composable
fun PantallaPrincipal(nucleo: Nucleo, sesion: UsuarioSesion, onCerrarSesion: () -> Unit) {
    var pantalla by remember { mutableStateOf<Pantalla>(Pantalla.Principal) }
    var menuCreacionAbierto by remember { mutableStateOf(false) }

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(
                sesion.nombre,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 8.dp),
            )
            Row {
                Box {
                    IconButton(onClick = { menuCreacionAbierto = true }) {
                        Icon(Icons.Default.Add, contentDescription = "Crear")
                    }
                    DropdownMenu(expanded = menuCreacionAbierto, onDismissRequest = { menuCreacionAbierto = false }) {
                        DropdownMenuItem(
                            text = { Text("Nuevo contratista") },
                            onClick = {
                                menuCreacionAbierto = false
                                pantalla = Pantalla.NuevoContratista
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Nueva empresa") },
                            onClick = {
                                menuCreacionAbierto = false
                                pantalla = Pantalla.NuevaEmpresa
                            },
                        )
                        // Sólo Root/Administrador — espejo de
                        // Operacion::GestionarUsuarios (domain/autorizacion.rs).
                        // Rust vuelve a exigirlo del lado real; esto es sólo
                        // para no ofrecerle a un Operador un botón que va a
                        // fallar.
                        if (sesion.rol != RolUsuario.OPERADOR) {
                            DropdownMenuItem(
                                text = { Text("Nuevo usuario") },
                                onClick = {
                                    menuCreacionAbierto = false
                                    pantalla = Pantalla.NuevoUsuario
                                },
                            )
                        }
                    }
                }
                TextButton(onClick = onCerrarSesion) {
                    Text("Salir")
                }
            }
        }

        when (val actual = pantalla) {
            is Pantalla.NuevoContratista, is Pantalla.NuevaEmpresa, is Pantalla.NuevoUsuario -> {
                TextButton(onClick = { pantalla = Pantalla.Principal }, modifier = Modifier.padding(start = 8.dp)) {
                    Text("← Volver")
                }
                when (actual) {
                    is Pantalla.NuevoContratista -> PantallaNuevoContratista(nucleo)
                    is Pantalla.NuevaEmpresa -> PantallaNuevaEmpresa(nucleo)
                    else -> PantallaNuevoUsuario(nucleo)
                }
            }
            else -> {
                var pestana by remember { mutableIntStateOf(0) }
                PrimaryTabRow(selectedTabIndex = pestana) {
                    Tab(selected = pestana == 0, onClick = { pestana = 0 }, text = { Text("Activos") })
                    Tab(selected = pestana == 1, onClick = { pestana = 1 }, text = { Text("Historial") })
                }
                when (pestana) {
                    0 -> PantallaActivos(nucleo)
                    else -> PantallaHistorial(nucleo)
                }
            }
        }
    }
}

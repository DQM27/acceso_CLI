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
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.RolUsuario
import uniffi.control_acceso_mobile.UsuarioSesion

/// Sólo las pantallas de uso frecuente son pestañas (Activos, Historial,
/// Nube) — las de creación (uso esporádico: dar de alta un contratista,
/// empresa o usuario nuevos) viven detrás del botón "+", no compitiendo por
/// espacio en la barra de pestañas. Nube entra como pestaña y no detrás del
/// "+" por el mismo motivo que Activos/Historial: su estado
/// (`NubeViewModel.ingresosRemotos`, `secretoGuardado`) debe sobrevivir
/// cambiar de pestaña y volver, no reiniciarse como si fuera un formulario
/// de alta de un solo uso. `Principal` es la única bandera "no es pantalla
/// de creación" — qué pestaña se ve la decide el `pestana` local más abajo.
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
fun PantallaPrincipal(nucleo: Nucleo, sesion: UsuarioSesion, directorio: String, onCerrarSesion: () -> Unit) {
    var pantalla by remember { mutableStateOf<Pantalla>(Pantalla.Principal) }
    var menuCreacionAbierto by remember { mutableStateOf(false) }
    var refrescarNube by remember { mutableIntStateOf(0) }
    val scope = rememberCoroutineScope()
    // `SincronizacionPeriodica`, no `NubeRealtime` -- ver el comentario de
    // esa clase sobre por qué (bug de plataforma en Supabase Realtime, no
    // arreglable desde acá).
    val sincronizacion = remember(nucleo, directorio, scope) {
        SincronizacionPeriodica(
            nucleo = nucleo,
            directorio = directorio,
            scope = scope,
            onSincronizado = { refrescarNube += 1 },
        )
    }

    // Atado a ON_START/ON_STOP, no sólo a la composición: sin esto, el pulso
    // periódico seguía vivo aunque el guardia bloqueara el teléfono o
    // cambiara de app -- `PantallaPrincipal` sigue en composición mientras
    // dure la sesión, la Activity no se destruye sólo por pasar a segundo
    // plano. Gastaba batería sin ningún beneficio: nadie ve pantalla para
    // que un refresco importe. Vuelve a sincronizar solo al volver al
    // primer plano.
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(sincronizacion, lifecycleOwner) {
        val observador = LifecycleEventObserver { _, evento ->
            when (evento) {
                Lifecycle.Event.ON_START -> sincronizacion.iniciar()
                Lifecycle.Event.ON_STOP -> sincronizacion.detener()
                else -> {}
            }
        }
        lifecycleOwner.lifecycle.addObserver(observador)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observador)
            sincronizacion.detener()
        }
    }

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
                BotonDiscretoBrisas(onClick = onCerrarSesion) {
                    Text("Salir")
                }
            }
        }

        when (val actual = pantalla) {
            is Pantalla.NuevoContratista, is Pantalla.NuevaEmpresa, is Pantalla.NuevoUsuario -> {
                BotonDiscretoBrisas(onClick = { pantalla = Pantalla.Principal }, modifier = Modifier.padding(start = 8.dp)) {
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
                    Tab(selected = pestana == 2, onClick = { pestana = 2 }, text = { Text("Nube") })
                }
                when (pestana) {
                    0 -> PantallaActivos(nucleo, refrescarNube)
                    1 -> PantallaHistorial(nucleo)
                    else -> PantallaNube(nucleo, sesion, directorio, refrescarNube)
                }
            }
        }
    }
}

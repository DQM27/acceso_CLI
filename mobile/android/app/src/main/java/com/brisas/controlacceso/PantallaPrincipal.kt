package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.DarkMode
import androidx.compose.material.icons.filled.LightMode
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.control_acceso_mobile.ConflictoIngresoActivo
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.UsuarioSesion

/// App limitada a registros rápidos + historial (decisión 2026-09-06) --
/// ya no hay menú "+" de altas (contratista/empresa/usuario nuevos, sacado
/// junto con la pestaña "Nube", ver `ARQUITECTURA.md`) ni una tercera
/// pestaña de nube. Sólo quedan las dos pantallas de uso frecuente
/// (Activos, Historial) y el ícono "Sincronizar" de la barra superior.
///
/// Sin `Pantalla`/navegación propia a propósito: con una sola superficie
/// posible (las dos pestañas) no hace falta esa indirección -- el `pestana`
/// local alcanza.
@Composable
fun PantallaPrincipal(
    nucleo: Nucleo,
    sesion: UsuarioSesion,
    directorio: String,
    secretoStore: SecretoDispositivoStore,
    onCerrarSesion: () -> Unit,
) {
    var refrescarNube by remember { mutableIntStateOf(0) }
    // `docs/pendientes.md`, "alertar luego al sincronizar" -- ver el mismo
    // campo en `desktop/src/App.tsx` (`manejarResumenSincronizacion`).
    // Alimentado desde los dos caminos de sync (pulso periódico y botón
    // manual), igual que `refrescarNube`.
    var conflictosIngreso by remember { mutableStateOf<List<ConflictoIngresoActivo>>(emptyList()) }
    val nubeViewModel: NubeViewModel =
        viewModel(
            factory = NubeViewModel.factory(nucleo, secretoStore, onCerrarSesion),
        )
    val scope = rememberCoroutineScope()
    val realtime = remember(nucleo, secretoStore, scope, sesion) {
        NubeRealtime(
            nucleo = nucleo,
            secretoStore = secretoStore,
            scope = scope,
            usuarioCedula = sesion.cedula,
            usuarioNombre = sesion.nombre,
        )
    }
    val sincronizacion = remember(nucleo, secretoStore, scope) {
        SincronizacionPeriodica(
            nucleo = nucleo,
            secretoStore = secretoStore,
            scope = scope,
            onSincronizado = { resumen ->
                // Si a esta sesión la desactivaron en otro dispositivo, el
                // pulso periódico (o Realtime, que dispara por el mismo
                // camino) ya trajo la baja -- cerrar sesión acá, no sólo
                // refrescar pantallas que ya no deberían verse.
                if (resumen.sesionExpulsada) {
                    onCerrarSesion()
                } else {
                    refrescarNube += 1
                    conflictosIngreso = resumen.conflictosIngreso
                }
            },
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
    DisposableEffect(sincronizacion, realtime, lifecycleOwner) {
        var trabajoBloqueo: Job? = null
        fun iniciarServicios() {
            trabajoBloqueo?.cancel()
            trabajoBloqueo = null
            sincronizacion.iniciar()
            realtime.iniciar()
        }
        fun detenerServiciosYProgramarBloqueo() {
            realtime.detener()
            sincronizacion.detener()
            trabajoBloqueo?.cancel()
            trabajoBloqueo = scope.launch {
                delay(DEMORA_BLOQUEO_SESION_MS)
                onCerrarSesion()
            }
        }
        val observador = LifecycleEventObserver { _, evento ->
            when (evento) {
                Lifecycle.Event.ON_START -> iniciarServicios()
                Lifecycle.Event.ON_STOP -> detenerServiciosYProgramarBloqueo()
                else -> {}
            }
        }
        lifecycleOwner.lifecycle.addObserver(observador)
        if (lifecycleOwner.lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)) {
            iniciarServicios()
        }
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observador)
            trabajoBloqueo?.cancel()
            realtime.detener()
            sincronizacion.detener()
        }
    }

    // El botón manual también refresca Activos/Historial al terminar --
    // mismo camino que ya usa el pulso periódico (`onSincronizado` más
    // arriba), para que tocar "Sincronizar" se sienta instantáneo en vez
    // de esperar al próximo ciclo de 2 minutos.
    LaunchedEffect(nubeViewModel.ultimoResumen) {
        val resumen = nubeViewModel.ultimoResumen
        if (resumen != null) {
            refrescarNube += 1
            conflictosIngreso = resumen.conflictosIngreso
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
                val oscuroActual = GestorTema.oscuroForzado ?: isSystemInDarkTheme()
                IconButton(onClick = { GestorTema.alternar(oscuroActual) }) {
                    Icon(
                        if (oscuroActual) Icons.Default.LightMode else Icons.Default.DarkMode,
                        contentDescription = if (oscuroActual) "Cambiar a modo claro" else "Cambiar a modo oscuro",
                    )
                }
                IconButton(
                    onClick = { nubeViewModel.sincronizar() },
                    enabled = !nubeViewModel.sincronizando,
                ) {
                    if (nubeViewModel.sincronizando) {
                        CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                    } else {
                        Icon(Icons.Default.Sync, contentDescription = "Sincronizar")
                    }
                }
                BotonDiscretoBrisas(onClick = onCerrarSesion) {
                    Text("Salir")
                }
            }
        }

        val errorSincronizacion = nubeViewModel.error
        if (errorSincronizacion != null) {
            Text(
                errorSincronizacion,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
            )
        }

        // `docs/pendientes.md`, "alertar luego al sincronizar" -- sigue
        // mostrándose en cada sync mientras el conflicto no se resuelva
        // (cerrando uno de los dos ingresos), no es un error transitorio
        // que convenga ocultar solo.
        for (conflicto in conflictosIngreso) {
            Text(
                "${conflicto.contratistaNombre} tiene un ingreso activo acá Y en ${conflicto.sitioConflicto} — hay que resolverlo.",
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
            )
        }

        var pestana by remember { mutableIntStateOf(0) }
        PrimaryTabRow(selectedTabIndex = pestana) {
            Tab(selected = pestana == 0, onClick = { pestana = 0 }, text = { Text("Activos") })
            Tab(selected = pestana == 1, onClick = { pestana = 1 }, text = { Text("Historial") })
        }
        when (pestana) {
            0 -> PantallaActivos(nucleo, secretoStore, refrescarNube)
            else -> PantallaHistorial(nucleo, refrescarNube)
        }
    }
}

private const val DEMORA_BLOQUEO_SESION_MS = 2 * 60_000L

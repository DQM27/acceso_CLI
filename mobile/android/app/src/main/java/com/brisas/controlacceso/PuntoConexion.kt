package com.brisas.controlacceso

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

/// Estado de conexión que resume el punto junto al nombre de la app. Mismo
/// papel que el indicador de la barra de estado del escritorio
/// (`desktop/src/componentes/BarraNube.tsx`), sin ocupar una fila entera.
enum class EstadoConexion(val descripcion: String) {
    EN_LINEA("En línea"),
    SIN_RED("Sin conexión"),
    ERROR_SINCRONIZACION("La última sincronización falló"),
}

/// Sin red manda sobre el error: sin señal el error de sincronizar es
/// consecuencia, no algo aparte que atender.
fun estadoConexion(hayRed: Boolean, falloSincronizacion: Boolean): EstadoConexion = when {
    !hayRed -> EstadoConexion.SIN_RED
    falloSincronizacion -> EstadoConexion.ERROR_SINCRONIZACION
    else -> EstadoConexion.EN_LINEA
}

/// Puntito verde (en línea), ámbar (sin conexión) o rojo (la última
/// sincronización falló). Sólo UI.
@Composable
fun PuntoConexion(estado: EstadoConexion, modifier: Modifier = Modifier) {
    val color = when (estado) {
        EstadoConexion.EN_LINEA -> ColorExitoBrisas
        EstadoConexion.SIN_RED -> ColorAdvertenciaBrisas
        EstadoConexion.ERROR_SINCRONIZACION -> MaterialTheme.colorScheme.error
    }
    Box(
        modifier = modifier
            .size(10.dp)
            .background(color, CircleShape)
            .semantics { contentDescription = estado.descripcion },
    )
}

/// `true` si el teléfono tiene una red con salida a internet, y se actualiza
/// al instante cuando cambia. Ante cualquier fallo del sistema, `true`: no
/// poder medir la red no debe pintar una alarma falsa.
@Composable
fun rememberHayRed(): Boolean {
    val contexto = LocalContext.current
    var hayRed by remember { mutableStateOf(hayRedAhora(contexto)) }
    DisposableEffect(contexto) {
        val gestor = contexto.getSystemService(ConnectivityManager::class.java)
        val llamada = object : ConnectivityManager.NetworkCallback() {
            override fun onCapabilitiesChanged(network: Network, capacidades: NetworkCapabilities) {
                hayRed = tieneSalida(capacidades)
            }

            override fun onLost(network: Network) {
                hayRed = false
            }
        }
        val registrada = runCatching { gestor.registerDefaultNetworkCallback(llamada) }.isSuccess
        onDispose {
            if (registrada) runCatching { gestor.unregisterNetworkCallback(llamada) }
        }
    }
    return hayRed
}

private fun hayRedAhora(contexto: Context): Boolean = runCatching {
    val gestor = contexto.getSystemService(ConnectivityManager::class.java)
    tieneSalida(gestor.getNetworkCapabilities(gestor.activeNetwork))
}.getOrDefault(true)

private fun tieneSalida(capacidades: NetworkCapabilities?): Boolean =
    capacidades != null &&
        capacidades.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) &&
        capacidades.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)

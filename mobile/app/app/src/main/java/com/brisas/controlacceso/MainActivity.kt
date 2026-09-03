package com.brisas.controlacceso

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import java.io.File
import uniffi.control_acceso_mobile.Nucleo

/// Tope de ancho para todo el contenido de la app — sin esto, en horizontal
/// (o en una tablet) cualquier pantalla se estira de punta a punta porque
/// ninguna usa `fillMaxWidth()` con límite (reportado con foto real: el
/// buscador de Activos quedaba enorme en horizontal). En un teléfono en
/// vertical no hace nada (la pantalla ya es más angosta que esto); sólo
/// entra en juego cuando sobra ancho.
private val ANCHO_MAXIMO_CONTENIDO = 480.dp

/// Punto de entrada de la app. Abre la única conexión SQLite del teléfono
/// una sola vez al arrancar — ver docs/plan-app-movil.md ("Toda la lógica
/// de negocio corre en el teléfono") — y la reusa en todas las pantallas
/// vía [PantallaLogin], que nunca la reabre.
///
/// El login (`PantallaLogin.kt`) y la navegación principal
/// (`PantallaPrincipal.kt`) viven en sus propios archivos — este sólo
/// arranca la Activity.
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            val rutaBaseDatos = File(filesDir, "control_acceso.db").absolutePath
            val nucleo = remember { Nucleo.abrir(rutaBaseDatos) }
            TemaBrisas {
                // `targetSdk` 36 (Android 15+) obliga a la app a dibujar
                // "borde a borde": sin este padding, el contenido queda
                // debajo de la barra de estado, el recorte de la cámara
                // frontal (en horizontal queda al costado, no arriba) y la
                // barra de navegación — se vio en un Honor 7 Pro real
                // tapando el nombre de sesión y "Salir". `safeDrawingPadding`
                // reserva ese espacio en cualquier orientación y en
                // cualquier versión de Android (no hace nada si el
                // dispositivo no tiene de qué protegerse).
                Surface(
                    modifier = Modifier.fillMaxSize().safeDrawingPadding(),
                    color = MaterialTheme.colorScheme.background,
                ) {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.TopCenter) {
                        Box(modifier = Modifier.widthIn(max = ANCHO_MAXIMO_CONTENIDO).fillMaxHeight()) {
                            PantallaLogin(nucleo, directorio = filesDir.absolutePath)
                        }
                    }
                }
            }
        }
    }
}

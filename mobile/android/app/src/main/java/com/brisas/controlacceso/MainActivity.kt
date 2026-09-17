package com.brisas.controlacceso

import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.unit.dp
import androidx.core.view.WindowCompat
import androidx.lifecycle.viewmodel.compose.viewModel

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
        enableEdgeToEdge()
        // Sólo en release -- en debug bloqueaba también las capturas de
        // pantalla/grabación usadas para revisar diseño en el emulador
        // (adb screencap/screenrecord devuelven negro contra una ventana
        // FLAG_SECURE, sea cual sea el backend de GPU). La protección real
        // contra shoulder-surfing/grabación sigue intacta en cualquier APK
        // de release o instalado desde un Release real de GitHub.
        if (!BuildConfig.DEBUG) {
            window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        }
        GestorTema.inicializar(this)
        setContent {
            val aplicacion: AplicacionViewModel = viewModel()
            val estado by aplicacion.estado.collectAsState()
            TemaBrisas {
                // `enableEdgeToEdge()` deja la barra de estado/navegación
                // transparentes -- sin sincronizar nada más, quedaban con el
                // blanco/negro por defecto del sistema encima del fondo
                // lavanda (u oscuro) real de la app, que nunca se pintaba
                // detrás de ellas (hallazgo 2026-09-15, con foto real en
                // claro Y oscuro). El `Surface` de acá abajo ahora sí llena
                // toda la pantalla -- ESE es el color que se termina viendo
                // detrás de las barras del sistema.
                val oscuro = GestorTema.oscuroForzado ?: isSystemInDarkTheme()
                val vista = LocalView.current
                SideEffect {
                    val controlador = WindowCompat.getInsetsController(window, vista)
                    controlador.isAppearanceLightStatusBars = !oscuro
                    controlador.isAppearanceLightNavigationBars = !oscuro
                }
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background,
                ) {
                    // `targetSdk` 36 (Android 15+) obliga a la app a dibujar
                    // "borde a borde": sin este padding, el contenido queda
                    // debajo de la barra de estado, el recorte de la cámara
                    // frontal (en horizontal queda al costado, no arriba) y
                    // la barra de navegación — se vio en un Honor 7 Pro real
                    // tapando el nombre de sesión y "Salir". `safeDrawingPadding`
                    // reserva ese espacio en cualquier orientación y en
                    // cualquier versión de Android (no hace nada si el
                    // dispositivo no tiene de qué protegerse) -- acá abajo,
                    // no en el `Surface`, para que el COLOR de fondo sí
                    // llegue hasta atrás de las barras del sistema aunque el
                    // contenido interactivo no.
                    Box(
                        modifier = Modifier.fillMaxSize().safeDrawingPadding(),
                        contentAlignment = Alignment.TopCenter,
                    ) {
                        Box(modifier = Modifier.widthIn(max = ANCHO_MAXIMO_CONTENIDO).fillMaxHeight()) {
                            when (val actual = estado) {
                                EstadoAplicacion.Cargando -> CircularProgressIndicator(Modifier.align(Alignment.Center))
                                is EstadoAplicacion.Fallo -> Text(
                                    actual.mensaje,
                                    color = MaterialTheme.colorScheme.error,
                                    modifier = Modifier.align(Alignment.Center).padding(24.dp),
                                )
                                is EstadoAplicacion.Lista -> ContenidoAplicacion(actual.entorno)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ContenidoAplicacion(entorno: EntornoAplicacion) {
    var requiereArranque by remember(entorno) { mutableStateOf(entorno.requiereConfiguracionInicial) }
    if (requiereArranque) {
        PantallaPrimerArranque(
            entorno.nucleo,
            secretoStore = entorno.secretoStore,
            metadata = entorno.metadata,
            onListo = { requiereArranque = false },
        )
    } else {
        PantallaLogin(entorno.nucleo, entorno.directorio, entorno.secretoStore)
    }
}

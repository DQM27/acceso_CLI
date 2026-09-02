package com.brisas.controlacceso

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import java.io.File
import uniffi.control_acceso_mobile.Nucleo

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
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background,
                ) {
                    PantallaLogin(nucleo)
                }
            }
        }
    }
}

package com.brisas.controlacceso

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import java.io.File
import uniffi.control_acceso_mobile.abrirNucleo

// Prueba de vida del puente Kotlin -> Rust: abre el núcleo real de
// `control_acceso` contra una base SQLite del área privada de la app y
// muestra el resultado en pantalla. Ver docs/plan-app-movil.md.
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            val rutaBaseDatos = File(filesDir, "control_acceso.db").absolutePath
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    PantallaPruebaDeVida(rutaBaseDatos)
                }
            }
        }
    }
}

@Composable
fun PantallaPruebaDeVida(rutaBaseDatos: String) {
    var resultado by remember { mutableStateOf("Abriendo núcleo de Rust...") }

    remember {
        resultado = try {
            abrirNucleo(rutaBaseDatos)
        } catch (error: Exception) {
            "Error: ${error.message}"
        }
        true
    }

    Column(modifier = Modifier.padding(24.dp)) {
        Text("Control de Acceso — piloto móvil", style = MaterialTheme.typography.titleLarge)
        Text(resultado, modifier = Modifier.padding(top = 16.dp))
    }
}

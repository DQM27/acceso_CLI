package com.brisas.controlacceso

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import java.io.File
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.UsuarioSesion

// El Nucleo abre la única conexión SQLite del teléfono una sola vez al
// arrancar la app y se reusa en todas las pantallas — ver
// docs/plan-app-movil.md ("Toda la lógica de negocio corre en el teléfono").
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            val rutaBaseDatos = File(filesDir, "control_acceso.db").absolutePath
            val nucleo = remember { Nucleo.abrir(rutaBaseDatos) }
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    PantallaLogin(nucleo)
                }
            }
        }
    }
}

@Composable
fun PantallaLogin(nucleo: Nucleo) {
    var cedula by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var sesion by remember { mutableStateOf<UsuarioSesion?>(null) }

    val sesionActual = sesion
    if (sesionActual != null) {
        Column(modifier = Modifier.padding(24.dp)) {
            Text("Brisas Control de Acceso", style = MaterialTheme.typography.titleLarge)
            Text(
                "Sesión iniciada: ${sesionActual.nombre} (${sesionActual.rol})",
                modifier = Modifier.padding(top = 16.dp),
            )
        }
        return
    }

    Column(modifier = Modifier.padding(24.dp)) {
        Text("Brisas Control de Acceso", style = MaterialTheme.typography.titleLarge)

        OutlinedTextField(
            value = cedula,
            onValueChange = { cedula = it },
            label = { Text("Cédula") },
            modifier = Modifier.fillMaxWidth().padding(top = 24.dp),
        )
        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("Contraseña") },
            visualTransformation = PasswordVisualTransformation(),
            modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
        )
        Button(
            onClick = {
                error = null
                try {
                    sesion = nucleo.autenticar(cedula, password)
                } catch (excepcion: NucleoException) {
                    error = excepcion.message
                }
            },
            modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
        ) {
            Text("Ingresar")
        }

        val mensajeError = error
        if (mensajeError != null) {
            Text(mensajeError, modifier = Modifier.padding(top = 16.dp))
        }
    }
}

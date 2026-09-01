package com.brisas.controlacceso

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
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

@Composable
fun PantallaLogin(nucleo: Nucleo) {
    var cedula by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var sesion by remember { mutableStateOf<UsuarioSesion?>(null) }

    val sesionActual = sesion
    if (sesionActual != null) {
        PantallaPrincipal(
            nucleo = nucleo,
            sesion = sesionActual,
            onCerrarSesion = {
                nucleo.cerrarSesion()
                sesion = null
                cedula = ""
                password = ""
            },
        )
        return
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Image(
            painter = painterResource(id = R.drawable.marca),
            contentDescription = null,
            modifier = Modifier.size(96.dp).clip(RoundedCornerShape(20.dp)),
        )

        Text(
            "Control de acceso",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 16.dp),
        )
        Text(
            "Brisas",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        OutlinedTextField(
            value = cedula,
            onValueChange = { cedula = it },
            label = { Text("Cédula") },
            singleLine = true,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 32.dp),
        )
        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("Contraseña") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                focusedLabelColor = MaterialTheme.colorScheme.primary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
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
            colors = ButtonDefaults.buttonColors(
                containerColor = MaterialTheme.colorScheme.primary,
                contentColor = MaterialTheme.colorScheme.onPrimary,
            ),
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        ) {
            Text("Ingresar")
        }

        val mensajeError = error
        if (mensajeError != null) {
            Text(
                mensajeError,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(top = 16.dp),
            )
        }
    }
}

/// Sólo las pantallas de uso frecuente son pestañas (Buscar, Activos) — las
/// de creación (uso esporádico: dar de alta un contratista o una empresa
/// nuevos) viven detrás del botón "+", no compitiendo por espacio en la
/// barra de pestañas.
private sealed class Pantalla {
    data object Buscar : Pantalla()

    data object Activos : Pantalla()

    data object NuevoContratista : Pantalla()

    data object NuevaEmpresa : Pantalla()
}

@Composable
fun PantallaPrincipal(nucleo: Nucleo, sesion: UsuarioSesion, onCerrarSesion: () -> Unit) {
    var pantalla by remember { mutableStateOf<Pantalla>(Pantalla.Buscar) }
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
                androidx.compose.foundation.layout.Box {
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
                    }
                }
                TextButton(onClick = onCerrarSesion) {
                    Text("Salir")
                }
            }
        }

        when (val actual = pantalla) {
            is Pantalla.NuevoContratista, is Pantalla.NuevaEmpresa -> {
                TextButton(onClick = { pantalla = Pantalla.Buscar }, modifier = Modifier.padding(start = 8.dp)) {
                    Text("← Volver")
                }
                if (actual is Pantalla.NuevoContratista) {
                    PantallaNuevoContratista(nucleo)
                } else {
                    PantallaNuevaEmpresa(nucleo)
                }
            }
            else -> {
                var pestana by remember { mutableIntStateOf(0) }
                PrimaryTabRow(selectedTabIndex = pestana) {
                    Tab(selected = pestana == 0, onClick = { pestana = 0 }, text = { Text("Buscar") })
                    Tab(selected = pestana == 1, onClick = { pestana = 1 }, text = { Text("Activos") })
                }
                if (pestana == 0) {
                    PantallaContratistas(nucleo)
                } else {
                    PantallaActivos(nucleo)
                }
            }
        }
    }
}

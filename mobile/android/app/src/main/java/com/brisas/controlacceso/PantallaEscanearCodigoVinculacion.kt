package com.brisas.controlacceso

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

/// Lee el QR que muestra el panel al crear o re-vincular un dispositivo
/// (`brisas-acceso://vincular?codigo=…`, ver [CodigoVinculacion.desdeQr]):
/// así el código pasa de la pantalla del administrador al teléfono sin que
/// nadie lo copie ni lo mande por chat. La cámara es la de [EscanerQr]; el
/// primer código válido cierra el escáner.
@Composable
fun PantallaEscanearCodigoVinculacion(
    onCodigoLeido: (String) -> Unit,
    onEscribirCodigo: () -> Unit,
    onCerrar: () -> Unit,
) {
    RegistrarPantalla("escanear_codigo_vinculacion")
    EscanerConPermisoCamara(
        mensajePermiso = "Se necesita permiso de cámara para leer el código QR del panel.",
        onCerrar = onCerrar,
    ) {
        EscanerQr(
            textoGuia = "Apunte al código QR que muestra el panel",
            interpretar = CodigoVinculacion::desdeQr,
            onLeido = onCodigoLeido,
            onCerrar = onCerrar,
            accionSecundaria = {
                TextButton(
                    onClick = onEscribirCodigo,
                    colors = ButtonDefaults.textButtonColors(contentColor = Color.White),
                    modifier = Modifier.padding(top = 8.dp),
                ) {
                    Text("Escribir el código")
                }
            },
        )
    }
}

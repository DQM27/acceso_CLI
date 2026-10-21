package com.brisas.controlacceso

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ErrorOutline
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp

/// Mientras se abre la base al arrancar: la marca y una barrita, en vez de
/// una ruedita sola en el medio de la nada.
@Composable
internal fun PantallaCargandoArranque() {
    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        MarcaPorteria(compacto = false)
        LinearProgressIndicator(
            modifier = Modifier.padding(top = 28.dp).width(120.dp).clip(RoundedCornerShape(50)),
        )
    }
}

/// La base no se pudo abrir: qué pasó en palabras simples y "Reintentar",
/// sobre la misma [HojaPorteria] del login. El detalle técnico nunca llega
/// acá (ver [AplicacionViewModel]).
@Composable
internal fun PantallaFalloArranque(mensaje: String, onReintentar: () -> Unit) {
    HojaPorteria(
        encabezado = { compacto ->
            IconoPorteria(
                Icons.Outlined.ErrorOutline,
                compacto,
                fondo = MaterialTheme.colorScheme.errorContainer,
                tinte = MaterialTheme.colorScheme.error,
            )
        },
    ) {
        TituloHoja(
            "No se pudo abrir la app",
            "$mensaje Pruebe de nuevo; si sigue pasando, avise a un administrador.",
        )
        BotonPorteria(
            texto = "Reintentar",
            onClick = onReintentar,
            modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
        )
    }
}

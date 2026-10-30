package com.brisas.controlacceso

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.control_acceso_mobile.Nucleo

/// Login real contra `Nucleo.autenticar` (Rust) — todo el estado y la
/// llamada viven en [LoginViewModel] (ver mobile/android/arquitectura.md), este
/// Composable sólo dibuja. Una vez hay sesión, delega a [PantallaPrincipal]
/// en vez de dibujar nada propio — mismo `Nucleo` para toda la app, no se
/// reabre la base al loguear.
///
/// Dos pasos sobre la misma [HojaPorteria]: cédula y contraseña, y, si la
/// contraseña era la temporal del alta, elegir una nueva antes de entrar
/// (antes eso sólo se podía en el escritorio).
@Composable
fun PantallaLogin(nucleo: Nucleo, directorio: String, metadata: MetadatosDispositivoLocal) {
    RegistrarPantalla("login")
    val viewModel: LoginViewModel =
        viewModel(factory = LoginViewModel.factory(nucleo))

    val sesionActual = viewModel.sesion
    val propietarioSesion = viewModel.propietarioSesion
    if (sesionActual != null && propietarioSesion != null) {
        CompositionLocalProvider(LocalViewModelStoreOwner provides propietarioSesion) {
            PantallaPrincipal(
                nucleo = nucleo,
                sesion = sesionActual,
                directorio = directorio,
                metadata = metadata,
                onCerrarSesion = { viewModel.cerrarSesion() },
                onBloquear = { viewModel.cerrarSesion(conservarCedula = true) },
            )
        }
        return
    }

    val textoUnidad = rememberTextoUnidadOperativa(nucleo)
    val estadoRed = estadoConexion(hayRed = rememberHayRed(), falloSincronizacion = false)
    val tema = temaActual()

    // Vibración de "rechazado" con cada intento fallido, junto con la
    // sacudida del formulario.
    val haptica = LocalHapticFeedback.current
    LaunchedEffect(viewModel.intentosFallidos) {
        if (viewModel.intentosFallidos > 0) haptica.performHapticFeedback(HapticFeedbackType.Reject)
    }

    // Discreto, a la manera del inicio de sesión de iOS (pedido del usuario
    // 2026-10-25: el anterior se sentía pomposo): ícono de la app mediano, el
    // título de lo que se hace ("Iniciar sesión") y, debajo, la unidad en una
    // línea gris; los campos agrupados en un solo bloque sobre el fondo, sin
    // hoja elevada, íconos ni marcos.
    // La nieve de temporada, de fondo en toda la pantalla (ver `NieveNavidena.kt`).
    Box(modifier = Modifier.fillMaxSize()) {
    NieveNavidena()
    HojaPorteria(
        conHoja = false,
        encabezado = { compacto ->
            MarcaPorteria(compacto, tamanoNormal = 72.dp, tamanoCompacto = 44.dp)
            Text(
                if (viewModel.cambiandoTemporal) "Contraseña nueva" else "Iniciar sesión",
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.padding(top = if (compacto) 10.dp else 18.dp),
            )
            // Unidad con que quedó registrado el teléfono: si lo registraron
            // en la unidad equivocada, se ve antes de entrar.
            if (textoUnidad != null) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    modifier = Modifier.padding(top = 6.dp),
                ) {
                    PuntoConexion(estadoRed)
                    Text(
                        textoUnidad,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                    )
                }
            }
        },
        arriba = {
            IconButton(
                onClick = { GestorTema.alternar(tema) },
                modifier = Modifier.align(Alignment.TopEnd).padding(4.dp),
            ) {
                IconoTema(tema)
            }
        },
    ) {
        AnimatedContent(
            targetState = viewModel.cambiandoTemporal,
            transitionSpec = {
                val haciaAdelante = targetState
                (fadeIn() + slideInHorizontally { ancho -> if (haciaAdelante) ancho / 4 else -ancho / 4 }) togetherWith
                    (fadeOut() + slideOutHorizontally { ancho -> if (haciaAdelante) -ancho / 4 else ancho / 4 })
            },
            label = "pasoLogin",
        ) { cambiando ->
            Column {
                if (cambiando) PasoPasswordTemporal(viewModel) else PasoCredenciales(viewModel)
            }
        }
    }
    }
}

@Composable
private fun ColumnScope.PasoCredenciales(viewModel: LoginViewModel) {
    var mostrarAyuda by rememberSaveable { mutableStateOf(false) }
    val focoPassword = remember { FocusRequester() }
    // Tras el cierre por inactividad la cédula ya está: el cursor espera en
    // la contraseña.
    LaunchedEffect(Unit) {
        if (viewModel.cedula.isNotBlank()) focoPassword.requestFocus()
    }
    val foco = LocalFocusManager.current
    val puedeEnviar = !viewModel.autenticando && viewModel.cedula.isNotBlank() && viewModel.password.isNotBlank()
    val enviar = {
        if (puedeEnviar) {
            foco.clearFocus()
            viewModel.autenticar()
        }
    }

    AvisoPorteria(viewModel.error)

    // Cédula y contraseña en un solo bloque, como en iOS; el formulario
    // entero se sacude con un intento fallido.
    GrupoCampos(modifier = Modifier.sacudirAl(viewModel.intentosFallidos)) {
        CampoAgrupado(
            valor = viewModel.cedula,
            onCambiar = { viewModel.cambiarCedula(it) },
            ayuda = "Cédula",
            habilitado = !viewModel.autenticando,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Next),
            keyboardActions = KeyboardActions(onNext = { focoPassword.requestFocus() }),
        )
        DivisorGrupo()
        CampoClaveAgrupado(
            valor = viewModel.password,
            onCambiar = { viewModel.cambiarPassword(it) },
            ayuda = "Contraseña",
            error = viewModel.errorCredenciales != null,
            habilitado = !viewModel.autenticando,
            onAccionTeclado = enviar,
            modifier = Modifier.focusRequester(focoPassword),
        )
    }
    // Cédula o contraseña incorrecta: debajo del bloque, chico y en rojo.
    viewModel.errorCredenciales?.let { mensaje ->
        Text(
            mensaje,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(start = 16.dp, top = 6.dp),
        )
    }

    BotonPorteria(
        texto = if (viewModel.autenticando) "Verificando…" else "Ingresar",
        onClick = enviar,
        enabled = puedeEnviar,
        trabajando = viewModel.autenticando,
        modifier = Modifier.padding(top = 16.dp),
    )

    TextButton(
        onClick = { mostrarAyuda = true },
        modifier = Modifier.align(Alignment.CenterHorizontally).padding(top = 4.dp),
    ) {
        Text("¿No puede entrar?")
    }
    // Versión instalada, discreta.
    Text(
        "Versión ${BuildConfig.VERSION_NAME}",
        style = MaterialTheme.typography.labelSmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.align(Alignment.CenterHorizontally),
    )

    if (mostrarAyuda) {
        AlertDialog(
            onDismissRequest = { mostrarAyuda = false },
            confirmButton = { TextButton(onClick = { mostrarAyuda = false }) { Text("Entendido") } },
            title = { Text("¿No puede entrar?") },
            text = {
                Text(
                    "Si olvidó su contraseña, pida a un administrador que la restablezca desde el " +
                        "panel. Le dará una contraseña temporal: al ingresar con ella, la app le " +
                        "pedirá elegir una nueva.\n\nSi la app dice que su usuario está inactivo, " +
                        "el administrador tiene que reactivarlo.",
                )
            },
        )
    }
}

@Composable
private fun ColumnScope.PasoPasswordTemporal(viewModel: LoginViewModel) {
    BackHandler(onBack = viewModel::cancelarCambioTemporal)
    val focoConfirmacion = remember { FocusRequester() }
    val foco = LocalFocusManager.current
    val revision = viewModel.revision
    val enviar = {
        if (revision.valida && !viewModel.guardandoNueva) {
            foco.clearFocus()
            viewModel.guardarPasswordNueva()
        }
    }

    // El título ("Contraseña nueva") ya va arriba; acá una línea corta: con el
    // teclado abierto, cada renglón de más empujaba "Guardar y entrar" debajo
    // del teclado (visto en el teléfono 2026-10-21).
    Text(
        "La suya es temporal: elija una nueva para entrar.",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
    )

    GrupoCampos {
        CampoClaveAgrupado(
            valor = viewModel.passwordNueva,
            onCambiar = { viewModel.cambiarPasswordNueva(it) },
            ayuda = "Contraseña nueva",
            imeAction = ImeAction.Next,
            onAccionTeclado = { focoConfirmacion.requestFocus() },
            habilitado = !viewModel.guardandoNueva,
        )
        DivisorGrupo()
        CampoClaveAgrupado(
            valor = viewModel.confirmacion,
            onCambiar = { viewModel.cambiarConfirmacion(it) },
            ayuda = "Repetir contraseña",
            onAccionTeclado = enviar,
            habilitado = !viewModel.guardandoNueva,
            modifier = Modifier.focusRequester(focoConfirmacion),
        )
    }

    Column(modifier = Modifier.padding(start = 4.dp, top = 8.dp, bottom = 12.dp)) {
        RequisitoPorteria("Al menos 8 caracteres", revision.largoSuficiente)
        RequisitoPorteria("Distinta de la temporal", revision.distintaDeLaTemporal)
        RequisitoPorteria("Las dos coinciden", revision.coinciden)
    }

    AvisoPorteria(viewModel.errorCambio)

    BotonPorteria(
        texto = if (viewModel.guardandoNueva) "Guardando…" else "Guardar y entrar",
        onClick = enviar,
        enabled = revision.valida,
        trabajando = viewModel.guardandoNueva,
    )
    TextButton(
        onClick = viewModel::cancelarCambioTemporal,
        enabled = !viewModel.guardandoNueva,
        modifier = Modifier.align(Alignment.CenterHorizontally).padding(top = 4.dp),
    ) {
        Text("Volver")
    }
}

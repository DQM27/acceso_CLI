package com.brisas.controlacceso

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Badge
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.withStyle
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

    HojaPorteria(
        encabezado = { compacto ->
            MarcaPorteria(compacto)
            // "ACCESO" liviano y "MÓVIL" en negrita: distingue esta app de
            // las otras plataformas (desktop/web) que comparten la marca.
            Text(
                buildAnnotatedString {
                    withStyle(SpanStyle(fontWeight = FontWeight.Normal)) { append("ACCESO ") }
                    withStyle(SpanStyle(fontWeight = FontWeight.Bold)) { append("MÓVIL") }
                },
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.padding(top = if (compacto) 8.dp else 16.dp),
            )
            // Unidad y etiqueta con que quedó registrado el teléfono: si lo
            // registraron en la unidad equivocada, se ve antes de entrar.
            if (textoUnidad != null) {
                ChipUnidad(textoUnidad, estadoRed, modifier = Modifier.padding(top = 10.dp))
            }
        },
        arriba = {
            BotonIconoCuadradoBrisas(
                onClick = { GestorTema.alternar(tema) },
                modifier = Modifier.align(Alignment.TopEnd).padding(12.dp),
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

@Composable
private fun ColumnScope.PasoCredenciales(viewModel: LoginViewModel) {
    var mostrarAyuda by rememberSaveable { mutableStateOf(false) }
    val focoPassword = remember { FocusRequester() }
    val foco = LocalFocusManager.current
    val puedeEnviar = !viewModel.autenticando && viewModel.cedula.isNotBlank() && viewModel.password.isNotBlank()
    val enviar = {
        if (puedeEnviar) {
            foco.clearFocus()
            viewModel.autenticar()
        }
    }

    AvisoPorteria(viewModel.error)
    TituloHoja("Iniciar sesión", "Ingrese con su cédula y contraseña.")

    Column(modifier = Modifier.padding(top = 16.dp).sacudirAl(viewModel.intentosFallidos)) {
        OutlinedTextField(
            value = viewModel.cedula,
            onValueChange = { viewModel.cambiarCedula(it) },
            label = { Text("Cédula") },
            leadingIcon = { Icon(Icons.Outlined.Badge, contentDescription = null) },
            singleLine = true,
            enabled = !viewModel.autenticando,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Next),
            keyboardActions = KeyboardActions(onNext = { focoPassword.requestFocus() }),
            shape = FormaCampoBrisas,
            colors = ColoresCampoBrisas(),
            modifier = Modifier.fillMaxWidth(),
        )
        // `KeyboardType.Password` (adentro de [CampoClavePorteria]) es lo que
        // de verdad apaga predicción/autocorrección/diccionario personal: sin
        // él la barra de sugerencias mostraba lo tipeado en la contraseña
        // (hallazgo real del usuario 2026-09-21).
        CampoClavePorteria(
            valor = viewModel.password,
            onCambiar = { viewModel.cambiarPassword(it) },
            etiqueta = "Contraseña",
            icono = Icons.Outlined.Lock,
            error = viewModel.errorCredenciales,
            habilitado = !viewModel.autenticando,
            onAccionTeclado = enviar,
            modifier = Modifier.padding(top = 8.dp).focusRequester(focoPassword),
        )
    }

    BotonPorteria(
        texto = if (viewModel.autenticando) "Verificando…" else "Ingresar",
        onClick = enviar,
        enabled = puedeEnviar,
        trabajando = viewModel.autenticando,
        modifier = Modifier.padding(top = 16.dp),
    )

    Row(
        modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextButton(onClick = { mostrarAyuda = true }) { Text("¿No puede entrar?") }
        // Versión instalada, discreta.
        Text(
            "v${BuildConfig.VERSION_NAME}",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(end = 8.dp),
        )
    }

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

    // Texto corto a propósito: con el teclado abierto, cada renglón de más
    // empujaba "Guardar y entrar" debajo del teclado (visto en el teléfono
    // 2026-10-21).
    TituloHoja("Contraseña nueva", "La suya es temporal: elija una nueva para entrar.")

    CampoClavePorteria(
        valor = viewModel.passwordNueva,
        onCambiar = { viewModel.cambiarPasswordNueva(it) },
        etiqueta = "Contraseña nueva",
        icono = Icons.Outlined.Lock,
        imeAction = ImeAction.Next,
        onAccionTeclado = { focoConfirmacion.requestFocus() },
        habilitado = !viewModel.guardandoNueva,
        modifier = Modifier.padding(top = 12.dp),
    )
    CampoClavePorteria(
        valor = viewModel.confirmacion,
        onCambiar = { viewModel.cambiarConfirmacion(it) },
        etiqueta = "Repetir contraseña",
        icono = Icons.Outlined.Lock,
        onAccionTeclado = enviar,
        habilitado = !viewModel.guardandoNueva,
        modifier = Modifier.padding(top = 8.dp).focusRequester(focoConfirmacion),
    )

    Column(modifier = Modifier.padding(top = 8.dp, bottom = 12.dp)) {
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

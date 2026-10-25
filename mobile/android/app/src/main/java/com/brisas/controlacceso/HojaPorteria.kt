package com.brisas.controlacceso

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.isImeVisible
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ErrorOutline
import androidx.compose.material.icons.outlined.RadioButtonUnchecked
import androidx.compose.material.icons.outlined.Visibility
import androidx.compose.material.icons.outlined.VisibilityOff
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp

/// Base común de las pantallas de antes de entrar (login, contraseña
/// temporal, vinculación, fallo al abrir): la marca arriba y el formulario en
/// una hoja redondeada abajo, al alcance del pulgar -- mismo criterio que la
/// barra flotante y el buscador inferior del resto de la app. Al abrir el
/// teclado la marca se achica y la hoja sube con él, sin que haya que
/// desplazar para ver el botón.
///
/// `arriba` dibuja sobre la esquina superior (cambio de tema, versión).
///
/// `conHoja = false` (el login): el formulario va directo sobre el fondo, sin
/// la hoja elevada, con los campos agrupados como en iOS ([GrupoCampos]).
@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun HojaPorteria(
    encabezado: @Composable ColumnScope.(compacto: Boolean) -> Unit,
    modifier: Modifier = Modifier,
    conHoja: Boolean = true,
    arriba: @Composable BoxScope.() -> Unit = {},
    contenido: @Composable ColumnScope.() -> Unit,
) {
    val compacto = WindowInsets.isImeVisible
    Column(modifier = modifier.fillMaxSize().imePadding()) {
        Box(modifier = Modifier.fillMaxWidth().weight(1f)) {
            Column(
                modifier = Modifier.align(Alignment.Center).padding(horizontal = 24.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                encabezado(compacto)
            }
            arriba()
        }
        if (conHoja) {
            Surface(
                shape = RoundedCornerShape(28.dp),
                color = MaterialTheme.colorScheme.surface,
                tonalElevation = 1.dp,
                modifier = Modifier.fillMaxWidth().padding(start = 12.dp, end = 12.dp, bottom = 12.dp),
            ) {
                Column(
                    modifier = Modifier.verticalScroll(rememberScrollState()).padding(20.dp),
                    content = contenido,
                )
            }
        } else {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .padding(start = 20.dp, end = 20.dp, bottom = 12.dp),
                content = contenido,
            )
        }
    }
}

/// Logo de la app que se achica con el teclado abierto (`compacto`).
@Composable
internal fun MarcaPorteria(compacto: Boolean, tamanoNormal: Dp = 104.dp, tamanoCompacto: Dp = 56.dp) {
    val tamano by animateDpAsState(if (compacto) tamanoCompacto else tamanoNormal, label = "tamanoMarca")
    Image(
        painter = painterResource(id = R.drawable.marca),
        contentDescription = null,
        modifier = Modifier.size(tamano).clip(RoundedCornerShape(tamano * 0.2f)),
    )
}

/// Ícono grande dentro de un círculo suave, para las pantallas que no son el
/// login (vinculación, fallo): dice de un vistazo de qué trata.
@Composable
internal fun IconoPorteria(
    icono: ImageVector,
    compacto: Boolean = false,
    fondo: androidx.compose.ui.graphics.Color = MaterialTheme.colorScheme.primaryContainer,
    tinte: androidx.compose.ui.graphics.Color = MaterialTheme.colorScheme.primary,
) {
    val tamano by animateDpAsState(if (compacto) 56.dp else 88.dp, label = "tamanoIcono")
    Box(
        modifier = Modifier.size(tamano).clip(RoundedCornerShape(tamano * 0.3f)).background(fondo),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icono, contentDescription = null, tint = tinte, modifier = Modifier.size(tamano * 0.5f))
    }
}

/// Cápsula con la unidad del teléfono y el punto de conexión: si lo
/// registraron en la unidad equivocada se ve antes de escribir nada.
@Composable
internal fun ChipUnidad(texto: String, estado: EstadoConexion, modifier: Modifier = Modifier) {
    Row(
        modifier = modifier
            .clip(RoundedCornerShape(50))
            .background(MaterialTheme.colorScheme.surface)
            .padding(horizontal = 12.dp, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        PuntoConexion(estado)
        Text(
            texto,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = 1,
        )
    }
}

/// Aviso de error con ícono: errores que no son de un campo (sin red,
/// cuenta inactiva). Aparece y se va con un despliegue suave.
@Composable
internal fun AvisoPorteria(texto: String?) {
    // Retiene el último texto mientras se oculta, si no la animación de
    // salida dibujaría una caja vacía.
    var ultimo by remember { mutableStateOf(texto.orEmpty()) }
    if (texto != null) ultimo = texto
    AnimatedVisibility(
        visible = texto != null,
        enter = fadeIn() + expandVertically(),
        exit = fadeOut() + shrinkVertically(),
    ) {
        val tinte = MaterialTheme.colorScheme.onErrorContainer
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(bottom = 16.dp)
                .clip(RoundedCornerShape(14.dp))
                .background(MaterialTheme.colorScheme.errorContainer)
                .padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Icon(Icons.Outlined.ErrorOutline, contentDescription = null, tint = tinte, modifier = Modifier.size(20.dp))
            Text(ultimo, color = tinte, style = MaterialTheme.typography.bodyMedium)
        }
    }
}

/// Título y bajada de la hoja.
@Composable
internal fun TituloHoja(titulo: String, bajada: String? = null) {
    Text(titulo, style = MaterialTheme.typography.titleLarge)
    if (bajada != null) {
        Text(
            bajada,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 4.dp),
        )
    }
}

/// Alto del botón principal de estas pantallas: más que [AlturaControlBrisas]
/// porque es la única acción y se toca de pie, a veces con guantes.
internal val AlturaBotonPorteria = 48.dp

/// Botón principal de la hoja: ancho completo y, mientras trabaja, una ruedita
/// adentro en vez de cambiar de tamaño.
@Composable
internal fun BotonPorteria(
    texto: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    trabajando: Boolean = false,
) {
    BotonBrisas(
        onClick = onClick,
        enabled = enabled && !trabajando,
        modifier = modifier.fillMaxWidth().height(AlturaBotonPorteria),
    ) {
        if (trabajando) {
            CircularProgressIndicator(
                strokeWidth = 2.dp,
                color = LocalContentColor.current,
                modifier = Modifier.size(18.dp),
            )
            Text(texto, modifier = Modifier.padding(start = 10.dp))
        } else {
            Text(texto)
        }
    }
}

/// Campos agrupados como en los formularios de iOS (Ajustes, iniciar sesión
/// con la cuenta): un solo bloque redondeado del color de la superficie, un
/// renglón por campo separados por una línea fina, sin íconos ni marcos. Más
/// discreto que un campo con borde y etiqueta flotante por cada dato.
@Composable
internal fun GrupoCampos(modifier: Modifier = Modifier, contenido: @Composable ColumnScope.() -> Unit) {
    Column(
        modifier = modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(14.dp))
            .background(MaterialTheme.colorScheme.surface),
        content = contenido,
    )
}

/// La línea fina entre dos renglones de [GrupoCampos], corrida desde el borde
/// izquierdo como en iOS.
@Composable
internal fun DivisorGrupo() {
    HorizontalDivider(
        modifier = Modifier.padding(start = 16.dp),
        thickness = 0.5.dp,
        color = MaterialTheme.colorScheme.outline.copy(alpha = 0.35f),
    )
}

/// Un renglón de [GrupoCampos]: el texto de ayuda adentro mientras está vacío
/// y, a la derecha, lo que haga falta (el ojo de la contraseña, la cámara).
/// Con [etiqueta], ésta va a la izquierda y el valor a su lado, como en los
/// formularios de iOS: lo que llenó el escáner se revisa de un vistazo. Con
/// [error], el texto en rojo. [modifier] va al campo (para su `focusRequester`).
@Composable
internal fun CampoAgrupado(
    valor: String,
    onCambiar: (String) -> Unit,
    ayuda: String,
    modifier: Modifier = Modifier,
    etiqueta: String? = null,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    keyboardActions: KeyboardActions = KeyboardActions.Default,
    visualTransformation: VisualTransformation = VisualTransformation.None,
    habilitado: Boolean = true,
    error: Boolean = false,
    alFinal: (@Composable () -> Unit)? = null,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 52.dp)
            .padding(start = 16.dp, end = if (alFinal != null) 4.dp else 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (etiqueta != null) {
            Text(
                etiqueta,
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier.width(ANCHO_ETIQUETA_CAMPO),
            )
        }
        BasicTextField(
            value = valor,
            onValueChange = onCambiar,
            enabled = habilitado,
            singleLine = true,
            textStyle = MaterialTheme.typography.bodyLarge.copy(
                color = if (error) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface,
            ),
            cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
            keyboardOptions = keyboardOptions,
            keyboardActions = keyboardActions,
            visualTransformation = visualTransformation,
            modifier = modifier.weight(1f),
            decorationBox = { campo ->
                Box(contentAlignment = Alignment.CenterStart) {
                    if (valor.isEmpty()) {
                        Text(ayuda, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    campo()
                }
            },
        )
        alFinal?.invoke()
    }
}

/// Ancho de la etiqueta de un [CampoAgrupado] (y de [FilaSelector]): todas
/// del mismo ancho, así los valores quedan alineados en columna.
internal val ANCHO_ETIQUETA_CAMPO = 96.dp

/// Renglón de contraseña de [GrupoCampos], con ojo para mostrarla: con prisa
/// o con guantes, revisar lo tipeado evita un intento fallido.
/// `KeyboardType.Password` es lo que de verdad apaga predicción,
/// autocorrección y diccionario personal: sin él la barra de sugerencias
/// mostraba lo tipeado (hallazgo real del usuario 2026-09-21).
@Composable
internal fun CampoClaveAgrupado(
    valor: String,
    onCambiar: (String) -> Unit,
    ayuda: String,
    modifier: Modifier = Modifier,
    error: Boolean = false,
    imeAction: ImeAction = ImeAction.Done,
    onAccionTeclado: () -> Unit = {},
    habilitado: Boolean = true,
) {
    var visible by rememberSaveable { mutableStateOf(false) }
    CampoAgrupado(
        valor = valor,
        onCambiar = onCambiar,
        ayuda = ayuda,
        modifier = modifier,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, imeAction = imeAction),
        keyboardActions = KeyboardActions(onAny = { onAccionTeclado() }),
        visualTransformation = if (visible) VisualTransformation.None else PasswordVisualTransformation(),
        habilitado = habilitado,
        error = error,
        alFinal = {
            IconButton(onClick = { visible = !visible }) {
                Icon(
                    if (visible) Icons.Outlined.VisibilityOff else Icons.Outlined.Visibility,
                    contentDescription = if (visible) "Ocultar contraseña" else "Mostrar contraseña",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.size(20.dp),
                )
            }
        },
    )
}

/// Sacudida corta a los costados cada vez que cambia `disparo` (no en el
/// primer dibujo): el gesto universal de "eso no era".
@Composable
internal fun Modifier.sacudirAl(disparo: Int): Modifier {
    val desplazamiento = remember { Animatable(0f) }
    LaunchedEffect(disparo) {
        if (disparo == 0) return@LaunchedEffect
        for (paso in listOf(14f, -12f, 9f, -6f, 3f, 0f)) {
            desplazamiento.animateTo(paso, androidx.compose.animation.core.tween(45))
        }
    }
    return graphicsLayer { translationX = desplazamiento.value * density }
}

/// Fila de requisito que se tilda cuando se cumple (contraseña nueva).
@Composable
internal fun RequisitoPorteria(texto: String, cumplido: Boolean) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.padding(vertical = 2.dp),
    ) {
        val color = if (cumplido) ColorExitoBrisas else MaterialTheme.colorScheme.onSurfaceVariant
        Icon(
            if (cumplido) Icons.Outlined.CheckCircle else Icons.Outlined.RadioButtonUnchecked,
            contentDescription = null,
            tint = color,
            modifier = Modifier.size(18.dp),
        )
        Text(
            texto,
            style = MaterialTheme.typography.bodyMedium,
            color = color,
            fontWeight = if (cumplido) FontWeight.Medium else FontWeight.Normal,
        )
    }
}

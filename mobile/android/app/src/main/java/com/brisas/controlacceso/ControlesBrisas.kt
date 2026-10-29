package com.brisas.controlacceso

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Cancel
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonColors
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextFieldColors
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.control_acceso_mobile.EncargadoRuta

/** Radio de los campos "filled" (buscadores, login) -- iguala el de
 * [FormaControlBrisas]; antes era 28.dp (cápsula) y se veía más redondeado
 * que el mockup, que usa esquinas visibles, no un óvalo completo. */
internal val FormaCampoBrisas = RoundedCornerShape(16.dp)

/** Altura de los campos de búsqueda "filled" (~10% más bajo que la altura
 * por defecto de un `TextField` sin label, pedido explícito 2026-09-15) --
 * distinto de [AlturaControlBrisas] (botones) a propósito. Compartido entre
 * varias pantallas con buscador (Activos, Rutas, Proveedores...) para que
 * todas luzcan igual (2026-09-15). */
internal val AlturaBusquedaBrisas = 50.dp

// Antes que [FormaPildoraBrisas]: las propiedades de un archivo se inicializan
// en orden.
private val RADIO_PILDORA = 12.dp

/** Forma de las píldoras de selector ([FilaPildoras]) -- antes cápsula
 * completa (percent=50), se veía más redondeada que el mockup, que usa
 * esquinas bien marcadas pero no un óvalo. */
internal val FormaPildoraBrisas = RoundedCornerShape(RADIO_PILDORA)

/** Colores compartidos para todo campo de texto de la app -- borde visible
 * (`outline` sin foco, `primary` con foco) sobre fondo transparente, mismo
 * look que ya tenía [PantallaNuevoContratista] -- pedido explícito del
 * usuario (2026-09-19) para homogeneizar TODOS los inputs a ese estilo, no
 * al revés. Reemplaza al estilo "filled" sin borde que tuvo este mismo
 * nombre hasta esa fecha -- usar siempre junto a `OutlinedTextField`, no
 * `TextField` (el filled no puede dibujar el marco completo). */
@Composable
internal fun ColoresCampoBrisas(): TextFieldColors = OutlinedTextFieldDefaults.colors(
    focusedBorderColor = MaterialTheme.colorScheme.primary,
    focusedLabelColor = MaterialTheme.colorScheme.primary,
)

/** El campo de búsqueda de la app (el del buscador de abajo y el de la
 * Ayuda), como el de iOS: cápsula del color de la superficie con lupa, una
 * ⓧ para borrar lo escrito en cuanto hay texto, y la tecla "Buscar" del
 * teclado, que lo esconde para ver los resultados enteros. Un
 * `OutlinedTextField` a [AlturaBusquedaBrisas] recorta el texto de ayuda
 * (deja 16dp de relleno arriba y abajo, pensado para 56dp): acá es el mismo
 * marco con menos relleno vertical. */
@Composable
internal fun CampoBusquedaBrisas(
    valor: String,
    onCambiar: (String) -> Unit,
    ayuda: String,
    modifier: Modifier = Modifier,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    forma: Shape = FormaBusquedaBrisas,
) {
    val interaccion = remember { MutableInteractionSource() }
    val teclado = LocalSoftwareKeyboardController.current
    val colores = OutlinedTextFieldDefaults.colors(
        focusedContainerColor = MaterialTheme.colorScheme.surface,
        unfocusedContainerColor = MaterialTheme.colorScheme.surface,
        focusedBorderColor = MaterialTheme.colorScheme.primary,
        unfocusedBorderColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.35f),
    )
    BasicTextField(
        value = valor,
        onValueChange = onCambiar,
        modifier = modifier.height(AlturaBusquedaBrisas),
        singleLine = true,
        keyboardOptions = keyboardOptions.copy(imeAction = ImeAction.Search),
        keyboardActions = KeyboardActions(onSearch = { teclado?.hide() }),
        textStyle = MaterialTheme.typography.bodyLarge.copy(color = MaterialTheme.colorScheme.onSurface),
        cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
        interactionSource = interaccion,
    ) { campo ->
        OutlinedTextFieldDefaults.DecorationBox(
            value = valor,
            innerTextField = campo,
            enabled = true,
            singleLine = true,
            visualTransformation = VisualTransformation.None,
            interactionSource = interaccion,
            placeholder = { Text(ayuda, maxLines = 1, overflow = TextOverflow.Ellipsis) },
            leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
            trailingIcon = if (valor.isEmpty()) {
                null
            } else {
                {
                    Box(
                        modifier = Modifier
                            .size(36.dp)
                            .clip(CircleShape)
                            .clickable { onCambiar("") }
                            .semantics {
                                contentDescription = "Borrar lo escrito"
                                role = Role.Button
                            },
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            Icons.Default.Cancel,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.size(20.dp),
                        )
                    }
                }
            },
            colors = colores,
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
            container = {
                OutlinedTextFieldDefaults.Container(
                    enabled = true,
                    isError = false,
                    interactionSource = interaccion,
                    colors = colores,
                    shape = forma,
                )
            },
        )
    }
}

/** Forma del buscador: la misma que la píldora de búsqueda de abajo
 * ([PildoraBusqueda]), así el campo que se abre parece la misma pieza. */
internal val FormaBusquedaBrisas: Shape get() = FormaPiezaEntera

// Las piezas de abajo (barra de secciones, botón de acción, búsqueda) con
// esquinas suaves y no de cápsula (pedido del usuario 2026-10-29). Una barra
// y su botón (o el campo de búsqueda y su X) van como una sola pieza con un
// corte: redondeadas por fuera, casi rectas donde se juntan, separadas por
// una ranura fina.

/** Radio de las esquinas de afuera de las piezas de abajo. */
internal val RADIO_PIEZA = 18.dp

/** Radio de las esquinas del corte, donde se juntan dos piezas. */
internal val RADIO_CORTE = 6.dp

/** La ranura entre las dos mitades de una pieza cortada. */
internal val ESPACIO_CORTE = 4.dp

internal val FormaPiezaEntera = RoundedCornerShape(RADIO_PIEZA)

/** La mitad izquierda de una pieza cortada: el corte a la derecha. */
internal val FormaPiezaInicio = RoundedCornerShape(
    topStart = RADIO_PIEZA,
    topEnd = RADIO_CORTE,
    bottomEnd = RADIO_CORTE,
    bottomStart = RADIO_PIEZA,
)

/** La mitad derecha de una pieza cortada: el corte a la izquierda. */
internal val FormaPiezaFin = RoundedCornerShape(
    topStart = RADIO_CORTE,
    topEnd = RADIO_PIEZA,
    bottomEnd = RADIO_PIEZA,
    bottomStart = RADIO_CORTE,
)

/** Cuánto tapa abajo la barra de secciones flotante de [PantallaPrincipal]:
 * el contenido pasa por detrás de ella (apenas se ve a través de su velo), así que
 * lo que tiene que quedar a la vista debe dejar ese espacio. Cero fuera de
 * las pestañas (la bandeja de avisos o "Nuevo contratista" no la tienen). */
internal val LocalMargenBarraInferior = compositionLocalOf { 0.dp }

/** Cuántas pantallas a pantalla completa (cámaras, formularios de ingreso)
 * hay a la vista ahora mismo: mientras haya alguna, [PantallaPrincipal]
 * esconde la barra de secciones. Es un contador y no un sí/no porque durante
 * una transición la pantalla que entra y la que sale conviven un instante. */
internal class PantallasCompletas {
    var cantidad by mutableIntStateOf(0)
        private set

    fun abrir() {
        cantidad++
    }

    fun cerrar() {
        cantidad--
    }
}

internal val LocalPantallasCompletas = staticCompositionLocalOf<PantallasCompletas?> { null }

/** Se llama dentro de una pantalla que ocupa todo el lugar (cámara, formulario)
 * para que la barra de secciones flotante se esconda mientras esté a la vista:
 * si no, quedaba encima de la cámara y se podía tocar (bug visto en el
 * teléfono, 2026-10-21). Fuera de las pestañas no hace nada. */
@Composable
internal fun OcultarBarraSecciones() {
    val pantallas = LocalPantallasCompletas.current ?: return
    DisposableEffect(pantallas) {
        pantallas.abrir()
        onDispose { pantallas.cerrar() }
    }
}

/** Espacio libre al final de las listas para que la última fila no quede
 * debajo de la barra de secciones ni de lo que flota con ella (el botón
 * redondo y la píldora de búsqueda). */
internal val EspacioPieLista: Dp
    @Composable get() = 72.dp + LocalMargenBarraInferior.current

/** Al enfocar el último campo de un formulario, lo lleva hasta el fondo (ahí
 * está el botón, pedido del usuario 2026-09-20) y lo sigue mientras el
 * teclado termina de abrirse: el `imePadding()` agranda el formulario cuadro
 * a cuadro y `maxValue` recién tiene el alto final al terminar. Sólo se mueve
 * cuando cambia lo que sobra, y cada cambio retoma la animación en curso.
 * Antes eran 15 `animateScrollTo` en fila, cada uno esperando a que terminara
 * el anterior: varios segundos de desplazamiento peleando con el teclado. */
internal suspend fun ScrollState.seguirFondoMientrasAbreTeclado() {
    withTimeoutOrNull(MS_APERTURA_TECLADO) {
        snapshotFlow { maxValue }.collectLatest { animateScrollTo(it) }
    }
}

/// Lo que tarda el teclado en abrirse, con margen.
private const val MS_APERTURA_TECLADO = 600L

/** Acciones principales: forma y área táctil comunes en todas las pantallas.
 * Dentro de una sección, en su color ([LocalRellenoBotones]). */
@Composable
fun BotonBrisas(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    colors: ButtonColors = ButtonDefaults.buttonColors(
        containerColor = LocalRellenoBotones.current?.relleno ?: ColorRellenoBrisas,
        contentColor = LocalRellenoBotones.current?.sobreRelleno ?: ColorSobreRellenoBrisas,
    ),
    content: @Composable RowScope.() -> Unit,
) {
    Button(
        onClick = onClick,
        modifier = modifier.heightIn(min = AlturaControlBrisas),
        enabled = enabled,
        colors = colors,
        shape = FormaControlBrisas,
        contentPadding = PaddingValues(horizontal = EspacioControlBrisas),
        content = content,
    )
}

/** Acciones de menor jerarquía, con la misma forma y área táctil. */
@Composable
fun BotonDiscretoBrisas(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable RowScope.() -> Unit,
) {
    TextButton(
        onClick = onClick,
        modifier = modifier.heightIn(min = AlturaControlBrisas),
        enabled = enabled,
        shape = FormaControlBrisas,
        contentPadding = PaddingValues(horizontal = EspacioControlBrisas),
        content = content,
    )
}

/** Selector de opciones (control segmentado), con el mismo lenguaje que la
 * barra de secciones de abajo: pista gris, la opción elegida es una pastilla
 * clara que se desliza hasta ella, y su texto va en el color de la sección.
 * Antes la elegida era una píldora sólida de color mientras que abajo la
 * barra marcaba con una píldora gris: dos maneras de decir "elegido". Cada
 * cambio da un toque de vibración, como en iOS. */
@Composable
fun FilaPildoras(
    opciones: List<String>,
    seleccionado: Int,
    onSeleccionar: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    // Una sola pastilla que se desliza hasta la opción elegida (en vez de
    // apagarse en una y prenderse en otra). Se dibuja detrás de la fila, en la
    // fase de dibujo: animar la posición no vuelve a medir ni a componer nada.
    val posicion by animateFloatAsState(
        targetValue = seleccionado.toFloat(),
        animationSpec = spring(dampingRatio = 0.8f, stiffness = Spring.StiffnessMediumLow),
        label = "posicionPildora",
    )
    val oscuro = temaActual().oscuro
    val pista = MaterialTheme.colorScheme.surfaceVariant
    // En oscuro la superficie es más oscura que la pista: ahí la pastilla
    // aclara la pista, como el control de iOS en modo oscuro.
    val colorPastilla = if (oscuro) lerp(pista, MaterialTheme.colorScheme.onSurface, 0.16f) else MaterialTheme.colorScheme.surfaceBright
    val bordePastilla = MaterialTheme.colorScheme.outline.copy(alpha = 0.25f)
    val hapticos = rememberHapticos()
    val separacion = 4.dp
    Row(
        modifier = modifier
            .fillMaxWidth()
            .background(pista, FormaPildoraBrisas)
            .padding(3.dp)
            .drawBehind {
                if (opciones.isEmpty() || seleccionado !in opciones.indices) return@drawBehind
                val hueco = separacion.toPx()
                val ancho = (size.width - hueco * (opciones.size - 1)) / opciones.size
                val radio = CornerRadius((RADIO_PILDORA - 2.dp).toPx())
                val origen = Offset(posicion * (ancho + hueco), 0f)
                drawRoundRect(color = colorPastilla, topLeft = origen, size = Size(ancho, size.height), cornerRadius = radio)
                drawRoundRect(
                    color = bordePastilla,
                    topLeft = origen,
                    size = Size(ancho, size.height),
                    cornerRadius = radio,
                    style = Stroke(width = 0.5.dp.toPx()),
                )
            },
        horizontalArrangement = Arrangement.spacedBy(separacion),
    ) {
        opciones.forEachIndexed { indice, etiqueta ->
            val activo = indice == seleccionado
            val colorTexto by animateColorAsState(
                targetValue = if (activo) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                animationSpec = tween(DURACION_FUNDIDO_PILDORA_MS),
                label = "textoPildora",
            )
            Box(
                modifier = Modifier
                    .weight(1f)
                    .clip(FormaPildoraBrisas)
                    .selectable(
                        selected = activo,
                        role = Role.Tab,
                        onClick = {
                            if (!activo) hapticos(Haptico.SELECCION)
                            onSeleccionar(indice)
                        },
                    )
                    .padding(vertical = 9.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    etiqueta,
                    color = colorTexto,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = if (activo) FontWeight.SemiBold else FontWeight.Medium,
                    maxLines = 1,
                )
            }
        }
    }
}

private const val DURACION_FUNDIDO_PILDORA_MS = 300

/** Avatar circular con inicial, para el header de [PantallaPrincipal]. */
@Composable
fun AvatarBrisas(inicial: String, modifier: Modifier = Modifier, tamano: Dp = 48.dp) {
    Box(
        modifier = modifier.size(tamano).clip(CircleShape).background(MaterialTheme.colorScheme.primary),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            inicial.uppercase(),
            color = MaterialTheme.colorScheme.onPrimary,
            fontWeight = FontWeight.Bold,
            style = MaterialTheme.typography.titleLarge,
        )
    }
}

/// Lado de [BotonIconoCuadradoBrisas]: 36dp y no 40 para que la barra de
/// arriba ocupe menos (pedido del usuario 2026-10-29); sigue sobre el mínimo
/// táctil con el aire alrededor.
internal val TAMANO_BOTON_CUADRADO = 36.dp

/** Botón de icono dentro de un cuadradito blanco redondeado -- reemplaza el
 * `IconButton` plano de Material3 en el header de [PantallaPrincipal]. */
@Composable
fun BotonIconoCuadradoBrisas(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable () -> Unit,
) {
    Box(
        modifier = modifier
            .size(TAMANO_BOTON_CUADRADO)
            .clip(RoundedCornerShape(12.dp))
            .background(MaterialTheme.colorScheme.surface)
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        content()
    }
}

/** Botón cuadrado con borde e ícono en el color de la sección, al lado de un
 * buscador o de un campo: cámara, "nuevo ingreso", etc. Mide lo mismo que
 * [CampoBusquedaBrisas] para quedar parejo con él. Deshabilitado, borde e
 * ícono pasan a `outline`. */
@Composable
internal fun BotonCuadradoBordeBrisas(
    icono: ImageVector,
    descripcion: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    tamano: Dp = AlturaBusquedaBrisas,
) {
    val color = if (enabled) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline
    Box(
        modifier = modifier
            .size(tamano)
            .border(1.dp, color, FormaCampoBrisas)
            .clip(FormaCampoBrisas)
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icono, contentDescription = descripcion, tint = color)
    }
}

/** Botón circular de cerrar/cancelar para las 4 pantallas de escaneo OCR
 * (Cédula, Carnet KOF, Vehículo/Ruta, Comprobante) -- fondo oscuro
 * semitransparente porque flota directo sobre el preview de cámara, no
 * sobre una tarjeta clara del resto de la app. Nació en
 * `PantallaEscanearCedula.kt` (2026-09-19, pedido explícito del usuario:
 * reemplazar el texto "Cancelar" que competía con el mensaje de estado) y se
 * subió acá para que las otras 3 pantallas de escaneo lo compartan en vez de
 * quedarse con el texto plano viejo. */
@Composable
fun BotonCerrarCamara(onClick: () -> Unit, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .size(56.dp)
            .clip(CircleShape)
            .background(Color.Black.copy(alpha = 0.55f))
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(Icons.Default.Close, contentDescription = "Cancelar", tint = Color.White)
    }
}

/** "Llenar a mano" en la cámara de documentos, como píldora con un lápiz
 * (pedido del usuario 2026-10-27: era un texto blanco suelto encima de la X y
 * pasaba desapercibido; con una cédula que no se dejaba leer no encontró cómo
 * escribirla). Mismo fondo que [BotonCerrarCamara] y un borde blanco fino: el
 * fondo de abajo de la cámara ya está oscurecido y sin borde la forma no se
 * distinguía. */
@Composable
fun BotonLlenarAManoCamara(onClick: () -> Unit, modifier: Modifier = Modifier) {
    val forma = RoundedCornerShape(percent = 50)
    Row(
        modifier = modifier
            .height(40.dp)
            .clip(forma)
            .background(Color.Black.copy(alpha = 0.55f))
            .border(1.dp, Color.White.copy(alpha = 0.7f), forma)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Icon(Icons.Default.Edit, contentDescription = null, tint = Color.White, modifier = Modifier.size(18.dp))
        Text("Llenar a mano", color = Color.White, style = MaterialTheme.typography.labelLarge)
    }
}

/** Envuelve una lista con un degradé del color de fondo a transparente en
 * el borde superior -- el contenido se "desvanece" suave debajo del
 * buscador al hacer scroll, en vez de cortar en seco. Un `Modifier.blur()`
 * real sólo pinta en Android 12+ (API 31) -- con `minSdk 26` de esta app,
 * en un dispositivo más viejo simplemente no haría nada; el degradé sí
 * funciona en cualquier versión, y da el mismo efecto visual acá. Usa
 * `contentPadding` (no un `Modifier.padding` externo) en el `LazyColumn`
 * de adentro para que el degradé, no el primer ítem, ocupe ese espacio --
 * si no, el scroll dejaría un hueco sin nada que desvanecer. Sólo en las
 * pantallas ocultas (Rutas, Visitas): las listas de las secciones ya no lo
 * usan desde que la búsqueda bajó (pedido del usuario 2026-10-29). */
@Composable
fun ListaConDesvanecido(contenido: @Composable () -> Unit) {
    Box {
        contenido()
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(16.dp)
                .align(Alignment.TopCenter)
                .background(
                    Brush.verticalGradient(
                        listOf(MaterialTheme.colorScheme.background, Color.Transparent),
                    ),
                ),
        )
    }
}

/// Tarjeta de un resultado de búsqueda de encargado de ruta -- nombre
/// arriba, código de empleado abajo con el número en azul y negrita (pedido
/// explícito del usuario, 2026-09-20: "darle más protagonismo", sólo al
/// número, no a la etiqueta "Código de empleado:"). Compartida entre el
/// paso 1 de Rutas (`PantallaRutas.kt`) y Gafetes Provisionales
/// (`PantallaGafetesProvisionales.kt`) -- mismo catálogo `encargados_ruta`,
/// mismo look pedido explícitamente ("usa el mismo buscador y animación que
/// tiene KOF"), una sola fuente de verdad en vez de dos copias que puedan
/// desalinearse.
@Composable
fun FilaEncargadoRuta(encargado: EncargadoRuta, onClick: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(textoResaltado(encargado.nombre), style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
        Row {
            Text(
                "Código de empleado: ",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                encargado.codigoEmpleado,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.primary,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

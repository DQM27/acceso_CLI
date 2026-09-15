package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonColors
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextFieldColors
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** Radio de los campos "filled" (buscadores, login) -- más redondeado que
 * [FormaControlBrisas], estilo cápsula, siguiendo el mockup "estilo Kash". */
internal val FormaCampoBrisas = RoundedCornerShape(28.dp)

/** Forma de las píldoras de selector ([FilaPildoras]) y del cuadradito de
 * los botones de icono del header -- redondeo total, cápsula. */
internal val FormaPildoraBrisas = RoundedCornerShape(percent = 50)

/** Colores compartidos para un campo de texto "filled" sin borde visible
 * (buscadores, login) -- mismo fondo blanco que las tarjetas, en vez del
 * `OutlinedTextField` con borde que traía Material3 por defecto. */
@Composable
internal fun ColoresCampoBrisas(): TextFieldColors = TextFieldDefaults.colors(
    focusedContainerColor = MaterialTheme.colorScheme.surface,
    unfocusedContainerColor = MaterialTheme.colorScheme.surface,
    disabledContainerColor = MaterialTheme.colorScheme.surface,
    focusedIndicatorColor = Color.Transparent,
    unfocusedIndicatorColor = Color.Transparent,
    disabledIndicatorColor = Color.Transparent,
)

/** Acciones principales: forma y área táctil comunes en todas las pantallas. */
@Composable
fun BotonBrisas(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    colors: ButtonColors = ButtonDefaults.buttonColors(
        containerColor = ColorRellenoBrisas,
        contentColor = ColorSobreRellenoBrisas,
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

/** Selector de opciones en formato píldora ("estilo Kash"): pista gris clara
 * de fondo, la opción activa se pinta como una píldora sólida en el color
 * primario -- reemplaza el look de `SegmentedButton` de Material3 (gris,
 * rectangular) en las pantallas que ya lo usaban (`SelectorModoBusqueda` en
 * [PantallaActivos], las pestañas de [PantallaPrincipal]). No reemplaza a
 * `SegmentedButton` en todos lados a propósito -- sólo donde el mockup lo pide. */
@Composable
fun FilaPildoras(
    opciones: List<String>,
    seleccionado: Int,
    onSeleccionar: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .background(MaterialTheme.colorScheme.surfaceVariant, FormaPildoraBrisas)
            .padding(4.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        opciones.forEachIndexed { indice, etiqueta ->
            val activo = indice == seleccionado
            Box(
                modifier = Modifier
                    .weight(1f)
                    .clip(FormaPildoraBrisas)
                    .background(if (activo) MaterialTheme.colorScheme.primary else Color.Transparent)
                    .clickable { onSeleccionar(indice) }
                    .padding(vertical = 10.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    etiqueta,
                    color = if (activo) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
                    fontWeight = if (activo) FontWeight.SemiBold else FontWeight.Normal,
                    maxLines = 1,
                )
            }
        }
    }
}

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
            .size(40.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(MaterialTheme.colorScheme.surface)
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        content()
    }
}

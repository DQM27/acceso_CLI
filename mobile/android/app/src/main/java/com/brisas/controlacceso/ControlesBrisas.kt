package com.brisas.controlacceso

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.heightIn
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonColors
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

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

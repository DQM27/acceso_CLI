package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.material.icons.filled.Close
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import uniffi.control_acceso_mobile.EmpresaProveedor
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

/** Forma de las píldoras de selector ([FilaPildoras]) -- antes cápsula
 * completa (percent=50), se veía más redondeada que el mockup, que usa
 * esquinas bien marcadas pero no un óvalo. */
internal val FormaPildoraBrisas = RoundedCornerShape(12.dp)

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

/** Envuelve una lista con un degradé del color de fondo a transparente en
 * el borde superior -- el contenido se "desvanece" suave debajo del
 * buscador al hacer scroll, en vez de cortar en seco. Un `Modifier.blur()`
 * real sólo pinta en Android 12+ (API 31) -- con `minSdk 26` de esta app,
 * en un dispositivo más viejo simplemente no haría nada; el degradé sí
 * funciona en cualquier versión, y da el mismo efecto visual acá. Usa
 * `contentPadding` (no un `Modifier.padding` externo) en el `LazyColumn`
 * de adentro para que el degradé, no el primer ítem, ocupe ese espacio --
 * si no, el scroll dejaría un hueco sin nada que desvanecer. Compartido
 * entre [PantallaActivos] y [PantallaHistorial] (2026-09-15). */
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
            .size(44.dp)
            .clip(CircleShape)
            .background(Color.Black.copy(alpha = 0.55f))
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(Icons.Default.Close, contentDescription = "Cancelar", tint = Color.White)
    }
}

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
        Text(encargado.nombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
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

/// Tarjeta de un resultado de búsqueda de empresa proveedora -- mismo look
/// que [FilaEncargadoRuta] (una sola fuente de verdad para "resultado de
/// búsqueda tocable"), pero sin segunda línea: `EmpresaProveedor` sólo tiene
/// nombre, no hay un segundo dato que mostrarle protagonismo. Reemplaza al
/// `ExposedDropdownMenuBox`/`DropdownMenu` que tenía antes
/// `PasoEmpresaProveedora` (`PantallaProveedores.kt`) -- mismo bug que ya se
/// arregló acá para Rutas y Gafetes Provisionales: el popup de
/// `DropdownMenu` compite por el foco con el `TextField` en cada
/// recomposición del anclaje y cerraba el teclado con cada letra tipeada
/// (reportado en pruebas reales, 2026-09-21).
@Composable
fun FilaEmpresaProveedor(empresa: EmpresaProveedor, onClick: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .background(MaterialTheme.colorScheme.surface)
            .clickable(onClick = onClick)
            .padding(16.dp),
    ) {
        Text(empresa.nombre, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
    }
}

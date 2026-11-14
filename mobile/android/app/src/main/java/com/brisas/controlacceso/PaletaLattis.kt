package com.brisas.controlacceso

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

// Todos los colores de la app, en un solo lugar (pedido del usuario
// 2026-10-27): para cambiar un tono se toca sólo este archivo. Lo demás lee
// de acá: el esquema de Material ([EsquemaDia], [EsquemaNoche]), los acentos
// de cada sección (`AcentoSeccion.kt`) y los colores sueltos de `DisenoMovil.kt`.
//
// Cada sección tiene dos tonos ([TonoSeccion]): el acento (texto, íconos,
// pestaña activa) y el relleno de los botones, siempre con texto blanco. En el
// tema claro son el mismo; en el oscuro el acento es más claro, para leerse
// sobre el fondo, y el relleno sigue profundo: antes los botones se rellenaban
// con el acento claro y quedaban como bloques fluorescentes.
//
// Contrastes medidos (WCAG) en `PaletaLattisTest` y `AcentoSeccionTest`.

/// Los dos tonos de una sección (ver arriba).
@Immutable
internal class TonoSeccion(
    /// Texto, íconos y pestaña activa.
    val acento: Color,
    /// Relleno de los botones principales, con [Paleta.sobreRelleno] encima.
    val relleno: Color,
)

@Immutable
internal class Paleta(
    /// Detrás de todo.
    val fondo: Color,
    /// Tarjetas, hojas, barra de secciones (`surface`).
    val tarjeta: Color,
    /// Campos y chips sobre una tarjeta (`surfaceVariant`).
    val variante: Color,
    /// Lo más elevado: diálogos, menús (`surfaceBright`, `surfaceContainerHigh`).
    val realce: Color,
    val texto: Color,
    val textoSecundario: Color,
    val borde: Color,
    val bordeSuave: Color,
    /// Contorno de la barra de secciones, la píldora de búsqueda y el botón
    /// redondo, de [ANCHO_BORDE_VIDRIO].
    val bordeVidrio: Color,
    /// Acento general, fuera de las secciones (login, avisos, ayuda).
    val acento: Color,
    val acentoSuave: Color,
    /// El acento del otro tema (`inversePrimary`).
    val acentoInverso: Color,
    /// Texto sobre un acento claro (`onPrimary`, `onError`...).
    val sobreAcento: Color,
    /// Relleno de los botones fuera de las secciones.
    val relleno: Color,
    /// Texto sobre cualquier relleno: el de los botones y el aviso de éxito.
    val sobreRelleno: Color,
    val info: Color,
    val infoSuave: Color,
    val error: Color,
    val errorSuave: Color,
    /// Texto e íconos de "confirmado".
    val exito: Color,
    /// Relleno de los avisos que bajan de arriba, uno por tipo, con
    /// [sobreRelleno] encima.
    val rellenoExito: Color,
    val rellenoInfo: Color,
    val rellenoError: Color,
    /// Ámbar de íconos y puntos (sin red).
    val advertencia: Color,
    val textoAdvertencia: Color,
    /// Fondo de una franja de advertencia ([TipoEstado.AVISO]), con
    /// [textoAdvertencia] encima: antes compartía el rojo del error y un
    /// "se reintenta solo" se leía igual que un fallo.
    val advertenciaSuave: Color,
    /// Relleno del aviso breve de advertencia, con [sobreRelleno] encima.
    val rellenoAdvertencia: Color,
    /// La tarjeta de quien ya está adentro en la búsqueda de contratistas:
    /// fondo ámbar suave, franja a la izquierda y la palabra "ADENTRO". Ámbar
    /// como la señal de [AmarilloSenal], que ya dice eso mismo.
    val adentroSuave: Color,
    val franjaAdentro: Color,
    val textoAdentro: Color,
    /// Las tarjetas de una búsqueda por gafete, que siempre es para la
    /// salida: azul, el acento general, distinto del ámbar de "adentro".
    val salidaSuave: Color,
    val franjaSalida: Color,
    /// La tarjeta de quien no puede entrar (acceso denegado o PRAIND
    /// vencido) en la búsqueda: el mismo rojo para los dos, el ícono y la
    /// palabra dicen el motivo.
    val bloqueadoSuave: Color,
    val franjaBloqueado: Color,
    val textoBloqueado: Color,
    val contratistas: TonoSeccion,
    val kof: TonoSeccion,
    val proveedores: TonoSeccion,
    val porCorreo: TonoSeccion,
) {
    val tinteAdentro = TinteTarjeta(adentroSuave, franjaAdentro)
    val tinteSalida = TinteTarjeta(salidaSuave, franjaSalida)
    val tinteBloqueado = TinteTarjeta(bloqueadoSuave, franjaBloqueado)
}

/// Fondo y franja izquierda de una tarjeta que se distingue de las demás.
@Immutable
internal class TinteTarjeta(val fondo: Color, val franja: Color)

/// Claro neutro de alto contraste (2026-11-14, `docs/handoff-rediseno-movil-ux.md`,
/// fase 1): reemplaza Tokyo Day, cuya tarjeta casi no se separaba del fondo
/// (1,13:1) y cuyo texto secundario quedaba en 3,7:1 sobre los campos. Se
/// lee al sol en la portería: tarjetas blancas sobre un gris muy claro,
/// texto casi negro, un solo azul para lo que se toca.
internal val PaletaDia = Paleta(
    fondo = Color(0xFFF2F4F7),
    tarjeta = Color(0xFFFFFFFF),
    variante = Color(0xFFE8ECF2),
    realce = Color(0xFFFFFFFF),
    texto = Color(0xFF111827),
    textoSecundario = Color(0xFF4B5563),
    borde = Color(0xFF667085),
    bordeSuave = Color(0xFFD0D5DD),
    bordeVidrio = Color(0xFF667085).copy(alpha = 0.25f),
    acento = Color(0xFF1D4ED8),
    acentoSuave = Color(0xFFE0E7FF),
    acentoInverso = Color(0xFF6EA8FF),
    sobreAcento = Color(0xFFFFFFFF),
    relleno = Color(0xFF1D4ED8),
    sobreRelleno = Color(0xFFFFFFFF),
    info = Color(0xFF0369A1),
    infoSuave = Color(0xFFE0F2FE),
    error = Color(0xFFB42318),
    errorSuave = Color(0xFFFEE4E2),
    exito = Color(0xFF15803D),
    rellenoExito = Color(0xFF15803D),
    rellenoInfo = Color(0xFF1D4ED8),
    rellenoError = Color(0xFFB42318),
    // Ícono o punto, no texto: 3,2:1 sobre la tarjeta.
    advertencia = Color(0xFFD97706),
    textoAdvertencia = Color(0xFF92400E),
    advertenciaSuave = Color(0xFFFEF3C7),
    rellenoAdvertencia = Color(0xFFB45309),
    adentroSuave = Color(0xFFFFF1C2),
    franjaAdentro = Color(0xFFB45309),
    textoAdentro = Color(0xFF92400E),
    salidaSuave = Color(0xFFDBEAFE),
    franjaSalida = Color(0xFF1D4ED8),
    bloqueadoSuave = Color(0xFFFEE4E2),
    franjaBloqueado = Color(0xFFB42318),
    textoBloqueado = Color(0xFFB42318),
    contratistas = TonoSeccion(acento = Color(0xFF00806A), relleno = Color(0xFF00806A)),
    kof = TonoSeccion(acento = Color(0xFFC4161C), relleno = Color(0xFFC4161C)),
    proveedores = TonoSeccion(acento = Color(0xFFB85C00), relleno = Color(0xFFB85C00)),
    porCorreo = TonoSeccion(acento = Color(0xFF6D32C9), relleno = Color(0xFF6D32C9)),
)

/// Oscuro neutro, sin el tinte lavanda de Tokyo Night (2026-10-27): casi
/// negro para la pantalla AMOLED, tres niveles con saltos de ~1,2:1 como iOS
/// (fondo, tarjeta, realce) y acentos desaturados.
internal val PaletaNoche = Paleta(
    fondo = Color(0xFF08080A),
    tarjeta = Color(0xFF1F1F24),
    variante = Color(0xFF2E2E35),
    realce = Color(0xFF2E2E35),
    texto = Color(0xFFF2F2F5),
    textoSecundario = Color(0xFFA1A1AA),
    borde = Color(0xFF48484F),
    bordeSuave = Color(0xFF2E2E35),
    bordeVidrio = Color(0xFFFFFFFF).copy(alpha = 0.22f),
    acento = Color(0xFF6EA8FF),
    acentoSuave = Color(0xFF1E2A40),
    acentoInverso = Color(0xFF2959AA),
    sobreAcento = Color(0xFF08080A),
    relleno = Color(0xFF2F5FD0),
    sobreRelleno = Color(0xFFFFFFFF),
    info = Color(0xFF6CC5E6),
    infoSuave = Color(0xFF1B2A33),
    error = Color(0xFFFF6B6B),
    errorSuave = Color(0xFF3A1F22),
    exito = Color(0xFF4ADE80),
    rellenoExito = Color(0xFF1F7A44),
    rellenoInfo = Color(0xFF2F5FD0),
    rellenoError = Color(0xFFC53030),
    advertencia = Color(0xFFFBBF24),
    textoAdvertencia = Color(0xFFFBBF24),
    advertenciaSuave = Color(0xFF3A2E10),
    rellenoAdvertencia = Color(0xFFB45309),
    adentroSuave = Color(0xFF2E2610),
    franjaAdentro = Color(0xFFFBBF24),
    textoAdentro = Color(0xFFFBBF24),
    salidaSuave = Color(0xFF16233A),
    franjaSalida = Color(0xFF6EA8FF),
    bloqueadoSuave = Color(0xFF331A1D),
    franjaBloqueado = Color(0xFFFF6B6B),
    textoBloqueado = Color(0xFFFF6B6B),
    contratistas = TonoSeccion(acento = Color(0xFF5CC8B2), relleno = Color(0xFF00806A)),
    kof = TonoSeccion(acento = Color(0xFFFF6B5E), relleno = Color(0xFFD0312D)),
    proveedores = TonoSeccion(acento = Color(0xFFFFB547), relleno = Color(0xFFB85C00)),
    porCorreo = TonoSeccion(acento = Color(0xFFB79CFF), relleno = Color(0xFF7C45D6)),
)

/// Ancho del contorno [Paleta.bordeVidrio].
internal val ANCHO_BORDE_VIDRIO = 1.dp

/// La paleta del tema elegido en la app ([temaActual]), no sólo el del sistema.
internal val paletaActual: Paleta
    @Composable get() = if (temaActual().oscuro) PaletaNoche else PaletaDia

internal val EsquemaDia: ColorScheme = PaletaDia.esquema(oscuro = false)
internal val EsquemaNoche: ColorScheme = PaletaNoche.esquema(oscuro = true)

/// Cada rol de Material sale de un color de la paleta.
private fun Paleta.esquema(oscuro: Boolean): ColorScheme {
    val base = if (oscuro) darkColorScheme() else lightColorScheme()
    return base.copy(
        primary = acento,
        onPrimary = sobreAcento,
        primaryContainer = acentoSuave,
        onPrimaryContainer = acento,
        inversePrimary = acentoInverso,
        secondary = acento,
        onSecondary = sobreAcento,
        secondaryContainer = acentoSuave,
        onSecondaryContainer = acento,
        tertiary = info,
        onTertiary = sobreAcento,
        tertiaryContainer = infoSuave,
        onTertiaryContainer = info,
        background = fondo,
        onBackground = texto,
        surface = tarjeta,
        onSurface = texto,
        surfaceVariant = variante,
        onSurfaceVariant = textoSecundario,
        surfaceTint = acento,
        inverseSurface = texto,
        inverseOnSurface = tarjeta,
        error = error,
        onError = sobreAcento,
        errorContainer = errorSuave,
        onErrorContainer = error,
        outline = borde,
        outlineVariant = bordeSuave,
        scrim = fondo,
        surfaceBright = realce,
        surfaceDim = fondo,
        surfaceContainer = tarjeta,
        surfaceContainerHigh = realce,
        surfaceContainerHighest = realce,
        surfaceContainerLow = tarjeta,
        surfaceContainerLowest = fondo,
    )
}

// Colores con significado fijo, iguales en los dos temas.

/// Marco y mensaje de las 4 pantallas de escaneo (identidad/gafete, vehículo
/// de ruta, comprobante de ruta, carnet KOF): el color comunica el estado
/// (buscando, inválido, confirmado, vencido) y debe leerse igual con
/// cualquier tema, sobre la imagen de la cámara.
val ColorEscaneoBuscando = Color(0xFF9E9E9E)
val ColorEscaneoInvalido = Color(0xFFE53935)
val ColorEscaneoConfirmado = Color(0xFF43A047)
val ColorEscaneoVencido = Color(0xFFFF8F00)

/// La señal de tránsito de "ya está adentro" (`IconoPaso.kt`).
internal val AmarilloSenal = Color(0xFFFFC20E)
internal val NegroSenal = Color(0xFF1F1F1F)

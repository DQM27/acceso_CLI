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
// de acá: el esquema de Material ([EsquemaDia], [EsquemaNoche]), el color de
// cada sección (`AcentoSeccion.kt`) y los colores sueltos de `DisenoMovil.kt`.
//
// Un color, un significado (2026-11-14, `docs/handoff-rediseno-movil-ux.md`):
// el azul ([Paleta.acento], [Paleta.relleno]) es lo que se toca; verde, ámbar
// y rojo dicen qué pasó; el color de sección sólo dice dónde se está, en su
// pestaña de abajo. Antes cada sección pintaba también sus botones y un
// "Entregar" rojo en KOF se leía como un error.
//
// Contrastes medidos (WCAG) en `PaletaLattisTest` y `AcentoSeccionTest`.

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
    /// Las tarjetas que al tocarlas abren la salida: la búsqueda por gafete
    /// y quien ya está adentro en la búsqueda de contratistas (antes ésta
    /// iba en ámbar, que se leía como advertencia). Azul, el de lo que se toca.
    val salidaSuave: Color,
    val franjaSalida: Color,
    /// La tarjeta de quien no puede entrar (acceso denegado o PRAIND
    /// vencido) en la búsqueda: el mismo rojo para los dos, el ícono y la
    /// palabra dicen el motivo.
    val bloqueadoSuave: Color,
    val franjaBloqueado: Color,
    val textoBloqueado: Color,
    /// El color de cada sección: ícono y nombre de su pestaña activa, y nada
    /// más. A 4,5:1 sobre la píldora gris de la barra (la etiqueta es de 12sp).
    val contratistas: Color,
    val kof: Color,
    val proveedores: Color,
    val porCorreo: Color,
) {
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
    rellenoError = Color(0xFFB42318),
    // Ícono o punto, no texto: 3,2:1 sobre la tarjeta.
    advertencia = Color(0xFFD97706),
    textoAdvertencia = Color(0xFF92400E),
    advertenciaSuave = Color(0xFFFEF3C7),
    rellenoAdvertencia = Color(0xFFB45309),
    salidaSuave = Color(0xFFDBEAFE),
    franjaSalida = Color(0xFF1D4ED8),
    bloqueadoSuave = Color(0xFFFEE4E2),
    franjaBloqueado = Color(0xFFB42318),
    textoBloqueado = Color(0xFFB42318),
    // Verde azulado y mango más oscuros que antes (#00806A, #B85C00): sobre la
    // píldora gris de la pestaña daban 4,2:1 y 4,0:1.
    contratistas = Color(0xFF00735F),
    kof = Color(0xFFC4161C),
    proveedores = Color(0xFFA35200),
    porCorreo = Color(0xFF6D32C9),
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
    rellenoError = Color(0xFFC53030),
    advertencia = Color(0xFFFBBF24),
    textoAdvertencia = Color(0xFFFBBF24),
    advertenciaSuave = Color(0xFF3A2E10),
    rellenoAdvertencia = Color(0xFFB45309),
    salidaSuave = Color(0xFF16233A),
    franjaSalida = Color(0xFF6EA8FF),
    bloqueadoSuave = Color(0xFF331A1D),
    franjaBloqueado = Color(0xFFFF6B6B),
    textoBloqueado = Color(0xFFFF6B6B),
    contratistas = Color(0xFF5CC8B2),
    // Más claro que #FF6B5E: sobre la píldora gris daba 4,2:1.
    kof = Color(0xFFFF8A7A),
    proveedores = Color(0xFFFFB547),
    porCorreo = Color(0xFFB79CFF),
)

/// Contorno de las piezas de abajo (barra de secciones, píldora de búsqueda,
/// botón de cerrar la búsqueda), en [Paleta.bordeSuave]: opacas desde
/// 2026-11-14 (fase 3 de `docs/handoff-rediseno-movil-ux.md`); el velo al
/// 94 % no dejaba ver nada útil de lo que pasaba detrás.
internal val ANCHO_BORDE_PIEZA = 1.dp

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

/// La señal de tránsito de la hoja de salida cuando se escaneó a alguien que
/// ya estaba adentro y quizá se esperaba darle el ingreso (`IconoPaso.kt`).
internal val AmarilloSenal = Color(0xFFFFC20E)
internal val NegroSenal = Color(0xFF1F1F1F)

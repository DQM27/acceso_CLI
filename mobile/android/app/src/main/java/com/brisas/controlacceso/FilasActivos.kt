package com.brisas.controlacceso

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import uniffi.control_acceso_mobile.AvisoAcceso
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.IngresoRemoto
import uniffi.control_acceso_mobile.TipoIngreso

// Filas de la lista de Activos (sacado de `PantallaActivos.kt`, punto M6
// de la auditoría móvil: sólo se movió código).

@Composable
internal fun FilaActivo(fila: FilaActiva, onClick: () -> Unit, tinte: TinteTarjeta? = null) {
    when (fila) {
        is FilaActiva.Local -> FilaActivoLocal(fila.activo, onClick, tinte)
        is FilaActiva.Remota -> FilaActivoRemota(fila.remoto, onClick, tinte)
    }
}

@Composable
private fun FilaActivoLocal(activo: IngresoActivoResumen, onClick: () -> Unit, tinte: TinteTarjeta?) {
    TarjetaActivo(
        nombre = activo.contratistaNombre,
        detalle = "${activo.cedula} · ${activo.empresaNombre}",
        gafeteNumero = activo.gafeteNumero,
        fechaHoraIngreso = activo.fechaHoraIngreso,
        otroEquipo = false,
        onClick = onClick,
        tinte = tinte,
    )
}

/// Ver el doc-comment de [FilaActiva] -- un ingreso abierto por el otro
/// dispositivo del sitio (la PC del puesto de control), cacheado en
/// `ingresos_remotos`. Cédula, empresa y gafete sí viajan en la caché desde
/// la migración 24, así que no hay motivo para mostrar menos acá.
@Composable
private fun FilaActivoRemota(remoto: IngresoRemoto, onClick: () -> Unit, tinte: TinteTarjeta?) {
    TarjetaActivo(
        nombre = remoto.contratistaNombre,
        detalle = "${remoto.contratistaCedula ?: "—"} · ${remoto.empresaNombre ?: "—"}",
        gafeteNumero = remoto.gafeteNumero,
        fechaHoraIngreso = remoto.horaEntrada,
        otroEquipo = true,
        onClick = onClick,
        tinte = tinte,
    )
}

/// Tarjeta de quien está adentro (tocar = salida). La comparten
/// contratistas, proveedores, por correo, visitas y los gafetes de KOF para
/// que se vean y se usen igual (pedido del usuario 2026-10-06).
///
/// Sólo lo esencial (rediseño 2026-10-27, "menos es más"), en dos renglones:
/// a la izquierda el nombre y, en gris, la cédula y la empresa; a la derecha
/// el gafete en un cuadro redondeado (lo que se compara en la salida) y
/// debajo la hora, todas alineadas en columna para leerlas de un vistazo (con
/// el día sólo si fue otro día), con la nube si entró por otro equipo. Quién
/// dio el ingreso quedó para el historial del escritorio y el panel.
///
/// Con [tinte], fondo y franja propios (la búsqueda por gafete la pinta de
/// azul de salida).
@Composable
internal fun TarjetaActivo(
    nombre: String,
    detalle: String,
    gafeteNumero: Long?,
    fechaHoraIngreso: String,
    otroEquipo: Boolean,
    onClick: () -> Unit,
    tinte: TinteTarjeta? = null,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .fondoTarjeta(tinte)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                textoResaltado(nombre),
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Medium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            // Dentro de una búsqueda, lo buscado (una cédula, por ejemplo) se
            // subraya en el color de la sección.
            //
            // Los textos se recuerdan por valor: cada refresco de la lista
            // trae objetos nuevos del núcleo (sus campos son `var`, así que
            // Compose no los da por iguales) y sin esto cada fila visible
            // volvía a armar la línea y a leer y formatear la fecha.
            val consulta = LocalConsultaResaltada.current
            val acento = MaterialTheme.colorScheme.primary
            val lineaDetalle = remember(detalle, consulta, acento) {
                val estiloCoincidencia = SpanStyle(color = acento, textDecoration = TextDecoration.Underline)
                buildAnnotatedString { agregarResaltado(detalle, consulta, estiloCoincidencia) }
            }
            Text(
                lineaDetalle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        Spacer(Modifier.width(12.dp))
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            CuadroGafete(gafeteNumero)
            val hora = remember(fechaHoraIngreso) { horaIngreso(fechaHoraIngreso) }
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (otroEquipo) {
                    IconoOtroEquipo(Modifier.size(13.dp))
                    Spacer(Modifier.width(4.dp))
                }
                Text(
                    hora.texto,
                    style = MaterialTheme.typography.labelMedium,
                    color = if (hora.deOtroDia) ColorTextoAdvertenciaBrisas else MaterialTheme.colorScheme.onSurface,
                    fontWeight = if (hora.deOtroDia) FontWeight.Bold else FontWeight.Medium,
                )
            }
        }
    }
}

/// El número de gafete en un cuadro de esquinas redondeadas (pedido del
/// usuario 2026-10-27: cuadrado, no píldora), en el color de la sección;
/// "S/G" en gris si entró sin gafete.
@Composable
private fun CuadroGafete(gafeteNumero: Long?) {
    val conGafete = gafeteNumero != null
    Box(
        modifier = Modifier
            .defaultMinSize(minWidth = 44.dp, minHeight = 30.dp)
            .clip(RoundedCornerShape(8.dp))
            .background(if (conGafete) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant)
            .padding(horizontal = 8.dp)
            .semantics(mergeDescendants = true) { contentDescription = if (conGafete) "Gafete $gafeteNumero" else "Sin gafete" },
        contentAlignment = Alignment.Center,
    ) {
        Text(
            gafeteNumero?.toString() ?: "S/G",
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.Bold,
            color = if (conGafete) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
}

/// La marca de "entró por otro equipo de la unidad": una nube, la misma en
/// todas las listas (contratistas, visitas, por correo, proveedores y
/// gafetes provisionales), nunca texto. Antes era una computadora aunque el
/// ingreso viniera de otro teléfono; lo que importa es que llegó de otro lado
/// (y que cerrarlo pide conexión).
@Composable
internal fun IconoOtroEquipo(modifier: Modifier = Modifier.size(18.dp)) {
    Icon(
        IconosLucide.Nube,
        contentDescription = "Registrado en otro equipo",
        tint = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = modifier,
    )
}

/// Cómo se marca una tarjeta de la búsqueda de contratistas, si hace falta
/// (ver [marcaContratista]).
internal enum class MarcaContratista(val palabra: String) {
    ADENTRO("ADENTRO"),
    DENEGADO("DENEGADO"),
    VENCIDO("VENCIDO"),
}

/// Adentro gana: con esa persona lo que se hace es la salida, que siempre se
/// permite. Entre denegado y vencido ya decidió el núcleo (un solo aviso).
internal fun marcaContratista(contratista: ContratistaResumen): MarcaContratista? = when {
    contratista.tieneIngresoActivo -> MarcaContratista.ADENTRO
    contratista.tipoAvisoAcceso == AvisoAcceso.ACCESO_DENEGADO -> MarcaContratista.DENEGADO
    contratista.tipoAvisoAcceso == AvisoAcceso.PRAIND_VENCIDO -> MarcaContratista.VENCIDO
    else -> null
}

/// En la búsqueda salen juntos quienes pueden entrar, quienes ya están
/// adentro y quienes no pueden entrar. Se distinguen sin leer (antes era una
/// línea de texto al final), con fondo, franja, ícono y una palabra:
/// adentro en ámbar con la señal de "ya está adentro"; acceso denegado y
/// PRAIND vencido en rojo, con el "prohibido" o el calendario con una X.
@Composable
internal fun FilaContratista(contratista: ContratistaResumen, onClick: () -> Unit) {
    val marca = marcaContratista(contratista)
    val adentro = marca == MarcaContratista.ADENTRO
    val paleta = paletaActual
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .fondoTarjeta(
                when (marca) {
                    MarcaContratista.ADENTRO -> paleta.tinteAdentro
                    MarcaContratista.DENEGADO, MarcaContratista.VENCIDO -> paleta.tinteBloqueado
                    null -> null
                },
            )
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                textoResaltado(contratista.nombre),
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Medium,
            )
            Text(
                textoResaltado("${contratista.cedula} · ${contratista.empresaNombre} · ${etiquetaTipoIngreso(contratista.tipoIngreso)}"),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            // Texto y regla del núcleo (`ContratistaResumen.avisoAcceso`), con
            // su reloj. Sin estar adentro, lo dicen el ícono y la palabra de
            // la derecha; adentro queda como línea chica, para que se sepa que
            // no podrá volver a entrar.
            val aviso = contratista.avisoAcceso
            if (aviso != null && (adentro || marca == null)) {
                Text(
                    aviso,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    fontWeight = FontWeight.Bold,
                )
            }
            if (adentro) {
                Text(
                    "Toque para registrar su salida",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        if (marca != null) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                when (marca) {
                    MarcaContratista.ADENTRO -> SenalAdentro()
                    MarcaContratista.DENEGADO -> IconoMarca(IconosLucide.Prohibido, paleta.textoBloqueado)
                    MarcaContratista.VENCIDO -> IconoMarca(IconosLucide.CalendarioConX, paleta.textoBloqueado)
                }
                Text(
                    marca.palabra,
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = FontWeight.Bold,
                    color = if (adentro) paleta.textoAdentro else paleta.textoBloqueado,
                )
            }
        }
    }
}

/// Ícono de la marca de una tarjeta, del alto de la señal de "adentro".
@Composable
private fun IconoMarca(icono: ImageVector, color: Color) {
    Icon(icono, contentDescription = null, tint = color, modifier = Modifier.size(24.dp))
}

/// El fondo de siempre (`surface`) o, con [tinte], el suyo con la franja a
/// la izquierda.
@Composable
private fun Modifier.fondoTarjeta(tinte: TinteTarjeta?): Modifier =
    if (tinte == null) {
        background(MaterialTheme.colorScheme.surface)
    } else {
        background(tinte.fondo).drawBehind {
            drawRect(tinte.franja, size = Size(ANCHO_FRANJA_TARJETA.toPx(), size.height))
        }
    }

private val ANCHO_FRANJA_TARJETA = 4.dp

private fun etiquetaTipoIngreso(tipo: TipoIngreso): String =
    when (tipo) {
        TipoIngreso.PRAIND -> "PRAIND"
        TipoIngreso.IN_HOUSE -> "IN HOUSE"
        TipoIngreso.POR_CORREO -> "Por correo"
        TipoIngreso.SWAT -> "SWAT"
    }

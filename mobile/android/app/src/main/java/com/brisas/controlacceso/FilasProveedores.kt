package com.brisas.controlacceso

import androidx.compose.runtime.Composable
import uniffi.control_acceso_mobile.IngresoProveedorRemoto
import uniffi.control_acceso_mobile.RegistroIngresoProveedorActivoResumen

// Filas de proveedores activos y diálogo de salida (sacado de
// `PantallaProveedores.kt`, punto M6 de la auditoría móvil: sólo se movió código).

@Composable
internal fun FilaProveedorActivo(fila: FilaProveedorActiva, onConfirmarSalida: () -> Unit) {
    when (fila) {
        is FilaProveedorActiva.Local -> FilaProveedorActivoLocal(fila.registro, onConfirmarSalida)
        is FilaProveedorActiva.Remota -> FilaProveedorActivoRemota(fila.remoto, onConfirmarSalida)
    }
}

/// Mismo orden de campos que `FilaActivoLocal`/`FilaActivoRemota`
/// (PantallaActivos.kt, contratista) -- nombre primero (lo más importante:
/// quién es), luego identidad + afiliación + gafete, luego cuándo entró.
/// Antes esta tarjeta abría con "Gafete N" en vez del nombre -- pedido
/// explícito del usuario en pruebas reales, 2026-09-17: unificar el orden
/// de importancia entre las dos pantallas.
///
/// Mayúscula + negrita + gafete en azul, y "dio ingreso" en la última
/// línea -- mismo tratamiento que `FilaActivoLocal` (contratista), pedido
/// explícito del usuario 2026-09-20 para unificar las dos tarjetas.
@Composable
private fun FilaProveedorActivoLocal(
    registro: RegistroIngresoProveedorActivoResumen,
    onConfirmarSalida: () -> Unit,
) {
    TarjetaActivo(
        nombre = registro.nombre,
        detalle = "${registro.cedula} · ${registro.empresaNombre}" + (registro.placa?.let { " · $it" } ?: ""),
        gafeteNumero = registro.gafeteNumero,
        fechaHoraIngreso = registro.fechaHoraIngreso,
        dioIngreso = registro.usuarioIngresoNombre,
        otroEquipo = false,
        onClick = onConfirmarSalida,
    )
}

/// Ver el doc-comment de [FilaProveedorActiva] -- un ingreso abierto por
/// OTRO dispositivo del sitio, sin `id` local (sólo `uuid` de la nube).
@Composable
private fun FilaProveedorActivoRemota(remoto: IngresoProveedorRemoto, onConfirmarSalida: () -> Unit) {
    TarjetaActivo(
        nombre = remoto.nombre,
        detalle = "${remoto.cedula} · ${remoto.empresaNombre}" + (remoto.placa?.let { " · $it" } ?: ""),
        gafeteNumero = remoto.gafeteNumero,
        fechaHoraIngreso = remoto.horaEntrada,
        dioIngreso = remoto.usuarioEntradaNombre,
        otroEquipo = true,
        onClick = onConfirmarSalida,
    )
}

/// Mismo layout e información que `DialogoConfirmarSalida` de
/// PantallaActivos.kt (contratista) -- ícono circular, título "Registrar
/// salida", y nombre · cédula · empresa (o "registrado en otro dispositivo"
/// para una fila remota) en vez de sólo "Gafete N · nombre". Pedido
/// explícito del usuario en pruebas reales, 2026-09-17: el modal de
/// Proveedores quedaba "muy laxo" comparado con el de Contratista.
@Composable
internal fun DialogoConfirmarSalidaProveedor(
    fila: FilaProveedorActiva?,
    onDismiss: () -> Unit,
    onConfirmar: (FilaProveedorActiva) -> Unit,
) {
    if (fila == null) return
    DialogoRegistrarSalida(
        detalle = when (fila) {
            is FilaProveedorActiva.Local ->
                "${fila.registro.nombre} · ${fila.registro.cedula} · ${fila.registro.empresaNombre} · Gafete ${fila.registro.gafeteNumero}"
            is FilaProveedorActiva.Remota -> "${fila.remoto.nombre} · $TEXTO_OTRO_DISPOSITIVO"
        },
        onDismiss = onDismiss,
        onConfirmar = { onConfirmar(fila) },
    )
}

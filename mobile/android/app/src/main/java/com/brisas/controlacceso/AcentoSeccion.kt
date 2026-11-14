package com.brisas.controlacceso

import androidx.compose.ui.graphics.Color

/// Color de cada sección principal (pedido del usuario 2026-10-19): con un
/// solo color para todo, Contratistas, KOF y Proveedores se confundían de un
/// vistazo. Lleva el contexto del guardia: "estoy en la pestaña roja".
///
/// Desde 2026-11-14 (`docs/handoff-rediseno-movil-ux.md`, fase 1) pinta sólo
/// el ícono y el nombre de la pestaña activa en la barra de abajo. Antes
/// reemplazaba también el color principal de toda la página (botones,
/// bordes, íconos): un "Entregar" rojo en KOF se leía como un error y un
/// "Registrar" mango en Proveedores como una advertencia. Ahora todo lo que
/// se toca va en el azul de la app y los colores de estado significan siempre
/// lo mismo.
///
/// Tonos en `PaletaLattis.kt`; contraste medido en `AcentoSeccionTest`.
///
/// `null` = sin color propio (Rutas y Visitas, en desarrollo y fuera de la
/// barra): la pestaña usa el azul de la app.
fun colorDeSeccion(seccion: SeccionPrincipal, oscuro: Boolean): Color? {
    val paleta = if (oscuro) PaletaNoche else PaletaDia
    return when (seccion) {
        SeccionPrincipal.ACTIVOS -> paleta.contratistas
        SeccionPrincipal.KOF -> paleta.kof
        SeccionPrincipal.PROVEEDORES -> paleta.proveedores
        SeccionPrincipal.CORREO -> paleta.porCorreo
        SeccionPrincipal.RUTAS, SeccionPrincipal.VISITAS -> null
    }
}

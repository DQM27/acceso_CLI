package com.brisas.controlacceso

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.unit.dp

/// Los mismos íconos que usa el escritorio (Lucide, `lucide-react` en
/// `desktop/`, licencia ISC), para que una sección se reconozca igual en las
/// dos apps. Desde 2026-11-14 son los únicos de la app (fase 2 de
/// `docs/handoff-rediseno-movil-ux.md`): se quitaron los de Material, que
/// mezclaban rellenos y contornos con estos de línea. Trazos copiados tal cual de `dist/esm/icons/*.mjs`; los
/// `circle`/`rect` de Lucide van como trazos equivalentes. Son de línea (sin
/// relleno), y `Icon` los pinta con su `tint` como cualquier otro.
internal object IconosLucide {
    /// Activos (`UserCheck`, la sección Activos del escritorio).
    val UsuarioConCheck: ImageVector by lazy {
        icono(
            "UserCheck",
            "m16 11 2 2 4-4",
            "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2",
            // circle cx=9 cy=7 r=4
            "M5 7a4 4 0 1 0 8 0a4 4 0 1 0 -8 0",
        )
    }

    /// KOF (`BadgeCheck`, "Gafete KOF" en el menú Ingreso del escritorio).
    val InsigniaConCheck: ImageVector by lazy {
        icono(
            "BadgeCheck",
            "M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.77 4.78 4 4 0 0 1-6.75 0 4 4 0 0 1-4.78-4.77 4 4 0 0 1 0-6.76Z",
            "m16 9-5.5 5.5L8 12",
        )
    }

    /// Proveedores (`Boxes`, "Proveedor" en el escritorio).
    val Cajas: ImageVector by lazy {
        icono(
            "Boxes",
            "M2.97 12.92A2 2 0 0 0 2 14.63v3.24a2 2 0 0 0 .97 1.71l3 1.8a2 2 0 0 0 2.06 0L12 19v-5.5l-5-3-4.03 2.42Z",
            "m7 16.5-4.74-2.85",
            "m7 16.5 5-3",
            "M7 16.5v5.17",
            "M12 13.5V19l3.97 2.38a2 2 0 0 0 2.06 0l3-1.8a2 2 0 0 0 .97-1.71v-3.24a2 2 0 0 0-.97-1.71L17 10.5l-5 3Z",
            "m17 16.5-5-3",
            "m17 16.5 4.74-2.85",
            "M17 16.5v5.17",
            "M7.97 4.42A2 2 0 0 0 7 6.13v4.37l5 3 5-3V6.13a2 2 0 0 0-.97-1.71l-3-1.8a2 2 0 0 0-2.06 0l-3 1.8Z",
            "M12 8 7.26 5.15",
            "m12 8 4.74-2.85",
            "M12 13.5V8",
        )
    }

    /// Correo (`Mail`, "Visita por correo" en el escritorio).
    val Sobre: ImageVector by lazy {
        icono(
            "Mail",
            "m22 7-8.991 5.727a2 2 0 0 1-2.009 0L2 7",
            // rect x=2 y=4 width=20 height=16 rx=2
            "M4 4h16a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Z",
        )
    }

    /// Tema claro (`Sun`, el del menú de usuario del escritorio).
    val Sol: ImageVector by lazy {
        icono(
            "Sun",
            // circle cx=12 cy=12 r=4
            "M8 12a4 4 0 1 0 8 0a4 4 0 1 0 -8 0",
            "M12 2v2",
            "M12 20v2",
            "m4.93 4.93 1.41 1.41",
            "m17.66 17.66 1.41 1.41",
            "M2 12h2",
            "M20 12h2",
            "m6.34 17.66-1.41 1.41",
            "m19.07 4.93-1.41 1.41",
        )
    }

    /// Tema oscuro (`Moon`, el del menú de usuario del escritorio).
    val Luna: ImageVector by lazy {
        icono(
            "Moon",
            "M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401",
        )
    }

    /// Registrado en otro equipo de la unidad, llegado por la nube (`Cloud`).
    val Nube: ImageVector by lazy {
        icono(
            "Cloud",
            "M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z",
        )
    }

    /// Tema automático, el del teléfono (`Smartphone`).
    val Telefono: ImageVector by lazy {
        icono(
            "Smartphone",
            // rect x=5 y=2 width=14 height=20 rx=2
            "M7 2h10a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2Z",
            "M12 18h.01",
        )
    }

    /// Contratista sin acceso, en la búsqueda (`Ban`, el "prohibido").
    val Prohibido: ImageVector by lazy {
        icono(
            "Ban",
            // circle cx=12 cy=12 r=10
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
            "M4.929 4.929 19.07 19.071",
        )
    }

    /// PRAIND vencido, en la búsqueda (`CalendarX`, `lucide-react` 1.62.0).
    val CalendarioConX: ImageVector by lazy {
        icono(
            "CalendarX",
            "M8 2v3",
            "M16 2v3",
            // rect x=3 y=3 width=18 height=18 rx=2
            "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Z",
            "M3 9h18",
            "m14 13-4 4",
            "m10 13 4 4",
        )
    }

    /// Buscar a una persona (`UserSearch`).
    val BuscarPersona: ImageVector by lazy {
        icono(
            "UserSearch",
            "M6 7a4 4 0 1 0 8 0a4 4 0 1 0 -8 0",
            "M10.3 15H7a4 4 0 0 0-4 4v2",
            "M14 17a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
            "m21 21-1.9-1.9",
        )
    }

    /// Advertencia (`TriangleAlert`).
    val Advertencia: ImageVector by lazy {
        icono(
            "TriangleAlert",
            "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3",
            "M12 9v4",
            "M12 17h.01",
        )
    }

    /// Cámara (`Camera`).
    val Camara: ImageVector by lazy {
        icono(
            "Camera",
            "M13.997 4a2 2 0 0 1 1.76 1.05l.486.9A2 2 0 0 0 18.003 7H20a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h1.997a2 2 0 0 0 1.759-1.048l.489-.904A2 2 0 0 1 10.004 4z",
            "M9 13a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
        )
    }

    /// Nuevo ingreso, el "+" de abajo (`Plus`).
    val Mas: ImageVector by lazy {
        icono(
            "Plus",
            "M5 12h14",
            "M12 5v14",
        )
    }

    /// Agregar (`CirclePlus`).
    val MasEnCirculo: ImageVector by lazy {
        icono(
            "CirclePlus",
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
            "M8 12h8",
            "M12 8v8",
        )
    }

    /// Cerrar (`X`).
    val Cerrar: ImageVector by lazy {
        icono(
            "X",
            "M18 6 6 18",
            "m6 6 12 12",
        )
    }

    /// Borrar lo escrito en un buscador (`CircleX`).
    val BorrarTexto: ImageVector by lazy {
        icono(
            "CircleX",
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
            "m15 9-6 6",
            "m9 9 6 6",
        )
    }

    /// Error (`CircleAlert`).
    val Error: ImageVector by lazy {
        icono(
            "CircleAlert",
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
            "M12 8L12 12",
            "M12 16L12.01 16",
        )
    }

    /// Confirmado, requisito cumplido (`CircleCheck`).
    val Confirmado: ImageVector by lazy {
        icono(
            "CircleCheck",
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
            "m16 9-5.5 5.5L8 12",
        )
    }

    /// Requisito pendiente (`Circle`).
    val Pendiente: ImageVector by lazy {
        icono(
            "Circle",
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
        )
    }

    /// Buscar (`Search`).
    val Lupa: ImageVector by lazy {
        icono(
            "Search",
            "m21 21-4.34-4.34",
            "M3 11a8 8 0 1 0 16 0a8 8 0 1 0 -16 0",
        )
    }

    /// Avisos (`Bell`).
    val Campana: ImageVector by lazy {
        icono(
            "Bell",
            "M10.268 21a2 2 0 0 0 3.464 0",
            "M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326",
        )
    }

    /// Avisos con una emergencia sin leer (`BellRing`).
    val CampanaUrgente: ImageVector by lazy {
        icono(
            "BellRing",
            "M10.268 21a2 2 0 0 0 3.464 0",
            "M22 8c0-2.3-.8-4.3-2-6",
            "M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326",
            "M4 2C2.8 3.7 2 5.7 2 8",
        )
    }

    /// Información (`Info`).
    val Informacion: ImageVector by lazy {
        icono(
            "Info",
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
            "M12 16v-4",
            "M12 8h.01",
        )
    }

    /// Lista de gente vacía (`Users`).
    val Personas: ImageVector by lazy {
        icono(
            "Users",
            "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2",
            "M16 3.128a4 4 0 0 1 0 7.744",
            "M22 21v-2a4 4 0 0 0-3-3.87",
            "M5 7a4 4 0 1 0 8 0a4 4 0 1 0 -8 0",
        )
    }

    /// Escanear un documento (`ScanText`).
    val EscanearDocumento: ImageVector by lazy {
        icono(
            "ScanText",
            "M3 7V5a2 2 0 0 1 2-2h2",
            "M17 3h2a2 2 0 0 1 2 2v2",
            "M21 17v2a2 2 0 0 1-2 2h-2",
            "M7 21H5a2 2 0 0 1-2-2v-2",
            "M7 8h8",
            "M7 12h10",
            "M7 16h6",
        )
    }

    /// Empresa (`Building2`).
    val Empresa: ImageVector by lazy {
        icono(
            "Building2",
            "M10 12h4",
            "M10 8h4",
            "M14 21v-3a2 2 0 0 0-4 0v3",
            "M6 10H4a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-2",
            "M6 21V5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v16",
        )
    }

    /// Archivar un aviso (`Archive`).
    val Archivar: ImageVector by lazy {
        icono(
            "Archive",
            "M3 3h18a1 1 0 0 1 1 1v3a1 1 0 0 1 -1 1h-18a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1Z",
            "M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8",
            "M10 12h4",
        )
    }

    /// Salida, cerrar sesión (`LogOut`).
    val Salida: ImageVector by lazy {
        icono(
            "LogOut",
            "m16 17 5-5-5-5",
            "M21 12H9",
            "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4",
        )
    }

    /// Ingreso (`LogIn`).
    val Ingreso: ImageVector by lazy {
        icono(
            "LogIn",
            "m10 17 5-5-5-5",
            "M15 12H3",
            "M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4",
        )
    }

    /// Abrir un detalle (`ChevronRight`).
    val FlechaDerecha: ImageVector by lazy {
        icono(
            "ChevronRight",
            "m9 18 6-6-6-6",
        )
    }

    /// Volver (`ChevronLeft`).
    val Volver: ImageVector by lazy {
        icono(
            "ChevronLeft",
            "m15 18-6-6 6-6",
        )
    }

    /// Mostrar la contraseña (`Eye`).
    val Ver: ImageVector by lazy {
        icono(
            "Eye",
            "M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0",
            "M9 12a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
        )
    }

    /// Ocultar la contraseña (`EyeOff`).
    val Ocultar: ImageVector by lazy {
        icono(
            "EyeOff",
            "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49",
            "M14.084 14.158a3 3 0 0 1-4.242-4.242",
            "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143",
            "m2 2 20 20",
        )
    }

    /// Escanear el código de vinculación (`ScanQrCode`).
    val EscanearQr: ImageVector by lazy {
        icono(
            "ScanQrCode",
            "M17 12v4a1 1 0 0 1-1 1h-4",
            "M17 3h2a2 2 0 0 1 2 2v2",
            "M17 8V7",
            "M21 17v2a2 2 0 0 1-2 2h-2",
            "M3 7V5a2 2 0 0 1 2-2h2",
            "M7 17h.01",
            "M7 21H5a2 2 0 0 1-2-2v-2",
            "M8 7h3a1 1 0 0 1 1 1v3a1 1 0 0 1 -1 1h-3a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1Z",
        )
    }

    /// Sincronizando con la nube (`CloudSync`).
    val NubeSincronizando: ImageVector by lazy {
        icono(
            "CloudSync",
            "m17 18-1.535 1.605a5 5 0 0 1-8-1.5",
            "M17 22v-4h-4",
            "M20.996 15.251A4.5 4.5 0 0 0 17.495 8h-1.79a7 7 0 1 0-12.709 5.607",
            "M7 10v4h4",
            "m7 14 1.535-1.605a5 5 0 0 1 8 1.5",
        )
    }

    /// Conflictos de la sincronización (`CloudAlert`).
    val NubeConProblema: ImageVector by lazy {
        icono(
            "CloudAlert",
            "M12 12v4",
            "M12 20h.01",
            "M8.128 16.949A7 7 0 1 1 15.71 8h1.79a1 1 0 0 1 0 9h-1.642",
        )
    }

    /// Vencido o por vencer (`Clock`).
    val Reloj: ImageVector by lazy {
        icono(
            "Clock",
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
            "M12 6v6l4 2",
        )
    }

    /// Rutas (`Route`).
    val Ruta: ImageVector by lazy {
        icono(
            "Route",
            "M3 19a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
            "M9 19h8.5a3.5 3.5 0 0 0 0-7h-11a3.5 3.5 0 0 1 0-7H15",
            "M15 5a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
        )
    }

    /// Visitas (`Calendar`).
    val Calendario: ImageVector by lazy {
        icono(
            "Calendar",
            "M8 2v3",
            "M16 2v3",
            "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2v-14a2 2 0 0 1 2 -2Z",
            "M3 9h18",
        )
    }

    /// Buscar actualizaciones (`RefreshCw`).
    val Actualizar: ImageVector by lazy {
        icono(
            "RefreshCw",
            "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8",
            "M21 3v5h-5",
            "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16",
            "M8 16H3v5",
        )
    }

    /// Desplegar (`ChevronDown`).
    val FlechaAbajo: ImageVector by lazy {
        icono(
            "ChevronDown",
            "m6 9 6 6 6-6",
        )
    }

    /// Plegar (`ChevronUp`).
    val FlechaArriba: ImageVector by lazy {
        icono(
            "ChevronUp",
            "m18 15-6-6-6 6",
        )
    }

    /// Llenar a mano (`Pencil`).
    val Lapiz: ImageVector by lazy {
        icono(
            "Pencil",
            "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
            "m15 5 4 4",
        )
    }

    /// Descargar la actualización (`Download`).
    val Descargar: ImageVector by lazy {
        icono(
            "Download",
            "M12 15V3",
            "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4",
            "m7 10 5 5 5-5",
        )
    }

    /// Borrar (`Trash2`).
    val Borrar: ImageVector by lazy {
        icono(
            "Trash2",
            "M10 11v6",
            "M14 11v6",
            "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6",
            "M3 6h18",
            "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",
        )
    }

    /// Paso completo (`Check`).
    val Check: ImageVector by lazy {
        icono(
            "Check",
            "M20 6 9 17l-5-5",
        )
    }

    /// Guía de la cámara (`Scan`).
    val Encuadre: ImageVector by lazy {
        icono(
            "Scan",
            "M3 7V5a2 2 0 0 1 2-2h2",
            "M17 3h2a2 2 0 0 1 2 2v2",
            "M21 17v2a2 2 0 0 1-2 2h-2",
            "M7 21H5a2 2 0 0 1-2-2v-2",
        )
    }

    /// Nuevo contratista (`IdCard`).
    val Ficha: ImageVector by lazy {
        icono(
            "IdCard",
            "M13 19a4 4 0 00-8 0",
            "M16 10h2",
            "M16 14h2",
            "M6 12a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
            "M4 5h16a2 2 0 0 1 2 2v10a2 2 0 0 1 -2 2h-16a2 2 0 0 1 -2 -2v-10a2 2 0 0 1 2 -2Z",
        )
    }

    /// Ayuda (`CircleQuestionMark`).
    val Ayuda: ImageVector by lazy {
        icono(
            "CircleQuestionMark",
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
            "M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3",
            "M12 17h.01",
        )
    }

    private fun icono(nombre: String, vararg trazos: String): ImageVector =
        ImageVector.Builder(
            name = "Lucide.$nombre",
            defaultWidth = 24.dp,
            defaultHeight = 24.dp,
            viewportWidth = 24f,
            viewportHeight = 24f,
        ).apply {
            trazos.forEach { trazo ->
                addPath(
                    pathData = addPathNodes(trazo),
                    fill = null,
                    stroke = SolidColor(Color.Black),
                    strokeLineWidth = 2f,
                    strokeLineCap = StrokeCap.Round,
                    strokeLineJoin = StrokeJoin.Round,
                )
            }
        }.build()
}

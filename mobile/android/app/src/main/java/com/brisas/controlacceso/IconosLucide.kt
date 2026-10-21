package com.brisas.controlacceso

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.unit.dp

/// Los mismos íconos que usa el escritorio (Lucide, `lucide-react` 1.47.0 en
/// `desktop/`, licencia ISC), para que una sección se reconozca igual en las
/// dos apps. Trazos copiados tal cual de `dist/esm/icons/*.mjs`; los
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

    /// Externos con Proveedor elegido (`Boxes`, "Proveedor" en el escritorio).
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

    /// Externos con Correo elegido (`Mail`, "Visita por correo" en el escritorio).
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

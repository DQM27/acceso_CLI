package com.brisas.controlacceso

import java.util.Locale

/// MV-10 (auditoría 2026-09-24): antes `PantallaNuevoContratista.kt`
/// (`aTextoDDMMYYYY`/`textoDDMMYYYYaIso`) y `PantallaRutas.kt`
/// (`aTextoDDMMYYYYRuta`/`textoDDMMYYYYaIsoRuta`) tenían cada una su propia
/// copia idéntica de estas dos conversiones -- riesgo de que una cambie y
/// la otra no, y ninguna de las dos fijaba `Locale`, así que `"%02d".format`
/// usaba el locale por defecto del dispositivo: en uno con dígitos no
/// latinos (persa, árabe-indostánico, etc.) el resultado no son dígitos
/// ASCII, y ni el `split("-")`/`padStart` de acá ni el parser ISO de Rust
/// (`DatosContratista.fechaVencimientoPraind`) lo entienden -- se rechaza
/// con un error que, para quien opera, no tiene ninguna relación con lo que
/// tipeó. `Locale.ROOT` fija dígitos ASCII sin importar el idioma del
/// dispositivo.
///
/// Misma convención día-mes-año que el resto de la app (`Tiempo.kt`,
/// `desktop/src/tiempo.ts`) -- Rust exige ISO (`AAAA-MM-DD`) para parsear la
/// fecha, pero mostrarla así en un formulario rompería la convención que la
/// persona ya espera en cualquier otra pantalla.
fun FechaDocumento.aTextoDDMMYYYY(): String = String.format(Locale.ROOT, "%02d-%02d-%04d", dia, mes, anio)

/// Inverso de [aTextoDDMMYYYY] -- convierte lo que la persona tipeó
/// (día-mes-año) al formato ISO que espera Rust. Si el texto no tiene la
/// forma esperada se devuelve tal cual: Rust igual la rechaza con un error
/// legible que cita el texto original.
fun textoDDMMYYYYaIso(texto: String): String {
    val partes = texto.split("-")
    if (partes.size != 3) return texto
    val (dia, mes, anio) = partes
    return String.format(Locale.ROOT, "%s-%s-%s", anio.padStart(4, '0'), mes.padStart(2, '0'), dia.padStart(2, '0'))
}

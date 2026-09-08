package com.brisas.controlacceso

/// Caja delimitadora plana, sin depender de `android.graphics.Rect` --
/// ese tipo funciona bien en un dispositivo real, pero en tests unitarios
/// de JVM sin Robolectric su constructor deja los campos en 0 en vez de
/// lanzar (a diferencia de métodos como `intersect`, que sí lanzan) --
/// confirmado con un test de diagnóstico antes de escribir esto así. Esta
/// lógica necesita valores reales para poder testearse, así que se
/// desacopla del framework de Android por completo; la conversión desde
/// `Rect` real vive en el único lugar que corre en el dispositivo
/// (ver uso en `PantallaEscanearCedula.kt`).
data class CajaTexto(val izquierda: Int, val arriba: Int, val derecha: Int, val abajo: Int)

/// Un bloque de texto ya reconocido por ML Kit, reducido a lo que necesita
/// el filtro de recorte -- desacoplado de `com.google.mlkit.vision.text.Text`
/// (clase final sin constructor público) para poder testear esta lógica sin
/// depender de ML Kit ni de un dispositivo/emulador.
data class BloqueTextoOcr(val caja: CajaTexto?, val texto: String)

/// Ancho/alto de la imagen ya "parada" (como la ve quien opera), a partir de
/// las dimensiones crudas del sensor y la rotación que reporta CameraX --
/// a 90°/270° el sensor entrega el frame acostado.
fun dimensionesUpright(ancho: Int, alto: Int, rotacionGrados: Int): Pair<Int, Int> =
    if (rotacionGrados == 90 || rotacionGrados == 270) alto to ancho else ancho to alto

/// Recorte del área de análisis al recuadro guía (plan, sección 9) -- pero
/// implementado sobre los bounding boxes que ML Kit YA calculó, no
/// convirtiendo el frame a Bitmap para recortar píxeles. Esto último costaría
/// una conversión YUV→RGB completa por frame, justo el tipo de trabajo
/// "O(imagen)" que el plan prohíbe agregar (sección 0.6/5): esto es
/// `O(número de bloques de texto)`, prácticamente gratis.
///
/// Mismas proporciones que `MarcoGuiaCedula` (82% de ancho, aspecto 1.586:1,
/// centrado) -- si alguna cambia, deben cambiar juntas.
fun filtrarTextoEnAreaGuia(bloques: List<BloqueTextoOcr>, anchoImagen: Int, altoImagen: Int): String {
    val anchoGuia = anchoImagen * 0.82f
    val altoGuia = anchoGuia / 1.586f
    val izquierda = (anchoImagen - anchoGuia) / 2f
    val arriba = (altoImagen - altoGuia) / 2f
    val derecha = izquierda + anchoGuia
    val abajo = arriba + altoGuia

    return bloques
        .filter { bloque ->
            // Sin caja (no debería pasar con ML Kit, pero por si acaso) no
            // se descarta -- mejor un falso positivo que perder texto real.
            val caja = bloque.caja ?: return@filter true
            caja.izquierda < derecha && caja.derecha > izquierda && caja.arriba < abajo && caja.abajo > arriba
        }
        .joinToString("\n") { it.texto }
}

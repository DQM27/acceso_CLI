package com.brisas.controlacceso

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/// Un registro que acaba de salir bien ("Ingreso registrado · Ana Mora").
/// `id` distingue dos confirmaciones con el mismo texto seguidas: cada una
/// vuelve a mostrarse y a contar su tiempo.
data class Confirmacion(val texto: String, val id: Long)

/// La última confirmación de una sección, hasta que [ReenviarConfirmaciones]
/// la pasa a los avisos de arriba. Vive en el ViewModel de cada sección (lo
/// pone quien registró), así un registro que termina mientras la pantalla se
/// recompone no pierde su aviso.
class ConfirmacionReciente {
    var actual by mutableStateOf<Confirmacion?>(null)
        private set
    private var siguienteId = 0L

    fun mostrar(texto: String) {
        actual = Confirmacion(texto, ++siguienteId)
    }

    /// Sólo oculta si sigue a la vista la misma confirmación: una nueva que
    /// llegó mientras tanto no se borra con el tiempo de la anterior.
    fun ocultar(id: Long) {
        if (actual?.id == id) actual = null
    }
}

/// Las confirmaciones de registro de todas las secciones salen arriba, en los
/// [avisos] breves, con el toque de confirmación de iOS (el registro salió
/// bien aunque no se esté mirando la pantalla). Va en la pantalla principal,
/// que no se va al cambiar de pestaña (pedido del usuario 2026-10-27): antes
/// cada pantalla dibujaba la suya abajo, tapando la lista, y su reloj vivía en
/// la pantalla, así que al cambiar de pestaña se cortaba y la confirmación
/// volvía a aparecer (y a vibrar) al regresar. Ahora cada una se toma apenas
/// llega y el reloj es el de los avisos, que no se corta.
@Composable
internal fun ReenviarConfirmaciones(avisos: AvisosBreves, vararg confirmaciones: ConfirmacionReciente) {
    val hapticos = rememberHapticos()
    for (confirmacion in confirmaciones) {
        key(confirmacion) {
            val actual = confirmacion.actual
            LaunchedEffect(actual?.id) {
                if (actual == null) return@LaunchedEffect
                hapticos(Haptico.CONFIRMAR)
                avisos.mostrar(actual.texto, TipoEstado.EXITO)
                confirmacion.ocultar(actual.id)
            }
        }
    }
}

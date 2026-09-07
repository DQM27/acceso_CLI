package com.brisas.controlacceso

import android.content.Context
import android.content.SharedPreferences
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

private const val PREFS = "tema"
private const val CLAVE_OSCURO = "oscuro"

/**
 * Preferencia de tema explícita (claro/oscuro), guardada en
 * SharedPreferences -- por defecto sigue el tema del sistema
 * (`oscuroForzado == null`, ver [TemaBrisas]). Objeto compartido (no un
 * ViewModel/CompositionLocal) a propósito: es un único booleano de
 * presentación, sin ninguna llamada a [uniffi.control_acceso_mobile.Nucleo]
 * detrás -- inicializarlo una vez desde `MainActivity.onCreate` y
 * leerlo/escribirlo desde cualquier pantalla alcanza. Mismo criterio que
 * `web/src/componentes/SelectorTema.tsx`/`desktop/src/componentes/
 * SelectorTema.tsx`, adaptado a que acá no existe un `localStorage`/CSS
 * `data-theme` -- es `MaterialTheme` el que decide el color scheme.
 */
object GestorTema {
    var oscuroForzado by mutableStateOf<Boolean?>(null)
        private set

    private lateinit var prefs: SharedPreferences

    fun inicializar(context: Context) {
        prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        oscuroForzado = if (prefs.contains(CLAVE_OSCURO)) prefs.getBoolean(CLAVE_OSCURO, false) else null
    }

    fun alternar(oscuroActual: Boolean) {
        val siguiente = !oscuroActual
        oscuroForzado = siguiente
        prefs.edit().putBoolean(CLAVE_OSCURO, siguiente).apply()
    }
}

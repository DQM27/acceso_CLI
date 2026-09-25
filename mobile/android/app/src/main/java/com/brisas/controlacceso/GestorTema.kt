package com.brisas.controlacceso

import android.content.Context
import android.content.SharedPreferences
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

private const val PREFS = "tema"
private const val CLAVE_TEMA = "tema"

/** Clave vieja (sólo claro/oscuro, antes de Tokyo Night): se sigue leyendo
 * para no perder la elección de quien ya la tenía guardada. */
private const val CLAVE_OSCURO_VIEJA = "oscuro"

/** Temas que recorre el botón de la barra superior, en este orden --
 * mismo recorrido que `desktop/src/componentes/SelectorTema.tsx`. */
enum class TemaApp(val oscuro: Boolean) {
    CLARO(oscuro = false),
    OSCURO(oscuro = true),
    TOKYO_NIGHT(oscuro = true),
    ;

    fun siguiente(): TemaApp = entries[(ordinal + 1) % entries.size]
}

/**
 * Preferencia de tema explícita (claro / oscuro / Tokyo Night), guardada en
 * SharedPreferences -- por defecto sigue el tema del sistema
 * (`temaForzado == null`, ver [temaActual]). Objeto compartido (no un
 * ViewModel/CompositionLocal) a propósito: es un único valor de
 * presentación, sin ninguna llamada a [uniffi.control_acceso_mobile.Nucleo]
 * detrás -- inicializarlo una vez desde `MainActivity.onCreate` y
 * leerlo/escribirlo desde cualquier pantalla alcanza. Mismo criterio que
 * `desktop/src/componentes/SelectorTema.tsx`, adaptado a que acá no existe
 * un `localStorage`/CSS `data-theme` -- es `MaterialTheme` el que decide el
 * color scheme.
 */
object GestorTema {
    var temaForzado by mutableStateOf<TemaApp?>(null)
        private set

    private lateinit var prefs: SharedPreferences

    fun inicializar(context: Context) {
        prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val guardado = prefs.getString(CLAVE_TEMA, null)
        temaForzado = TemaApp.entries.firstOrNull { it.name == guardado }
            ?: when {
                !prefs.contains(CLAVE_OSCURO_VIEJA) -> null
                prefs.getBoolean(CLAVE_OSCURO_VIEJA, false) -> TemaApp.OSCURO
                else -> TemaApp.CLARO
            }
    }

    fun alternar(actual: TemaApp) {
        val siguiente = actual.siguiente()
        temaForzado = siguiente
        prefs.edit().putString(CLAVE_TEMA, siguiente.name).apply()
    }
}

/** Tema en uso: el elegido a mano o, si nunca se eligió, el del sistema. */
@Composable
fun temaActual(): TemaApp =
    GestorTema.temaForzado ?: if (isSystemInDarkTheme()) TemaApp.OSCURO else TemaApp.CLARO

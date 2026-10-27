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
    TOKYO_DAY(oscuro = false),
    TOKYO_NIGHT(oscuro = true),
    ;

    fun siguiente(): TemaApp = entries[(ordinal + 1) % entries.size]
}

/// El tema que corresponde a lo que había guardado. Antes los temas eran
/// "claro", "oscuro" y "Tokyo Night": los dos primeros pasan a su equivalente
/// Tokyo (igual que en el escritorio), así nadie pierde su elección. Más
/// atrás se guardaba sólo un booleano (`oscuroViejo`, `null` si no existía).
/// `null`: nunca eligió, se sigue al sistema.
internal fun temaGuardado(guardado: String?, oscuroViejo: Boolean?): TemaApp? =
    TemaApp.entries.firstOrNull { it.name == guardado }
        ?: when (guardado) {
            "CLARO" -> TemaApp.TOKYO_DAY
            "OSCURO" -> TemaApp.TOKYO_NIGHT
            else -> when (oscuroViejo) {
                null -> null
                true -> TemaApp.TOKYO_NIGHT
                false -> TemaApp.TOKYO_DAY
            }
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
        temaForzado = temaGuardado(
            guardado = prefs.getString(CLAVE_TEMA, null),
            oscuroViejo = if (prefs.contains(CLAVE_OSCURO_VIEJA)) prefs.getBoolean(CLAVE_OSCURO_VIEJA, false) else null,
        )
    }

    fun alternar(actual: TemaApp) {
        elegir(actual.siguiente())
    }

    /// `null` = automático, según el sistema (pedido del usuario 2026-10-27:
    /// una vez elegido claro u oscuro no había cómo volver). Se borra también
    /// la clave vieja: si quedara, [temaGuardado] la volvería a aplicar.
    fun elegir(tema: TemaApp?) {
        temaForzado = tema
        prefs.edit().apply {
            if (tema == null) remove(CLAVE_TEMA) else putString(CLAVE_TEMA, tema.name)
            remove(CLAVE_OSCURO_VIEJA)
        }.apply()
    }
}

/** Tema en uso: el elegido a mano o, si nunca se eligió, el del sistema. */
@Composable
fun temaActual(): TemaApp =
    GestorTema.temaForzado ?: if (isSystemInDarkTheme()) TemaApp.TOKYO_NIGHT else TemaApp.TOKYO_DAY

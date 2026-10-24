package com.brisas.controlacceso

import android.content.Context
import android.media.AudioManager
import android.media.ToneGenerator
import android.os.Handler
import android.os.Looper

/// Qué pasó con la persona al escanear su documento para ingresar (pedido del
/// usuario 2026-10-21): el guardia lo sabe por el sonido y la vibración, sin
/// mirar la pantalla. Sólo al escanear: buscando a mano no suena nada.
///
/// "No encontrado" no suena a propósito: la cámara ya hizo su "bip" al leer
/// el documento y un segundo sonido enseguida sonaba doble (visto en el
/// teléfono). Se ve en pantalla.
enum class AlertaEscaneo {
    /// El núcleo no deja pasar (sin acceso, PRAIND vencido u otro bloqueo).
    DENEGADO,

    /// Puede entrar, pero con un aviso de PRAIND (por ejemplo, por vencer).
    ADVERTENCIA,
}

/// Una alerta para sonar; `id` distingue dos iguales seguidas.
data class AvisoEscaneo(val alerta: AlertaEscaneo, val id: Long)

/// La alerta tras escanear, o `null` si no hay nada que avisar (entra sin
/// novedad, la búsqueda no trajo a nadie o trajo varios parecidos y hay que
/// elegir). Lo decide el núcleo (`mensajeBloqueo`, `avisoPraind`); acá sólo se
/// traduce a una señal.
internal fun alertaTrasEscaneo(seleccion: SeleccionIngreso): AlertaEscaneo? =
    when {
        seleccion is SeleccionIngreso.Bloqueada -> AlertaEscaneo.DENEGADO
        seleccion is SeleccionIngreso.Formulario && seleccion.preparacion.avisoPraind != null -> AlertaEscaneo.ADVERTENCIA
        else -> null
    }

/// Suena y vibra según la alerta. El tono va por el canal de ALARMA (pedido
/// del usuario: que suene siempre): suena aunque el teléfono esté en silencio,
/// como un despertador, porque en la portería un teléfono olvidado en silencio
/// no puede dejar pasar un acceso denegado sin aviso. Cada caso con su tono:
/// - denegado: el tono de error del sistema (tres notas), largo;
/// - advertencia: dos pitidos.
fun sonarAlertaEscaneo(contexto: Context, alerta: AlertaEscaneo) {
    when (alerta) {
        AlertaEscaneo.DENEGADO -> {
            reproducirTonoAlarma(ToneGenerator.TONE_SUP_ERROR, 700)
            vibrarError(contexto)
        }
        AlertaEscaneo.ADVERTENCIA -> {
            reproducirTonoAlarma(ToneGenerator.TONE_PROP_BEEP2, 400)
            vibrarAdvertencia(contexto)
        }
    }
}

private fun reproducirTonoAlarma(tono: Int, duracionMs: Int) {
    try {
        val generador = ToneGenerator(AudioManager.STREAM_ALARM, VOLUMEN_ALERTA)
        generador.startTone(tono, duracionMs)
        // Mismo cuidado que `reproducirSonidoConfirmacion`: soltar el recurso
        // nativo de audio cuando el tono terminó.
        Handler(Looper.getMainLooper()).postDelayed(generador::release, duracionMs + 50L)
    } catch (_: RuntimeException) {
        // El sonido es un complemento: nunca debe tumbar el ingreso.
    }
}

/// Sobre 100, relativo al volumen de alarma del teléfono: se tiene que oír,
/// a diferencia del "bip" sutil de confirmación (40).
private const val VOLUMEN_ALERTA = 85

package com.brisas.controlacceso

import android.content.Context
import android.os.Build

/// Datos del teléfono físico, capturados una sola vez para la activación
/// inicial (ver `PrimerArranqueViewModel.conectar` y
/// `Nucleo.configurarDispositivoInicialConSecreto`). Ninguno es secreto en
/// sí mismo -- observables por cualquier app en el propio teléfono -- ver
/// `docs/plan-sesion-unica-dispositivos.md`.
data class MetadatosDispositivoLocal(
    val androidId: String,
    val modelo: String,
    val fabricante: String,
    val fingerprint: String,
    val appVersion: String,
) {
    companion object {
        fun capturar(context: Context, androidId: String): MetadatosDispositivoLocal {
            val version =
                try {
                    context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: ""
                } catch (_: Exception) {
                    ""
                }
            return MetadatosDispositivoLocal(
                androidId = androidId,
                modelo = Build.MODEL ?: "",
                fabricante = Build.MANUFACTURER ?: "",
                fingerprint = Build.FINGERPRINT ?: "",
                appVersion = version,
            )
        }
    }
}

package com.brisas.controlacceso

import android.content.Context
import android.os.Build

/// Datos del teléfono físico, capturados una sola vez para la activación
/// inicial (ver `PrimerArranqueViewModel.conectar` y
/// `Nucleo.configurarDispositivoInicialConSecreto`). Ninguno es secreto en
/// sí mismo -- observables por cualquier app en el propio teléfono -- ver
/// `docs/plan-sesion-unica-dispositivos.md`. Nombres de campo neutrales a
/// propósito -- el mismo contrato (`MetadatosDispositivo` en Rust) también
/// lo llena escritorio, con su propio significado.
data class MetadatosDispositivoLocal(
    val identificadorHardware: String,
    val nombreDispositivo: String,
    val plataforma: String,
    val versionBuild: String,
    val appVersion: String,
) {
    companion object {
        fun capturar(context: Context, identificadorHardware: String): MetadatosDispositivoLocal {
            val version =
                try {
                    context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: ""
                } catch (_: Exception) {
                    ""
                }
            return MetadatosDispositivoLocal(
                identificadorHardware = identificadorHardware,
                nombreDispositivo = Build.MODEL ?: "",
                plataforma = Build.MANUFACTURER ?: "",
                versionBuild = Build.FINGERPRINT ?: "",
                appVersion = version,
            )
        }
    }
}

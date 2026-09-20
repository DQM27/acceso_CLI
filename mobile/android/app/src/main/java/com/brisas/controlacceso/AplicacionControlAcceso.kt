package com.brisas.controlacceso

import android.app.Application
import android.system.Os

/// Punto de entrada del proceso, antes que cualquier Activity/ViewModel --
/// necesario para fijar `CONTROL_ACCESO_SUPABASE_URL`/`APIKEY` (leídas por
/// el núcleo Rust vía `std::env::var`, cacheadas en un `OnceLock` la primera
/// vez que se llaman) antes de la primera llamada al núcleo.
///
/// Android no hereda variables de entorno de shell como Windows/desktop
/// (ver `scripts/activar_sandbox.ps1` y `docs/recuperacion-sitio-staging.md`)
/// -- `Os.setenv` es el único equivalente: pone la variable en el propio
/// proceso antes de que la lea el getenv() de Rust.
///
/// Solo en builds DEBUG -- release (firmado, distribuido vía GitHub
/// Releases) sigue apuntando a producción sin tocar nada, mismo criterio
/// que el `FLAG_SECURE` de MainActivity. La app ya está en producción real:
/// un build de prueba NUNCA debe poder escribir en esa base por accidente.
class AplicacionControlAcceso : Application() {
    override fun onCreate() {
        super.onCreate()
        if (BuildConfig.DEBUG) {
            Os.setenv("CONTROL_ACCESO_SUPABASE_URL", URL_STAGING, true)
            Os.setenv("CONTROL_ACCESO_SUPABASE_APIKEY", APIKEY_STAGING, true)
        }
    }

    private companion object {
        // Proyecto `control-acceso-staging` (ver docs/recuperacion-sitio-staging.md)
        // -- URL y apikey publicable, no son secretos (RLS decide todo según
        // el JWT que las acompañe).
        const val URL_STAGING = "https://pmrytjktlyiuikxuuxpr.supabase.co"
        const val APIKEY_STAGING = "sb_publishable_29DwMvfyj8Jq--LBcqxtBA_pTwWrDH4"
    }
}

package com.brisas.controlacceso

class SecretoDispositivoStoreDePrueba(
    private var secreto: String? = null,
    // MV-05 (auditoría 2026-09-24): simula el Keystore/archivo del secreto
    // fallando al leer (corrupto, `AEADBadTagException` tras una
    // restauración, etc.) -- ver `ActivosViewModelTest` para el caso que
    // esto cubre.
    private val lanzarAlCargar: Boolean = false,
) : SecretoDispositivoStore {
    override fun guardar(secreto: String) {
        this.secreto = secreto.trim()
    }

    override fun cargar(): String? {
        if (lanzarAlCargar) throw SecretoDispositivoStoreException(IllegalStateException("secreto de prueba dañado"))
        return secreto
    }
}

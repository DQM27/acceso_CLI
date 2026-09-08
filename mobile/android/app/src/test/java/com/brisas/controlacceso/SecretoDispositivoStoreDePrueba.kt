package com.brisas.controlacceso

class SecretoDispositivoStoreDePrueba(
    private var secreto: String? = null,
) : SecretoDispositivoStore {
    override fun guardar(secreto: String) {
        this.secreto = secreto.trim()
    }

    override fun cargar(): String? = secreto
}

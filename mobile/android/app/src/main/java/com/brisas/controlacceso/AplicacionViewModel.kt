package com.brisas.controlacceso

import android.annotation.SuppressLint
import android.app.Application
import android.provider.Settings
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.Nucleo

data class EntornoAplicacion(
    val nucleo: Nucleo,
    val directorio: String,
    val secretoStore: SecretoDispositivoStore,
    val metadata: MetadatosDispositivoLocal,
    val requiereConfiguracionInicial: Boolean,
)

sealed interface EstadoAplicacion {
    data object Cargando : EstadoAplicacion
    data class Lista(val entorno: EntornoAplicacion) : EstadoAplicacion
    data class Fallo(val mensaje: String) : EstadoAplicacion
}

/** Dueño único del núcleo nativo durante toda la vida lógica de la Activity. */
class AplicacionViewModel(application: Application) : AndroidViewModel(application) {
    private val _estado = MutableStateFlow<EstadoAplicacion>(EstadoAplicacion.Cargando)
    val estado: StateFlow<EstadoAplicacion> = _estado.asStateFlow()

    init {
        viewModelScope.launch {
            _estado.value = abrirEntorno()
        }
    }

    @SuppressLint("HardwareIds")
    private suspend fun abrirEntorno(): EstadoAplicacion = withContext(Dispatchers.IO) {
        var nucleo: Nucleo? = null
        try {
            val context = getApplication<Application>()
            val directorio = context.filesDir.absolutePath
            val identificador =
                Settings.Secure.getString(context.contentResolver, Settings.Secure.ANDROID_ID) ?: ""
            // MV-03 (auditoría 2026-09-24): antes `Nucleo.abrir` -- sin
            // clave, sin importar qué motor SQLite estuviera compilado en
            // el .so, la base quedaba en texto plano de verdad. La clave
            // sale del Keystore (ver ClaveBaseDatosStore.kt, mismo esquema
            // que SecretoDispositivoStore), nunca derivada acá. Si el
            // archivo existente no es legible con esta clave -- el caso de
            // todo teléfono con la app instalada antes de este cambio --
            // Nucleo.abrirCifrado ya lo descarta y reconstruye solo (ver su
            // doc-comment en mobile/rust-core/src/lib.rs); no hace falta
            // ningún manejo especial acá.
            val claveBaseDatos = AndroidKeystoreClaveBaseDatosStore(context).obtenerOCrear()
            val abierto = Nucleo.abrirCifrado(
                File(context.filesDir, "control_acceso.db").absolutePath,
                claveBaseDatos,
            )
            nucleo = abierto
            val store = AndroidKeystoreSecretoDispositivoStore(context, abierto, directorio, identificador)
            EstadoAplicacion.Lista(
                EntornoAplicacion(
                    nucleo = abierto,
                    directorio = directorio,
                    secretoStore = store,
                    metadata = MetadatosDispositivoLocal.capturar(context, identificador),
                    requiereConfiguracionInicial = abierto.requiereConfiguracionInicial(),
                ),
            )
        } catch (_: Exception) {
            nucleo?.let { runCatching { it.close() } }
            EstadoAplicacion.Fallo("No fue posible abrir los datos de la aplicación.")
        } catch (_: LinkageError) {
            nucleo?.let { runCatching { it.close() } }
            EstadoAplicacion.Fallo("No fue posible cargar el núcleo de la aplicación.")
        }
    }

    override fun onCleared() {
        (_estado.value as? EstadoAplicacion.Lista)?.entorno?.nucleo?.close()
        super.onCleared()
    }
}

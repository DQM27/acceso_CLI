package com.brisas.controlacceso

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.IngresoRemoto
import uniffi.control_acceso_mobile.ModoBusquedaActivos
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.PreparacionIngreso

/// Mismo árbol de estados que `Seleccion` en NuevoIngresoModal.tsx: sin
/// selección (buscador visible), verificando (prepararIngreso en vuelo),
/// bloqueada (Rust ya decidió que no puede continuar) o lista para
/// confirmar. Kotlin sólo despacha sobre lo que Rust ya calculó.
sealed class SeleccionIngreso {
    data object Ninguna : SeleccionIngreso()

    data class Cargando(val contratista: ContratistaResumen) : SeleccionIngreso()

    data class Bloqueada(val preparacion: PreparacionIngreso, val mensaje: String) : SeleccionIngreso()

    data class Formulario(val preparacion: PreparacionIngreso) : SeleccionIngreso()
}

/// Con pocos activos alcanza con recorrer la lista a ojo, pero con muchos
/// (imaginemos 100) hace falta poder acotarla — el mismo campo de texto no
/// puede a la vez buscar en el catálogo completo (para entrada) Y filtrar
/// los activos (para salida), así que un selector de tres decide cuál de
/// las dos cosas está haciendo el campo. `SALIDA_NOMBRE`/`SALIDA_GAFETE` van
/// separados (y no uno solo "salida") porque la búsqueda de texto libre de
/// Rust ya mezcla nombre/cédula con gafete en el mismo OR — buscar "7" como
/// gafete también traería cualquier cédula que lo contenga, ruidoso con
/// muchos activos. `Nucleo.listarIngresosActivos` recibe `ModoBusquedaActivos`
/// para pedirle a Rust el filtro exacto en vez de resolverlo del lado del
/// teléfono.
enum class ModoBusqueda { ENTRADA, SALIDA_NOMBRE, SALIDA_GAFETE }

/// Un número de gafete escrito y, si ya se buscó, el activo encontrado (o
/// `null` si nadie adentro tiene ese gafete puesto) — la fila de
/// `CoincidenciaGafete` es lo que se pinta en el modo Salida: gafete antes
/// de confirmar, mismo rol que la tabla de vista previa de
/// `SalidaModal.tsx` en desktop.
data class CoincidenciaGafete(val numero: Int, val activo: IngresoActivoResumen?)

/// Fila local (este dispositivo) o remota (abierta por el otro dispositivo
/// del mismo sitio, cacheada en `ingresos_remotos` -- nunca vive en el
/// historial local) -- mismo espíritu que `FilaActiva` en Activos.tsx de
/// desktop: una sola lista para las dos, la pantalla no necesita dos
/// secciones separadas para poder cerrar cualquiera de las dos desde acá.
sealed class FilaActiva {
    abstract val contratistaNombre: String

    data class Local(val activo: IngresoActivoResumen) : FilaActiva() {
        override val contratistaNombre get() = activo.contratistaNombre
    }

    data class Remota(val remoto: IngresoRemoto) : FilaActiva() {
        override val contratistaNombre get() = remoto.contratistaNombre
    }
}

private const val MAX_LARGO_GAFETES = 60

/// Espejo de `sanearGafetes` (desktop/src/api/ingresos.ts): sólo dígitos,
/// comas y espacios — todo lo demás que el usuario pegue o teclee se
/// descarta en silencio en vez de rechazarlo con un error.
private fun sanearGafetesTexto(texto: String): String =
    texto.filter { it.isDigit() || it == ',' || it.isWhitespace() }.take(MAX_LARGO_GAFETES)

/// Espejo de `gafetesDe` (desktop/src/api/ingresos.ts): "2, 25, 85" -> [2,
/// 25, 85]; tokens vacíos o no numéricos se ignoran en vez de fallar toda
/// la búsqueda por un error de tipeo en un solo número.
private fun gafetesDeTexto(texto: String): List<Int> =
    texto.split(",")
        .map { it.trim() }
        .filter { it.isNotEmpty() }
        .mapNotNull { it.toIntOrNull() }

/// Dueño de todo el estado de [PantallaActivos] y de las llamadas a
/// [Nucleo] — ver mobile/android/ARQUITECTURA.md. El `@Composable` sólo lee
/// este estado (propiedades de sólo lectura desde afuera) y reporta
/// eventos a través de estas funciones; ninguna decisión de negocio ni
/// llamada a `Nucleo` vive del lado de la UI.
///
/// Sólo captura [NucleoException] — la única excepción que `Nucleo` lanza
/// por una regla de negocio real (ver `NucleoError` en
/// mobile/rust-core/src/lib.rs). Cualquier otra excepción (incluida
/// `CancellationException`, que antes se colaba por capturar `Exception`
/// genérico y podía interferir con la cancelación de `viewModelScope`) se
/// propaga sin envolver — es un bug, no un caso de negocio esperado.
class ActivosViewModel(
    private val nucleo: Nucleo,
    private val secretoStore: SecretoDispositivoStore,
    // Inyectable para poder correr los tests con un dispatcher de tiempo
    // controlado (StandardTestDispatcher) en vez de hilos reales — sin
    // esto los tests dependerían de una carrera real entre corrutinas,
    // exactamente el tipo de cosa que no queremos dejar al azar. El valor
    // por defecto es el real que usa la app.
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var texto by mutableStateOf("")
        private set
    var modo by mutableStateOf(ModoBusqueda.ENTRADA)
        private set
    var activos by mutableStateOf<List<FilaActiva>>(emptyList())
        private set
    var resultadosBusqueda by mutableStateOf<List<ContratistaResumen>>(emptyList())
        private set
    var coincidenciasGafete by mutableStateOf<List<CoincidenciaGafete>>(emptyList())
        private set
    var error by mutableStateOf<String?>(null)
        private set

    // Aparte de `error` (fallas de consulta) — lo pone únicamente la
    // confirmación masiva de gafetes, así que no hay riesgo de que la
    // recarga de la lista tras confirmar (dispara la misma búsqueda) lo
    // borre antes de que el guardia llegue a verlo.
    var mensaje by mutableStateOf<String?>(null)
        private set
    var mensajeEsError by mutableStateOf(false)
        private set
    var enviandoGafetes by mutableStateOf(false)
        private set
    var seleccionSalida by mutableStateOf<FilaActiva?>(null)
        private set
    var seleccionIngreso by mutableStateOf<SeleccionIngreso>(SeleccionIngreso.Ninguna)
        private set
    var automatico by mutableStateOf(true)
        private set

    // Cancela la búsqueda anterior si `texto`/`modo` cambiaron antes de que
    // terminara — mismo comportamiento que daba `LaunchedEffect(texto,
    // recargas, modo)` en la versión previa (se reinicia solo si cambia su
    // key), ahora explícito porque ya no hay una key de Compose disparando
    // esto solo.
    private var trabajoBusqueda: Job? = null

    init {
        buscar()
    }

    fun cambiarTexto(nuevo: String) {
        texto = if (modo == ModoBusqueda.SALIDA_GAFETE) sanearGafetesTexto(nuevo) else nuevo
        // Mismo criterio que `cambiarTexto` en SalidaModal.tsx: escribir de
        // nuevo abandona el mensaje de la confirmación anterior.
        mensaje = null
        // Con debounce: tipear rápido no debe disparar una consulta a SQLite
        // por cada tecla (en modo gafete, una por cada número escrito) --
        // sólo la última pulsación después de una pausa llega a `buscar`.
        // `trabajoBusqueda?.cancel()` dentro de `buscar` ya mata la espera
        // anterior antes de que llegue a consultar nada.
        buscar(debounce = true)
    }

    fun cambiarAutomatico(nuevo: Boolean) {
        automatico = nuevo
    }

    fun usarDocumentoEscaneadoIngreso(valor: String) {
        modo = ModoBusqueda.ENTRADA
        texto = valor
        mensaje = null
        trabajoBusqueda?.cancel()
        trabajoBusqueda = viewModelScope.launch {
            try {
                val resultados = withContext(dispatcherIO) { nucleo.buscarContratistas(valor) }
                resultadosBusqueda = resultados
                val contratista = contratistaEscaneadoClaro(valor, resultados)
                if (contratista != null) {
                    elegir(contratista)
                }
                error = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            }
        }
    }

    fun cambiarModo(nuevo: ModoBusqueda) {
        modo = nuevo
        // Al cambiar de modo el texto que había queda escrito con otro
        // sentido (un nombre no significa nada en modo Gafete) — se limpia
        // para no arrastrar una búsqueda que ya no aplica.
        texto = ""
        mensaje = null
        buscar()
    }

    fun refrescar() {
        buscar()
    }

    /// `listarIngresosRemotos` no hace red -- lee la caché local que ya
    /// llenó la última sincronización (manual o automática) -- pero sí
    /// exige sesión autenticada, y explota (`NucleoException`) si todavía
    /// no hay una (recién abierta la app, o la nube nunca se configuró en
    /// este dispositivo). Ninguno de esos dos casos debe voltear la lista
    /// de activos LOCALES ni ensuciar `error` -- son normales, no una
    /// falla real: sin nube configurada, simplemente no hay nada remoto
    /// que mostrar.
    private suspend fun remotosSeguro(): List<IngresoRemoto> =
        try {
            withContext(dispatcherIO) { nucleo.listarIngresosRemotos() }
        } catch (_: NucleoException) {
            emptyList()
        }

    private fun buscar(debounce: Boolean = false) {
        trabajoBusqueda?.cancel()
        trabajoBusqueda = viewModelScope.launch {
            if (debounce) delay(DEBOUNCE_BUSQUEDA_MS)
            try {
                when (modo) {
                    ModoBusqueda.ENTRADA -> {
                        if (texto.isBlank()) {
                            val locales = withContext(dispatcherIO) {
                                nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA)
                            }
                            val remotos = remotosSeguro()
                            activos = locales.map { FilaActiva.Local(it) } + remotos.map { FilaActiva.Remota(it) }
                        } else {
                            resultadosBusqueda =
                                withContext(dispatcherIO) { nucleo.buscarContratistas(texto) }
                        }
                    }
                    ModoBusqueda.SALIDA_NOMBRE -> {
                        // A diferencia de Entrada, acá un campo vacío no debe
                        // traer a todo el mundo — es un buscador para acotar
                        // entre muchos activos, no una lista para recorrer
                        // (esa ya existe en el modo Entrada).
                        activos = if (texto.isBlank()) {
                            emptyList()
                        } else {
                            val locales = withContext(dispatcherIO) {
                                nucleo.listarIngresosActivos(texto, ModoBusquedaActivos.NOMBRE_CEDULA)
                            }
                            val remotos = remotosSeguro()
                                .filter { it.contratistaNombre.contains(texto, ignoreCase = true) }
                            locales.map { FilaActiva.Local(it) } + remotos.map { FilaActiva.Remota(it) }
                        }
                    }
                    ModoBusqueda.SALIDA_GAFETE -> {
                        val numeros = gafetesDeTexto(texto)
                        coincidenciasGafete = if (numeros.isEmpty()) {
                            emptyList()
                        } else {
                            withContext(dispatcherIO) {
                                numeros.map { numero ->
                                    val resultado =
                                        nucleo.listarIngresosActivos(numero.toString(), ModoBusquedaActivos.GAFETE)
                                    CoincidenciaGafete(numero, resultado.firstOrNull())
                                }
                            }
                        }
                    }
                }
                error = null
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoNoEncontradoException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoStoreException) {
                error = excepcion.message
            }
        }
    }

    fun elegir(contratista: ContratistaResumen) {
        viewModelScope.launch {
            seleccionIngreso = SeleccionIngreso.Cargando(contratista)
            try {
                val preparacion = withContext(dispatcherIO) { nucleo.prepararIngreso(contratista.id) }
                seleccionIngreso = if (puedeContinuar(preparacion)) {
                    SeleccionIngreso.Formulario(preparacion)
                } else {
                    SeleccionIngreso.Bloqueada(preparacion, mensajeBloqueo(preparacion))
                }
            } catch (excepcion: NucleoException) {
                error = excepcion.message
                seleccionIngreso = SeleccionIngreso.Ninguna
            }
        }
    }

    fun cancelarSeleccionIngreso() {
        seleccionIngreso = SeleccionIngreso.Ninguna
    }

    fun onIngresoRegistrado() {
        CambiosNube.solicitar()
        seleccionIngreso = SeleccionIngreso.Ninguna
        texto = ""
        buscar()
    }

    fun elegirSeleccionSalida(fila: FilaActiva?) {
        seleccionSalida = fila
    }

    /// Local: cierra en `registro_ingresos` (este dispositivo). Remota:
    /// cierra directo contra la nube (`Nucleo.cerrarIngresoRemoto`) -- nunca
    /// toca el historial local, esa fila no es -- ni fue -- de este
    /// teléfono. Mismo criterio que `cerrarFila` en Activos.tsx de desktop.
    fun confirmarSalida(fila: FilaActiva) {
        seleccionSalida = null
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    when (fila) {
                        is FilaActiva.Local -> nucleo.registrarSalida(fila.activo.registroId)
                        is FilaActiva.Remota -> {
                            val secreto = secretoStore.cargar()
                                ?: throw SecretoDispositivoNoEncontradoException()
                            nucleo.cerrarIngresoRemotoConSecreto(secreto, fila.remoto.uuid)
                        }
                    }
                }
                CambiosNube.solicitar()
                buscar()
            } catch (excepcion: NucleoException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoNoEncontradoException) {
                error = excepcion.message
            } catch (excepcion: SecretoDispositivoStoreException) {
                error = excepcion.message
            }
        }
    }

    fun registrarSalidaPorGafetes() {
        viewModelScope.launch {
            enviandoGafetes = true
            val registrados = mutableListOf<String>()
            val fallidos = mutableListOf<String>()
            for (coincidencia in coincidenciasGafete) {
                val activoCoincidente = coincidencia.activo
                if (activoCoincidente == null) {
                    fallidos.add("gafete ${coincidencia.numero}: sin ingreso activo")
                    continue
                }
                try {
                    withContext(dispatcherIO) { nucleo.registrarSalida(activoCoincidente.registroId) }
                    CambiosNube.solicitar()
                    registrados.add(activoCoincidente.contratistaNombre)
                } catch (excepcion: NucleoException) {
                    fallidos.add("gafete ${coincidencia.numero}: ${excepcion.message}")
                }
            }
            val partes = mutableListOf<String>()
            if (registrados.isNotEmpty()) {
                partes.add("Salida registrada: ${registrados.joinToString(", ")}")
            }
            if (fallidos.isNotEmpty()) {
                partes.add(fallidos.joinToString(" · "))
            }
            mensaje = partes.joinToString(" · ").ifEmpty { null }
            mensajeEsError = registrados.isEmpty() && fallidos.isNotEmpty()
            texto = ""
            enviandoGafetes = false
            buscar()
        }
    }

    fun registrarSalidaPorGafeteEscaneado(valor: String) {
        val numero = valor.filter(Char::isDigit).toIntOrNull()
        if (numero == null) {
            mensaje = "Gafete no válido"
            mensajeEsError = true
            return
        }
        modo = ModoBusqueda.SALIDA_GAFETE
        texto = numero.toString()
        mensaje = null
        trabajoBusqueda?.cancel()
        trabajoBusqueda = viewModelScope.launch {
            enviandoGafetes = true
            try {
                val activo = withContext(dispatcherIO) {
                    nucleo.listarIngresosActivos(numero.toString(), ModoBusquedaActivos.GAFETE).firstOrNull()
                }
                coincidenciasGafete = listOf(CoincidenciaGafete(numero, activo))
                if (activo == null) {
                    mensaje = "Gafete $numero: sin ingreso activo"
                    mensajeEsError = true
                    return@launch
                }
                withContext(dispatcherIO) { nucleo.registrarSalida(activo.registroId) }
                CambiosNube.solicitar()
                mensaje = "Salida registrada: ${activo.contratistaNombre}"
                mensajeEsError = false
                texto = ""
                coincidenciasGafete = emptyList()
            } catch (excepcion: NucleoException) {
                mensaje = "Gafete $numero: ${excepcion.message}"
                mensajeEsError = true
            } finally {
                enviandoGafetes = false
            }
        }
    }

    companion object {
        // Ver el comentario en `cambiarTexto`. 300ms es el mismo orden de
        // magnitud que usan la mayoría de buscadores con debounce -- ya no
        // se siente el retraso al escribir, pero absorbe una racha normal
        // de tecleo.
        private const val DEBOUNCE_BUSQUEDA_MS = 300L

        fun factory(
            nucleo: Nucleo,
            secretoStore: SecretoDispositivoStore,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer { ActivosViewModel(nucleo, secretoStore) }
        }
    }
}

private fun contratistaEscaneadoClaro(valor: String, resultados: List<ContratistaResumen>): ContratistaResumen? {
    if (resultados.size == 1) return resultados.single()
    val digitos = valor.filter(Char::isDigit)
    if (digitos.isEmpty()) return null
    return resultados.singleOrNull { it.cedula.filter(Char::isDigit) == digitos }
}

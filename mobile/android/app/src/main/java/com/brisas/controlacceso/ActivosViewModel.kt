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
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import uniffi.control_acceso_mobile.ContratistaResumen
import uniffi.control_acceso_mobile.IngresoActivoResumen
import uniffi.control_acceso_mobile.IngresoRemoto
import uniffi.control_acceso_mobile.MedioIngreso
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
/// `null` si nadie adentro tiene ese gafete puesto). Puede ser local o
/// remoto: un ingreso con gafete registrado en la PC del puesto de control
/// vive en la caché `ingresos_remotos`, no en el historial del teléfono, y
/// también tiene que poder cerrarse por número desde acá — la fila de
/// `CoincidenciaGafete` es lo que se pinta en el modo Salida: gafete antes
/// de confirmar, mismo rol que la tabla de vista previa de
/// `SalidaModal.tsx` en desktop.
data class CoincidenciaGafete(val numero: Int, val fila: FilaActiva?)

/// Fila local (este dispositivo) o remota (abierta por el otro dispositivo
/// del mismo sitio, cacheada en `ingresos_remotos` -- nunca vive en el
/// historial local) -- mismo espíritu que `FilaActiva` en Activos.tsx de
/// desktop: una sola lista para las dos, la pantalla no necesita dos
/// secciones separadas para poder cerrar cualquiera de las dos desde acá.
sealed class FilaActiva {
    abstract val contratistaNombre: String
    abstract val empresaNombre: String?

    data class Local(val activo: IngresoActivoResumen) : FilaActiva() {
        override val contratistaNombre get() = activo.contratistaNombre
        override val empresaNombre get() = activo.empresaNombre
    }

    data class Remota(val remoto: IngresoRemoto) : FilaActiva() {
        override val contratistaNombre get() = remoto.contratistaNombre
        override val empresaNombre get() = remoto.empresaNombre
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
/// [Nucleo] — ver mobile/android/arquitectura.md. El `@Composable` sólo lee
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
    var cargando by mutableStateOf(false)
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

    // Cancela la búsqueda anterior si `texto`/`modo` cambiaron antes de que
    // terminara — mismo comportamiento que daba `LaunchedEffect(texto,
    // recargas, modo)` en la versión previa (se reinicia solo si cambia su
    // key), ahora explícito porque ya no hay una key de Compose disparando
    // esto solo.
    private var trabajoBusqueda: Job? = null
    private var versionBusqueda = 0L
    // Las escrituras de entrada/salida no comparten Job con las búsquedas.
    // Cancelar un debounce jamás debe cancelar una operación SQLite/UniFFI
    // que ya pudo haber producido un efecto irreversible.
    private val mutexMutaciones = Mutex()

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

    fun usarDocumentoEscaneadoIngreso(documento: DocumentoDetectado) {
        val valor = documento.textoBusqueda ?: documento.numeroDocumento
        modo = ModoBusqueda.ENTRADA
        texto = valor
        mensaje = null
        trabajoBusqueda?.cancel()
        trabajoBusqueda = viewModelScope.launch {
            try {
                val resultados = withContext(dispatcherIO) { medirNucleo("buscarContratistas") { nucleo.buscarContratistas(valor) } }
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
            withContext(dispatcherIO) { medirNucleo("listarIngresosRemotos") { nucleo.listarIngresosRemotos() } }
        } catch (_: NucleoException) {
            emptyList()
        }

    private fun buscar(debounce: Boolean = false) {
        trabajoBusqueda?.cancel()
        val version = ++versionBusqueda
        cargando = true
        trabajoBusqueda = viewModelScope.launch {
            try {
                if (debounce) delay(DEBOUNCE_BUSQUEDA_MS)
                when (modo) {
                    ModoBusqueda.ENTRADA -> {
                        if (texto.isBlank()) {
                            val locales = withContext(dispatcherIO) {
                                medirNucleo("listarIngresosActivos") { nucleo.listarIngresosActivos("", ModoBusquedaActivos.NOMBRE_CEDULA) }
                            }
                            val remotos = remotosSeguro()
                            activos = locales.map { FilaActiva.Local(it) } + remotos.map { FilaActiva.Remota(it) }
                        } else {
                            resultadosBusqueda =
                                withContext(dispatcherIO) { medirNucleo("buscarContratistas") { nucleo.buscarContratistas(texto) } }
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
                                medirNucleo("listarIngresosActivos") { nucleo.listarIngresosActivos(texto, ModoBusquedaActivos.NOMBRE_CEDULA) }
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
                            // Una sola lectura de la caché remota para todos
                            // los números escritos, no una por gafete.
                            val remotos = remotosSeguro()
                            numeros.map { numero -> CoincidenciaGafete(numero, filaPorGafete(numero, remotos)) }
                        }
                    }
                }
                error = null
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            } finally {
                if (version == versionBusqueda) cargando = false
            }
        }
    }

    /// Primero el historial local (lo abierto en este teléfono); si nadie
    /// adentro tiene ese gafete acá, la caché `ingresos_remotos` (lo abierto
    /// en otro dispositivo del sitio, p. ej. la PC del puesto de control).
    /// Antes sólo se miraba lo local, así que un ingreso con gafete hecho en
    /// la PC aparecía como "sin ingreso activo" en el teléfono.
    private suspend fun filaPorGafete(numero: Int, remotos: List<IngresoRemoto>): FilaActiva? {
        val local = withContext(dispatcherIO) {
            medirNucleo("listarIngresosActivos") { nucleo.listarIngresosActivos(numero.toString(), ModoBusquedaActivos.GAFETE) }
        }.firstOrNull()
        if (local != null) return FilaActiva.Local(local)
        return remotos.firstOrNull { it.gafeteNumero == numero.toLong() }?.let { FilaActiva.Remota(it) }
    }

    /// Local: cierra en `registro_ingresos` (este dispositivo). Remota:
    /// cierra directo contra la nube (`Nucleo.cerrarIngresoRemoto`) -- nunca
    /// toca el historial local, esa fila no es -- ni fue -- de este
    /// teléfono. Mismo criterio que `cerrarFila` en Activos.tsx de desktop.
    /// Único camino de cierre para las tres formas de sacar a alguien
    /// (selección, gafetes escritos y gafete escaneado).
    private suspend fun cerrarFila(fila: FilaActiva) {
        withContext(dispatcherIO) {
            when (fila) {
                is FilaActiva.Local -> medirNucleo("registrarSalida") { nucleo.registrarSalida(fila.activo.registroId) }
                is FilaActiva.Remota -> medirNucleo("cerrarIngresoRemoto") { nucleo.cerrarIngresoRemoto(fila.remoto.uuid) }
            }
        }
        CambiosNube.cambioLocal()
    }

    fun elegir(contratista: ContratistaResumen) {
        viewModelScope.launch {
            seleccionIngreso = SeleccionIngreso.Cargando(contratista)
            errorIngreso = null
            try {
                // Un solo cruce FFI: Rust decide localmente y, si los
                // chequeos locales ya dejaron pasar, intenta además el
                // chequeo cruzado entre sitios (`docs/pendientes.md`) si el
                // teléfono está vinculado. `mensajeBloqueo` llega ya
                // resuelto (o `null` si se puede continuar); acá no se
                // evalúa ninguna condición propia, sólo se lee el resultado.
                // La clave del teléfono la usa el núcleo a través de
                // `AlmacenClaveKeystore`; un fallo del Keystore llega como
                // `NucleoException` y se trata abajo (hallazgo MV-05).
                val preparacion = withContext(dispatcherIO) {
                    medirNucleo("prepararIngresoVerificado") { nucleo.prepararIngresoVerificado(contratista.id) }
                }
                seleccionIngreso = preparacion.mensajeBloqueo?.let { mensaje ->
                    SeleccionIngreso.Bloqueada(preparacion, mensaje)
                } ?: SeleccionIngreso.Formulario(preparacion)
            } catch (excepcion: NucleoException) {
                error = excepcion.message
                seleccionIngreso = SeleccionIngreso.Ninguna
            } finally {
                // Red de seguridad: si algo inesperado escapó de los catch
                // de arriba, no dejar la pantalla trabada en "Cargando"
                // para siempre (parte del hallazgo MV-05).
                if (seleccionIngreso is SeleccionIngreso.Cargando) {
                    seleccionIngreso = SeleccionIngreso.Ninguna
                }
            }
        }
    }

    fun cancelarSeleccionIngreso() {
        seleccionIngreso = SeleccionIngreso.Ninguna
        errorIngreso = null
    }

    /// Error del formulario de ingreso abierto (validación o del núcleo).
    /// Se limpia al abrir/cerrar/registrar, así no arrastra el intento de un
    /// contratista al siguiente.
    var errorIngreso by mutableStateOf<String?>(null)
        private set
    var registrandoIngreso by mutableStateOf(false)
        private set

    fun limpiarErrorIngreso() {
        errorIngreso = null
    }

    /// Registra el ingreso del contratista del formulario abierto (punto M2
    /// de la auditoría móvil: antes lo hacía `PantallaConfirmarIngreso`
    /// directo contra el núcleo). Las reglas de gafete y placa las aplica el
    /// núcleo; acá sólo se hace el chequeo
    /// de "gafete ocupado en otro dispositivo del sitio" + la escritura en
    /// UNA sola llamada (`registrarIngresoVerificado`): antes eran dos
    /// cruces FFI con una ventana entre medio donde otro dispositivo podía
    /// colarse. Corre en `viewModelScope`, así que salir de la pantalla a
    /// mitad de camino ya no se salta el aviso a la nube.
    fun registrarIngreso(medio: MedioIngreso, gafeteTexto: String, placaTexto: String) {
        val preparacion = (seleccionIngreso as? SeleccionIngreso.Formulario)?.preparacion ?: return
        if (registrandoIngreso) return
        errorIngreso = null
        // Sólo se convierte el texto a número; si el gafete falta, o la
        // placa no corresponde al medio, lo decide y lo dice el núcleo.
        val gafete: Long? = if (preparacion.requiereGafete && gafeteTexto.isNotBlank()) {
            gafeteTexto.trim().toLongOrNull() ?: run {
                errorIngreso = "Ingrese un número de gafete válido"
                return
            }
        } else {
            null
        }
        registrandoIngreso = true
        viewModelScope.launch {
            try {
                withContext(dispatcherIO) {
                    medirNucleo("registrarIngresoVerificado") { nucleo.registrarIngresoVerificado(preparacion.contratistaId, medio, gafete, placaTexto) }
                }
                onIngresoRegistrado()
            } catch (excepcion: Exception) {
                errorIngreso = excepcion.mensajeDeErrorEsperado()
            } finally {
                registrandoIngreso = false
            }
        }
    }

    fun onIngresoRegistrado() {
        CambiosNube.cambioLocal()
        seleccionIngreso = SeleccionIngreso.Ninguna
        errorIngreso = null
        texto = ""
        buscar()
    }

    fun elegirSeleccionSalida(fila: FilaActiva?) {
        seleccionSalida = fila
    }

    /// Ver [cerrarFila].
    fun confirmarSalida(fila: FilaActiva) {
        seleccionSalida = null
        viewModelScope.launch {
            try {
                mutexMutaciones.withLock { cerrarFila(fila) }
                buscar()
            } catch (excepcion: Exception) {
                error = excepcion.mensajeDeErrorEsperado()
            }
        }
    }

    fun registrarSalidaPorGafetes() {
        if (enviandoGafetes) return
        enviandoGafetes = true
        val coincidencias = coincidenciasGafete
        viewModelScope.launch {
            try {
                mutexMutaciones.withLock {
                    val registrados = mutableListOf<String>()
                    val fallidos = mutableListOf<String>()
                    for (coincidencia in coincidencias) {
                        val fila = coincidencia.fila
                        if (fila == null) {
                            fallidos.add("gafete ${coincidencia.numero}: sin ingreso activo")
                            continue
                        }
                        try {
                            cerrarFila(fila)
                            registrados.add(fila.contratistaNombre)
                        } catch (excepcion: Exception) {
                            // Un gafete que falla (p. ej. remoto sin red) no
                            // corta el resto del lote; lo inesperado se
                            // relanza desde `mensajeDeErrorEsperado`.
                            fallidos.add("gafete ${coincidencia.numero}: ${excepcion.mensajeDeErrorEsperado()}")
                        }
                    }
                    val partes = mutableListOf<String>()
                    if (registrados.isNotEmpty()) partes.add("Salida registrada: ${registrados.joinToString(", ")}")
                    if (fallidos.isNotEmpty()) partes.add(fallidos.joinToString(" · "))
                    mensaje = partes.joinToString(" · ").ifEmpty { null }
                    mensajeEsError = registrados.isEmpty() && fallidos.isNotEmpty()
                    texto = ""
                }
                buscar()
            } finally {
                enviandoGafetes = false
            }
        }
    }

    suspend fun registrarSalidaPorGafeteEscaneado(documento: DocumentoDetectado) {
        val valor = documento.textoBusqueda ?: documento.numeroDocumento
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
        // El trabajo pertenece al ViewModel, no a la pantalla de cámara.
        // Si Android recompone o cierra la cámara mientras Rust está
        // escribiendo, la mutación concluye y deja un resultado coherente.
        val mutacion = viewModelScope.async {
            mutexMutaciones.withLock {
                enviandoGafetes = true
                try {
                    val fila = filaPorGafete(numero, remotosSeguro())
                    coincidenciasGafete = listOf(CoincidenciaGafete(numero, fila))
                    if (fila == null) {
                        mensaje = "Gafete $numero: sin ingreso activo"
                        mensajeEsError = true
                    } else {
                        cerrarFila(fila)
                        mensaje = "Salida registrada: ${fila.contratistaNombre}"
                        mensajeEsError = false
                        texto = ""
                        coincidenciasGafete = emptyList()
                    }
                } catch (excepcion: Exception) {
                    mensaje = "Gafete $numero: ${excepcion.mensajeDeErrorEsperado()}"
                    mensajeEsError = true
                } finally {
                    enviandoGafetes = false
                }
            }
        }
        mutacion.await()
    }

    companion object {
        // Ver el comentario en `cambiarTexto`. Bajado de 300ms a 150ms
        // (pedido explícito del usuario, 2026-09-21, tras probar 200ms y
        // confirmar que la búsqueda local no lo resiente) -- mismo valor
        // que el default de `BuscadorConDebounce` (MV-10, auditoría
        // 2026-09-24), que usan los otros 6 buscadores de la app. Este
        // ViewModel se queda con su propio `Job`/constante porque comparte
        // ese `Job` con `versionBusqueda`/`cargando` para varios modos a la
        // vez -- no encaja en la forma genérica sin exponer esos dos
        // conceptos también ahí.
        private const val DEBOUNCE_BUSQUEDA_MS = 150L

        fun factory(nucleo: Nucleo): ViewModelProvider.Factory = viewModelFactory {
            initializer { ActivosViewModel(nucleo) }
        }
    }
}

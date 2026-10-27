package com.brisas.controlacceso

import android.Manifest
import android.app.Activity
import android.os.Build
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.NotificationImportant
import androidx.compose.material.icons.filled.Route
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material.icons.filled.PersonAdd
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.EstadoBandeja
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.UsuarioSesion

/// App limitada a registros rápidos + historial (decisión 2026-09-06),
/// con una única excepción reincorporada el 2026-09-12: el alta de
/// contratista (`PantallaNuevoContratista`, ícono "+" de la barra
/// superior), ahora con OCR de carnet PRAIND además del formulario manual
/// original. El resto de la decisión sigue en pie -- empresa y usuario
/// nuevos siguen sin pantalla propia acá (escritorio/web/CLI), y tampoco
/// volvió la pestaña de nube.
///
/// `mostrarNuevoContratista`, `mostrarAvisos` (bandeja de la campanita,
/// 2026-10-18) y `mostrarAyuda` (2026-10-24) son toda la navegación que hace
/// falta -- con pocas pantallas
/// adicionales que nunca se abren a la vez no se justifica un enum
/// `Pantalla` como el que existió antes: la que está en `true` reemplaza por
/// completo la Column de pestañas, igual que hacía aquel enum, sin la
/// indirección de más.
@Composable
fun PantallaPrincipal(
    nucleo: Nucleo,
    sesion: UsuarioSesion,
    directorio: String,
    metadata: MetadatosDispositivoLocal,
    onCerrarSesion: () -> Unit,
    // Cierre por inactividad (teléfono bloqueado o app minimizada mucho
    // rato): el login vuelve con la cédula puesta, sólo falta la contraseña.
    onBloquear: () -> Unit = onCerrarSesion,
) {
    RegistrarPantalla("principal")
    var refrescarNube by remember { mutableIntStateOf(0) }
    var mostrarNuevoContratista by remember { mutableStateOf(false) }
    // Lo que se buscó en Activos sin encontrar a nadie, para arrancar el alta
    // con eso (vacío desde el ícono de arriba).
    var textoNuevoContratista by remember { mutableStateOf("") }
    // Bandeja de avisos (campanita): otra pantalla que reemplaza a las
    // pestañas, igual que `mostrarNuevoContratista`.
    var mostrarAvisos by remember { mutableStateOf(false) }
    // La Ayuda (manual de uso, desde el menú de la cuenta): otra más, igual.
    var mostrarAyuda by remember { mutableStateOf(false) }
    // Reconectar este teléfono retirado en el panel (ver
    // `PantallaReconectarEquipo`): otra más. `equipoRetirado` lo decide el
    // núcleo; se vuelve a preguntar tras cada sincronización y al llegar la
    // expulsión en vivo.
    var mostrarReconexion by remember { mutableStateOf(false) }
    var equipoRetirado by remember { mutableStateOf(false) }
    // La pestaña elegida y, dentro de Externos, el tipo (Proveedor o Correo,
    // que cambia el color de la sección y de la pestaña de arriba: ver
    // `AcentoSeccion.kt`). Viven acá y no junto a las pestañas: al abrir la
    // bandeja o "Nuevo contratista" las pestañas salen de la composición, y
    // al volver se caía siempre en Activos.
    var pestana by rememberSaveable { mutableIntStateOf(0) }
    var indiceExterno by rememberSaveable { mutableIntStateOf(0) }
    // Sin esto, atrás del sistema en la raíz (pestañas) caía directo al
    // default de Android -- cerraba/minimizaba la app de un solo toque, sin
    // que hubiera ningún BackHandler compose que lo interceptara (hallazgo
    // 2026-09-19, pedido explícito del usuario: doble tap para salir, mismo
    // patrón estándar de Android). `enabled = !mostrarNuevoContratista`
    // para no competir con el BackHandler propio de esa pantalla cuando
    // está abierta.
    val contexto = LocalContext.current
    var ultimoTapAtras by remember { mutableLongStateOf(0L) }
    BackHandler(enabled = !mostrarNuevoContratista && !mostrarAvisos && !mostrarAyuda && !mostrarReconexion) {
        val ahora = System.currentTimeMillis()
        if (ahora - ultimoTapAtras < 2000) {
            (contexto as? Activity)?.finish()
        } else {
            ultimoTapAtras = ahora
            Toast.makeText(contexto, "Toque de nuevo para salir", Toast.LENGTH_SHORT).show()
        }
    }
    // Notificaciones push (NotificacionesPush.kt): se piden acá, con la
    // sesión ya abierta, y no en el primer arranque -- quien opera ya sabe
    // para qué es la app. Si lo niega, Android deja de mostrar el diálogo
    // después del segundo rechazo; la app sigue igual, sin avisos con la app
    // cerrada.
    //
    // El permiso se pide aunque Firebase no esté disponible: antes dependía
    // de eso, y un APK compilado sin google-services.json no pedía nada ni
    // decía por qué (visto en la primera prueba en un teléfono real). En
    // los builds de prueba se avisa en pantalla que a ese APK le falta la
    // configuración de Firebase.
    val pedirPermisoNotificaciones = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {}
    LaunchedEffect(Unit) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU && !NotificacionesPush.permisoConcedido(contexto)) {
            pedirPermisoNotificaciones.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
        if (NotificacionesPush.disponible(contexto)) {
            NotificacionesPush.tokenActual(contexto)
            NotificacionesPush.registrarEnNube(contexto, nucleo)
        } else if (BuildConfig.DEBUG || BuildConfig.AMBIENTE_STAGING) {
            Toast.makeText(contexto, "Este APK no trae google-services.json: sin notificaciones push", Toast.LENGTH_LONG).show()
        }
    }
    // Bandeja de avisos (campanita): el estado lo decide el núcleo y lo
    // publica AvisosViewModel; se actualiza solo cuando llega un push.
    val avisosViewModel: AvisosViewModel = viewModel(factory = AvisosViewModel.factory(contexto, nucleo))
    val bandeja by avisosViewModel.estado.collectAsState()
    // Tocaron la notificación de un aviso (MainActivity): abrir la bandeja.
    val aperturaAvisos by avisosViewModel.aperturaSolicitada.collectAsState()
    LaunchedEffect(aperturaAvisos) {
        if (aperturaAvisos) {
            mostrarNuevoContratista = false
            mostrarAyuda = false
            mostrarAvisos = true
            avisosViewModel.aperturaAtendida()
        }
    }
    // Con la bandeja a la vista, que "hace 5 min" avance.
    LaunchedEffect(mostrarAvisos) {
        while (mostrarAvisos) {
            delay(60_000)
            avisosViewModel.refrescar()
        }
    }
    // `docs/pendientes.md`, "alertar luego al sincronizar": los conflictos de
    // la última sincronización (contratistas, proveedores y correo activos
    // también en otro sitio, y gafetes que la nube rechazó), ya redactados
    // por el núcleo (`ResumenSincronizacion.mensajesConflictos`). Alimentado
    // desde los dos caminos de sync (pulso periódico y botón manual), igual
    // que `refrescarNube`.
    var mensajesConflictos by remember { mutableStateOf<List<String>>(emptyList()) }
    // La sesión se cerró desde la nube (usuario desactivado, o entró en otra
    // unidad): se avisa sólo en el segundo caso, que no es obvio para quien
    // tiene el teléfono en la mano.
    val alCerrarseLaSesion: (Boolean) -> Unit = { enOtraUnidad ->
        if (enOtraUnidad) {
            Toast.makeText(contexto, ExpulsionNube.MENSAJE_SESION_EN_OTRA_UNIDAD, Toast.LENGTH_LONG).show()
        }
        onCerrarSesion()
    }
    val nubeViewModel: NubeViewModel =
        viewModel(
            factory = NubeViewModel.factory(nucleo, alCerrarseLaSesion),
        )
    // Los avisos cortos que bajan desde arriba (ver `FranjaEstado.kt`), en vez
    // de Toasts que salían abajo tapando la barra de secciones.
    val avisosBreves = remember { AvisosBreves() }
    val scope = rememberCoroutineScope()
    val realtime = remember(nucleo, scope, sesion) {
        NubeRealtime(
            nucleo = nucleo,
            scope = scope,
            usuarioCedula = sesion.cedula,
            usuarioNombre = sesion.nombre,
            // Aviso en vivo con la fila ya guardada: refresca Activos al
            // instante, sin esperar la sincronización.
            onCambioAplicado = { refrescarNube += 1 },
            // El canal corre en `scope` (hilo principal): el aviso puede
            // mostrarse directo. El trabajo local sigue disponible.
            onExpulsado = {
                avisosBreves.mostrar(ExpulsionNube.MENSAJE, TipoEstado.AVISO)
                equipoRetirado = true
            },
        )
    }
    LaunchedEffect(refrescarNube, nubeViewModel.error) {
        equipoRetirado = withContext(Dispatchers.IO) { nucleo.equipoRetirado() }
    }
    val sincronizacion = remember(nucleo, scope) {
        SincronizacionPeriodica(
            nucleo = nucleo,
            scope = scope,
            onSincronizado = { resumen ->
                // Si a esta sesión la desactivaron en otro dispositivo, el
                // pulso periódico (o Realtime, que dispara por el mismo
                // camino) ya trajo la baja -- cerrar sesión acá, no sólo
                // refrescar pantallas que ya no deberían verse.
                if (resumen.sesionExpulsada) {
                    alCerrarseLaSesion(resumen.sesionEnOtraUnidad)
                } else {
                    refrescarNube += 1
                    mensajesConflictos = resumen.mensajesConflictos
                }
            },
        )
    }

    // Atado a ON_START/ON_STOP, no sólo a la composición: sin esto, el pulso
    // periódico seguía vivo aunque el guardia bloqueara el teléfono o
    // cambiara de app -- `PantallaPrincipal` sigue en composición mientras
    // dure la sesión, la Activity no se destruye sólo por pasar a segundo
    // plano. Gastaba batería sin ningún beneficio: nadie ve pantalla para
    // que un refresco importe. Vuelve a sincronizar solo al volver al
    // primer plano.
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(sincronizacion, realtime, lifecycleOwner) {
        var trabajoBloqueo: Job? = null
        // Al volver de segundo plano la sincronización sale sin esperar a
        // Realtime (ver `SincronizacionPeriodica.iniciar`); en el primer
        // arranque no, porque el login ya lanzó una.
        var volviendoDeSegundoPlano = false
        fun iniciarServicios() {
            trabajoBloqueo?.cancel()
            trabajoBloqueo = null
            sincronizacion.iniciar(inmediata = volviendoDeSegundoPlano)
            volviendoDeSegundoPlano = false
            realtime.iniciar()
        }
        fun detenerServiciosYProgramarBloqueo() {
            volviendoDeSegundoPlano = true
            realtime.detener()
            sincronizacion.detener()
            trabajoBloqueo?.cancel()
            trabajoBloqueo = scope.launch {
                delay(DEMORA_BLOQUEO_SESION_MS)
                onBloquear()
            }
        }
        val observador = LifecycleEventObserver { _, evento ->
            when (evento) {
                Lifecycle.Event.ON_START -> iniciarServicios()
                Lifecycle.Event.ON_STOP -> detenerServiciosYProgramarBloqueo()
                else -> {}
            }
        }
        lifecycleOwner.lifecycle.addObserver(observador)
        if (lifecycleOwner.lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)) {
            iniciarServicios()
        }
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observador)
            trabajoBloqueo?.cancel()
            realtime.detener()
            sincronizacion.detener()
        }
    }

    // Tirar de una lista hacia abajo (sincronización manual) también
    // refresca las pantallas al terminar -- mismo camino que ya usa el pulso
    // periódico (`onSincronizado` más arriba), para que se sienta instantáneo
    // en vez de esperar al próximo ciclo de 5 minutos.
    LaunchedEffect(nubeViewModel.ultimoResumen) {
        val resumen = nubeViewModel.ultimoResumen
        if (resumen != null) {
            refrescarNube += 1
            mensajesConflictos = resumen.mensajesConflictos
        }
    }

    // Se relee al terminar cada sincronización: es cuando puede llegar un
    // token con la unidad cambiada. Sólo el nombre de la unidad, en mayúsculas:
    // la etiqueta del equipo se ve en el login, acá saturaba.
    val textoUnidad = rememberTextoUnidadOperativa(nucleo, recargar = nubeViewModel.sincronizando)

    // Cuántos hay adentro en cada sección, para los globitos de la barra de
    // abajo. Son los mismos ViewModels que usan las pantallas (los guarda la
    // Activity), así que los números se mueven solos con cada registro.
    val activosViewModel: ActivosViewModel = viewModel(factory = ActivosViewModel.factory(nucleo))
    val gafetesViewModel: GafetesProvisionalesViewModel =
        viewModel(factory = GafetesProvisionalesViewModel.factory(nucleo))
    val proveedoresViewModel: ProveedoresViewModel = viewModel(factory = ProveedoresViewModel.factory(nucleo))
    val correoViewModel: CorreoViewModel = viewModel(factory = CorreoViewModel.factory(nucleo))
    // Recordado por las cuentas: un mapa nuevo en cada recomposición de esta
    // pantalla obligaba a la barra de secciones a recomponerse también.
    val totalActivos = activosViewModel.totalAdentro ?: 0
    val totalKof = gafetesViewModel.activos.size
    val totalExternos = proveedoresViewModel.activos.size + correoViewModel.activos.size
    val adentro = remember(totalActivos, totalKof, totalExternos) {
        mapOf(
            SeccionPrincipal.ACTIVOS to totalActivos,
            SeccionPrincipal.KOF to totalKof,
            SeccionPrincipal.EXTERNOS to totalExternos,
        )
    }
    // "Ingreso registrado · …" y compañía, de cualquier sección, arriba.
    ReenviarConfirmaciones(
        avisosBreves,
        activosViewModel.confirmacion,
        gafetesViewModel.confirmacion,
        proveedoresViewModel.confirmacion,
        correoViewModel.confirmacion,
    )
    // Cada pantalla se refresca sola tras una sincronización mientras está a la
    // vista; las que no se ven se refrescan acá para que su globito no quede
    // atrasado.
    LaunchedEffect(refrescarNube) {
        if (refrescarNube == 0) return@LaunchedEffect
        val seccion = SECCIONES_VISIBLES.getOrElse(pestana) { SeccionPrincipal.ACTIVOS }
        val externo = tipoExternoVisible(indiceExterno)
        val fueraDePestanas = mostrarAvisos || mostrarNuevoContratista || mostrarAyuda || mostrarReconexion
        val enExternos = seccion == SeccionPrincipal.EXTERNOS && !fueraDePestanas
        if (seccion != SeccionPrincipal.ACTIVOS || fueraDePestanas) activosViewModel.refrescar()
        if (seccion != SeccionPrincipal.KOF || fueraDePestanas) gafetesViewModel.refrescarActivos()
        if (!(enExternos && externo == TipoExterno.PROVEEDOR)) proveedoresViewModel.refrescarActivos()
        if (!(enExternos && externo == TipoExterno.POR_CORREO)) correoViewModel.refrescarActivos()
    }

    // Qué ocupa la pantalla debajo de la barra superior.
    val panel = when {
        mostrarAvisos -> PanelPrincipal.AVISOS
        mostrarNuevoContratista -> PanelPrincipal.NUEVO_CONTRATISTA
        mostrarAyuda -> PanelPrincipal.AYUDA
        mostrarReconexion -> PanelPrincipal.RECONEXION
        else -> PanelPrincipal.PESTANAS
    }
    // Cámaras y formularios dentro de las pestañas (ver [OcultarBarraSecciones]):
    // esconden la barra de secciones de abajo y también la de arriba.
    val pantallasCompletas = remember { PantallasCompletas() }
    // La acción del botón redondo y la búsqueda de la píldora las pone la
    // pantalla a la vista.
    val accionesInferiores = remember { AccionesInferiores() }
    val seccion = SECCIONES_VISIBLES.getOrElse(pestana) { SeccionPrincipal.ACTIVOS }
    val externo = tipoExternoVisible(indiceExterno)

    CompositionLocalProvider(LocalAvisosBreves provides avisosBreves) {
    Box(modifier = Modifier.fillMaxSize()) {
    Column(modifier = Modifier.fillMaxSize()) {
        // La barra de arriba sólo con las pestañas a la vista: las pantallas
        // que se abren encima (avisos, ayuda, formularios, cámaras) traen su
        // propio "‹ Volver" y su título, como una pantalla nueva de iOS.
        if (panel == PanelPrincipal.PESTANAS && pantallasCompletas.cantidad == 0) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 4.dp, bottom = 6.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // El título grande es la sección a la vista (como el de cada
            // pestaña de iOS) y, debajo, la unidad. Cuántos hay adentro lo
            // dice el globito de la barra de abajo; repetirlo acá sobraba. El
            // nombre de la app ya se ve en el login. Quién tiene la sesión
            // vive en el menú de la cuenta.
            Column(modifier = Modifier.weight(1f)) {
                AnimatedContent(
                    targetState = seccion,
                    transitionSpec = { (fadeIn(tween(MS_FUNDIDO_ENTRADA)) togetherWith fadeOut(tween(MS_SALIDA))) using null },
                    label = "tituloSeccion",
                ) { seccionTitulo ->
                    Text(
                        seccionTitulo.etiqueta,
                        style = MaterialTheme.typography.headlineMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                if (textoUnidad != null) {
                    Text(
                        textoUnidad,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                BotonIconoCuadradoBrisas(onClick = {
                    mostrarAvisos = false
                    mostrarAyuda = false
                    textoNuevoContratista = ""
                    mostrarNuevoContratista = true
                }) {
                    Icon(Icons.Default.PersonAdd, contentDescription = "Nuevo contratista")
                }
                CampanitaAvisos(bandeja, onClick = {
                    mostrarNuevoContratista = false
                    mostrarAyuda = false
                    mostrarAvisos = true
                })
                // Sin botón "Sincronizar": cada lista sincroniza tirando hacia
                // abajo (ver `ListaConEstados`), y además está el pulso
                // periódico y Realtime.
                // La misma instancia que ya creó `MainActivity` para la franja
                // (mismo dueño: la Activity).
                val actualizador: ActualizacionViewModel =
                    viewModel(factory = ActualizacionViewModel.factory(LocalContext.current))
                MenuCuenta(
                    sesion = sesion,
                    temaElegido = GestorTema.temaForzado,
                    onElegirTema = GestorTema::elegir,
                    onAyuda = {
                        mostrarAvisos = false
                        mostrarNuevoContratista = false
                        mostrarAyuda = true
                    },
                    onCerrarSesion = onCerrarSesion,
                    actualizador = actualizador,
                )
            }
        }
        }

        // Arriba de todo lo demás: una emergencia sin leer es lo primero que
        // tiene que ver quien opera.
        val emergencia = bandeja?.emergenciaSinLeer
        if (emergencia != null && !mostrarAvisos) {
            FranjaEmergencia(
                emergencia,
                onAbrir = {
                    mostrarNuevoContratista = false
                    mostrarAyuda = false
                    mostrarAvisos = true
                },
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
            )
        }

        // Teléfono retirado en el panel: la franja queda fija con el botón
        // para reconectarlo con un código del panel.
        if (equipoRetirado && !mostrarReconexion) {
            FranjaEquipoRetirado(
                onReconectar = {
                    mostrarAvisos = false
                    mostrarNuevoContratista = false
                    mostrarAyuda = false
                    mostrarReconexion = true
                },
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
            )
        }

        val errorSincronizacion = nubeViewModel.error
        if (errorSincronizacion != null) {
            FranjaEstado(
                errorSincronizacion,
                TipoEstado.ERROR,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
            )
        }

        // `docs/pendientes.md`, "alertar luego al sincronizar": todos los
        // conflictos en una sola franja (ver `FranjaConflictos.kt`). Sólo
        // informativa por ahora (fase 3, PR #62): sin botón de acción directa.
        FranjaConflictos(
            mensajesConflictos,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
        )

        // La bandeja de avisos y "Nuevo contratista" suben encima de las
        // pestañas; al volver, las pestañas reaparecen (ver
        // `TransicionesPantalla.kt`).
        AnimatedContent(
            targetState = panel,
            modifier = Modifier.weight(1f).fillMaxWidth(),
            // "Nuevo contratista" se abre en la cámara, y las cámaras no se
            // animan (ver `TransicionesPantalla.kt`): la vista previa arranca
            // en el hilo principal y no debe competir con un deslizamiento.
            // Tampoco al cerrarla desde la cámara (su X); desde el formulario
            // sí vuelve con la transición de siempre.
            transitionSpec = {
                val conCamara = targetState == PanelPrincipal.NUEVO_CONTRATISTA ||
                    (initialState == PanelPrincipal.NUEVO_CONTRATISTA && pantallasCompletas.cantidad > 0)
                if (conCamara) SinTransicion else transicionDePantalla(avanza = targetState != PanelPrincipal.PESTANAS)
            },
            label = "panelPrincipal",
        ) { mostrado ->
            if (mostrado == PanelPrincipal.AVISOS) {
                PantallaAvisos(
                    estado = bandeja,
                    onLeer = avisosViewModel::leer,
                    onLeerTodos = avisosViewModel::leerTodos,
                    onArchivar = { id -> avisosViewModel.archivar(id, sesion.nombre) },
                    onBorrar = { id -> avisosViewModel.borrar(id, sesion.nombre) },
                    onVolver = { mostrarAvisos = false },
                )
            } else if (mostrado == PanelPrincipal.NUEVO_CONTRATISTA) {
                // Con `LocalPantallasCompletas`, su cámara avisa que está a la
                // vista (ver la transición de arriba).
                CompositionLocalProvider(LocalPantallasCompletas provides pantallasCompletas) {
                    PantallaNuevoContratista(
                        nucleo,
                        onVolver = { mostrarNuevoContratista = false },
                        // Recién dado de alta, directo a su ingreso en Contratistas:
                        // antes volvía a la lista y había que buscarlo de nuevo. Se
                        // espera a que el formulario de ingreso esté listo y recién
                        // entonces se cierra: así hay una sola transición (la de
                        // volver), en vez de esa más la de la lista al formulario
                        // encima.
                        onGuardado = { id ->
                            scope.launch {
                                activosViewModel.elegir(id).join()
                                pestana = SECCIONES_VISIBLES.indexOf(SeccionPrincipal.ACTIVOS)
                                mostrarNuevoContratista = false
                            }
                        },
                        textoInicial = textoNuevoContratista,
                    )
                }
            } else if (mostrado == PanelPrincipal.AYUDA) {
                PantallaAyuda(onVolver = { mostrarAyuda = false })
            } else if (mostrado == PanelPrincipal.RECONEXION) {
                PantallaReconectarEquipo(
                    nucleo = nucleo,
                    metadata = metadata,
                    onReconectado = { unidad ->
                        mostrarReconexion = false
                        equipoRetirado = false
                        avisosBreves.mostrar("Teléfono reconectado en $unidad", TipoEstado.EXITO)
                        // Lo que quedó pendiente mientras estuvo retirado sube ya.
                        nubeViewModel.sincronizar()
                    },
                    onVolver = { mostrarReconexion = false },
                )
            } else {
                // Las secciones van en una barra flotante abajo (ver
                // [BarraSecciones]) y el contenido pasa por detrás, apenas
                // visible a través de su velo (sin desenfoque: redibujarlo en
                // cada frame duplicó el costo de dibujo de Activos). Cada
                // pantalla deja libre lo que tapa la barra con
                // `LocalMargenBarraInferior`.
                // Cámaras y formularios la esconden mientras están a la vista
                // (ver [OcultarBarraSecciones]).
                //
                // Al deslizar la lista hacia abajo la píldora de búsqueda se
                // repliega y deja más lista a la vista (como la barra de iOS 26
                // al desplazar); al volver hacia arriba, o al cambiar de
                // sección, reaparece.
                var barraCompacta by remember { mutableStateOf(false) }
                LaunchedEffect(seccion, externo) { barraCompacta = false }
                val replegarAlDesplazar = remember {
                    object : NestedScrollConnection {
                        override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
                            if (source == NestedScrollSource.UserInput) {
                                if (consumed.y < -UMBRAL_REPLIEGUE_PX) barraCompacta = true
                                if (consumed.y > UMBRAL_REPLIEGUE_PX || available.y > 0f) barraCompacta = false
                            }
                            return Offset.Zero
                        }
                    }
                }
                Box(modifier = Modifier.fillMaxSize().nestedScroll(replegarAlDesplazar)) {
                    // `AnimatedContent` (que por dentro es un `Box`) le pasa a sus
                    // hijos restricciones acotadas: un `LazyColumn` (como el de
                    // `PantallaActivos`) revienta con `IllegalStateException` si lo
                    // miden con altura infinita, como hace un `Column` sin peso.
                    // Desliza y funde entre pestañas (ver `TransicionesPantalla.kt`).
                    CompositionLocalProvider(
                        LocalMargenBarraInferior provides MARGEN_BARRA_SECCIONES,
                        LocalPantallasCompletas provides pantallasCompletas,
                        LocalAccionesInferiores provides accionesInferiores,
                    ) {
                        AnimatedContent(
                            targetState = seccion,
                            modifier = Modifier.fillMaxSize(),
                            transitionSpec = {
                                transicionDePestana(SECCIONES_VISIBLES.indexOf(initialState), SECCIONES_VISIBLES.indexOf(targetState))
                            },
                            label = "seccion",
                        ) { seccionMostrada ->
                            // `when` exhaustivo sobre el enum: agregar una sección sin
                            // su pantalla no compila (antes eran textos y un `else`
                            // que caía en Activos en silencio ante un error de tipeo).
                            // Con `seccionMostrada` y no `seccion`: durante la
                            // transición la pantalla que se va sigue con su propio
                            // acento y no salta al de la nueva.
                            TemaSeccion(seccionMostrada, externo) {
                                when (seccionMostrada) {
                                    SeccionPrincipal.ACTIVOS -> PantallaActivos(
                                        nucleo,
                                        refrescarNube,
                                        sincronizando = nubeViewModel.sincronizando,
                                        onSincronizar = nubeViewModel::sincronizar,
                                        onNuevoContratista = { texto ->
                                            mostrarAvisos = false
                                            mostrarAyuda = false
                                            textoNuevoContratista = texto
                                            mostrarNuevoContratista = true
                                        },
                                    )
                                    SeccionPrincipal.RUTAS -> PantallaRutas(nucleo)
                                    SeccionPrincipal.KOF -> PantallaGafetesProvisionales(
                                        nucleo,
                                        refrescarNube,
                                        sincronizando = nubeViewModel.sincronizando,
                                        onSincronizar = nubeViewModel::sincronizar,
                                    )
                                    SeccionPrincipal.EXTERNOS -> PantallaExternos(
                                        nucleo,
                                        refrescarNube,
                                        tipo = indiceExterno,
                                        onCambiarTipo = { indiceExterno = it },
                                        sincronizando = nubeViewModel.sincronizando,
                                        onSincronizar = nubeViewModel::sincronizar,
                                    )
                                }
                            }
                        }
                    }
                    // Baja y se funde al esconderse; vuelve a subir al cerrar la
                    // cámara o el formulario. Escondida no se puede tocar. Con
                    // el nombre completo: dentro de la `Column` de afuera, el
                    // `AnimatedVisibility` a secas es el de `ColumnScope`.
                    androidx.compose.animation.AnimatedVisibility(
                        visible = pantallasCompletas.cantidad == 0,
                        modifier = Modifier.align(Alignment.BottomCenter),
                        enter = fadeIn(tween(MS_FUNDIDO_ENTRADA)) + slideInVertically(tween(MS_ENTRADA)) { alto -> alto },
                        exit = fadeOut(tween(MS_SALIDA)) + slideOutVertically(tween(MS_SALIDA)) { alto -> alto },
                        label = "barraSecciones",
                    ) {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(
                                    start = MARGEN_LATERAL_BARRA,
                                    end = MARGEN_LATERAL_BARRA,
                                    top = MARGEN_SUPERIOR_BARRA,
                                    bottom = MARGEN_INFERIOR_BARRA,
                                ),
                        ) {
                            // Dónde se busca en todas las pestañas: la píldora,
                            // encima de la barra (ver `AccionInferior.kt`).
                            val busqueda = accionesInferiores.busqueda
                            androidx.compose.animation.AnimatedVisibility(
                                visible = busqueda != null && !barraCompacta,
                                enter = fadeIn(tween(MS_FUNDIDO_ENTRADA)) + slideInVertically(tween(MS_ENTRADA)) { alto -> alto / 2 },
                                exit = fadeOut(tween(MS_SALIDA)) + slideOutVertically(tween(MS_SALIDA)) { alto -> alto / 2 },
                                label = "pildoraBusqueda",
                            ) {
                                // La última que hubo, para seguir dibujándola mientras se va.
                                var mostrada by remember { mutableStateOf(busqueda) }
                                if (busqueda != null) mostrada = busqueda
                                mostrada?.let { PildoraBusqueda(it, modifier = Modifier.padding(bottom = ESPACIO_PILDORA_BARRA)) }
                            }
                            // La cápsula de secciones y, a su derecha, el botón
                            // redondo con la acción de la pantalla a la vista (ver
                            // `AccionInferior.kt`), como la app Teléfono de iOS.
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                BarraSecciones(
                                    seleccionada = seccion,
                                    externo = externo,
                                    adentro = adentro,
                                    onSeleccionar = { pestana = SECCIONES_VISIBLES.indexOf(it) },
                                    modifier = Modifier.weight(1f),
                                )
                                val accion = accionesInferiores.actual
                                if (accion != null) {
                                    BotonRedondoAccion(accion, tamano = ALTO_BARRA_SECCIONES)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Encima de todo, los avisos cortos.
    HostAvisosBreves(
        avisosBreves,
        modifier = Modifier.align(Alignment.TopCenter).padding(top = 8.dp, start = 16.dp, end = 16.dp),
    )
    }
    }
}

/// Cuánto hay que desplazar de una vez para replegar o mostrar la píldora:
/// un temblor del dedo no la hace aparecer y desaparecer.
private const val UMBRAL_REPLIEGUE_PX = 6f

/// Campanita de la barra superior: el globito (qué número, si se ve) y si
/// hay una emergencia pendiente lo decide el núcleo (`EstadoBandeja`). El
/// globito va sobre la esquina del botón, no adentro: el cuadradito recorta
/// su contenido.
@Composable
private fun CampanitaAvisos(bandeja: EstadoBandeja?, onClick: () -> Unit) {
    val contador = bandeja?.contador
    val hayEmergencia = bandeja?.emergenciaSinLeer != null
    BadgedBox(badge = { if (contador != null) Badge { Text(contador) } }) {
        BotonIconoCuadradoBrisas(onClick = onClick) {
            Icon(
                if (hayEmergencia) Icons.Default.NotificationImportant else Icons.Default.Notifications,
                contentDescription = if (contador == null) "Avisos" else "Avisos, ${bandeja?.noLeidos} sin leer",
                tint = if (hayEmergencia) MaterialTheme.colorScheme.error else LocalContentColor.current,
            )
        }
    }
}

/// Las secciones en una cápsula flotante abajo (como la barra nueva de
/// WhatsApp, pedido del usuario 2026-10-21), en vez de la fila de píldoras
/// arriba: se cambia de sección con el pulgar y arriba queda una sola fila de
/// opciones. Más baja que la `NavigationBar` de Material 3 (80dp) y separada
/// de los bordes.
///
/// - El fondo: un velo casi opaco del color de la superficie ([ALFA_VELO]).
///   Sin desenfoque a propósito (Haze, 2026-10-22): redibujar el contenido
///   desenfocado detrás de la barra en cada frame duplicó el render de
///   Activos (p90 de 5,5 a 10,1 ms) y subió los frames trabados.
/// - La pestaña elegida: una píldora gris suave que se desliza hasta ella
///   (como [FilaPildoras]); su ícono y su nombre van en el acento de la
///   sección (ver `AcentoSeccion.kt`).
/// - Cada sección lleva un globito con cuántos hay adentro.
///
/// "KOF" a secas, no "Gafetes KOF" (pedido explícito del usuario
/// 2026-09-19). "Rutas" no aparece (pedido del usuario 2026-09-23: control de
/// rutas sin terminar); `PantallaRutas` sigue intacta y para volver a
/// mostrarla basta con sacarla de `enDesarrollo` en [SeccionPrincipal].
@Composable
private fun BarraSecciones(
    seleccionada: SeccionPrincipal,
    externo: TipoExterno,
    adentro: Map<SeccionPrincipal, Int>,
    onSeleccionar: (SeccionPrincipal) -> Unit,
    modifier: Modifier = Modifier,
) {
    val oscuro = temaActual().oscuro
    val superficie = MaterialTheme.colorScheme.surface
    val colorPildora = MaterialTheme.colorScheme.onSurface.copy(alpha = if (oscuro) 0.12f else 0.07f)
    val posicion by animateFloatAsState(
        targetValue = SECCIONES_VISIBLES.indexOf(seleccionada).toFloat(),
        animationSpec = spring(dampingRatio = 0.8f, stiffness = Spring.StiffnessMediumLow),
        label = "posicionSeccion",
    )
    val forma = RoundedCornerShape(percent = 50)
    val hapticos = rememberHapticos()
    Row(
        modifier = modifier
            .height(ALTO_BARRA_SECCIONES)
            .clip(forma)
            .background(superficie.copy(alpha = ALFA_VELO))
            .border(ANCHO_BORDE_VIDRIO, paletaActual.bordeVidrio, forma)
            .padding(5.dp)
            .drawBehind {
                val cantidad = SECCIONES_VISIBLES.size
                val ancho = size.width / cantidad
                drawRoundRect(
                    color = colorPildora,
                    topLeft = Offset(posicion * ancho, 0f),
                    size = Size(ancho, size.height),
                    cornerRadius = CornerRadius(size.height / 2),
                )
            },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        SECCIONES_VISIBLES.forEach { seccion ->
            val elegida = seccion == seleccionada
            val acento = acentoDeSeccion(seccion, oscuro, externo)?.color ?: MaterialTheme.colorScheme.primary
            val color by animateColorAsState(
                targetValue = if (elegida) acento else MaterialTheme.colorScheme.onSurfaceVariant,
                animationSpec = tween(MS_FUNDIDO_ENTRADA),
                label = "colorSeccion",
            )
            val cantidad = adentro[seccion] ?: 0
            Column(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxHeight()
                    .clip(forma)
                    .selectable(
                        selected = elegida,
                        role = Role.Tab,
                        onClick = {
                            if (!elegida) hapticos(Haptico.SELECCION)
                            onSeleccionar(seccion)
                        },
                    )
                    .semantics { if (cantidad > 0) stateDescription = "$cantidad adentro" },
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                // Globito gris, no rojo: es una cuenta, no algo que pida
                // atención (en iOS el rojo es para lo pendiente, como la
                // campanita de avisos).
                BadgedBox(
                    badge = {
                        if (cantidad > 0) {
                            Badge(
                                containerColor = MaterialTheme.colorScheme.onSurfaceVariant,
                                contentColor = MaterialTheme.colorScheme.surface,
                            ) { Text("$cantidad") }
                        }
                    },
                ) {
                    Icon(iconoSeccion(seccion, externo), contentDescription = null, tint = color, modifier = Modifier.size(22.dp))
                }
                Text(
                    seccion.etiqueta,
                    color = color,
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = if (elegida) FontWeight.SemiBold else FontWeight.Medium,
                    maxLines = 1,
                )
            }
        }
    }
}

private val ALTO_BARRA_SECCIONES = 58.dp
private val MARGEN_LATERAL_BARRA = 12.dp
/// Despegada del borde (pedido del usuario 2026-10-21): pegada abajo quedaba
/// incómoda para el pulgar. 18dp se sentía bajo y 28dp alto; 24dp.
private val MARGEN_INFERIOR_BARRA = 24.dp
private val MARGEN_SUPERIOR_BARRA = 6.dp

/// Entre la píldora de búsqueda y la barra de secciones.
private val ESPACIO_PILDORA_BARRA = 8.dp

/// Lo que tapa la barra flotante con la píldora de búsqueda encima (altos +
/// margen de abajo + un respiro), para [LocalMargenBarraInferior]. Fijo
/// aunque la píldora se repliegue al desplazar: si cambiara, la lista
/// saltaría mientras se la desliza.
private val MARGEN_BARRA_SECCIONES =
    ALTO_BARRA_SECCIONES + ALTO_PILDORA_BUSQUEDA + ESPACIO_PILDORA_BARRA + MARGEN_INFERIOR_BARRA + 6.dp

/// Los mismos íconos que el escritorio (ver [IconosLucide]). Externos cambia
/// con el tipo elegido, igual que su color: Proveedor o Correo.
private fun iconoSeccion(seccion: SeccionPrincipal, externo: TipoExterno): ImageVector = when (seccion) {
    SeccionPrincipal.ACTIVOS -> IconosLucide.UsuarioConCheck
    // Oculta de la barra (en desarrollo).
    SeccionPrincipal.RUTAS -> Icons.Default.Route
    SeccionPrincipal.KOF -> IconosLucide.InsigniaConCheck
    SeccionPrincipal.EXTERNOS -> if (externo == TipoExterno.POR_CORREO) IconosLucide.Sobre else IconosLucide.Cajas
}

/// Qué ocupa la pantalla principal debajo de la barra superior.
private enum class PanelPrincipal { AVISOS, NUEVO_CONTRATISTA, AYUDA, RECONEXION, PESTANAS }

/// Tiempo fuera de primer plano (app minimizada o teléfono bloqueado) tras
/// el cual se cierra la sesión. Eran 2 minutos: en la portería el guardia
/// bloquea el teléfono entre ingreso e ingreso y tenía que volver a entrar
/// casi siempre. 25 minutos cubre esas pausas y sigue cerrando la sesión de
/// un teléfono olvidado.
private const val DEMORA_BLOQUEO_SESION_MS = 25 * 60_000L

/// Secciones de la pantalla principal, en el orden de la barra de abajo
/// (punto M5 de la auditoría móvil: antes eran textos sueltos).
/// `enDesarrollo` las oculta de la barra sin borrar su pantalla (ver
/// [BarraSecciones]).
enum class SeccionPrincipal(val etiqueta: String, val enDesarrollo: Boolean = false) {
    // "Contratistas" y no "Activos" (pedido del usuario 2026-10-26): la
    // pestaña es sólo de contratistas; KOF y Externos también tienen gente
    // adentro.
    ACTIVOS("Contratistas"),
    RUTAS("Rutas", enDesarrollo = true),
    KOF("KOF"),
    // Antes "Proveedores": ahora agrupa a quien entra sin catálogo
    // (proveedores e ingresos por correo, ver [PantallaExternos]).
    EXTERNOS("Externos"),
}

private val SECCIONES_VISIBLES = SeccionPrincipal.entries.filterNot { it.enDesarrollo }

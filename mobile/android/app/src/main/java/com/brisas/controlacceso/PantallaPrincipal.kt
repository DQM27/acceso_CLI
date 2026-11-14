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
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.runtime.snapshotFlow
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.material3.HorizontalDivider
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LifecycleStartEffect
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.control_acceso_mobile.Nucleo
import uniffi.control_acceso_mobile.UsuarioSesion
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue

/// App limitada a registros rápidos + historial (decisión 2026-09-06),
/// con una única excepción reincorporada el 2026-09-12: el alta de
/// contratista (`PantallaNuevoContratista`, desde el menú de la cuenta o
/// al no encontrar a nadie en la búsqueda), ahora con OCR de carnet PRAIND además del formulario manual
/// original. El resto de la decisión sigue en pie -- empresa y usuario
/// nuevos siguen sin pantalla propia acá (escritorio/web/CLI), y tampoco
/// volvió la pestaña de nube.
///
/// La navegación es una sola variable, `panel` ([PanelPrincipal]): las
/// pestañas o una pantalla que se abre encima y las reemplaza por completo
/// (bandeja de avisos, "Nuevo contratista", Ayuda, reconexión). Con tan pocas
/// pantallas no hace falta Navigation-Compose.
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
    // Qué ocupa la pantalla debajo de la barra de arriba: las pestañas o una
    // de las pantallas que se abren encima (bandeja de avisos, "Nuevo
    // contratista", Ayuda, reconexión). Una sola variable: antes eran cuatro
    // sí/no y cada apertura tenía que apagar las otras tres a mano.
    var panel by remember { mutableStateOf(PanelPrincipal.PESTANAS) }
    // Lo que se buscó en Contratistas sin encontrar a nadie, para arrancar el
    // alta con eso (vacío desde el menú de la cuenta).
    var textoNuevoContratista by remember { mutableStateOf("") }
    // Teléfono retirado en el panel (ver `PantallaReconectarEquipo`): lo
    // decide el núcleo; se vuelve a preguntar tras cada sincronización y al
    // llegar la expulsión en vivo.
    var equipoRetirado by remember { mutableStateOf(false) }
    // La pestaña elegida. Vive acá y no junto a las pestañas: al abrir la
    // bandeja o "Nuevo contratista" las pestañas salen de la composición, y
    // al volver se caía siempre en Activos.
    var pestana by rememberSaveable { mutableIntStateOf(0) }
    // Sin esto, atrás del sistema en la raíz (pestañas) caía directo al
    // default de Android -- cerraba/minimizaba la app de un solo toque, sin
    // que hubiera ningún BackHandler compose que lo interceptara (hallazgo
    // 2026-09-19, pedido explícito del usuario: doble tap para salir, mismo
    // patrón estándar de Android). Sólo con las pestañas a la vista: las
    // pantallas de encima tienen su propio BackHandler.
    val contexto = LocalContext.current
    var ultimoTapAtras by remember { mutableLongStateOf(0L) }
    BackHandler(enabled = panel == PanelPrincipal.PESTANAS) {
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
            panel = PanelPrincipal.AVISOS
            avisosViewModel.aperturaAtendida()
        }
    }
    // Con la bandeja a la vista, que "hace 5 min" avance.
    val enAvisos = panel == PanelPrincipal.AVISOS
    LaunchedEffect(enAvisos) {
        while (enAvisos) {
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
    // El último aviso en vivo ya guardado en la base local; lo atiende el
    // efecto de más abajo, que tiene a mano los ViewModels de cada sección.
    var cambioEnVivo by remember { mutableStateOf<CambioEnVivo?>(null) }
    val realtime = remember(nucleo, scope, sesion) {
        NubeRealtime(
            nucleo = nucleo,
            scope = scope,
            usuarioCedula = sesion.cedula,
            usuarioNombre = sesion.nombre,
            // Aviso en vivo con la fila ya guardada: refresca al instante,
            // sin esperar la sincronización, sólo la sección de esa tabla
            // (2026-11-14, fase 5 de `docs/handoff-rediseno-movil-ux.md`):
            // antes cada aviso recargaba las cuatro, ocho lecturas al núcleo
            // por el mismo candado.
            onCambioAplicado = { tabla ->
                cambioEnVivo = CambioEnVivo(seccionDeTabla(tabla), (cambioEnVivo?.id ?: 0L) + 1)
            },
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
                // Lo pendiente pudo haber subido: el aviso se va enseguida.
                nubeViewModel.revisarBandeja()
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

    // El aviso de registros sin subir se revisa cada 15 s sólo con la app a
    // la vista: con el teléfono bloqueado nadie lo lee.
    LifecycleStartEffect(nubeViewModel) {
        val revision = scope.launch {
            while (true) {
                nubeViewModel.revisarBandeja()
                delay(NubeViewModel.INTERVALO_REVISION_BANDEJA_MS)
            }
        }
        onStopOrDispose { revision.cancel() }
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
    val totalProveedores = proveedoresViewModel.activos.size
    val totalCorreo = correoViewModel.activos.size
    val adentro = remember(totalActivos, totalKof, totalProveedores, totalCorreo) {
        mapOf(
            SeccionPrincipal.CONTRATISTAS to totalActivos,
            SeccionPrincipal.KOF to totalKof,
            SeccionPrincipal.PROVEEDORES to totalProveedores,
            SeccionPrincipal.CORREO to totalCorreo,
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
        val seccion = SECCIONES_VISIBLES.getOrElse(pestana) { SeccionPrincipal.CONTRATISTAS }
        val fueraDePestanas = panel != PanelPrincipal.PESTANAS
        if (seccion != SeccionPrincipal.CONTRATISTAS || fueraDePestanas) activosViewModel.refrescar()
        if (seccion != SeccionPrincipal.KOF || fueraDePestanas) gafetesViewModel.refrescarActivos()
        if (seccion != SeccionPrincipal.PROVEEDORES || fueraDePestanas) proveedoresViewModel.refrescarActivos()
        if (seccion != SeccionPrincipal.CORREO || fueraDePestanas) correoViewModel.refrescarActivos()
    }
    // Un aviso en vivo ya guardado: sólo su sección, esté o no a la vista (la
    // pantalla lee del mismo ViewModel). Tabla desconocida: todas, como una
    // sincronización.
    LaunchedEffect(cambioEnVivo) {
        val cambio = cambioEnVivo ?: return@LaunchedEffect
        when (cambio.seccion) {
            SeccionPrincipal.CONTRATISTAS -> activosViewModel.refrescar()
            SeccionPrincipal.KOF -> gafetesViewModel.refrescarActivos()
            SeccionPrincipal.PROVEEDORES -> proveedoresViewModel.refrescarActivos()
            SeccionPrincipal.CORREO -> correoViewModel.refrescarActivos()
            SeccionPrincipal.RUTAS, SeccionPrincipal.VISITAS, null -> refrescarNube += 1
        }
    }

    // Qué ocupa la pantalla debajo de la barra superior.
    // Cámaras y formularios dentro de las pestañas (ver [OcultarBarraSecciones]):
    // esconden la barra de secciones de abajo y también la de arriba.
    val pantallasCompletas = remember { PantallasCompletas() }
    // La acción del botón redondo y la búsqueda de la píldora las pone la
    // pantalla a la vista.
    val accionesInferiores = remember { AccionesInferiores() }
    // Las secciones son páginas que se pasan deslizando de lado (pedido del
    // usuario 2026-10-28), además de tocando la barra. `pestana` sigue siendo
    // la sección elegida; las dos se sincronizan dentro de las pestañas.
    val paginas = rememberPagerState(initialPage = pestana) { SECCIONES_VISIBLES.size }
    val hapticosSecciones = rememberHapticos()
    // La lista a la vista está desplazada: línea fina bajo la barra de arriba.
    val listaDesplazada = remember { mutableStateOf(false) }
    // La que está a la vista: cambia a mitad del deslizamiento, así el título,
    // el color y la barra acompañan al dedo.
    val seccion = SECCIONES_VISIBLES.getOrElse(paginas.currentPage) { SeccionPrincipal.CONTRATISTAS }

    // Alto de la barra de arriba, para que la nieve sepa dónde termina.
    var altoBarraSuperiorPx by remember { mutableIntStateOf(0) }

    CompositionLocalProvider(LocalAvisosBreves provides avisosBreves) {
    Box(modifier = Modifier.fillMaxSize()) {
    Column(modifier = Modifier.fillMaxSize()) {
        // La barra de arriba sólo con las pestañas a la vista: las pantallas
        // que se abren encima (avisos, ayuda, formularios, cámaras) traen su
        // propio "‹ Volver" y su título, como una pantalla nueva de iOS.
        if (panel == PanelPrincipal.PESTANAS && pantallasCompletas.cantidad == 0) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .onSizeChanged { altoBarraSuperiorPx = it.height }
                // Los botones de la derecha traen su propio aire tocable
                // (`MARGEN_TACTIL_BOTON_CUADRADO`, 4dp por lado): se descuenta
                // acá para que la barra mida y se alinee igual que antes.
                .padding(start = 16.dp, end = 16.dp - MARGEN_TACTIL_BOTON_CUADRADO, top = 0.dp, bottom = 2.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // El título es la sección a la vista (como el de cada pestaña de
            // iOS), en una sola línea y no tan grande: la barra ocupaba
            // mucho alto (pedido del usuario 2026-10-29). Cuántos hay adentro
            // lo dice el globito de la barra de abajo; quién tiene la sesión
            // y la unidad, el menú de la cuenta.
            AnimatedContent(
                targetState = seccion,
                modifier = Modifier.weight(1f),
                transitionSpec = { (fadeIn(tween(MS_FUNDIDO_ENTRADA)) togetherWith fadeOut(tween(MS_SALIDA))) using null },
                label = "tituloSeccion",
            ) { seccionTitulo ->
                Text(
                    seccionTitulo.etiqueta,
                    style = MaterialTheme.typography.titleLarge,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            // Sin separación propia: los 8dp entre cuadraditos los ponen sus
            // márgenes tocables.
            // Sin botón "Nuevo contratista" (2026-11-14, decisión del dueño):
            // su ícono era el mismo que "nuevo ingreso" de Proveedores y
            // Correo, y dar de alta sin buscar antes dejaba duplicados. El
            // alta se ofrece al buscar o escanear a alguien que no está
            // registrado (`ResultadosIngreso`) y queda en el menú de la cuenta.
            Row {
                CampanitaAvisos(bandeja, onClick = { panel = PanelPrincipal.AVISOS })
                // Sin botón "Sincronizar": cada lista sincroniza tirando hacia
                // abajo (ver `ListaConEstados`), y además está el pulso
                // periódico y Realtime.
                // La misma instancia que ya creó `MainActivity` para la franja
                // (mismo dueño: la Activity).
                val actualizador: ActualizacionViewModel =
                    viewModel(factory = ActualizacionViewModel.factory(LocalContext.current))
                MenuCuenta(
                    sesion = sesion,
                    unidad = textoUnidad,
                    temaElegido = GestorTema.temaForzado,
                    onElegirTema = GestorTema::elegir,
                    onNuevoContratista = {
                        textoNuevoContratista = ""
                        panel = PanelPrincipal.NUEVO_CONTRATISTA
                    },
                    onAyuda = { panel = PanelPrincipal.AYUDA },
                    onCerrarSesion = onCerrarSesion,
                    actualizador = actualizador,
                )
            }
        }
        // Sin degradado: con la lista desplazada, una línea fina marca dónde
        // pasan las tarjetas por debajo; arriba de todo no se ve (como iOS).
        val alfaLinea by animateFloatAsState(
            targetValue = if (listaDesplazada.value) 1f else 0f,
            animationSpec = tween(MS_FUNDIDO_ENTRADA),
            label = "lineaBarraSuperior",
        )
        HorizontalDivider(
            thickness = 0.5.dp,
            color = MaterialTheme.colorScheme.outline.copy(alpha = 0.35f),
            modifier = Modifier.graphicsLayer { alpha = alfaLinea },
        )
        }

        // Arriba de todo lo demás: una emergencia sin leer es lo primero que
        // tiene que ver quien opera.
        val emergencia = bandeja?.emergenciaSinLeer
        if (emergencia != null && panel != PanelPrincipal.AVISOS) {
            FranjaEmergencia(
                emergencia,
                onAbrir = { panel = PanelPrincipal.AVISOS },
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
            )
        }

        // Teléfono retirado en el panel: la franja queda fija con el botón
        // para reconectarlo con un código del panel.
        if (equipoRetirado && panel != PanelPrincipal.RECONEXION) {
            FranjaEquipoRetirado(
                onReconectar = { panel = PanelPrincipal.RECONEXION },
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
            )
        }

        // Algo registrado en este teléfono que no llegó a la nube (ver
        // `NubeViewModel.avisoBandeja`): ámbar mientras se reintenta solo,
        // rojo si ya no.
        val avisoBandeja = nubeViewModel.avisoBandeja
        if (avisoBandeja != null) {
            FranjaEstado(
                avisoBandeja.texto,
                if (avisoBandeja.critico) TipoEstado.ERROR else TipoEstado.AVISO,
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
                    onVolver = { panel = PanelPrincipal.PESTANAS },
                )
            } else if (mostrado == PanelPrincipal.NUEVO_CONTRATISTA) {
                // Con `LocalPantallasCompletas`, su cámara avisa que está a la
                // vista (ver la transición de arriba).
                CompositionLocalProvider(LocalPantallasCompletas provides pantallasCompletas) {
                    PantallaNuevoContratista(
                        nucleo,
                        onVolver = { panel = PanelPrincipal.PESTANAS },
                        // Recién dado de alta, directo a su ingreso en Contratistas:
                        // antes volvía a la lista y había que buscarlo de nuevo. Se
                        // espera a que el formulario de ingreso esté listo y recién
                        // entonces se cierra: así hay una sola transición (la de
                        // volver), en vez de esa más la de la lista al formulario
                        // encima.
                        onGuardado = { id ->
                            scope.launch {
                                activosViewModel.elegir(id).join()
                                pestana = SECCIONES_VISIBLES.indexOf(SeccionPrincipal.CONTRATISTAS)
                                panel = PanelPrincipal.PESTANAS
                            }
                        },
                        textoInicial = textoNuevoContratista,
                    )
                }
            } else if (mostrado == PanelPrincipal.AYUDA) {
                PantallaAyuda(onVolver = { panel = PanelPrincipal.PESTANAS })
            } else if (mostrado == PanelPrincipal.RECONEXION) {
                PantallaReconectarEquipo(
                    nucleo = nucleo,
                    metadata = metadata,
                    onReconectado = { unidad ->
                        panel = PanelPrincipal.PESTANAS
                        equipoRetirado = false
                        avisosBreves.mostrar("Teléfono reconectado en $unidad", TipoEstado.EXITO)
                        // Lo que quedó pendiente mientras estuvo retirado sube ya.
                        nubeViewModel.sincronizar()
                    },
                    onVolver = { panel = PanelPrincipal.PESTANAS },
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
                LaunchedEffect(seccion) { barraCompacta = false }
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
                    // Tocar la barra lleva a la página; deslizar hasta otra la
                    // elige. `settledPage` y no `currentPage`: al tocar una
                    // sección lejana el carrusel pasa por las del medio y no
                    // debe quedarse en la primera que cruza.
                    LaunchedEffect(pestana) {
                        if (paginas.settledPage != pestana) paginas.animateScrollToPage(pestana)
                    }
                    // Sin el primer valor: al volver de otra pantalla que eligió
                    // la sección (p. ej. "Nuevo contratista" deja Contratistas),
                    // la página vieja la pisaría antes de que el carrusel llegue.
                    //
                    // Si la página nueva no es la que ya estaba elegida, el cambio
                    // vino de deslizar: vibra como al tocar la barra (al tocarla,
                    // `pestana` ya cambió y vibró el toque).
                    LaunchedEffect(paginas) {
                        snapshotFlow { paginas.settledPage }.drop(1).collect { pagina ->
                            if (pagina != pestana) hapticosSecciones(Haptico.SELECCION)
                            pestana = pagina
                        }
                    }
                    CompositionLocalProvider(
                        LocalMargenBarraInferior provides MARGEN_BARRA_SECCIONES,
                        LocalPantallasCompletas provides pantallasCompletas,
                    ) {
                        // Con una cámara, un formulario o el buscador abiertos
                        // (lo que esconde la barra) no se desliza: no se cambia
                        // de sección por accidente a mitad de un ingreso.
                        HorizontalPager(
                            state = paginas,
                            modifier = Modifier.fillMaxSize(),
                            userScrollEnabled = pantallasCompletas.cantidad == 0,
                            key = { SECCIONES_VISIBLES[it] },
                        ) { pagina ->
                            val seccionMostrada = SECCIONES_VISIBLES[pagina]
                            // Mientras se desliza conviven dos páginas: sólo la
                            // que está a la vista pone su botón redondo y su
                            // búsqueda abajo (sin `AccionesInferiores`, las
                            // demás no registran nada).
                            val aLaVista = pagina == paginas.currentPage
                            CompositionLocalProvider(
                                LocalAccionesInferiores provides accionesInferiores.takeIf { aLaVista },
                                LocalListaDesplazada provides listaDesplazada.takeIf { aLaVista },
                            ) {
                            // `when` exhaustivo sobre el enum: agregar una sección sin
                            // su pantalla no compila (antes eran textos y un `else`
                            // que caía en Activos en silencio ante un error de tipeo).
                            // El color de la sección sólo va en su pestaña (ver
                            // `AcentoSeccion.kt`): la página usa el azul de la app.
                                when (seccionMostrada) {
                                    SeccionPrincipal.CONTRATISTAS -> PantallaActivos(
                                        nucleo,
                                        refrescarNube,
                                        sincronizando = nubeViewModel.sincronizando,
                                        onSincronizar = nubeViewModel::sincronizar,
                                        onNuevoContratista = { texto ->
                                            textoNuevoContratista = texto
                                            panel = PanelPrincipal.NUEVO_CONTRATISTA
                                        },
                                    )
                                    SeccionPrincipal.RUTAS -> PantallaRutas(nucleo)
                                    SeccionPrincipal.KOF -> PantallaGafetesProvisionales(
                                        nucleo,
                                        refrescarNube,
                                        sincronizando = nubeViewModel.sincronizando,
                                        onSincronizar = nubeViewModel::sincronizar,
                                    )
                                    SeccionPrincipal.PROVEEDORES -> PantallaProveedores(
                                        nucleo,
                                        refrescarNube,
                                        nubeViewModel.sincronizando,
                                        nubeViewModel::sincronizar,
                                    )
                                    SeccionPrincipal.CORREO -> PantallaPorCorreo(
                                        nucleo,
                                        refrescarNube,
                                        nubeViewModel.sincronizando,
                                        nubeViewModel::sincronizar,
                                    )
                                    SeccionPrincipal.VISITAS -> PantallaVisitas(nucleo, refrescarNube)
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
                                horizontalArrangement = Arrangement.spacedBy(ESPACIO_CORTE),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                val accion = accionesInferiores.actual
                                BarraSecciones(
                                    seleccionada = seccion,
                                    posicion = { paginas.currentPage + paginas.currentPageOffsetFraction },
                                    adentro = adentro,
                                    onSeleccionar = { pestana = SECCIONES_VISIBLES.indexOf(it) },
                                    modifier = Modifier.weight(1f),
                                )
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
    // Nieve de temporada sobre la barra de arriba, sólo con las pestañas a la
    // vista (ver `NieveNavidena.kt`).
    if (panel == PanelPrincipal.PESTANAS && pantallasCompletas.cantidad == 0 && altoBarraSuperiorPx > 0) {
        NieveNavidena(
            altoBarra = with(LocalDensity.current) { altoBarraSuperiorPx.toDp() },
            modifier = Modifier.align(Alignment.TopCenter),
        )
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

/// Qué ocupa la pantalla principal debajo de la barra superior.
private enum class PanelPrincipal { AVISOS, NUEVO_CONTRATISTA, AYUDA, RECONEXION, PESTANAS }

/// Tiempo fuera de primer plano (app minimizada o teléfono bloqueado) tras
/// el cual se cierra la sesión. Eran 2 minutos: en la portería el guardia
/// bloquea el teléfono entre ingreso e ingreso y tenía que volver a entrar
/// casi siempre. 25 minutos cubre esas pausas y sigue cerrando la sesión de
/// un teléfono olvidado.
private const val DEMORA_BLOQUEO_SESION_MS = 25 * 60_000L

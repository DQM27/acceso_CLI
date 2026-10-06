package com.brisas.controlacceso

import android.app.Service
import android.content.Intent
import android.os.IBinder
import kotlin.concurrent.thread
import uniffi.control_acceso_mobile.Nucleo

/// Cerrar la app con una sesión abierta cuenta como cerrar sesión, igual que
/// el escritorio al cerrar la ventana: si no, la bitácora del panel mostraba
/// la sesión abierta hasta el próximo ingreso y la cerraba como "sin
/// cierre", sin la hora real. Dos caminos llegan acá:
///
/// - salir con "atrás" (la Activity termina): `AplicacionViewModel.onCleared`;
/// - deslizar la app fuera de las recientes: [ServicioCierreSesion].
///
/// La regla (qué sesión se cierra y cómo se avisa) vive en el núcleo
/// (`Nucleo::cerrar_sesion_al_salir`); acá sólo se decide cuándo llamarla.
/// Un cierre forzado (Android mata el proceso, batería agotada) no pasa por
/// acá y sigue quedando "sin cierre".
internal object CierreSesionAlSalir {
    @Volatile
    private var nucleo: Nucleo? = null

    fun registrar(nucleo: Nucleo) {
        this.nucleo = nucleo
    }

    /// Lo olvida antes de cerrarlo, para que el servicio no lo use cerrado.
    fun olvidar(nucleo: Nucleo) {
        if (this.nucleo === nucleo) this.nucleo = null
    }

    /// Avisa el cierre en un hilo propio (hace red) y después corre
    /// [despues]. Sin núcleo abierto sólo corre [despues].
    fun ejecutar(nucleo: Nucleo? = this.nucleo, despues: () -> Unit = {}) {
        if (nucleo == null) {
            despues()
            return
        }
        thread(name = "cierre-sesion-al-salir") {
            runCatching { nucleo.cerrarSesionAlSalir() }
            despues()
        }
    }
}

/// Sólo existe para enterarse de que el operador deslizó la app fuera de
/// las recientes (`onTaskRemoved`, gracias a `stopWithTask="false"` en el
/// manifiesto). No hace nada más ni muestra notificación. Lo arranca
/// `MainActivity.onStart`; si Android lo detiene por estar la app mucho
/// tiempo en segundo plano, ese cierre queda como antes ("sin cierre").
class ServicioCierreSesion : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int = START_NOT_STICKY

    override fun onTaskRemoved(rootIntent: Intent?) {
        CierreSesionAlSalir.ejecutar { stopSelf() }
    }
}

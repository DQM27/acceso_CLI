package com.brisas.controlacceso

import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import uniffi.control_acceso_mobile.avisoDeMensaje

/// Recibe los mensajes de Firebase Cloud Messaging. Android lo levanta solo
/// cuando llega un mensaje, aunque la app esté cerrada (declarado en
/// AndroidManifest.xml).
class ServicioMensajesPush : FirebaseMessagingService() {
    override fun onMessageReceived(mensaje: RemoteMessage) {
        val aviso = avisoDeMensaje(mensaje.data, mensaje.notification?.title, mensaje.notification?.body) ?: return
        // Primero a la bandeja (la campanita): la notificación de Android se
        // puede descartar, la bandeja queda.
        AlmacenBandejaAvisos.guardar(this, aviso)
        NotificacionesPush.mostrar(this, aviso)
    }

    override fun onNewToken(token: String) {
        NotificacionesPush.guardarToken(this, token)
    }
}

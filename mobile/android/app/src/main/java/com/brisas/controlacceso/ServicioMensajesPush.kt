package com.brisas.controlacceso

import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage

/// Recibe los mensajes de Firebase Cloud Messaging. Android lo levanta solo
/// cuando llega un mensaje, aunque la app esté cerrada (declarado en
/// AndroidManifest.xml).
class ServicioMensajesPush : FirebaseMessagingService() {
    override fun onMessageReceived(mensaje: RemoteMessage) {
        val aviso = avisoDeMensaje(mensaje.data, mensaje.notification?.title, mensaje.notification?.body) ?: return
        NotificacionesPush.mostrar(this, aviso)
    }

    override fun onNewToken(token: String) {
        NotificacionesPush.guardarToken(this, token)
    }
}

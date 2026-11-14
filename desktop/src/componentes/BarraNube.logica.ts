/** Estado crudo que entrega `RealtimeChannel.subscribe` (ver
 * `iniciarRealtimeNube`, `onEstado`) -- `null` antes de que la primera
 * conexión siquiera se intente (arranque de la app). */
export type EstadoConexionNube =
  | "SUBSCRIBED"
  | "CHANNEL_ERROR"
  | "TIMED_OUT"
  | "CLOSED"
  | null;

export function descripcion(
  estado: EstadoConexionNube,
  enLinea: boolean,
): { texto: string; color: string; parpadea: boolean } {
  // Sin red o sin canal en vivo se sigue trabajando: lo registrado se guarda
  // en esta PC y sube al volver. Por eso ámbar y no rojo, y el texto dice qué
  // pasa (auditoría 2026-11-14, U9). El rojo queda para lo que exige actuar.
  if (!enLinea) {
    return { texto: "Sin internet — se guarda en esta PC", color: "var(--advertencia)", parpadea: true };
  }
  switch (estado) {
    case "SUBSCRIBED":
      return { texto: "En línea", color: "var(--exito)", parpadea: false };
    case null:
      return { texto: "Conectando…", color: "var(--muted)", parpadea: false };
    default:
      // CHANNEL_ERROR/TIMED_OUT/CLOSED con red del SO activa -- ej. la
      // nube está caída, o un firewall bloquea el WebSocket específicamente.
      // `iniciarRealtimeNube` ya reintenta solo con backoff creciente, esto
      // sólo informa que el aviso en vivo no está llegando ahora mismo. El
      // botón "Sincronizar" y el pulso automático (cada 5 min) siguen
      // funcionando igual sin Realtime.
      return { texto: "Sin conexión en vivo", color: "var(--advertencia)", parpadea: true };
  }
}

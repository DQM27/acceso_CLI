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
): { texto: string; color: string } {
  if (!enLinea) return { texto: "Sin conexión — modo offline", color: "var(--error)" };
  switch (estado) {
    case "SUBSCRIBED":
      return { texto: "En línea", color: "var(--exito)" };
    case null:
      return { texto: "Conectando…", color: "var(--muted)" };
    default:
      // CHANNEL_ERROR/TIMED_OUT/CLOSED con red del SO activa -- ej. la
      // nube está caída, o un firewall bloquea el WebSocket específicamente.
      // `iniciarRealtimeNube` ya reintenta solo con backoff creciente, esto
      // sólo informa que el aviso en vivo no está llegando ahora mismo. El
      // botón "Sincronizar" y el pulso automático (cada 2 min) siguen
      // funcionando igual sin Realtime.
      return { texto: "Sin conexión en vivo", color: "var(--error)" };
  }
}

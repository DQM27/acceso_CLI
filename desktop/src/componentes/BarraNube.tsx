import { useEffect, useState } from "react";
import { toast } from "sonner";
import { RefreshCw } from "lucide-react";

/** Estado crudo que entrega `RealtimeChannel.subscribe` (ver
 * `iniciarRealtimeNube`, `onEstado`) -- `null` antes de que la primera
 * conexión siquiera se intente (arranque de la app). */
export type EstadoConexionNube =
  | "SUBSCRIBED"
  | "CHANNEL_ERROR"
  | "TIMED_OUT"
  | "CLOSED"
  | null;

/** `navigator.onLine` refleja si el sistema operativo tiene una interfaz
 * de red activa -- cambia al instante (evento `online`/`offline`) al
 * desconectar el cable/wifi. El estado del canal de Realtime, en cambio,
 * depende de su propio heartbeat interno (~30s) para darse cuenta de que
 * el socket murió -- probado a mano: desconectar la red no lo reflejaba
 * de inmediato. Esta señal del SO es la que manda para "sin conexión";
 * Realtime sólo afina el texto cuando el SO sí dice que hay red. */
function useEnLinea(): boolean {
  const [enLinea, setEnLinea] = useState(() => navigator.onLine);
  useEffect(() => {
    // Los eventos `online`/`offline` sólo disparan en una transición real
    // -- nunca al montar -- así que avisar acá nunca duplica el aviso del
    // primer render.
    const marcarEnLinea = () => {
      setEnLinea(true);
      toast.success("Conexión restablecida.");
    };
    const marcarSinConexion = () => {
      setEnLinea(false);
      toast.warning("Sin conexión a internet — trabajando en modo offline.");
    };
    window.addEventListener("online", marcarEnLinea);
    window.addEventListener("offline", marcarSinConexion);
    return () => {
      window.removeEventListener("online", marcarEnLinea);
      window.removeEventListener("offline", marcarSinConexion);
    };
  }, []);
  return enLinea;
}

function descripcion(estado: EstadoConexionNube, enLinea: boolean): { texto: string; color: string } {
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

/**
 * Botón "Sincronizar" + indicador de conexión en vivo, en la barra de
 * estado — mismo lenguaje visual que `MenuUsuario` (`.barra-estado-boton`,
 * texto plano hasta el hover). Visible para cualquier rol activo:
 * sincronizar (`Operacion::UsarNube`) ya es uso diario normal, no exclusivo
 * de ROOT como configurar el secreto del dispositivo
 * (`Operacion::GestionarNube`, eso sigue solo en la pantalla Nube). Antes
 * este botón vivía únicamente ahí, detrás de una pestaña que ni
 * Administrador ni Operador podían abrir.
 *
 * El botón permite forzar una sincronización además de los avisos de
 * Realtime y del pulso periódico de respaldo; el punto de color aparte es
 * sólo el estado del canal de avisos en vivo (`iniciarRealtimeNube`), no
 * "si hay internet" en general -- una máquina sin red igual puede seguir
 * usando la app local sin problema, esto sólo dice si los avisos
 * instantáneos de otro dispositivo están llegando ahora mismo.
 */
export default function BarraNube({
  sincronizando,
  onSincronizar,
  estadoConexion,
}: {
  sincronizando: boolean;
  onSincronizar: () => void;
  estadoConexion: EstadoConexionNube;
}) {
  const enLinea = useEnLinea();
  const { texto, color } = descripcion(estadoConexion, enLinea);
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
      <span
        title={`Avisos en vivo de la nube: ${texto}`}
        style={{ display: "flex", alignItems: "center", gap: "0.3rem", fontSize: "0.8rem", color }}
      >
        <span
          aria-hidden="true"
          style={{
            width: "0.5rem",
            height: "0.5rem",
            borderRadius: "50%",
            backgroundColor: color,
            display: "inline-block",
          }}
        />
        {texto}
      </span>
      <button
        type="button"
        className="barra-estado-boton"
        onClick={onSincronizar}
        disabled={sincronizando}
        style={{ display: "flex", alignItems: "center", gap: "0.3rem" }}
      >
        <RefreshCw
          size={13}
          strokeWidth={2}
          aria-hidden="true"
          className={sincronizando ? "girando" : undefined}
        />
        {sincronizando ? "Sincronizando…" : "Sincronizar"}
      </button>
    </div>
  );
}

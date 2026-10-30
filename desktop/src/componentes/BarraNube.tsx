import { useEffect, useState } from "react";
import { avisos } from "../avisos/avisos";
import { RefreshCw, TriangleAlert } from "lucide-react";
import { avisoBandejaSalida, type AvisoBandejaSalida } from "../api/nube";
import { descripcion } from "./BarraNube.logica";
import type { EstadoConexionNube } from "./BarraNube.logica";

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
      avisos.exito("Conexión restablecida.");
    };
    const marcarSinConexion = () => {
      setEnLinea(false);
      avisos.advertencia("Sin conexión a internet — trabajando en modo offline.");
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

/** Cada cuánto se revisa la bandeja de salida (lectura local, sin red). */
const MS_REVISION_BANDEJA = 15_000;

/** Aviso de registros de esta PC que no llegaron a la nube (lo decide el
 * núcleo, ver `nube::aviso_bandeja`). Se revisa cada 15 s y al terminar
 * cada sincronización, así se va apenas lo pendiente sube. Incidente del
 * 2026-10-29: una salida quedó 17 min sin subir y ningún equipo lo avisó. */
function useAvisoBandeja(sincronizando: boolean): AvisoBandejaSalida | null {
  const [aviso, setAviso] = useState<AvisoBandejaSalida | null>(null);
  useEffect(() => {
    if (sincronizando) return;
    let vigente = true;
    const revisar = () => {
      avisoBandejaSalida()
        .then((nuevo) => {
          if (vigente) setAviso(nuevo);
        })
        .catch(() => {
          // Sin sesión o base ocupada: no hay nada confiable que mostrar.
          if (vigente) setAviso(null);
        });
    };
    revisar();
    const intervalo = window.setInterval(revisar, MS_REVISION_BANDEJA);
    return () => {
      vigente = false;
      window.clearInterval(intervalo);
    };
  }, [sincronizando]);
  return aviso;
}

/**
 * Botón "Sincronizar" + indicador de conexión en vivo, en la barra de
 * estado — mismo lenguaje visual que `MenuUsuario` (`.barra-estado-boton`,
 * texto plano hasta el hover). Visible para cualquier rol activo:
 * sincronizar (`Operacion::UsarNube`) ya es uso diario normal, no exclusivo
 * de ROOT como vincular este equipo
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
  const { texto, color, parpadea } = descripcion(estadoConexion, enLinea);
  const avisoBandeja = useAvisoBandeja(sincronizando);
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
      {avisoBandeja && (
        <span
          role="status"
          title="Registros hechos en esta PC que todavía no llegaron a la nube. Se reintentan solos; pulse Sincronizar para forzarlo."
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.3rem",
            fontSize: "0.8rem",
            fontWeight: 600,
            color: avisoBandeja.critico ? "var(--error)" : "var(--advertencia)",
          }}
        >
          <TriangleAlert size={13} strokeWidth={2} aria-hidden="true" />
          {avisoBandeja.texto}
        </span>
      )}
      <span
        title={`Avisos en vivo de la nube: ${texto}`}
        style={{ display: "flex", alignItems: "center", gap: "0.3rem", fontSize: "0.8rem", color }}
      >
        <span
          aria-hidden="true"
          className={parpadea ? "punto-conexion-parpadeo" : undefined}
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

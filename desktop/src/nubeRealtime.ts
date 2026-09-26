import { listen } from "@tauri-apps/api/event";
import {
  detenerRealtimeNubeComando,
  iniciarRealtimeNubeComando,
  sincronizarConNube,
} from "./api/nube";
import type { ResumenSincronizacion } from "./api/nube";
import { EVENTO_CAMBIO_LOCAL_NUBE, EVENTO_NUBE_ACTUALIZADA } from "./eventosNube";

export { EVENTO_NUBE_ACTUALIZADA } from "./eventosNube";

export interface NubeActualizadaDetalle {
  origen: "realtime" | "manual";
  resumen: ResumenSincronizacion;
}

/** Mismos 4 valores que entregaba `RealtimeChannel.subscribe` de
 * supabase-js -- ver `EstadoConexionNube` en `componentes/BarraNube.tsx`,
 * que agrega el `null` de "todavía no se intentó conectar". Ahora los
 * emite `desktop/src-tauri/src/realtime_nube.rs` (evento Tauri
 * `nube://estado_realtime`). */
export type EstadoCanalRealtime = "SUBSCRIBED" | "CHANNEL_ERROR" | "TIMED_OUT" | "CLOSED";

interface OpcionesRealtimeNube {
  onSincronizado?: (resumen: ResumenSincronizacion) => void;
  onEstado?: (estado: EstadoCanalRealtime) => void;
  // Quién tiene la sesión abierta en esta PC ahora -- viaja como Presence
  // (ver `realtime_nube::iniciar`), para que el panel pueda mostrar
  // "usuarios en línea y desde dónde" sin abrir una conexión nueva (ver
  // docs/features-futuras/plan-sesion-unica-dispositivos.md).
  usuario?: { cedula: string; nombre: string };
}

/** Avisa a quien esté escuchando (hoy: la pantalla Nube, si está montada)
 * que hay un resumen nuevo — lo usa tanto el aviso en vivo de Realtime como
 * el botón "Sincronizar" de la barra de estado (`BarraNube.tsx`), para que
 * ambos caminos actualicen lo mismo sin duplicar la lógica de refresco. */
export function emitirActualizacion(
  resumen: ResumenSincronizacion,
  origen: NubeActualizadaDetalle["origen"] = "realtime",
) {
  window.dispatchEvent(
    new CustomEvent<NubeActualizadaDetalle>(EVENTO_NUBE_ACTUALIZADA, {
      detail: { origen, resumen },
    }),
  );
}

// Debounce de un cambio LOCAL (registro en ESTA PC) -- mismo valor que
// usaba la versión anterior (`supabase-js`) para coalescer varias altas
// seguidas en una sola sincronización.
const DEBOUNCE_CAMBIO_LOCAL_MS = 600;

/**
 * Arranca el canal privado real de Supabase Realtime -- desde 2026-09-26,
 * mantenido enteramente del lado Rust (`desktop/src-tauri/src/realtime_nube.rs`,
 * cliente Phoenix Channels escrito a medida: reconexión con backoff, JWT de
 * dispositivo con renovación sin reconectar, Presence -- ver
 * `benchmarks/realtime-rust/HANDOFF.md`). Esta función sólo pide que
 * arranque/pare y traduce sus eventos Tauri a los mismos callbacks que ya
 * usaban `App.tsx`/`BarraNube.tsx`, así que ninguno de los dos cambió.
 */
export function iniciarRealtimeNube(opciones: OpcionesRealtimeNube = {}): () => void {
  let cancelado = false;
  let temporizadorCambioLocal: ReturnType<typeof window.setTimeout> | null = null;

  void iniciarRealtimeNubeComando(opciones.usuario?.cedula ?? "", opciones.usuario?.nombre ?? "").catch(
    (error) => console.info("Realtime de nube no quedó activo todavía:", error),
  );

  const cancelarEstado = listen<EstadoCanalRealtime>("nube://estado_realtime", ({ payload }) => {
    if (!cancelado) opciones.onEstado?.(payload);
  });
  const cancelarSincronizado = listen<ResumenSincronizacion>(
    "nube://sincronizado_realtime",
    ({ payload }) => {
      if (cancelado) return;
      opciones.onSincronizado?.(payload);
      emitirActualizacion(payload);
    },
  );

  // Un cambio LOCAL (alta/registro en ESTA PC) dispara un sync casi
  // inmediato para no esperar al pulso periódico de 2 minutos -- separado
  // del canal privado (que sólo reacciona a cambios REMOTOS): un alta local
  // ya está reflejada acá mismo, lo único que falta es subirla.
  function programarSincronizacionLocal() {
    if (cancelado) return;
    if (temporizadorCambioLocal) window.clearTimeout(temporizadorCambioLocal);
    temporizadorCambioLocal = window.setTimeout(() => {
      temporizadorCambioLocal = null;
      sincronizarConNube()
        .then((resumen) => {
          if (cancelado) return;
          opciones.onSincronizado?.(resumen);
          emitirActualizacion(resumen, "manual");
        })
        .catch((error) => console.error("No se pudo sincronizar tras un cambio local:", error));
    }, DEBOUNCE_CAMBIO_LOCAL_MS);
  }
  window.addEventListener(EVENTO_CAMBIO_LOCAL_NUBE, programarSincronizacionLocal);

  return () => {
    cancelado = true;
    window.removeEventListener(EVENTO_CAMBIO_LOCAL_NUBE, programarSincronizacionLocal);
    if (temporizadorCambioLocal) window.clearTimeout(temporizadorCambioLocal);
    cancelarEstado.then((cancelar) => cancelar());
    cancelarSincronizado.then((cancelar) => cancelar());
    void detenerRealtimeNubeComando().catch(() => {});
  };
}

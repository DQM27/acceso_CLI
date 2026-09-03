import { createClient } from "@supabase/supabase-js";
import type { RealtimeChannel, SupabaseClient } from "@supabase/supabase-js";
import { sesionRealtimeNube, sincronizarConNube } from "./api/nube";
import type { ResumenSincronizacion } from "./api/nube";

export const EVENTO_NUBE_ACTUALIZADA = "nube:actualizada";

export interface NubeActualizadaDetalle {
  origen: "realtime" | "manual";
  resumen: ResumenSincronizacion;
}

interface OpcionesRealtimeNube {
  onSincronizado?: (resumen: ResumenSincronizacion) => void;
  onEstado?: (estado: string) => void;
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

// Tope del backoff exponencial de reconexión -- sin esto, un canal que
// nunca logra autorizarse (ver docs/migracion-supabase-realtime-broadcast.sql,
// bug de plataforma de Realtime con JWT Signing Keys) reintenta cada 2
// segundos para siempre: cada vuelta abre un cliente Supabase nuevo, pide un
// token (llamada de red bloqueante del lado Rust) y abre un websocket, todo
// compitiendo por el mismo hilo que atiende la UI -- eso es lo que se sentía
// como "la app se pega" (2026-09-03).
const REINTENTO_BASE_MS = 2_000;
const REINTENTO_TOPE_MS = 60_000;

export function iniciarRealtimeNube(opciones: OpcionesRealtimeNube = {}): () => void {
  let cancelado = false;
  let cliente: SupabaseClient | null = null;
  let canal: RealtimeChannel | null = null;
  let temporizadorRenovar: ReturnType<typeof window.setTimeout> | null = null;
  let temporizadorReconectar: ReturnType<typeof window.setTimeout> | null = null;
  let temporizadorSincronizar: ReturnType<typeof window.setTimeout> | null = null;
  let sincronizando = false;
  // Intentos fallidos seguidos desde la última vez que el canal quedó
  // realmente suscrito -- crece el backoff (2s, 4s, 8s… hasta el tope) en
  // vez de reintentar siempre a los 2s. Se reinicia a 0 en cuanto
  // `subscribe` avisa "SUBSCRIBED", así una falla puntual después de mucho
  // andar bien no arranca desde el tope.
  let intentosSeguidos = 0;

  function limpiarCanal() {
    if (temporizadorRenovar) window.clearTimeout(temporizadorRenovar);
    if (temporizadorReconectar) window.clearTimeout(temporizadorReconectar);
    temporizadorRenovar = null;
    temporizadorReconectar = null;

    if (canal && cliente) {
      void cliente.removeChannel(canal);
    }
    cliente?.realtime.disconnect();
    canal = null;
    cliente = null;
  }

  function reconectar() {
    if (cancelado || temporizadorReconectar) return;
    const espera = Math.min(REINTENTO_BASE_MS * 2 ** intentosSeguidos, REINTENTO_TOPE_MS);
    intentosSeguidos += 1;
    temporizadorReconectar = window.setTimeout(() => {
      temporizadorReconectar = null;
      limpiarCanal();
      void conectar();
    }, espera);
  }

  async function sincronizarPorAviso() {
    if (sincronizando || cancelado) return;
    sincronizando = true;
    try {
      const resumen = await sincronizarConNube();
      opciones.onSincronizado?.(resumen);
      emitirActualizacion(resumen);
    } catch (error) {
      console.error("No se pudo sincronizar tras aviso Realtime:", error);
    } finally {
      sincronizando = false;
    }
  }

  function programarSincronizacion() {
    if (cancelado) return;
    if (temporizadorSincronizar) window.clearTimeout(temporizadorSincronizar);
    temporizadorSincronizar = window.setTimeout(() => {
      temporizadorSincronizar = null;
      void sincronizarPorAviso();
    }, 600);
  }

  async function conectar() {
    if (cancelado) return;
    try {
      const sesion = await sesionRealtimeNube();
      if (cancelado) return;

      cliente = createClient(sesion.base_url, sesion.apikey, {
        auth: {
          persistSession: false,
          autoRefreshToken: false,
          detectSessionInUrl: false,
        },
      });
      await cliente.realtime.setAuth(sesion.access_token);

      canal = cliente
        .channel(sesion.topic, { config: { private: true } })
        .on("broadcast", { event: "cambio_nube" }, programarSincronizacion)
        .subscribe((estado) => {
          opciones.onEstado?.(estado);
          if (estado === "SUBSCRIBED") {
            intentosSeguidos = 0;
          } else if (estado === "CHANNEL_ERROR" || estado === "TIMED_OUT" || estado === "CLOSED") {
            reconectar();
          }
        });

      const renovarEnSegundos = Math.max(60, sesion.expires_in - 60);
      temporizadorRenovar = window.setTimeout(reconectar, renovarEnSegundos * 1000);
    } catch (error) {
      console.info("Realtime de nube no quedó activo todavía:", error);
      reconectar();
    }
  }

  void conectar();

  return () => {
    cancelado = true;
    if (temporizadorSincronizar) window.clearTimeout(temporizadorSincronizar);
    limpiarCanal();
  };
}

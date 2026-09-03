import { createClient } from "@supabase/supabase-js";
import type { RealtimeChannel, SupabaseClient } from "@supabase/supabase-js";
import { sesionRealtimeNube, sincronizarConNube } from "./api/nube";
import type { ResumenSincronizacion } from "./api/nube";

export const EVENTO_NUBE_ACTUALIZADA = "nube:actualizada";

export interface NubeActualizadaDetalle {
  origen: "realtime";
  resumen: ResumenSincronizacion;
}

interface OpcionesRealtimeNube {
  onSincronizado?: (resumen: ResumenSincronizacion) => void;
  onEstado?: (estado: string) => void;
}

function emitirActualizacion(resumen: ResumenSincronizacion) {
  window.dispatchEvent(
    new CustomEvent<NubeActualizadaDetalle>(EVENTO_NUBE_ACTUALIZADA, {
      detail: { origen: "realtime", resumen },
    }),
  );
}

export function iniciarRealtimeNube(opciones: OpcionesRealtimeNube = {}): () => void {
  let cancelado = false;
  let cliente: SupabaseClient | null = null;
  let canal: RealtimeChannel | null = null;
  let temporizadorRenovar: ReturnType<typeof window.setTimeout> | null = null;
  let temporizadorReconectar: ReturnType<typeof window.setTimeout> | null = null;
  let temporizadorSincronizar: ReturnType<typeof window.setTimeout> | null = null;
  let sincronizando = false;

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

  function reconectar(pronto = false) {
    if (cancelado || temporizadorReconectar) return;
    temporizadorReconectar = window.setTimeout(
      () => {
        temporizadorReconectar = null;
        limpiarCanal();
        void conectar();
      },
      pronto ? 2_000 : 30_000,
    );
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
          if (estado === "CHANNEL_ERROR" || estado === "TIMED_OUT" || estado === "CLOSED") {
            reconectar(true);
          }
        });

      const renovarEnSegundos = Math.max(60, sesion.expires_in - 60);
      temporizadorRenovar = window.setTimeout(() => reconectar(true), renovarEnSegundos * 1000);
    } catch (error) {
      console.info("Realtime de nube no quedó activo todavía:", error);
      reconectar(false);
    }
  }

  void conectar();

  return () => {
    cancelado = true;
    if (temporizadorSincronizar) window.clearTimeout(temporizadorSincronizar);
    limpiarCanal();
  };
}

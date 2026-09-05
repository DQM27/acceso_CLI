import { createClient } from "@supabase/supabase-js";
import type { RealtimeChannel, SupabaseClient } from "@supabase/supabase-js";
import { sesionRealtimeNube, sincronizarConNube } from "./api/nube";
import type { ResumenSincronizacion } from "./api/nube";
import { EVENTO_CAMBIO_LOCAL_NUBE, EVENTO_NUBE_ACTUALIZADA } from "./eventosNube";

export { EVENTO_NUBE_ACTUALIZADA } from "./eventosNube";

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

// Espacia los reintentos cuando falta conexión o la sesión no está lista.
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
  let sincronizacionPendiente = false;
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

    const clienteAnterior = cliente;
    const canalAnterior = canal;
    canal = null;
    cliente = null;
    if (canalAnterior && clienteAnterior) {
      void clienteAnterior.removeChannel(canalAnterior);
    }
    clienteAnterior?.realtime.disconnect();
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
    if (cancelado) return;
    if (sincronizando) {
      sincronizacionPendiente = true;
      return;
    }
    sincronizacionPendiente = false;
    sincronizando = true;
    try {
      const resumen = await sincronizarConNube();
      if (!cancelado) {
        opciones.onSincronizado?.(resumen);
        emitirActualizacion(resumen);
      }
    } catch (error) {
      console.error("No se pudo sincronizar tras aviso Realtime:", error);
    } finally {
      sincronizando = false;
      if (sincronizacionPendiente && !cancelado) programarSincronizacion();
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

      const clienteActual = createClient(sesion.base_url, sesion.apikey, {
        // Supabase vuelve a consultar este callback al conectar y renovar.
        // setAuth por sí solo se reemplaza por la sesión de Auth (aquí vacía).
        accessToken: async () => sesion.access_token,
        auth: {
          persistSession: false,
          autoRefreshToken: false,
          detectSessionInUrl: false,
        },
      });
      cliente = clienteActual;
      await clienteActual.realtime.setAuth(sesion.access_token);
      if (cancelado || cliente !== clienteActual) {
        clienteActual.realtime.disconnect();
        return;
      }

      canal = cliente
        .channel(sesion.topic, { config: { private: true } })
        .on("broadcast", { event: "cambio_nube" }, ({ payload }) => {
          if (cancelado || cliente !== clienteActual) return;
          if (payload?.dispositivo_id !== sesion.dispositivo_id) programarSincronizacion();
        })
        .subscribe((estado, error) => {
          if (cancelado || cliente !== clienteActual) return;
          opciones.onEstado?.(estado);
          if (estado === "SUBSCRIBED") {
            intentosSeguidos = 0;
            // Recupera cambios ocurridos mientras el cliente estuvo desconectado.
            programarSincronizacion();
          } else if (estado === "CHANNEL_ERROR" || estado === "TIMED_OUT" || estado === "CLOSED") {
            if (error) console.info("No se pudo suscribir al canal de nube:", error.message);
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

  window.addEventListener(EVENTO_CAMBIO_LOCAL_NUBE, programarSincronizacion);
  void conectar();

  return () => {
    cancelado = true;
    window.removeEventListener(EVENTO_CAMBIO_LOCAL_NUBE, programarSincronizacion);
    if (temporizadorSincronizar) window.clearTimeout(temporizadorSincronizar);
    limpiarCanal();
  };
}

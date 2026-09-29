import { createClient } from "@supabase/supabase-js";
import type { RealtimeChannel, SupabaseClient } from "@supabase/supabase-js";
import {
  aplicarCambioNube,
  desfaseRelojMs,
  enviarCambiosNube,
  sesionRealtimeNube,
  sincronizarCambiosNube,
  sincronizarConNube,
} from "./api/nube";
import type { MotivoExpulsion, ResumenSincronizacion } from "./api/nube";
import { motivoSiEsParaEsteEquipo } from "./expulsionNube";
import { EVENTO_CAMBIO_EN_VIVO, EVENTO_CAMBIO_LOCAL_NUBE, EVENTO_NUBE_ACTUALIZADA } from "./eventosNube";
import { realtimeTelemetria, telemetriaActiva } from "./telemetria";
import { latenciaDesde, tipoDeError } from "./telemetriaCalculos";

export { EVENTO_NUBE_ACTUALIZADA } from "./eventosNube";

export interface NubeActualizadaDetalle {
  origen: "realtime" | "manual";
  resumen: ResumenSincronizacion;
}

/** Mismos 4 valores que entrega `RealtimeChannel.subscribe` de
 * supabase-js -- ver `EstadoConexionNube` en `componentes/BarraNube.tsx`,
 * que agrega el `null` de "todavía no se intentó conectar". */
export type EstadoCanalRealtime = "SUBSCRIBED" | "CHANNEL_ERROR" | "TIMED_OUT" | "CLOSED";

interface OpcionesRealtimeNube {
  onSincronizado?: (resumen: ResumenSincronizacion) => void;
  onEstado?: (estado: EstadoCanalRealtime) => void;
  /** Este equipo fue revocado, suspendido o re-vinculado en otro equipo
   * (ver `expulsionNube.ts`). */
  onExpulsado?: (motivo: MotivoExpulsion) => void;
  // Quién tiene la sesión abierta en esta PC ahora -- viaja en el mismo
  // `track()` que ya marca el dispositivo como presente, para que el panel
  // pueda mostrar "usuarios en línea y desde dónde" (ver
  // docs/features-futuras/plan-sesion-unica-dispositivos.md). Cédula, no el `id` local (el
  // rowid de este SQLite no es el id global de `usuarios` en Supabase que
  // ve el panel -- la cédula es la única clave que coincide en los dos
  // lados).
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

// Espacia los reintentos cuando falta conexión o la sesión no está lista.
const REINTENTO_BASE_MS = 2_000;
const REINTENTO_TOPE_MS = 60_000;
// Junta avisos remotos que llegan casi a la vez en una sola corrida.
const PAUSA_AGRUPACION_MS = 600;
// Cada cuánto, como mucho, se relee el desfase del reloj (telemetría).
const INTERVALO_DESFASE_MS = 10_000;

export function iniciarRealtimeNube(opciones: OpcionesRealtimeNube = {}): () => void {
  let cancelado = false;
  let cliente: SupabaseClient | null = null;
  let canal: RealtimeChannel | null = null;
  let temporizadorRenovar: ReturnType<typeof window.setTimeout> | null = null;
  let temporizadorReconectar: ReturnType<typeof window.setTimeout> | null = null;
  let temporizadorSincronizar: ReturnType<typeof window.setTimeout> | null = null;
  let sincronizando = false;
  let sincronizacionPendiente = false;
  // Qué hay que sincronizar en la próxima corrida. Un aviso remoto que no
  // se pudo aplicar con su fila pide sólo su tabla (`payload.table`, ver
  // `sincronizarCambiosNube`); la (re)conexión del canal o un aviso sin
  // tabla piden la sincronización completa (incremental: sólo lo que
  // cambió). Un cambio local no pide nada de eso: con nada pendiente, la
  // corrida sólo envía (`enviarCambiosNube`). Antes TODO aviso corría la
  // completa (~13 consultas a la nube por un solo cambio) -- el aviso
  // llegaba al instante, lo que tardaba era lo que se hacía después.
  let pendienteCompleta = false;
  const tablasPendientes = new Set<string>();
  // Intentos fallidos seguidos desde la última vez que el canal quedó
  // realmente suscrito -- crece el backoff (2s, 4s, 8s… hasta el tope) en
  // vez de reintentar siempre a los 2s. Se reinicia a 0 en cuanto
  // `subscribe` avisa "SUBSCRIBED", así una falla puntual después de mucho
  // andar bien no arranca desde el tope.
  let intentosSeguidos = 0;
  // Telemetría de diagnóstico (sin efecto si no está activa): desde cuándo
  // está suscrito el canal actual y el desfase del reloj con que se corrige
  // la latencia de los avisos.
  let suscritoDesde: number | null = null;
  let desfaseReloj: number | null = null;
  let desfaseLeidoEn = 0;

  /** Vuelve a leer el desfase, como mucho cada `INTERVALO_DESFASE_MS`: la
   * medición en milisegundos corre en segundo plano tras autenticar y se
   * aplica recién con el token siguiente, así que la lectura al conectar
   * puede ser todavía la del header `Date` (±1 s). No espera: el aviso en
   * curso usa el valor anterior. */
  function refrescarDesfase() {
    if (!telemetriaActiva()) return;
    const ahora = performance.now();
    if (ahora - desfaseLeidoEn < INTERVALO_DESFASE_MS) return;
    desfaseLeidoEn = ahora;
    void desfaseRelojMs()
      .then((leido) => {
        desfaseReloj = leido;
        realtimeTelemetria()?.desfaseReloj(leido);
      })
      .catch(() => undefined);
  }

  /** El canal actual dejó de estar suscrito (o nunca llegó a estarlo). */
  function anotarFinDeConexion(motivo: string) {
    const telemetria = realtimeTelemetria();
    if (!telemetria) return;
    if (suscritoDesde === null) telemetria.error(motivo);
    else telemetria.terminado(motivo, performance.now() - suscritoDesde);
    suscritoDesde = null;
  }

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
    const completa = pendienteCompleta;
    const tablas = [...tablasPendientes];
    pendienteCompleta = false;
    tablasPendientes.clear();
    try {
      const resumen = completa
        ? await sincronizarConNube()
        : tablas.length > 0
          ? await sincronizarCambiosNube(tablas)
          : await enviarCambiosNube();
      if (!cancelado) {
        opciones.onSincronizado?.(resumen);
        emitirActualizacion(resumen);
      }
    } catch (error) {
      console.error("No se pudo sincronizar tras aviso Realtime:", error);
    } finally {
      sincronizando = false;
      // Lo que llegó mientras corría ya quedó anotado en
      // `tablasPendientes`/`pendienteCompleta`.
      if (sincronizacionPendiente && !cancelado) programarCorrida();
    }
  }

  function programarSincronizacion(tabla?: string) {
    if (cancelado) return;
    if (tabla) tablasPendientes.add(tabla);
    else pendienteCompleta = true;
    programarCorrida();
  }

  function programarCorrida() {
    if (cancelado) return;
    if (temporizadorSincronizar) window.clearTimeout(temporizadorSincronizar);
    // La pausa junta avisos remotos que llegan casi a la vez. Si lo único
    // pendiente es subir un cambio hecho en esta PC, sale enseguida:
    // esperar sólo demoraba ~0,6 s que el otro equipo lo viera.
    const soloEnvio = !pendienteCompleta && tablasPendientes.size === 0;
    temporizadorSincronizar = window.setTimeout(
      () => {
        temporizadorSincronizar = null;
        void sincronizarPorAviso();
      },
      soloEnvio ? 0 : PAUSA_AGRUPACION_MS,
    );
  }

  async function conectar() {
    if (cancelado) return;
    realtimeTelemetria()?.conectando();
    const inicioIntento = performance.now();
    try {
      const sesion = await sesionRealtimeNube();
      if (cancelado) return;
      if (telemetriaActiva()) {
        desfaseReloj = await desfaseRelojMs().catch(() => null);
        desfaseLeidoEn = performance.now();
        realtimeTelemetria()?.desfaseReloj(desfaseReloj);
      }

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
          refrescarDesfase();
          const ecoPropio = payload?.dispositivo_id === sesion.dispositivo_id;
          realtimeTelemetria()?.aviso(
            typeof payload?.table === "string" ? payload.table : undefined,
            JSON.stringify(payload ?? null).length,
            ecoPropio,
            typeof payload?.changed_at === "string"
              ? latenciaDesde(payload.changed_at, Date.now(), desfaseReloj)
              : null,
          );
          if (ecoPropio) return;
          const tabla = typeof payload?.table === "string" ? payload.table : undefined;
          // El aviso trae la fila: se guarda SÓLO esa fila (Activos e
          // Historial) y se refresca la pantalla, sin consultar la nube. Si
          // no se pudo aplicar, se sincroniza sólo su tabla. El pulso
          // periódico sigue siendo la red de seguridad.
          if (payload?.registro || payload?.operation === "DELETE") {
            const inicioAplicar = performance.now();
            void aplicarCambioNube(payload)
              .then((aplicado) => {
                realtimeTelemetria()?.aplicado(aplicado, performance.now() - inicioAplicar);
                if (cancelado) return;
                if (aplicado) window.dispatchEvent(new Event(EVENTO_CAMBIO_EN_VIVO));
                else programarSincronizacion(tabla);
              })
              .catch((error) => {
                realtimeTelemetria()?.aplicado(false, performance.now() - inicioAplicar);
                console.info("No se pudo aplicar el cambio en vivo:", error);
                programarSincronizacion(tabla);
              });
            return;
          }
          programarSincronizacion(tabla);
        })
        .on("broadcast", { event: "dispositivo_expulsado" }, ({ payload }) => {
          if (cancelado || cliente !== clienteActual) return;
          const motivo = motivoSiEsParaEsteEquipo(payload, sesion);
          if (motivo) opciones.onExpulsado?.(motivo);
        })
        .subscribe((estado, error) => {
          if (cancelado || cliente !== clienteActual) return;
          opciones.onEstado?.(estado);
          if (estado === "SUBSCRIBED") {
            intentosSeguidos = 0;
            suscritoDesde = performance.now();
            realtimeTelemetria()?.suscrito(suscritoDesde - inicioIntento);
            // Recupera cambios ocurridos mientras el cliente estuvo desconectado.
            programarSincronizacion();
            // Presencia (docs/features-futuras/plan-sesion-unica-dispositivos.md, "Panel de
            // presencia en tiempo real"): marca este dispositivo como
            // conectado mientras dure la suscripción -- sin "untrack"
            // explícito, `limpiarCanal`/el cierre del socket ya lo saca de
            // la lista de presentes del lado del servidor.
            void canal?.track({
              dispositivo_id: sesion.dispositivo_id,
              usuario_cedula: opciones.usuario?.cedula,
              usuario_nombre: opciones.usuario?.nombre,
            });
          } else if (estado === "CHANNEL_ERROR" || estado === "TIMED_OUT" || estado === "CLOSED") {
            if (error) console.info("No se pudo suscribir al canal de nube:", error.message);
            anotarFinDeConexion(estado);
            reconectar();
          }
        });

      const renovarEnSegundos = Math.max(60, sesion.expires_in - 60);
      temporizadorRenovar = window.setTimeout(() => {
        anotarFinDeConexion("renovacion");
        reconectar();
      }, renovarEnSegundos * 1000);
    } catch (error) {
      if (cancelado) return;
      realtimeTelemetria()?.error(tipoDeError(error));
      opciones.onEstado?.("CHANNEL_ERROR");
      console.info("Realtime de nube no quedó activo todavía:", error);
      reconectar();
    }
  }

  // Envoltorio propio: `addEventListener` le pasaría el `Event` como primer
  // argumento. Un cambio local sólo sube lo pendiente: no hay nada que bajar
  // por un ingreso/salida que se registró acá mismo.
  const alCambioLocal = () => programarCorrida();
  window.addEventListener(EVENTO_CAMBIO_LOCAL_NUBE, alCambioLocal);
  void conectar();

  return () => {
    cancelado = true;
    window.removeEventListener(EVENTO_CAMBIO_LOCAL_NUBE, alCambioLocal);
    if (temporizadorSincronizar) window.clearTimeout(temporizadorSincronizar);
    limpiarCanal();
  };
}

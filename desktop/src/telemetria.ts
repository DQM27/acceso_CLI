// Telemetría del escritorio, lado frontend. Un solo código para todos los
// builds: sólo mide los grupos que el backend diga que rigen ahora (la orden
// del panel para este equipo; ver `src-tauri/src/telemetria.rs`). Lo que no
// rige cuesta un chequeo de un conjunto. El backend vuelve a filtrar por
// grupo, así que acá es sólo para no gastar en lo que no se manda.
//
// Qué junta acá: el tiempo de cada comando de Tauri (IPC incluido), el canal
// de avisos en vivo, las pantallas, los errores de JavaScript (sólo su
// tipo), las tareas largas que traban la ventana y la memoria del WebView.
// Cada 30 s lo resume y se lo pasa al backend, que lo encola y lo manda a
// staging. Nunca cédulas, nombres, contenido de formularios ni mensajes de
// error.

import { invoke } from "@tauri-apps/api/core";
import {
  AgregadorBloqueos,
  AgregadorLlamadas,
  AgregadorRealtime,
  clasificarErrorComando,
  nombreCanonicoOperacion,
  tipoDeError,
  tipoDeRed,
} from "./telemetriaCalculos";

const INTERVALO_VOLCADO_MS = 30_000;
/** La memoria del WebView cambia despacio: se anota una vez cada tantos
 * volcados (cada 2 minutos), no en todos. Con 30 s eran la mitad de las filas
 * de una PC en reposo. */
const VOLCADOS_POR_MUESTRA_WEBVIEW = 4;
const PRIMERA_REVISION_MS = 5_000;

interface Evento {
  tipo: string;
  datos: Record<string, unknown>;
}

/** Los grupos que rigen ahora (`base` siempre). Vacío hasta que
 * `iniciarTelemetria` pregunte. */
let grupos: ReadonlySet<string> = new Set();
const llamadas = new AgregadorLlamadas();
const bloqueos = new AgregadorBloqueos();
const agregadorRealtime = new AgregadorRealtime();
const erroresJs = new Map<string, number>();
const pendientes: Evento[] = [];
let pantallaActual = "(ninguna)";
let volcados = 0;

/** ¿Rige este grupo de telemetría ahora? Falso hasta que `iniciarTelemetria`
 * pregunte. */
export function mide(grupo: string): boolean {
  return grupos.has(grupo);
}

/** El agregador del canal en vivo, o `null` si no rige `sincronizacion`
 * (mismo patrón que `Telemetria.realtime?` en Kotlin). */
export function realtimeTelemetria(): AgregadorRealtime | null {
  return mide("sincronizacion") ? agregadorRealtime : null;
}

/** Un comando de Tauri terminó (ver `api/invocar.ts`). Si falló, `error` es sólo
 * para elegir su tipo de una lista fija: el mensaje nunca se guarda. */
export function registrarLlamada(nombre: string, ms: number, ok: boolean, error?: unknown) {
  if (!mide("rendimiento")) return;
  llamadas.registrar(
    nombreCanonicoOperacion(nombre),
    ms,
    ok,
    ok ? undefined : clasificarErrorComando(error, navigator.onLine),
  );
}

/** Se mostró una pantalla o sección. `ms_hasta_primer_frame`: hasta que el
 * navegador pintó el cuadro siguiente (dos `requestAnimationFrame`). */
export function registrarPantalla(nombre: string) {
  if (nombre === pantallaActual) return;
  const anterior = pantallaActual;
  // Se anota aunque todavía no se sepa si mide (`iniciarTelemetria` es
  // asíncrona): las muestras siguientes dicen en qué pantalla ocurrieron.
  pantallaActual = nombre;
  if (!mide("rendimiento")) return;
  const inicio = performance.now();
  requestAnimationFrame(() =>
    requestAnimationFrame(() => {
      evento("pantalla", {
        pantalla: nombre,
        anterior,
        ms_hasta_primer_frame: Math.round(performance.now() - inicio),
      });
    }),
  );
}

function evento(tipo: string, datos: Record<string, unknown>) {
  pendientes.push({ tipo, datos });
}

/** Un error que atrapó un `ErrorBoundary` de React (no llega a
 * `window.onerror`). */
export function registrarErrorRender(error: unknown) {
  registrarErrorJs("render", error);
}

function registrarErrorJs(origen: string, error: unknown) {
  const clave = `${origen}:${tipoDeError(error)}`;
  erroresJs.set(clave, (erroresJs.get(clave) ?? 0) + 1);
}

/** Memoria del motor de JavaScript (sólo Chromium/WebView2 la informa) y
 * tamaño del DOM: una pantalla que crece sin bajar es una fuga. */
function muestraWebview(): Record<string, unknown> {
  const memoria = (performance as Performance & {
    memory?: { usedJSHeapSize: number; totalJSHeapSize: number };
  }).memory;
  const mb = (bytes: number) => Math.round(bytes / (1024 * 1024));
  return {
    pantalla: pantallaActual,
    js_heap_usada_mb: memoria ? mb(memoria.usedJSHeapSize) : null,
    js_heap_total_mb: memoria ? mb(memoria.totalJSHeapSize) : null,
    nodos_dom: document.getElementsByTagName("*").length,
  };
}

function volcar() {
  const red = tipoDeRed(navigator.onLine);
  // Los agregadores se vacían siempre (lo que quedó de un grupo que ya no
  // rige no debe acumularse); el backend descarta lo que no corresponde.
  llamadas.vaciar().forEach((fila) => evento("llamada_nucleo", { ...fila, red_tipo: red }));
  const realtime = agregadorRealtime.vaciar();
  if (realtime) evento("realtime", realtime);
  const trabas = bloqueos.vaciar();
  if (trabas) evento("ui_bloqueos", { ...trabas, pantalla: pantallaActual });
  if (erroresJs.size > 0) {
    evento("error_js", { por_tipo: Object.fromEntries(erroresJs), pantalla: pantallaActual });
    erroresJs.clear();
  }
  if (mide("recursos") && volcados % VOLCADOS_POR_MUESTRA_WEBVIEW === 0) {
    evento("muestra_webview", muestraWebview());
  }
  volcados += 1;
  const eventos = pendientes.splice(0);
  if (eventos.length === 0) return;
  // Directo a `invoke`, no por `api/invocar.ts`: no se mide a sí misma.
  void invoke("telemetria_eventos", { eventos }).catch(() => {
    // Sólo diagnóstico: si el backend no la recibe, se pierde.
  });
}

let escuchasListas = false;

/** Las escuchas globales se instalan una vez y se quedan: cada una anota
 * sólo mientras rige su grupo. */
function instalarEscuchas() {
  if (escuchasListas) return;
  escuchasListas = true;
  window.addEventListener("error", (e) => {
    registrarErrorJs("error", e.error);
  });
  window.addEventListener("unhandledrejection", (e) => {
    registrarErrorJs("promesa", e.reason);
  });
  if (PerformanceObserver.supportedEntryTypes?.includes("longtask")) {
    new PerformanceObserver((lista) => {
      if (mide("fluidez")) lista.getEntries().forEach((entrada) => bloqueos.registrar(entrada.duration));
    }).observe({ type: "longtask", buffered: true });
  }
}

/** Pregunta al backend qué grupos rigen ahora. Sin respuesta queda sólo
 * `base`: nunca se mide de más por no poder preguntar. */
async function sincronizarEstado() {
  try {
    grupos = new Set(await invoke<string[]>("telemetria_grupos"));
  } catch {
    grupos = new Set(["base"]);
  }
  instalarEscuchas();
}

/** Arranca la telemetría del frontend: pregunta al backend qué grupos rigen, y
 * sigue preguntando cada vez que vuelca (es una llamada local, sin red): la
 * orden del panel puede llegar o vencer con la app abierta. Llamar una sola
 * vez al arrancar. */
export async function iniciarTelemetria(): Promise<void> {
  await sincronizarEstado();
  // La primera sincronización de la nube puede traer una orden nueva unos
  // segundos después de abrir.
  window.setTimeout(() => void sincronizarEstado(), PRIMERA_REVISION_MS);
  window.setInterval(() => {
    volcar();
    void sincronizarEstado();
  }, INTERVALO_VOLCADO_MS);
}

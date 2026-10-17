// Telemetría del escritorio, lado frontend. Sólo mide mientras el backend
// diga que está encendida (build de diagnóstico, o un release con una regla
// vigente en staging; ver `src-tauri/src/telemetria.rs` y
// `docs/telemetria-diagnostico.md`): si no, cada punto de medición es un
// chequeo de un booleano.
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
  nombreCanonicoOperacion,
  tipoDeError,
  tipoDeRed,
} from "./telemetriaCalculos";

const INTERVALO_VOLCADO_MS = 30_000;
const PRIMERA_REVISION_MS = 5_000;

interface Evento {
  tipo: string;
  datos: Record<string, unknown>;
}

let activa = false;
const llamadas = new AgregadorLlamadas();
const bloqueos = new AgregadorBloqueos();
const agregadorRealtime = new AgregadorRealtime();
const erroresJs = new Map<string, number>();
const pendientes: Evento[] = [];
let pantallaActual = "(ninguna)";

/** ¿Esta compilación mide? Falso hasta que `iniciarTelemetria` pregunte. */
export function telemetriaActiva(): boolean {
  return activa;
}

/** El agregador del canal en vivo, o `null` sin telemetría (mismo patrón
 * que `Telemetria.realtime?` en Kotlin). */
export function realtimeTelemetria(): AgregadorRealtime | null {
  return activa ? agregadorRealtime : null;
}

/** Un comando de Tauri terminó (ver `api/invocar.ts`). */
export function registrarLlamada(nombre: string, ms: number, ok: boolean) {
  if (activa) llamadas.registrar(nombreCanonicoOperacion(nombre), ms, ok);
}

/** Se mostró una pantalla o sección. `ms_hasta_primer_frame`: hasta que el
 * navegador pintó el cuadro siguiente (dos `requestAnimationFrame`). */
export function registrarPantalla(nombre: string) {
  if (nombre === pantallaActual) return;
  const anterior = pantallaActual;
  // Se anota aunque todavía no se sepa si mide (`iniciarTelemetria` es
  // asíncrona): las muestras siguientes dicen en qué pantalla ocurrieron.
  pantallaActual = nombre;
  if (!activa) return;
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
  if (activa) registrarErrorJs("render", error);
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
  if (!activa && pendientes.length === 0) return;
  const red = tipoDeRed(navigator.onLine);
  llamadas.vaciar().forEach((fila) => evento("llamada_nucleo", { ...fila, red_tipo: red }));
  const realtime = agregadorRealtime.vaciar();
  if (realtime) evento("realtime", realtime);
  const trabas = bloqueos.vaciar();
  if (trabas) evento("ui_bloqueos", { ...trabas, pantalla: pantallaActual });
  if (erroresJs.size > 0) {
    evento("error_js", { por_tipo: Object.fromEntries(erroresJs), pantalla: pantallaActual });
    erroresJs.clear();
  }
  evento("muestra_webview", muestraWebview());
  const eventos = pendientes.splice(0);
  // Directo a `invoke`, no por `api/invocar.ts`: no se mide a sí misma.
  void invoke("telemetria_eventos", { eventos }).catch(() => {
    // Sólo diagnóstico: si el backend no la recibe, se pierde.
  });
}

let escuchasListas = false;

/** Las escuchas globales se instalan la primera vez que se enciende y se
 * quedan: mientras `activa` sea falso, no anotan nada. */
function instalarEscuchas() {
  if (escuchasListas) return;
  escuchasListas = true;
  window.addEventListener("error", (e) => {
    if (activa) registrarErrorJs("error", e.error);
  });
  window.addEventListener("unhandledrejection", (e) => {
    if (activa) registrarErrorJs("promesa", e.reason);
  });
  if (PerformanceObserver.supportedEntryTypes?.includes("longtask")) {
    new PerformanceObserver((lista) => {
      if (activa) lista.getEntries().forEach((entrada) => bloqueos.registrar(entrada.duration));
    }).observe({ type: "longtask", buffered: true });
  }
}

/** Pregunta al backend si está midiendo (el build de diagnóstico siempre; un
 * release, mientras haya una regla vigente en staging) y ajusta. */
async function sincronizarEstado() {
  try {
    activa = await invoke<boolean>("telemetria_activa");
  } catch {
    activa = false;
  }
  if (activa) instalarEscuchas();
}

/** Arranca la telemetría del frontend: pregunta al backend si mide, y sigue
 * preguntando cada vez que vuelca, porque una regla puede encenderla o
 * apagarla con la app abierta. Sin regla no hace nada más que esa pregunta.
 * Llamar una sola vez al arrancar. */
export async function iniciarTelemetria(): Promise<void> {
  await sincronizarEstado();
  // La consulta de reglas del backend termina unos segundos después de abrir.
  window.setTimeout(() => void sincronizarEstado(), PRIMERA_REVISION_MS);
  window.setInterval(() => {
    volcar();
    void sincronizarEstado();
  }, INTERVALO_VOLCADO_MS);
}

// Registro de errores del frontend: cada error de JavaScript va al registro
// del núcleo (`reportar_error_js` en `src-tauri/src/errores.rs`), que lo
// guarda en disco y lo manda al receptor propio. Siempre encendido: no
// depende de la orden de telemetría. El núcleo limpia los datos personales
// del mensaje y de la traza, y limita las repeticiones.
//
// Tres fuentes: `window.onerror`, las promesas rechazadas sin `catch` y los
// errores de render que atrapa un `ErrorBoundary` (esos no llegan a
// `window.onerror`).

import { invoke } from "@tauri-apps/api/core";

export type OrigenErrorJs = "error" | "promesa" | "render";

const LARGO_MAXIMO_TITULO = 300;

/** Título y traza de un error cualquiera (lo que se lanzó puede no ser un
 * `Error`). */
export function describirError(error: unknown, origen: OrigenErrorJs): { titulo: string; traza: string } {
  const prefijo = origen === "promesa" ? "Promesa rechazada: " : origen === "render" ? "Render: " : "";
  if (error instanceof Error) {
    const mensaje = error.message ? `${error.name}: ${error.message}` : error.name;
    return { titulo: `${prefijo}${mensaje}`.slice(0, LARGO_MAXIMO_TITULO), traza: error.stack ?? "" };
  }
  let texto: string;
  try {
    texto = typeof error === "string" ? error : (JSON.stringify(error) ?? String(error));
  } catch {
    texto = String(error);
  }
  return { titulo: `${prefijo}${texto || "(sin mensaje)"}`.slice(0, LARGO_MAXIMO_TITULO), traza: "" };
}

/** Manda el error al núcleo. Nunca lanza: reportar un error no puede causar
 * otro. */
export function reportarErrorJs(error: unknown, origen: OrigenErrorJs) {
  const { titulo, traza } = describirError(error, origen);
  void invoke("reportar_error_js", { titulo, traza }).catch(() => {
    // Sin backend (o falló el IPC): queda sólo la consola.
  });
}

let pantallaInformada: string | null = null;

/** La pantalla que está viendo quien opera: viaja con el próximo error
 * (también con un pánico del lado Rust). */
export function fijarPantallaErrores(nombre: string) {
  if (nombre === pantallaInformada) return;
  pantallaInformada = nombre;
  void invoke("errores_pantalla", { nombre }).catch(() => {});
}

let instalado = false;

/** Instala las escuchas globales. Una sola vez, antes de montar React. */
export function instalarCapturaDeErrores() {
  if (instalado) return;
  instalado = true;
  window.addEventListener("error", (e) => reportarErrorJs(e.error ?? e.message, "error"));
  window.addEventListener("unhandledrejection", (e) => reportarErrorJs(e.reason, "promesa"));
}

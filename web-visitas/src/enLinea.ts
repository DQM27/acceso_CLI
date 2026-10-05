import { useSyncExternalStore } from "react";

function suscribir(oyente: () => void) {
  window.addEventListener("online", oyente);
  window.addEventListener("offline", oyente);
  return () => {
    window.removeEventListener("online", oyente);
    window.removeEventListener("offline", oyente);
  };
}

/** `false` cuando el navegador sabe que no hay conexión. */
export function useEnLinea() {
  return useSyncExternalStore(suscribir, () => navigator.onLine);
}

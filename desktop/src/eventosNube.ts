export const EVENTO_CAMBIO_LOCAL_NUBE = "nube:cambio-local";
export const EVENTO_NUBE_ACTUALIZADA = "nube:actualizada";

/** El registro local termina primero; la subida se procesa en segundo plano. */
export function solicitarSincronizacionNube() {
  window.dispatchEvent(new Event(EVENTO_CAMBIO_LOCAL_NUBE));
}

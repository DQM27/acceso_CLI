export const EVENTO_CAMBIO_LOCAL_NUBE = "nube:cambio-local";
export const EVENTO_NUBE_ACTUALIZADA = "nube:actualizada";
/** Se guardó en la base local la fila que trajo un aviso en vivo: las
 * pantallas que muestran esos datos se recargan sin esperar la
 * sincronización. */
export const EVENTO_CAMBIO_EN_VIVO = "nube:cambio-en-vivo";

/** El registro local termina primero; la subida se procesa en segundo plano. */
export function solicitarSincronizacionNube() {
  window.dispatchEvent(new Event(EVENTO_CAMBIO_LOCAL_NUBE));
}

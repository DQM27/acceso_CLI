import type { ResumenSincronizacion } from "./api/nube";

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

export interface NubeActualizadaDetalle {
  origen: "realtime" | "manual";
  resumen: ResumenSincronizacion;
}

/** Avisa a quien esté escuchando que hay un resumen nuevo — lo usa tanto el
 * aviso en vivo de Realtime como el botón "Sincronizar" de la barra de estado
 * (`BarraNube.tsx`), para que ambos caminos actualicen lo mismo sin duplicar
 * la lógica de refresco. Vive acá y no en `nubeRealtime.ts` para que `App`
 * pueda usarla sin cargar `@supabase/supabase-js` de entrada. */
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

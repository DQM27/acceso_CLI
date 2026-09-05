import { useEffect, useRef } from "react";

/**
 * Refresca sola cada `intervaloMs` mientras la pestaña está visible --
 * sin esto, un ingreso ya sincronizado en Supabase no aparece hasta que
 * alguien apriete "actualizar" a mano. No usa Supabase Realtime a
 * propósito: sus canales privados no logran autorizarse hoy (bug de la
 * plataforma con el sistema nuevo de JWT Signing Keys, confirmado en
 * `src/nube/sincronizacion.rs` -- mismo motivo por el que desktop/mobile
 * ya reemplazaron Realtime por sync periódico). `recargar` se pasa por
 * ref para no reiniciar el intervalo cada vez que cambia de identidad
 * (ej. en Historial, que depende del rango de fechas elegido). Se pausa
 * con la pestaña oculta para no gastar llamadas de más en una pestaña
 * que nadie está mirando.
 */
export function useAutoRefresh(recargar: () => void, intervaloMs: number) {
  const recargarRef = useRef(recargar);
  recargarRef.current = recargar;

  useEffect(() => {
    const id = setInterval(() => {
      if (!document.hidden) recargarRef.current();
    }, intervaloMs);
    return () => clearInterval(id);
  }, [intervaloMs]);
}

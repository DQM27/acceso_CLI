import { startTransition, useEffect, useRef } from "react";
import { supabase } from "../lib/supabase";

/**
 * Las tablas publicadas disparan la recarga vía Postgres Changes, con la
 * sesión y las políticas RLS del panel. El intervalo y la vuelta a una
 * pestaña visible recuperan cambios que ocurrieron sin conexión.
 *
 * `recargarRef.current()` corre dentro de `startTransition` (React 19.3):
 * este refresco "silencioso" puede llegar en cualquier momento mientras el
 * usuario está filtrando/escribiendo en la pantalla -- sin la transición,
 * el refetch/rerender que dispara compite en prioridad con eso. Mismo
 * criterio que `setRefrescarActivos` en `desktop/src/App.tsx`.
 */
export function useAutoRefresh(recargar: () => void, intervaloMs: number, tablas = "") {
  const recargarRef = useRef(recargar);
  useEffect(() => {
    recargarRef.current = recargar;
  });

  useEffect(() => {
    let vigente = true;
    let temporizador: ReturnType<typeof setTimeout> | undefined;
    const programar = () => {
      if (!vigente || document.hidden || temporizador) return;
      temporizador = setTimeout(() => {
        temporizador = undefined;
        if (!document.hidden) startTransition(() => recargarRef.current());
      }, 300);
    };
    const canal = tablas ? supabase.channel(`panel:${tablas}:${crypto.randomUUID()}`) : null;
    for (const tabla of tablas.split(",").filter(Boolean)) {
      canal?.on("postgres_changes", { event: "*", schema: "public", table: tabla }, programar);
    }
    canal?.subscribe((estado, error) => {
      if (estado === "SUBSCRIBED") programar();
      if (error) console.info("No se pudo suscribir el panel a los cambios:", error.message);
    });
    document.addEventListener("visibilitychange", programar);
    const id = setInterval(() => {
      if (!document.hidden) startTransition(() => recargarRef.current());
    }, intervaloMs);
    return () => {
      vigente = false;
      clearInterval(id);
      clearTimeout(temporizador);
      document.removeEventListener("visibilitychange", programar);
      if (canal) void supabase.removeChannel(canal);
    };
  }, [intervaloMs, tablas]);
}

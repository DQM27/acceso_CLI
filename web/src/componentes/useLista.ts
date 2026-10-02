import { useCallback, useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { mensajeError } from "../mensajeError";
import { useAutoRefresh } from "./useAutoRefresh";

/**
 * Lista que se carga entera desde Supabase (Contratistas, Usuarios,
 * Dispositivos): una consulta de React Query más el refresco por Realtime.
 * Reemplaza el `cargando`/`filas`/`recargar` que cada pantalla armaba a mano.
 *
 * - Realtime y el intervalo sólo INVALIDAN la consulta: si cambió algo, se
 *   vuelve a pedir en segundo plano y la tabla conserva las filas viejas
 *   mientras tanto.
 * - Dos pantallas que usen la misma `clave` comparten una sola petición.
 * - El error se avisa sólo cuando todavía no hay datos: un refresco en
 *   segundo plano que falla no molesta con un aviso cada vez.
 *
 * `recargar()` devuelve una promesa que se cumple cuando los datos ya se
 * volvieron a pedir (para `await recargar()` después de una acción).
 */
export function useLista<T>(
  clave: readonly unknown[],
  consulta: () => Promise<T>,
  { intervaloMs, tablas }: { intervaloMs: number; tablas: string },
) {
  const cliente = useQueryClient();
  // `clave` llega como literal nuevo en cada render; el texto mantiene estable
  // a `recargar` (varias pantallas la usan como dependencia de un useCallback).
  const claveTexto = JSON.stringify(clave);
  const { data, error, isLoading } = useQuery({ queryKey: clave, queryFn: consulta });

  const recargar = useCallback(
    () => cliente.invalidateQueries({ queryKey: JSON.parse(claveTexto) as unknown[] }),
    [cliente, claveTexto],
  );

  useAutoRefresh(() => void recargar(), intervaloMs, tablas);

  const sinDatos = data === undefined;
  useEffect(() => {
    if (error && sinDatos) toast.error(mensajeError(error));
  }, [error, sinDatos]);

  return { datos: data, cargando: isLoading, recargar };
}
